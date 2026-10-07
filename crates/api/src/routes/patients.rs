//! Patient endpoints: triage listing and per-patient drift summaries.

use axum::Json;
use axum::extract::{Path, Query, State};

use super::views::{PatientTriageView, PatientsView, SummaryView};
use crate::analysis::AnalysisParams;
use crate::error::ApiError;
use crate::reports;
use crate::review;
use crate::state::AppState;
use crate::triage;

/// Patients worst first (status, alert count, watch count, id). Analyses
/// every series — N + 1 store calls, fine at demo scale.
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<AnalysisParams>,
) -> Result<Json<PatientsView>, ApiError> {
    let options = params.options(state.config.window_days)?;
    let mut patients = Vec::new();
    for entry in state.store.listing().await? {
        let events = state.store.reviews(&entry.patient_id, None).await?;
        let analysis = reports::patient_reports(&state, &entry.patient_id, options).await?;
        let triage = triage::triage(&analysis.reports, &events);
        patients.push(PatientTriageView { entry, triage });
    }
    patients.sort_by_key(|p| triage::sort_key(&p.triage, &p.entry.patient_id));
    Ok(Json(PatientsView {
        as_of: options.as_of,
        window_days: options.window_days,
        patients,
    }))
}

pub async fn summary(
    State(state): State<AppState>,
    Path(patient_id): Path<String>,
    Query(params): Query<AnalysisParams>,
) -> Result<Json<SummaryView>, ApiError> {
    let options = params.options(state.config.window_days)?;
    let analysis = reports::patient_reports(&state, &patient_id, options).await?;
    let reports = analysis.reports;
    if reports.is_empty() {
        return Err(ApiError::NotFound(format!(
            "no observations for patient {patient_id}"
        )));
    }
    let events = state.store.reviews(&patient_id, None).await?;
    let reviews = reports
        .iter()
        .map(|r| (r.code.clone(), review::report_reviews(r, &events)))
        .collect();
    Ok(Json(SummaryView {
        patient_id,
        as_of: options.as_of,
        window_days: options.window_days,
        reports,
        reviews,
        derived: analysis.derived,
        demographics: analysis.demographics,
    }))
}
