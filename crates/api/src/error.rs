//! API error taxonomy mapped onto HTTP responses.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("unknown patient or biomarker: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("store error: {0}")]
    Store(String),
    #[error("csv error: {0}")]
    Csv(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::Store(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::Csv(_) => (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<crate::store::StoreError> for ApiError {
    fn from(err: crate::store::StoreError) -> Self {
        ApiError::Store(err.to_string())
    }
}
