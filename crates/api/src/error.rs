//! API error taxonomy mapped onto HTTP responses.
//!
//! Every error body is `{"error": "<message>", "code": "<stable code>"}`;
//! clients branch on `code`, humans read `error`. Store/internal details
//! are logged (with the request id span), never sent to the client.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::store::StoreError;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("unknown patient or biomarker: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("csv error: {0}")]
    Csv(String),
    #[error("internal error")]
    Internal,
}

impl ApiError {
    /// Stable machine-readable code for the response body.
    pub fn code(&self) -> &'static str {
        match self {
            ApiError::NotFound(_) => "not_found",
            ApiError::BadRequest(_) => "bad_request",
            ApiError::Csv(_) => "invalid_csv",
            ApiError::Internal => "internal",
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Csv(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({ "error": self.to_string(), "code": self.code() });
        (self.status(), Json(body)).into_response()
    }
}

impl From<StoreError> for ApiError {
    fn from(err: StoreError) -> Self {
        tracing::error!(error = %err, "store error");
        ApiError::Internal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_errors_never_reach_the_client() {
        let err: ApiError =
            StoreError::Database("select: relation \"observations\" does not exist".into()).into();
        assert!(matches!(err, ApiError::Internal));
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.code(), "internal");
        assert!(
            !err.to_string().contains("observations"),
            "SQL stays in the log"
        );
    }
}
