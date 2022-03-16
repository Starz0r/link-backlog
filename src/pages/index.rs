use std::sync::Arc;

use {
    anyhow::{Error, Result},
    axum::{extract::Extension, response::Html},
    tera::Tera,
};

pub async fn index(Extension(tmpl): Extension<Arc<Tera>>) -> Html<String> {
    Html(
        tmpl.render("index.html.tera", &tera::Context::new())
            .unwrap(),
    )
}
