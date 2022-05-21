use std::sync::Arc;

use {
    anyhow::{Error, Result},
    axum::{
        extract::{Extension, Query},
        response::Html,
    },
    sea_orm::{entity::prelude::*, DatabaseConnection, QueryOrder},
    serde::Deserialize,
    tera::{Context, Tera},
    tower_cookies::Cookies,
    tracing::{error, info},
};

use crate::{
    api::{links::Link, UserId},
    app::{Sessions, User},
    database::entity::links,
    identity,
};

#[derive(Deserialize)]
pub struct IndexParameters {
    page: Option<usize>,
    links_per_page: Option<usize>,
}

pub async fn index(
    Extension(tmpl): Extension<Arc<Tera>>,
    cookies: Cookies,
    Extension(sessions): Extension<Arc<Sessions>>,
    Extension(dbconn): Extension<Arc<DatabaseConnection>>,
    Query(req): Query<IndexParameters>,
) -> Html<String> {
    let mut ctx = Context::new();

    let user = match cookies.get("sess") {
        Some(c) => match identity::user_from_session(sessions, c.value().to_string()) {
            Some(u) => {
                ctx.insert("user", &u);
                u
            }
            None => return Html(tmpl.render("index.html.tera", &ctx).unwrap()),
        },
        None => return Html(tmpl.render("index.html.tera", &ctx).unwrap()),
    };
    let user_id: UserId = user.id;

    let page = req.page.unwrap_or(1);
    let links_per_page = req.links_per_page.unwrap_or(50);
    let paginator = links::Entity::find()
        .order_by_asc(links::Column::DateCreated)
        .filter(links::Column::CreatedBy.eq(user_id))
        .paginate(dbconn.as_ref(), links_per_page);

    let links = match paginator.fetch_page(page - 1).await {
        Ok(ok) => ok,
        Err(e) => {
            error!("fetching a page of links from the database failed: {e}");
            ctx.insert("error", "Database did not return any links.");
            return Html(tmpl.render("index.html.tera", &ctx).unwrap());
        }
    };

    // convert der's to a rust struct
    let mut converted_links = Vec::new();
    for link in links {
        converted_links.push(match Link::from_inactive_der(link) {
            Ok(mut ok) => {
                ok.created_by.clear();
                ok
            }
            Err(e) => {
                error!("der link couldn't be casted into rust repr link: {e}");
                ctx.insert(
                    "error",
                    "Links retrieved from database were corrupted and could not be displayed.",
                );
                return Html(tmpl.render("index.html.tera", &ctx).unwrap());
            }
        })
    }
    ctx.insert("links", &converted_links);
    ctx.insert("current_page", &page);

    Html(tmpl.render("index.html.tera", &ctx).unwrap())
}
