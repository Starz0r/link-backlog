mod app;

use std::sync::Arc;

use {
    anyhow::{Error, Result},
    axum::{extract::Extension, response::Html, response::IntoResponse, routing::get, Router},
    tera::Tera,
};

use app::Application;

pub fn main() -> Result<(), Error> {
    // TODO: Config stuff to go here
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let mut app = Application::with_routes(Router::new().route("/", get(index)))?;
        app.enable_templates();
        app.listen_and_serve().await?;

        Ok(())
    })
}

pub async fn index(Extension(tmpl): Extension<Arc<Tera>>) -> axum::response::Html<String> {
    Html(tmpl.render("index.html", &tera::Context::new()).unwrap())
}
