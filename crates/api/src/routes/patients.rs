//! Patient endpoints: listing and per-patient drift summaries.

use axum::Json;
use axum::extract::{Path, Query, State};

use super::views::{PatientsView, SummaryView};
use crate::analysis::{self, AnalysisParams};
use crate::error::ApiError;
use crate::state::AppState;

pub async fn list(State(state): State<AppState>) -> Result<Json<PatientsView>, ApiError> {
    let patients = state.store.listing().await?;
    Ok(Json(PatientsView { patients }))
}

pub async fn summary(
    State(state): State<AppState>,
    Path(patient_id): Path<String>,
    Query(params): Query<AnalysisParams>,
) -> Result<Json<SummaryView>, ApiError> {
    let options = params.options(state.config.window_days)?;
    let codes = state.store.patient_codes(&patient_id).await?;
    if codes.is_empty() {
        return Err(ApiError::NotFound(format!(
            "no observations for patient {patient_id}"
        )));
    }

    let mut reports = Vec::new();
    for code in &codes {
        let series = state.store.series(&patient_id, code).await?;
        reports.extend(analysis::report(&series, code, options));
    }

    Ok(Json(SummaryView {
        patient_id,
        as_of: options.as_of,
        window_days: options.window_days,
        reports,
    }))
}
