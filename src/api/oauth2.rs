use std::sync::Arc;

use {
    anyhow::Result,
    axum::{
        extract::{Extension, Query},
        http::StatusCode,
        response::{IntoResponse, Json, Redirect},
    },
    serde::Deserialize,
    tower_cookies::{Cookie, Cookies},
    tracing::{debug, error, info},
};

use super::error::{resp_err, ApiError};
use crate::app::{OpenIDClient, Session, Sessions, User};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub code: String,
    pub state: Option<String>,
}

async fn request_token(
    openid: Arc<OpenIDClient>,
    req: &LoginRequest,
) -> Result<Option<(openid::Token, openid::Userinfo)>> {
    let mut token: openid::Token = openid.request_token(&req.code).await?.into();

    if let Some(mut id_token) = token.id_token.as_mut() {
        openid.decode_token(&mut id_token)?;
        openid.validate_token(&id_token, None, None)?;
    } else {
        return Ok(None);
    };

    let userinfo = openid.request_userinfo(&token).await?;

    Ok(Some((token, userinfo)))
}

pub async fn authenticate(
    Extension(openid): Extension<Arc<OpenIDClient>>,
    Extension(sessions): Extension<Arc<Sessions>>,
    cookies: Cookies,
    Query(req): Query<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiError<'static>>)> {
    // TODO: check that the state matches
    let request_token = request_token(openid, &req).await;
    match request_token {
        Ok(Some((token, userinfo))) => {
            // TODO: randomize the session id
            let id = "randomize this";

            let login = userinfo.preferred_username.clone();
            let email = userinfo.email.clone();

            let user = User {
                id: userinfo.sub.clone().unwrap_or_default(),
                login,
                last_name: userinfo.family_name.clone(),
                first_name: userinfo.name.clone(),
                email,
                activated: userinfo.email_verified,
                image_url: userinfo.picture.clone().map(|x| x.to_string()),
                lang_key: Some("en".to_string()),
                authorities: vec!["user".to_string()],
            };

            let mut auth_cookie = Cookie::new("sess", id);
            auth_cookie.set_path("/");
            auth_cookie.set_http_only(true);

            cookies.add(auth_cookie);

            debug!("user: {user:?}");
            sessions.insert(
                id.to_string(),
                Session {
                    user,
                    token,
                    userinfo,
                },
            );

            info!("user logged in successfully");
            // redirect back to home
            Ok(Redirect::permanent("/".parse().unwrap()))
        }
        Ok(None) => {
            error!("auth server did not return a response with an id token");
            Err(resp_err(
                StatusCode::UNAUTHORIZED,
                "id token was not found in response",
            ))
        }
        Err(e) => {
            error!("token request failed: {e}");
            Err(resp_err(
                StatusCode::UNAUTHORIZED,
                "auth server did not complete token handoff",
            ))
        }
    }
}

pub async fn login(Extension(openid): Extension<Arc<OpenIDClient>>) -> impl IntoResponse {
    // TODO: randomize the state per request
    let auth_url = openid.auth_url(&openid::Options {
        scope: Some("openid email profile".into()),
        state: Some("randomizeonstartup".to_string()),
        ..Default::default()
    });

    Redirect::found(auth_url.into_string().parse().unwrap())
}

pub async fn logout(
    Extension(sessions): Extension<Arc<Sessions>>,
    cookies: Cookies,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiError<'static>>)> {
    // HACK: dumb juggling because the cookie taken from the jar isn't static
    let session = cookies
        .get("sess")
        .and_then(|c| Some(c.value().clone().to_string()))
        .unwrap_or("".to_string());

    if session.is_empty() {
        error!("user attempted logout with no active session");
        return Err(resp_err(
            StatusCode::UNAUTHORIZED,
            "not currently authenticated",
        ));
    }

    info!("session id: {session}");
    sessions.remove(&session);
    cookies.remove(Cookie::new("sess", session));

    info!("user logged out");
    Ok(Redirect::permanent("/".parse().unwrap()))
}
