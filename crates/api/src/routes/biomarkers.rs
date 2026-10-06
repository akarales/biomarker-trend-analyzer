//! Biomarker endpoint: the stored series plus its drift report, for charting.

use axum::Json;
use axum::extract::{Path, Query, State};

use super::views::{ObservationView, SeriesView};
use crate::analysis::{self, AnalysisParams};
use crate::error::ApiError;
use crate::state::AppState;

pub async fn series(
    State(state): State<AppState>,
    Path((patient_id, code)): Path<(String, String)>,
    Query(params): Query<AnalysisParams>,
) -> Result<Json<SeriesView>, ApiError> {
    let options = params.options(state.config.window_days)?;
    let code = code.to_uppercase();
    let series = state.store.series(&patient_id, &code).await?;
    let report = analysis::report(&series, &code, options)
        .ok_or_else(|| ApiError::NotFound(format!("no observations for {patient_id} / {code}")))?;

    Ok(Json(SeriesView {
        observations: series.iter().map(ObservationView::from).collect(),
        patient_id,
        code,
        report,
    }))
}
