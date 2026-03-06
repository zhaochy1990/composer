use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub enum ServiceError {
    NotFound(String),
    BadRequest(String),
    Conflict(String),
    Internal(anyhow::Error),
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ServiceError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ServiceError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ServiceError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            ServiceError::Internal(err) => {
                // Log full error chain for debugging, but only expose the top-level
                // message to clients (avoids leaking SQL queries, file paths, stack traces)
                tracing::error!("Internal error: {:?}", err);
                let msg = format!("{}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl<E> From<E> for ServiceError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::Internal(err.into())
    }
}
