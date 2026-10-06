//! Patient and biomarker endpoints: listings, per-patient drift
//! summaries, full series with baseline band for charting.

use axum::Json;
use axum::extract::{Path, State};
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::state::AppState;

pub async fn patients(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let listing = state.store.listing().await?;
    Ok(Json(json!({ "patients": listing })))
}

pub async fn patient_summary(
    State(state): State<AppState>,
    Path(patient_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
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
        if series.is_empty() {
            continue;
        }
        let unit = series.last().expect("checked non-empty").unit.clone();
        let points: Vec<biomarker_drift::SeriesPoint> = series
            .iter()
            .map(|o| biomarker_drift::SeriesPoint {
                t: o.taken_at.and_utc().timestamp(),
                v: o.value,
            })
            .collect();
        let report = biomarker_drift::analyze(&points, code, &unit, state.config.window_days, now);
        reports.push(report);
    }

    Ok(Json(json!({
        "patient_id": patient_id,
        "window_days": state.config.window_days,
        "reports": reports,
    })))
}

pub async fn biomarker_series(
    State(state): State<AppState>,
    Path((patient_id, code)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let code = code.to_uppercase();
    let series = state.store.series(&patient_id, &code).await?;
    if series.is_empty() {
        return Err(ApiError::NotFound(format!(
            "no observations for {patient_id} / {code}"
        )));
    }

    let now = chrono::Utc::now().timestamp();
    let points: Vec<biomarker_drift::SeriesPoint> = series
        .iter()
        .map(|o| biomarker_drift::SeriesPoint {
            t: o.taken_at.and_utc().timestamp(),
            v: o.value,
        })
        .collect();
    let report = biomarker_drift::analyze(
        &points,
        &code,
        &series.last().expect("series non-empty").unit,
        state.config.window_days,
        now,
    );

    let observations: Vec<Value> = series
        .iter()
        .map(|o| {
            json!({
                "taken_at": o.taken_at.to_string(),
                "value": o.value,
                "unit": o.unit,
                "source": o.source,
            })
        })
        .collect();

    Ok(Json(json!({
        "patient_id": patient_id,
        "code": code,
        "observations": observations,
        "report": report,
    })))
}
