// SPDX-FileCopyrightText: © Matteo Settenvini <matteo.settenvini@montecristosoftware.eu>
// SPDX-License-Identifier: EUPL-1.2

use {
    lazy_static::lazy_static,
    rocket::http::ContentType,
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
enum Error {
    #[response(status = 422)]
    InvalidArgument(String),

    #[response(status = 404)]
    NotFound(String),
}

#[rocket::get("/<path..>")]
async fn index(path: PathBuf) -> Result<(ContentType, Template), Error> {
    let path: String = path
        .to_str()
        .ok_or_else(|| Error::InvalidArgument(format!("Invalid path '{}'", path.display())))?
        .into();

    let s3_objects = BUCKET
        .list(path, Some("/".into()))
        .await
        .map_err(|_| Error::NotFound("Object not found".into()))?;

    let objects: Vec<String> = s3_objects
        .into_iter()
        .flat_map(|list| -> Vec<String> {
            if let Some(common_prefixes) = list.common_prefixes {
                common_prefixes.into_iter().map(|dir| dir.prefix).collect()
            } else {
                list.contents.into_iter().map(|obj| obj.key).collect()
            }
        })
        .collect();

    let rendered = Template::render(
        "index",
        context! {
            objects
        },
    );
    Ok((ContentType::HTML, rendered))
}

#[rocket::launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", rocket::routes![index])
        .attach(Template::fairing())
}
