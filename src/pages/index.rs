use std::sync::Arc;

use {
    anyhow::{Error, Result},
    axum::{extract::Extension, response::Html},
    tera::{Context, Tera},
    tower_cookies::Cookies,
};

use crate::app::Sessions;

pub async fn index(
    Extension(tmpl): Extension<Arc<Tera>>,
    cookies: Cookies,
    Extension(sessions): Extension<Arc<Sessions>>,
) -> Html<String> {
    let mut ctx = Context::new();

    match cookies.get("sess") {
        Some(c) => match sessions.get(c.value()) {
            Some(v) => {
                ctx.insert("user", &(*v).user);
            }
            None => (),
        },
        None => (),
    };

    Html(tmpl.render("index.html.tera", &ctx).unwrap())
}
