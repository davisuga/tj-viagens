use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    Forbidden(&'static str),
    NotFound(&'static str),
    Unprocessable(&'static str),
    UnprocessableWith(&'static str, serde_json::Value),
    Conflict(&'static str),
    Internal(String),
}

pub type ApiResult<T> = Result<T, ApiError>;

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> ApiError {
        ApiError::Internal(err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            ApiError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, json!({ "error": "NAO_AUTENTICADO" }))
            }
            ApiError::Forbidden(code) => (StatusCode::FORBIDDEN, json!({ "error": code })),
            ApiError::NotFound(code) => (StatusCode::NOT_FOUND, json!({ "error": code })),
            ApiError::Unprocessable(code) => {
                (StatusCode::UNPROCESSABLE_ENTITY, json!({ "error": code }))
            }
            ApiError::UnprocessableWith(code, detail) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({ "error": code, "detail": detail }),
            ),
            ApiError::Conflict(code) => (StatusCode::CONFLICT, json!({ "error": code })),
            ApiError::Internal(msg) => {
                eprintln!("internal error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": "ERRO_INTERNO" }))
            }
        };
        (status, Json(body)).into_response()
    }
}
