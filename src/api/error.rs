use {
    axum::{http::StatusCode, Json},
    serde::Serialize,
};

#[derive(Serialize)]
pub struct ApiError<'a> {
    pub err: &'a str,
}

pub fn resp_err<'a>(code: StatusCode, msg: &'a str) -> (StatusCode, Json<ApiError<'a>>) {
    let e = ApiError { err: msg };
    (code, Json(e))
}
