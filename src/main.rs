mod app;

use {
    anyhow::{Error, Result},
    axum::{response::IntoResponse, routing::get, Router},
};

use app::Application;

fn main() -> Result<(), Error> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let mut app = Application::with_routes(Router::new().route("/", get(index)));
        app.listen_and_serve().await?;

        Ok(())
    })
}

async fn index() -> impl IntoResponse {
    "Hello, world!\n"
}
