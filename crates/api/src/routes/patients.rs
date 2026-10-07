//! Patient endpoints: triage listing and per-patient drift summaries.

use axum::Json;
use axum::extract::{Path, Query, State};
use biomarker_drift::DriftReport;

use super::views::{PatientTriageView, PatientsView, SummaryView};
use crate::analysis::{self, AnalysisParams, Options};
use crate::error::ApiError;
use crate::review;
use crate::state::AppState;
use crate::triage;

/// One drift report per biomarker of a patient (codes in name order).
async fn reports(
    state: &AppState,
    patient_id: &str,
    options: Options,
) -> Result<Vec<DriftReport>, ApiError> {
    let mut out = Vec::new();
    for code in state.store.patient_codes(patient_id).await? {
        let series = state.store.series(patient_id, &code).await?;
        out.extend(analysis::report(&series, &code, options));
    }
    Ok(out)
}

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
        let triage = triage::triage(&reports(&state, &entry.patient_id, options).await?, &events);
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
    let reports = reports(&state, &patient_id, options).await?;
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
    }))
}
