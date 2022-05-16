use std::{fs::read, path::Path};

use {
    anyhow::{Error, Result},
    reqwest::Url,
    serde::{Deserialize, Serialize},
};

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct OpenID {
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
    pub(crate) issuer: Url,
    pub(crate) redirect: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct Database {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: String,
    pub(crate) pass: String,
    pub(crate) database: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct Configuration {
    pub(crate) openid: OpenID,
    pub(crate) database: Database,
}

impl Configuration {
    pub(crate) fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        // QUEST: is it possible to try/catch this?
        Ok(toml::from_slice(&read(path)?).unwrap())
    }
}
