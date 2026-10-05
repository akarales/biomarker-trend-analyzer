//! Observation upload: CSV body in, typed rows stored, summary back.

use std::collections::BTreeSet;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::error::ApiError;
use crate::state::AppState;

pub async fn upload(State(state): State<AppState>, body: String) -> Result<Response, ApiError> {
    let observations = biomarker_ingest::parse_csv_bytes(body.as_bytes())
        .map_err(|e| ApiError::Csv(e.to_string()))?;
    if observations.is_empty() {
        return Err(ApiError::BadRequest(
            "upload contained no observations".into(),
        ));
    }

    let patients: BTreeSet<String> = observations.iter().map(|o| o.patient_id.clone()).collect();
    let biomarkers: BTreeSet<String> = observations.iter().map(|o| o.code.clone()).collect();

    let inserted = state
        .store
        .insert_observations(&observations)
        .await
        .map_err(ApiError::from)?;

    let payload = json!({
        "inserted": inserted,
        "patients": patients.len(),
        "biomarkers": biomarkers,
    });
    Ok((StatusCode::CREATED, Json(payload)).into_response())
}
