//! Observation upload: CSV or FHIR R4 JSON body in, rows stored
//! idempotently, summary (incl. skipped resources and why) back.

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};

use super::views::UploadView;
use crate::error::ApiError;
use crate::import;
use crate::state::AppState;

pub async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<UploadView>), ApiError> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok());
    let parsed = import::parse(import::detect(content_type, &body), &body)?;
    if parsed.observations.is_empty() {
        let skipped = match parsed.skipped_total() {
            0 => String::new(),
            n => format!(" ({n} skipped: {})", parsed.skipped[0].reason),
        };
        return Err(ApiError::BadRequest(format!(
            "upload contained no observations{skipped}"
        )));
    }
    let report = state
        .store
        .insert_observations(&parsed.observations)
        .await?;
    state.store.upsert_demographics(&parsed.patients).await?;
    Ok((StatusCode::CREATED, Json(UploadView::new(report, &parsed))))
}
