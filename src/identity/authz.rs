use std::sync::Arc;

use crate::app::{Sessions, User};

pub fn user_from_session(sessions: Arc<Sessions>, token: String) -> Option<User> {
    let entry = match sessions.get(&token) {
        Some(e) => e,
        None => return None,
    };

    let (_, session) = entry.pair();
    Some(session.user.clone())
}
