// SPDX-FileCopyrightText: © Matteo Settenvini <matteo.settenvini@montecristosoftware.eu>
// SPDX-License-Identifier: EUPL-1.2

use {anyhow::anyhow, rocket::serde::Deserialize, serde::de::Error};

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Settings {
    #[serde(deserialize_with = "deserialize_s3_bucket")]
    pub s3_bucket: s3::Bucket,
}

fn deserialize_s3_bucket<'de, D>(deserializer: D) -> Result<s3::Bucket, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let config = S3Config::deserialize(deserializer)?;
    config.try_into().map_err(D::Error::custom)
}

#[derive(Deserialize)]
pub struct S3Config {
    pub name: String,
    pub endpoint: String,
    pub region: String,

    #[serde(default)]
    pub path_style: bool,

    pub access_key_id: String,
    pub secret_access_key: String,
}

impl TryInto<s3::Bucket> for S3Config {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<s3::Bucket, Self::Error> {
        let region = s3::Region::Custom {
            region: self.region,
            endpoint: self.endpoint,
        };

        let credentials = s3::creds::Credentials::new(
            Some(&self.access_key_id),
            Some(&self.secret_access_key),
            None,
            None,
            None,
        )?;

        log::info!(
            "Serving contents from bucket {} at {}",
            &self.name,
            region.endpoint()
        );

        let bucket = s3::Bucket::new(&self.name, region, credentials).map_err(|e| anyhow!(e));
        if self.path_style {
            bucket.map(|mut b| {
                b.set_path_style();
                b
            })
        } else {
            bucket
        }
    }
}
