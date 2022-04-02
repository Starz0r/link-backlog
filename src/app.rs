use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use {
    anyhow::{Error, Result},
    axum::{
        extract::Extension,
        routing::{get, get_service},
        Router, Server,
    },
    dashmap::DashMap,
    openid::Userinfo,
    reqwest::{StatusCode, Url},
    serde::{Deserialize, Serialize},
    tera::Tera,
    tower_cookies::CookieManagerLayer,
    tower_http::services::ServeDir,
    tracing::{info, trace},
};

pub type OpenIDClient = openid::Client<openid::Discovered, openid::StandardClaims>;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub login: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub image_url: Option<String>,
    pub activated: bool,
    pub lang_key: Option<String>,
    pub authorities: Vec<String>,
}

pub struct Session {
    pub user: User,
    pub token: openid::Token,
    pub userinfo: Userinfo,
}

pub type Sessions = DashMap<String, Session>;

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
        let tera = Arc::new(Tera::new("static/templates/**/*.html.tera")?);
        let openid_client = Arc::new(
            openid::DiscoveredClient::discover(
                config.openid.client_id.clone(),
                config.openid.client_secret.clone(),
                Some(config.openid.redirect.clone()),
                config.openid.issuer.clone(),
            )
            .await?,
        );
        trace!("OpenID Config: {:?}", openid_client.config());
        let sessions = Arc::new(DashMap::new() as Sessions);
        let apis = Router::new()
            .route("/auth/oauth2/code/oidc", get(super::api::authenticate))
            .route("/oauth2/login/oidc", get(super::api::login))
            .route("/oauth2/logout/oidc", get(super::api::logout));
        let router = Router::new()
            .nest("/api/v0", apis)
            .route("/", get(super::pages::index))
            .nest(
                "/static",
                get_service(ServeDir::new("static/")).handle_error(
                    |e: std::io::Error| async move {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("static filesystem error: {e}"),
                        )
                    },
                ),
            )
            .layer(CookieManagerLayer::new())
            .layer(Extension(tera.clone()))
            .layer(Extension(openid_client.clone()))
            .layer(Extension(sessions.clone()));
        Ok(Self {
            cfg: config,
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3030),
            router,
            templates: tera,
            openid: openid_client,
            sessions,
        })
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
