//! Biomarker endpoint: the full series plus its drift report, for charting.

use axum::Json;
use axum::extract::{Path, State};

use super::views::{ObservationView, SeriesView};
use crate::analysis;
use crate::error::ApiError;
use crate::state::AppState;

pub async fn series(
    State(state): State<AppState>,
    Path((patient_id, code)): Path<(String, String)>,
) -> Result<Json<SeriesView>, ApiError> {
    let code = code.to_uppercase();
    let series = state.store.series(&patient_id, &code).await?;
    let now = chrono::Utc::now().timestamp();
    let report = analysis::report(&series, &code, state.config.window_days, now)
        .ok_or_else(|| ApiError::NotFound(format!("no observations for {patient_id} / {code}")))?;

    Ok(Json(SeriesView {
        observations: series.iter().map(ObservationView::from).collect(),
        patient_id,
        code,
        report,
    }))
}
