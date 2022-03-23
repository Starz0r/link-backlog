mod api;
mod app;
mod config;
mod pages;

use std::sync::Arc;

use {
    anyhow::{Context, Error, Result},
    axum::{extract::Extension, response::Html, response::IntoResponse, routing::get, Router},
    tera::Tera,
    tracing::{debug, info, Level},
    tracing_subscriber::FmtSubscriber,
};

use app::Application;
use config::Configuration;

pub fn main() -> Result<(), Error> {
    let logging = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(logging)?;

    info!("Preparing Application.");

    let cfg = Configuration::from_file("config.toml")
        .with_context(|| "Configuration could not be read.")?;
    debug!("{cfg:?}");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let mut app = Application::prepare(cfg).await?;
        app.listen_and_serve().await?;

        Ok(())
    })
}
