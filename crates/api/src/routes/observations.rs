//! Observation upload: CSV body in, typed rows stored idempotently, summary back.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use super::views::UploadView;
use crate::error::ApiError;
use crate::state::AppState;

pub async fn upload(
    State(state): State<AppState>,
    body: String,
) -> Result<(StatusCode, Json<UploadView>), ApiError> {
    let observations = biomarker_ingest::parse_csv_bytes(body.as_bytes())
        .map_err(|e| ApiError::Csv(e.to_string()))?;
    if observations.is_empty() {
        return Err(ApiError::BadRequest(
            "upload contained no observations".into(),
        ));
    }
    let report = state.store.insert_observations(&observations).await?;
    Ok((
        StatusCode::CREATED,
        Json(UploadView::new(report, &observations)),
    ))
}
