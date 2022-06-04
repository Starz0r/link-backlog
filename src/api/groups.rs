use std::sync::Arc;

use {
    anyhow::{Error, Result},
    axum::{
        extract::{Extension, Query},
        http::StatusCode,
        response::{IntoResponse, Json},
    },
    axum_auth::AuthBearer,
    sea_orm::{
        entity::{prelude::*, Set},
        DatabaseConnection, QueryOrder,
    },
    serde::{Deserialize, Serialize},
    tracing::error,
    ulid::Ulid,
};

use super::{
    error::{resp_err, ApiError},
    UserId,
};

use crate::{app::Sessions, database::entity::groups, identity};

pub type GroupId = Ulid;
#[derive(Deserialize, Serialize, Clone)]
pub struct Group {
    pub id: GroupId,
    pub name: String,
    pub description: Option<String>,
    pub created_by: UserId,
    pub date_created: DateTimeWithTimeZone,
    pub modified_at: Option<DateTimeWithTimeZone>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

impl Group {
    pub fn new(
        name: String,
        description: Option<String>,
        created_at: DateTimeWithTimeZone,
        owner: UserId,
    ) -> Self {
        Self {
            id: GroupId::new(),
            name,
            description,
            created_by: owner,
            date_created: created_at,
            modified_at: None,
            deleted_at: None,
        }
    }

    // converts the link into it's database entity representation
    pub fn into_der(mut self) -> groups::ActiveModel {
        groups::ActiveModel {
            id: Set(self.id.to_string()),
            name: Set(self.name),
            description: Set(self.description),
            created_by: Set(self.created_by),
            date_created: Set(self.date_created),
            modified_at: Set(self.modified_at),
            deleted_at: Set(self.deleted_at),
        }
    }

    pub fn from_inactive_der(der: groups::Model) -> Result<Self, Error> {
        Ok(Self {
            id: Ulid::from_string(&der.id)?,
            name: der.name,
            description: der.description,
            created_by: der.created_by,
            date_created: der.date_created,
            modified_at: der.modified_at,
            deleted_at: der.deleted_at,
        })
    }
}

#[derive(Deserialize)]
pub struct SubmitRequest {
    pub name: String,
    pub description: Option<String>,
    pub timestamptz: DateTimeWithTimeZone,
}

#[axum_macros::debug_handler]
pub async fn submit(
    Extension(sessions): Extension<Arc<Sessions>>,
    Extension(dbconn): Extension<Arc<DatabaseConnection>>,
    AuthBearer(auth_token): AuthBearer,
    Json(req): Json<SubmitRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiError<'static>>)> {
    // get api key context
    let apikey = match identity::valid_api_key(dbconn.clone(), auth_token).await {
        Some(apikey) => apikey,
        None => return Err(resp_err(StatusCode::UNAUTHORIZED, "api key is not valid")),
    };
    let user_id = apikey.created_by as UserId;

    // commit to database
    let group = Group::new(req.name, req.description, req.timestamptz, user_id);
    match groups::Entity::insert(group.clone().into_der())
        .exec(dbconn.as_ref())
        .await
    {
        Ok(_) => (),
        Err(e) => {
            error!("tried committing link to database: {e}");
            return Err(resp_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database was unreachable",
            ));
        }
    };

    // return the response
    Ok((StatusCode::CREATED, Json(group)))
}

#[derive(Deserialize)]
pub struct ListRequest {
    page: Option<usize>,
    groups_per_page: Option<usize>,
}

pub async fn list(
    Extension(sessions): Extension<Arc<Sessions>>,
    Extension(dbconn): Extension<Arc<DatabaseConnection>>,
    AuthBearer(auth_token): AuthBearer,
    Query(req): Query<ListRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiError<'static>>)> {
    // get api key context
    let apikey = match identity::valid_api_key(dbconn.clone(), auth_token).await {
        Some(apikey) => apikey,
        None => return Err(resp_err(StatusCode::UNAUTHORIZED, "api key is not valid")),
    };
    let user_id = apikey.created_by as UserId;

    let page = req.page.unwrap_or(1);
    let groups_per_page = req.groups_per_page.unwrap_or(50);
    let paginator = groups::Entity::find()
        .order_by_desc(groups::Column::DateCreated)
        .filter(groups::Column::CreatedBy.eq(user_id))
        .paginate(dbconn.as_ref(), groups_per_page);

    let groups = match paginator.fetch_page(page - 1).await {
        Ok(res) => res,
        Err(e) => {
            error!("fetching a page of groups from the database failed: {e}");
            return Err(resp_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "couldn't retrieve groups from database",
            ));
        }
    };

    // convert der's to a rust struct
    let mut converted_groups = Vec::new();
    for group in groups {
        converted_groups.push(match Group::from_inactive_der(group) {
            Ok(mut converted_group) => {
                converted_group.created_by.clear();
                converted_group
            }
            Err(e) => {
                error!("der group couldn't be casted into rust repr group: {e}");
                return Err(resp_err(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "groups in database couldn't be processed",
                ));
            }
        })
    }

    Ok((StatusCode::OK, Json(converted_groups)))
}
