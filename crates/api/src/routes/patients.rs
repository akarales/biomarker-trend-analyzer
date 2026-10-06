//! Patient endpoints: listing and per-patient drift summaries.

use axum::Json;
use axum::extract::{Path, State};

use super::views::{PatientsView, SummaryView};
use crate::analysis;
use crate::error::ApiError;
use crate::state::AppState;

pub async fn list(State(state): State<AppState>) -> Result<Json<PatientsView>, ApiError> {
    let patients = state.store.listing().await?;
    Ok(Json(PatientsView { patients }))
}

pub async fn summary(
    State(state): State<AppState>,
    Path(patient_id): Path<String>,
) -> Result<Json<SummaryView>, ApiError> {
    let codes = state.store.patient_codes(&patient_id).await?;
    if codes.is_empty() {
        return Err(ApiError::NotFound(format!(
            "no observations for patient {patient_id}"
        )));
    }

    let now = chrono::Utc::now().timestamp();
    let mut reports = Vec::new();
    for code in &codes {
        let series = state.store.series(&patient_id, code).await?;
        reports.extend(analysis::report(
            &series,
            code,
            state.config.window_days,
            now,
        ));
    }

    Ok(Json(SummaryView {
        patient_id,
        window_days: state.config.window_days,
        reports,
    }))
}
