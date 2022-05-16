use std::sync::Arc;

use {
    anyhow::{Error, Result},
    axum::{
        extract::{Extension, Query},
        http::StatusCode,
        response::{IntoResponse, Json, Redirect},
    },
    axum_auth::AuthBearer,
    reqwest::Url,
    sea_orm::{
        entity::{prelude::*, Set},
        DatabaseConnection, QueryOrder,
    },
    serde::{Deserialize, Serialize},
    tower_cookies::{Cookie, Cookies},
    tracing::{debug, error, info, warn},
    ulid::Ulid,
};

use crate::{app::Sessions, database::entity::links, identity};

use super::{
    error::{resp_err, ApiError},
    UserId,
};

pub type LinkId = Ulid;

#[derive(Deserialize, Serialize, Clone)]
pub struct Link {
    pub id: LinkId,
    pub url: Url,
    pub title: Option<String>,
    pub sensitive: bool,
    pub created_by: UserId,
    pub date_created: DateTimeWithTimeZone,
    pub modified_at: Option<DateTimeWithTimeZone>,
    pub archived_at: Option<DateTimeWithTimeZone>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

impl Link {
    pub fn new(
        url: Url,
        created_at: DateTimeWithTimeZone,
        title: Option<String>,
        sensitive: bool,
        owner: UserId,
    ) -> Self {
        Self {
            id: LinkId::new(),
            url,
            title,
            sensitive,
            created_by: owner,
            date_created: created_at,
            modified_at: None,
            archived_at: None,
            deleted_at: None,
        }
    }

    // converts the link into it's database entity representation
    pub fn into_der(mut self) -> links::ActiveModel {
        links::ActiveModel {
            id: Set(self.id.to_string()),
            url: Set(self.url.into()),
            title: Set(self.title),
            sensitive: Set(self.sensitive),
            created_by: Set(self.created_by),
            date_created: Set(self.date_created),
            modified_at: Set(self.modified_at),
            archived_at: Set(self.archived_at),
            deleted_at: Set(self.deleted_at),
        }
    }

    pub fn from_der(der: links::ActiveModel) -> Result<Self, Error> {
        Ok(Self {
            id: Ulid::from_string(&der.id.unwrap())?,
            url: der.url.unwrap().parse()?,
            title: der.title.unwrap(),
            sensitive: der.sensitive.unwrap(),
            created_by: der.created_by.unwrap(),
            date_created: der.date_created.unwrap(),
            modified_at: der.modified_at.unwrap(),
            archived_at: der.archived_at.unwrap(),
            deleted_at: der.deleted_at.unwrap(),
        })
    }

    pub fn from_inactive_der(der: links::Model) -> Result<Self, Error> {
        Ok(Self {
            id: Ulid::from_string(&der.id)?,
            url: der.url.parse()?,
            title: der.title,
            sensitive: der.sensitive,
            created_by: der.created_by,
            date_created: der.date_created,
            modified_at: der.modified_at,
            archived_at: der.archived_at,
            deleted_at: der.deleted_at,
        })
    }
}

#[derive(Deserialize)]
pub struct SubmitRequest {
    link: String,
    submitted_at: DateTimeWithTimeZone,
    title: Option<String>,
    sensitive: Option<bool>,
}

#[derive(Serialize)]
pub struct SubmitResponse {
    pub id: LinkId,
    pub url: Url,
    pub title: Option<String>,
    pub sensitive: bool,
    pub created_by: UserId,
    pub date_created: DateTimeWithTimeZone,
}

pub async fn submit(
    Extension(sessions): Extension<Arc<Sessions>>,
    Extension(dbconn): Extension<Arc<DatabaseConnection>>,
    AuthBearer(auth_token): AuthBearer,
    Json(req): Json<SubmitRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiError<'static>>)> {
    // get user
    let user = match identity::user_from_session(sessions, auth_token) {
        Some(u) => u,
        None => return Err(resp_err(StatusCode::UNAUTHORIZED, "user is not signed in")),
    };

    // try parsing url
    let url = match req.link.parse() {
        Ok(ok) => ok,
        Err(e) => {
            return Err(resp_err(
                StatusCode::UNPROCESSABLE_ENTITY,
                "not a valid url",
            ))
        }
    };

    // commit to database
    let link = Link::new(
        url,
        req.submitted_at,
        req.title,
        req.sensitive.unwrap_or_default(),
        user.id as UserId,
    );
    match links::Entity::insert(link.clone().into_der())
        .exec(dbconn.as_ref())
        .await
    {
        Ok(l) => l,
        Err(e) => {
            error!("tried committing link to database: {e}");
            return Err(resp_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database was unreachable",
            ));
        }
    };

    // return the response
    Ok((StatusCode::CREATED, Json(link)))
}

#[derive(Deserialize)]
pub struct ListRequest {
    page: Option<usize>,
    links_per_page: Option<usize>,
}

pub async fn list(
    Extension(sessions): Extension<Arc<Sessions>>,
    Extension(dbconn): Extension<Arc<DatabaseConnection>>,
    AuthBearer(auth_token): AuthBearer,
    Query(req): Query<ListRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiError<'static>>)> {
    // get user
    let user = match identity::user_from_session(sessions, auth_token) {
        Some(u) => u,
        None => return Err(resp_err(StatusCode::UNAUTHORIZED, "user is not signed in")),
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
            return Err(resp_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "couldn't retrieve links from database",
            ));
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
                return Err(resp_err(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "link in database couldn't be processed",
                ));
            }
        })
    }

    Ok((StatusCode::OK, Json(converted_links)))
}
