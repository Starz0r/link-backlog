#![feature(int_roundings)]

mod api;
mod app;
mod config;
mod database;
mod identity;
mod pages;

use std::{str::FromStr, sync::Arc};

use {
    anyhow::{bail, Context, Error, Result},
    axum::{extract::Extension, response::Html, response::IntoResponse, routing::get, Router},
    tera::Tera,
    tracing::{debug, error, info, Level},
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

    match cfg.tracing {
        Some(t) => {
            let lvl = match Level::from_str(t.level.as_str()) {
                Ok(lvl) => lvl,
                Err(e) => {
                    bail!("Tracing level is malformed and could not be parsed: {e}");
                }
            };
            let fmt_sub = FmtSubscriber::builder().with_max_level(lvl).finish();
            tracing::subscriber::set_global_default(fmt_sub)?;
        }
        _ => {}
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let mut app = Application::prepare(cfg).await?;
        app.listen_and_serve().await?;

        Ok(())
    })
}
