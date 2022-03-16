use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use {
    anyhow::{Error, Result},
    axum::{extract::Extension, routing::get, Router, Server},
    dashmap::DashMap,
    openid::Userinfo,
    reqwest::Url,
    serde::{Deserialize, Serialize},
    tera::Tera,
    tracing::{debug, info},
};

type OpenIDClient = openid::Client<openid::Discovered, openid::StandardClaims>;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct User {
    id: String,
    login: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
    image_url: Option<String>,
    activated: bool,
    lang_key: Option<String>,
    authorities: Vec<String>,
}

struct Session {
    user: User,
    token: openid::Token,
    userinfo: Userinfo,
}

type Sessions = DashMap<String, Session>;

pub(crate) struct Application {
    cfg: super::config::Configuration,
    addr: SocketAddr,
    router: Router,
    templates: Arc<Tera>,
    openid: Arc<OpenIDClient>,
    sessions: Arc<Sessions>,
}

impl Application {
    pub async fn prepare(config: super::config::Configuration) -> Result<Self, Error> {
        let router = Router::new().route("/", get(super::pages::index));
        let tera = Arc::new(Tera::new("src/pages/templates/**/*.html.tera")?);
        let openid_client = Arc::new(
            openid::DiscoveredClient::discover(
                config.openid.client_id.clone(),
                config.openid.client_secret.clone(),
                Some(config.openid.redirect.clone()),
                config.openid.issuer.clone(),
            )
            .await?,
        );
        debug!("OpenID Config: {:?}", openid_client.config());
        let sessions = Arc::new(DashMap::new() as Sessions);
        Ok(Self {
            cfg: config,
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3030),
            router,
            templates: tera,
            openid: openid_client,
            sessions,
        })
    }

    pub fn enable_templates(&mut self) {
        // QUEST: Can we do this without cloning?
        let routes = self.router.clone();
        self.router = routes.layer(Extension(self.templates.clone()));
    }

    pub async fn listen_and_serve(&mut self) -> Result<(), Error> {
        // QUEST: Can we do this without cloning?
        let routes = self.router.clone();
        Server::bind(&self.addr)
            .serve(routes.into_make_service())
            .await?;

        Ok(())
    }
}
