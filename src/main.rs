// SPDX-FileCopyrightText: © Matteo Settenvini <matteo.settenvini@montecristosoftware.eu>
// SPDX-License-Identifier: EUPL-1.2

use s3::serde_types::ListBucketResult;

use {
    lazy_static::lazy_static,
    rocket::response::Responder,
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
}

#[derive(Responder)]
enum FileView {
    #[response(content_type = "text/html")]
    Folder(Template),

    #[response(content_type = "application/octet-stream")]
    File(Vec<u8>),
}

#[derive(Responder, Debug)]
enum Error {
    #[response(status = 404)]
    NotFound(String),
}

#[rocket::get("/<path..>")]
async fn index(path: PathBuf) -> Result<FileView, Error> {
    let parent = path.parent();

    let s3_path = format!(
        "{}{}",
        path.display(),
        match parent {
            Some(_) => "/",
            None => "",
        }
    );

    /*
       The way things work in S3, the following holds for us:
       - we need to use a slash as separator
       - folders need to be queried ending with a slash

       We try first to retrieve list an object as a folder. If we fail,
       we fallback to retrieving the object itself.
    */
    let s3_objects = BUCKET.list(s3_path, Some("/".into())).await;

    let s3_objects = match s3_objects {
        Ok(s3_objects) => s3_objects,
        Err(_) => {
            // TODO: this can be big, we should use streaming,
            // not loading in memory.
            let data = BUCKET
                .get_object(format!("{}", path.display()))
                .await
                .map_err(|_| Error::NotFound("Object not found".into()))?
                .bytes()
                .to_vec();
            return Ok(FileView::File(data));
        }
    };

    let objects = s3_fileview(&s3_objects);
    let rendered = Template::render(
        "index",
        context! {
            path: format!("{}/", path.display()),
            has_parent: !path.as_os_str().is_empty(),
            objects
        },
    );
    Ok(FileView::Folder(rendered))
}

fn s3_fileview(s3_objects: &Vec<ListBucketResult>) -> Vec<&str> {
    /*
        if listing a folder:
        - folders will be under 'common_prefixes'
        - files will be under the 'contents' property
    */
    s3_objects
        .iter()
        .flat_map(|list| -> Vec<Option<&str>> {
            let prefix = if let Some(p) = &list.prefix {
                p.as_str()
            } else {
                ""
            };

            let folders = list
                .common_prefixes
                .iter()
                .flatten()
                .map(|dir| dir.prefix.strip_prefix(&prefix));
            let files = list
                .contents
                .iter()
                .map(|obj| obj.key.strip_prefix(&prefix));
            folders.chain(files).collect()
        })
        .flatten()
        .collect()
}

#[rocket::launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", rocket::routes![index])
        .attach(Template::fairing())
}
