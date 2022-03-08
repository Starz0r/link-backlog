use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use {
    anyhow::{Error, Result},
    axum::{Router, Server},
};

pub(crate) struct Application {
    pub(crate) addr: SocketAddr,
    pub(crate) router: Router,
}

impl Application {
    pub fn new() -> Self {
        Self {
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3030),
            router: Router::new(),
        }
    }

    pub fn with_routes(routes: Router) -> Self {
        Self {
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3030),
            router: routes,
        }
    }

    pub async fn listen_and_serve(&mut self) -> Result<(), Error> {
        let routes = self.router.clone();
        Server::bind(&self.addr)
            .serve(routes.into_make_service())
            .await?;

        Ok(())
    }
}
