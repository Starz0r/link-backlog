use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use {
    anyhow::{Error, Result},
    axum::{extract::Extension, routing::get, Router, Server},
    tera::Tera,
};

pub(crate) struct Application {
    pub(crate) addr: SocketAddr,
    pub(crate) router: Router,
    pub(crate) templates: Arc<Tera>,
}

impl Application {
    pub fn new() -> Result<Self, Error> {
        let tera = Arc::new(Tera::new("src/pages/templates/**/*.html")?);
        Ok(Self {
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3030),
            router: Router::new(),
            templates: tera,
        })
    }

    pub fn with_routes(routes: Router) -> Result<Self, Error> {
        let tera = Arc::new(Tera::new("src/pages/templates/**/*.html")?);
        Ok(Self {
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3030),
            router: routes,
            templates: tera,
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
