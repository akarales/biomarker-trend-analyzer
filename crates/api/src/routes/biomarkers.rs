//! Biomarker endpoint: the stored series plus its drift report, for charting.

use axum::Json;
use axum::extract::{Path, Query, State};

use super::views::{ObservationView, SeriesView};
use crate::analysis::AnalysisParams;
use crate::error::ApiError;
use crate::reports::{self, SeriesReport};
use crate::review;
use crate::state::AppState;

pub async fn series(
    State(state): State<AppState>,
    Path((patient_id, code)): Path<(String, String)>,
    Query(params): Query<AnalysisParams>,
) -> Result<Json<SeriesView>, ApiError> {
    let options = params.options(state.config.window_days)?;
    let code = code.to_uppercase();
    let SeriesReport {
        observations,
        report,
        derived,
    } = reports::series_report(&state, &patient_id, &code, options).await?;

    let events = state.store.reviews(&patient_id, Some(&code)).await?;
    Ok(Json(SeriesView {
        observations: observations.iter().map(ObservationView::from).collect(),
        derived,
        reviews: review::report_reviews(&report, &events),
        history: events.into_iter().rev().collect(),
        patient_id,
        code,
        report,
    }))
}
