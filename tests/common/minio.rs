// SPDX-FileCopyrightText: © Matteo Settenvini <matteo.settenvini@montecristosoftware.eu>
// SPDX-License-Identifier: EUPL-1.2

use {testcontainers::core::WaitFor, testcontainers::Image, testcontainers_modules::minio};

const MINIO_IMAGE_TAG: &'static str = "RELEASE.2024-05-28T17-19-04Z";

pub struct MinIO {
    inner: minio::MinIO,
}

impl Image for MinIO {
    type Args = minio::MinIOServerArgs;

    fn name(&self) -> String {
        self.inner.name()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("API:")]
    }

    fn tag(&self) -> String {
        MINIO_IMAGE_TAG.into()
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        self.inner.env_vars()
    }
}

impl Default for MinIO {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}
