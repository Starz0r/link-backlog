use std::sync::Arc;

use {
    sea_orm::{entity::prelude::*, DatabaseConnection},
    tracing::warn,
};

use crate::{
    app::{Sessions, User},
    database::entity::api_keys,
    pages::apikeys::APIKey,
};

pub fn user_from_session(sessions: Arc<Sessions>, token: String) -> Option<User> {
    let entry = match sessions.get(&token) {
        Some(e) => e,
        None => return None,
    };

    let (_, session) = entry.pair();
    Some(session.user.clone())
}

pub async fn valid_api_key(dbconn: Arc<DatabaseConnection>, key: String) -> Option<APIKey> {
    // TODO: tokio::select between a hashmap cache and the current database pull
    match api_keys::Entity::find()
        .filter(api_keys::Column::Key.eq(key))
        .one(dbconn.as_ref())
        .await
    {
        // TODO: verify key is valid by checking the deleted_at timestamp
        Ok(maybe_apikey) => match maybe_apikey {
            Some(apikey_der) => match APIKey::from_inactive_der(apikey_der) {
                Ok(apikey) => Some(apikey),
                Err(e) => {
                    warn!("api key from database could not be converted into rust repr: {e}");
                    None
                }
            },
            None => None,
        },
        Err(e) => {
            warn!("database search for api key failed: {e}");
            None
        }
    }
}
