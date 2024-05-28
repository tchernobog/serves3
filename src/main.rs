// SPDX-FileCopyrightText: © Matteo Settenvini <matteo.settenvini@montecristosoftware.eu>
// SPDX-License-Identifier: EUPL-1.2

use {
    lazy_static::lazy_static,
    rocket::response::Responder,
    rocket::serde::Serialize,
    rocket_dyn_templates::{context, Template},
    std::path::PathBuf,
};

struct Settings {
    access_key_id: String,
    secret_access_key: String,
    bucket_name: String,
    endpoint: String,
    region: String,
}

lazy_static! {
    static ref SETTINGS: Settings = {
        let settings = config::Config::builder()
            .add_source(config::File::with_name("Settings.toml"))
            .add_source(config::Environment::with_prefix("SERVES3"))
            .build()
            .unwrap();

        Settings {
            access_key_id: settings
                .get_string("access_key_id")
                .expect("Missing configuration key access_key_id"),
            secret_access_key: settings
                .get_string("secret_access_key")
                .expect("Missing configuration key secret_access_key"),
            bucket_name: settings
                .get_string("bucket")
                .expect("Missing configuration key bucket"),
            region: settings
                .get_string("region")
                .expect("Missing configuration key region"),
            endpoint: settings
                .get_string("endpoint")
                .expect("Missing configuration key endpoint"),
        }
    };
    static ref BUCKET: s3::bucket::Bucket = {
        let region = s3::Region::Custom {
            region: SETTINGS.region.clone(),
            endpoint: SETTINGS.endpoint.clone(),
        };

        let credentials = s3::creds::Credentials::new(
            Some(&SETTINGS.access_key_id),
            Some(&SETTINGS.secret_access_key),
            None,
            None,
            None,
        )
        .expect("Wrong server S3 configuration");
        s3::bucket::Bucket::new(&SETTINGS.bucket_name, region, credentials)
            .expect("Cannot find or authenticate to S3 bucket")
    };
    static ref FILEVIEW_TEMPLATE: &'static str = std::include_str!("../templates/index.html.tera");

    // Workaround for https://github.com/SergioBenitez/Rocket/issues/1792
    static ref EMPTY_DIR: tempfile::TempDir = tempfile::tempdir()
        .expect("Unable to create an empty temporary folder, is the whole FS read-only?");
}

#[derive(Responder)]
enum FileView {
    #[response(content_type = "text/html")]
    Folder(Template),

    #[response(content_type = "application/octet-stream")]
    File(Vec<u8>),
}

#[derive(Serialize)]
struct FileViewItem {
    path: String,
    size: String,
    size_bytes: u64,
    last_modification: String,
}

#[derive(Responder, Debug)]
enum Error {
    #[response(status = 404)]
    NotFound(String),

    #[response(status = 500)]
    UnknownError(String),
}

#[rocket::get("/<path..>")]
async fn index(path: PathBuf) -> Result<FileView, Error> {
    /*
       The way things work in S3, the following holds for us:
       - we need to use a slash as separator
       - folders need to be queried ending with a slash
       - getting the bucket address (empty prefix) will
         return an XML file with all properties; we don't
         want that.

       We try first to retrieve list an object as a file. If we fail,
       we fallback to retrieving the equivalent folder.
    */

    if let Ok(result) = s3_serve_file(&path).await {
        Ok(result)
    } else {
        let objects = s3_fileview(&path).await?;
        let rendered = Template::render(
            "index",
            context! {
                path: format!("{}/", path.display()),
                objects
            },
        );
        Ok(FileView::Folder(rendered))
    }
}

async fn s3_serve_file(path: &PathBuf) -> Result<FileView, Error> {
    let is_root_prefix = path.as_os_str().is_empty();
    if is_root_prefix {
        return Err(Error::NotFound("Root prefix is not a file".into()));
    }

    // FIXME: this can be big, we should use streaming,
    // not loading in memory!
    let response = BUCKET
        .get_object(format!("{}", path.display()))
        .await
        .map_err(|_| Error::UnknownError("Unable to connect to S3 bucket".into()))?;

    match response.status_code() {
        200 | 204 => {
            let bytes = response.bytes().to_vec();
            Ok(FileView::File(bytes))
        }
        404 => Err(Error::NotFound("Object not found".into())),
        _ => Err(Error::UnknownError("Unknown S3 error".into())),
    }
}

async fn s3_fileview(path: &PathBuf) -> Result<Vec<FileViewItem>, Error> {
    /*
        if listing a folder:
        - folders will be under 'common_prefixes'
        - files will be under the 'contents' property
    */

    let parent = path.parent();
    let s3_folder_path = match parent {
        Some(_) => format!("{}/", path.display()),
        None => "".into(),
    };

    let s3_objects = BUCKET
        .list(s3_folder_path.clone(), Some("/".into()))
        .await
        .map_err(|_| Error::NotFound("Object not found".into()))?;

    let objects = s3_objects
        .iter()
        .flat_map(|list| -> Vec<Option<FileViewItem>> {
            let prefix = if let Some(p) = &list.prefix {
                p.as_str()
            } else {
                ""
            };

            let folders = list.common_prefixes.iter().flatten().map(|dir| {
                let path = dir.prefix.strip_prefix(&prefix);
                path.map(|path| FileViewItem {
                    path: path.to_owned(),
                    size_bytes: 0,
                    size: "[DIR]".to_owned(),
                    last_modification: String::default(),
                })
            });

            let files = list.contents.iter().map(|obj| {
                let path = obj.key.strip_prefix(&prefix);
                path.map(|path| FileViewItem {
                    path: path.to_owned(),
                    size_bytes: obj.size,
                    size: size_bytes_to_human(obj.size),
                    last_modification: obj.last_modified.clone(),
                })
            });

            folders.chain(files).collect()
        })
        .flatten()
        .collect();

    Ok(objects)
}

fn size_bytes_to_human(bytes: u64) -> String {
    use human_size::{Any, SpecificSize};

    let size: f64 = bytes as f64;
    let digits = size.log10().floor() as u32;
    let mut order = digits / 3;
    let unit = match order {
        0 => Any::Byte,
        1 => Any::Kilobyte,
        2 => Any::Megabyte,
        _ => {
            order = 3; // Let's stop here.
            Any::Gigabyte
        }
    };

    format!(
        "{:.3}",
        SpecificSize::new(size / 10u64.pow(order * 3) as f64, unit)
            .unwrap_or(SpecificSize::new(0., Any::Byte).unwrap())
    )
}

#[rocket::launch]
fn rocket() -> _ {
    eprintln!("Proxying to {} for {}", BUCKET.host(), BUCKET.name());

    let config_figment = rocket::Config::figment().merge(("template_dir", EMPTY_DIR.path())); // We compile the templates in anyway.

    rocket::custom(config_figment)
        .mount("/", rocket::routes![index])
        .attach(Template::custom(|engines| {
            engines
                .tera
                .add_raw_template("index", *FILEVIEW_TEMPLATE)
                .unwrap()
        }))
}

// Test section starts

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(1024, "1.024 kB")]
    #[case(10240, "10.240 kB")]
    #[case(1024*1024, "1.049 MB")]
    #[case(1024*1024*1024, "1.074 GB")]
    #[case(0, "0.000 B")]
    #[case(u64::MAX, format!("{:.3} GB",u64::MAX as f64/(1_000_000_000.0)))]
    #[case(u64::MIN, format!("{:.3} B",u64::MIN as f64))]

    fn test_size_bytes_to_human(#[case] bytes: u64, #[case] expected: String) {
        println!("{}", size_bytes_to_human(bytes));
        assert_eq!(size_bytes_to_human(bytes), expected);
    }
}
