use std::{fs::read, path::Path};

use {
    anyhow::{Error, Result},
    reqwest::Url,
    serde::{Deserialize, Serialize},
};

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct Configuration {
    pub(crate) openid: OpenID,
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct OpenID {
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
    pub(crate) issuer: Url,
    pub(crate) redirect: String,
}

impl Configuration {
    pub(crate) fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        // QUEST: is it possible to try/catch this?
        Ok(toml::from_slice(&read(path)?).unwrap())
    }
}
