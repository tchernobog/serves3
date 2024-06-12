// SPDX-FileCopyrightText: © Matteo Settenvini <matteo.settenvini@montecristosoftware.eu>
// SPDX-License-Identifier: EUPL-1.2

mod minio;

use {
    anyhow::{anyhow, Result},
    reqwest::Url,
    std::{ptr::null_mut, str::FromStr},
    testcontainers::{runners::AsyncRunner, ContainerAsync},
    tokio::io::AsyncBufReadExt as _,
};

pub struct Test {
    pub base_url: Url,
    pub bucket: s3::Bucket,
    pub serves3: tokio::process::Child,
    pub minio: ContainerAsync<minio::MinIO>,
}

const MAXIMUM_SERVES3_INIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

const BUCKET_NAME: &'static str = "integration-test-bucket";
const REGION: &'static str = "test-region";
const ACCESS_KEY: &'static str = "minioadmin";
const SECRET_KEY: &'static str = "minioadmin";

impl Test {
    pub async fn new() -> Result<Self> {
        // NOTE: right now there is a bug in bollard
        // that makes testcontainers work in Docker only and
        // not podman (it is not able to fetch exposed ports).
        // If this test fails make sure you are using docker.
        std::env::remove_var("DOCKER_HOST");

        let image = minio::MinIO::default();
        let container = image.start().await?;

        let endpoint = format!(
            "http://{host}:{port}",
            host = container.get_host().await?,
            port = container.get_host_port_ipv4(9000).await?
        );

        let credentials = s3::creds::Credentials::new(
            Some(&ACCESS_KEY),
            Some(&SECRET_KEY),
            None,
            None,
            Some("test"),
        )?;
        let bucket = s3::Bucket::create_with_path_style(
            &BUCKET_NAME,
            s3::Region::Custom {
                region: REGION.into(),
                endpoint: endpoint.clone(),
            },
            credentials,
            s3::BucketConfiguration::private(),
        )
        .await?
        .bucket;

        let bin = std::env!("CARGO_BIN_EXE_serves3");
        let mut child = tokio::process::Command::new(bin)
            .env("SERVES3_ADDRESS", "127.0.0.1")
            .env("SERVES3_PORT", "0")
            .env("SERVES3_LOG_LEVEL", "debug")
            .env(
                "SERVES3_S3_BUCKET",
                format!(
                    r#"{{
                    name = "{name}",
                    endpoint = "{endpoint}",
                    region = "{region}",
                    access_key_id = "{user}",
                    secret_access_key = "{secret}",
                    path_style = true
                }}"#,
                    name = BUCKET_NAME,
                    endpoint = endpoint,
                    region = &REGION,
                    user = ACCESS_KEY,
                    secret = SECRET_KEY
                ),
            )
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let base_url = tokio::time::timeout(MAXIMUM_SERVES3_INIT_TIMEOUT, async {
            let stdout = child.stdout.as_mut().unwrap();
            let mut lines = tokio::io::BufReader::new(stdout).lines();
            let re = regex::Regex::new("^Rocket has launched from (http://.+)$").unwrap();
            while let Some(line) = lines.next_line().await? {
                println!("{}", &line);
                if let Some(captures) = re.captures(&line) {
                    let url = captures.get(1).unwrap().as_str();
                    return Ok(Url::from_str(url)?);
                }
            }

            Err(anyhow!("Rocket did not print that it has started"))
        })
        .await??;

        Ok(Self {
            base_url,
            bucket,
            serves3: child,
            minio: container,
        })
    }
}

impl Drop for Test {
    fn drop(&mut self) {
        unsafe {
            let pid = self.serves3.id().unwrap() as i32;
            libc::kill(pid, libc::SIGTERM);
            libc::waitpid(pid, null_mut(), 0);
        }
    }
}
