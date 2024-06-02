// SPDX-FileCopyrightText: © Matteo Settenvini <matteo.settenvini@montecristosoftware.eu>
// SPDX-License-Identifier: EUPL-1.2

mod settings;
mod sizes;

use {
    anyhow::Result,
    lazy_static::lazy_static,
    rocket::{
        fairing::AdHoc,
        figment::{
            providers::{Env, Format as _, Toml},
            Profile,
        },
        response::Responder,
        serde::Serialize,
        State,
    },
    rocket_dyn_templates::{context, Template},
    settings::Settings,
    std::path::PathBuf,
};

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
async fn index(path: PathBuf, state: &State<Settings>) -> Result<FileView, Error> {
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

    if let Ok(result) = s3_serve_file(&path, &state).await {
        Ok(result)
    } else {
        let objects = s3_fileview(&path, &state).await?;
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

async fn s3_serve_file(path: &PathBuf, settings: &Settings) -> Result<FileView, Error> {
    let is_root_prefix = path.as_os_str().is_empty();
    if is_root_prefix {
        return Err(Error::NotFound("Root prefix is not a file".into()));
    }

    // FIXME: this can be big, we should use streaming,
    // not loading in memory!
    let response = settings
        .s3_bucket
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

async fn s3_fileview(path: &PathBuf, settings: &Settings) -> Result<Vec<FileViewItem>, Error> {
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

    let s3_objects = settings
        .s3_bucket
        .list(s3_folder_path, Some("/".into()))
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
                    size: sizes::bytes_to_human(obj.size),
                    last_modification: obj.last_modified.clone(),
                })
            });

            folders.chain(files).collect()
        })
        .flatten()
        .collect();

    Ok(objects)
}

lazy_static! {
    // Workaround for https://github.com/SergioBenitez/Rocket/issues/1792
    static ref EMPTY_DIR: tempfile::TempDir = tempfile::tempdir()
        .expect("Unable to create an empty temporary folder, is the whole FS read-only?");
}

#[rocket::launch]
fn rocket() -> _ {
    let config_figment = rocket::Config::figment()
        .merge(Toml::file("serves3.toml").nested())
        .merge(Env::prefixed("SERVES3_").global())
        .merge(("template_dir", EMPTY_DIR.path())) // We compile the templates in anyway
        .select(Profile::from_env_or("SERVES3_PROFILE", "default"));

    rocket::custom(config_figment)
        .mount("/", rocket::routes![index])
        .attach(AdHoc::config::<Settings>())
        .attach(Template::custom(|engines| {
            engines
                .tera
                .add_raw_template("index", std::include_str!("../templates/index.html.tera"))
                .unwrap()
        }))
}
