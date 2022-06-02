use std::sync::{Arc, Mutex};

use {
    anyhow::{Error, Result},
    axum::{
        extract::{Extension, Multipart, Query},
        response::{Html, IntoResponse, Redirect},
    },
    rand::{distributions::Alphanumeric, Rng},
    rand_pcg::Pcg64Mcg,
    sea_orm::{
        entity::{prelude::*, Set},
        DatabaseConnection, QueryOrder,
    },
    serde::{Deserialize, Serialize},
    tera::{Context, Tera},
    tower_cookies::Cookies,
    tracing::{error, warn},
    ulid::Ulid,
};

use crate::{api::UserId, app::Sessions, database::entity::api_keys, identity};

pub type APIKeyId = Ulid;

#[derive(Deserialize, Serialize, Clone)]
pub struct APIKey {
    pub id: APIKeyId,
    pub created_by: UserId,
    pub key: String,
    pub date_created: DateTimeWithTimeZone,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

impl APIKey {
    pub fn new(user: UserId, key: String, date: DateTimeWithTimeZone) -> Self {
        // TODO: generate the actual key here
        Self {
            id: Ulid::new(),
            created_by: user,
            key,
            date_created: date,
            deleted_at: None,
        }
    }

    pub fn into_der(mut self) -> api_keys::ActiveModel {
        api_keys::ActiveModel {
            id: Set(self.id.to_string()),
            created_by: Set(self.created_by),
            key: Set(self.key),
            date_created: Set(self.date_created),
            deleted_at: Set(self.deleted_at),
        }
    }

    pub fn from_inactive_der(der: api_keys::Model) -> Result<Self, Error> {
        Ok(Self {
            id: Ulid::from_string(&der.id)?,
            created_by: der.created_by,
            key: der.key,
            date_created: der.date_created,
            deleted_at: der.deleted_at,
        })
    }
}

pub async fn page(
    Extension(tmpl): Extension<Arc<Tera>>,
    cookies: Cookies,
    Extension(sessions): Extension<Arc<Sessions>>,
    Extension(dbconn): Extension<Arc<DatabaseConnection>>,
) -> Html<String> {
    let mut ctx = Context::new();
    let user = match cookies.get("sess") {
        Some(c) => match identity::user_from_session(sessions, c.value().to_string()) {
            Some(u) => {
                ctx.insert("user", &u);
                u
            }
            None => return Html(tmpl.render("apikeys.html.tera", &ctx).unwrap()),
        },
        None => return Html(tmpl.render("apikeys.html.tera", &ctx).unwrap()),
    };
    let user_id: UserId = user.id;

    let page = 1;
    let apikeys_per_page = 25;
    let paginator = api_keys::Entity::find()
        .order_by_desc(api_keys::Column::DateCreated)
        .filter(api_keys::Column::CreatedBy.eq(user_id.clone()))
        .paginate(dbconn.as_ref(), apikeys_per_page);
    match api_keys::Entity::find()
        .filter(api_keys::Column::CreatedBy.eq(user_id))
        .count(dbconn.as_ref())
        .await
    {
        Ok(total_apikeys) => ctx.insert("pages", &total_apikeys.div_ceil(apikeys_per_page)),
        Err(e) => warn!("amount of pages couldn't be counted: {e}"),
    }

    let apikeys = match paginator.fetch_page(page - 1).await {
        Ok(ok) => ok,
        Err(e) => {
            error!("fetching a page of apikeys from the database failed: {e}");
            ctx.insert("error", "Database did not return any apikeys.");
            return Html(tmpl.render("apikeys.html.tera", &ctx).unwrap());
        }
    };

    // convert der's to a rust struct
    let mut converted_apikeys = Vec::new();
    for apikey in apikeys {
        converted_apikeys.push(match APIKey::from_inactive_der(apikey) {
            Ok(mut ok) => {
                ok.created_by.clear();
                ok
            }
            Err(e) => {
                error!("der apikey couldn't be casted into rust repr apikey: {e}");
                ctx.insert(
                    "error",
                    "API keys retrieved from database were corrupted and could not be displayed.",
                );
                return Html(tmpl.render("apikeys.html.tera", &ctx).unwrap());
            }
        })
    }
    ctx.insert("keys", &converted_apikeys);
    ctx.insert("current_page", &page);

    Html(tmpl.render("apikeys.html.tera", &ctx).unwrap())
}

fn rand_alphanumeric_string(rng: Arc<Mutex<Pcg64Mcg>>, amt: usize) -> String {
    // QUEST: why do I have to do this???
    let c_rng = rng.clone();
    let c_rng = c_rng.as_ref().lock().unwrap();

    c_rng
        .clone() // QUEST: or this????
        .sample_iter(&Alphanumeric)
        .take(amt)
        .map(char::from)
        .collect::<String>()
}

pub async fn create(
    Extension(tmpl): Extension<Arc<Tera>>,
    cookies: Cookies,
    Extension(sessions): Extension<Arc<Sessions>>,
    Extension(dbconn): Extension<Arc<DatabaseConnection>>,
    Extension(rng): Extension<Arc<Mutex<Pcg64Mcg>>>,
    mut req: Multipart,
) -> impl IntoResponse {
    let user = match cookies.get("sess") {
        Some(c) => match identity::user_from_session(sessions, c.value().to_string()) {
            Some(u) => u,
            // TODO: set a flash cookie along with error context
            None => return Err(Redirect::to("/apikeys".parse().unwrap())),
        },
        None => return Err(Redirect::to("/apikeys".parse().unwrap())),
    };

    let date: DateTimeWithTimeZone = {
        if let Some(mut field) = req.next_field().await.unwrap() {
            let name = field.name().unwrap().to_string();
            let data = field.bytes().await.unwrap();
            DateTimeWithTimeZone::parse_from_rfc3339(std::str::from_utf8(data.as_ref()).unwrap())
                .unwrap()
        } else {
            return Err(Redirect::to("/apikeys".parse().unwrap()));
        }
    };

    let key = rand_alphanumeric_string(rng, 32);

    let apikey = APIKey::new(user.id, key, date);
    match api_keys::Entity::insert(apikey.clone().into_der())
        .exec(dbconn.as_ref())
        .await
    {
        Ok(res) => res,
        Err(e) => {
            error!("tried committing new apikey to database: {e}");
            // TODO: set a flash cookie along with error context
            return Err(Redirect::to("/apikeys".parse().unwrap()));
        }
    };

    // return the response
    Ok(Redirect::to("/apikeys".parse().unwrap()))
}
