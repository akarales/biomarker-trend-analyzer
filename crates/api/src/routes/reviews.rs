//! Signal review: `POST …/biomarkers/{code}/reviews` appends one event to
//! the audit trail; `GET` returns the trail. The client names a signal
//! (rule + the result time it fired on); the server recomputes the report
//! with the same options, refuses a signal it no longer computes (409
//! `stale_signal`) and stores its OWN snapshot of the signal.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use biomarker_drift::DriftReport;
use serde::Deserialize;

use super::views::{HistoryView, ReviewCreatedView};
use crate::analysis::AnalysisParams;
use crate::error::ApiError;
use crate::reports;
use crate::review;
use crate::state::AppState;
use crate::store::{NewReviewEvent, ReviewAction};
use crate::validate;

#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    /// drift rule of the signal (`prri`, `rcv`, …)
    pub rule: String,
    /// epoch seconds of the result the signal fired on
    pub t: i64,
    pub action: ReviewAction,
    #[serde(default)]
    pub reason: Option<String>,
    /// the analysis options the clinician was looking at
    #[serde(default)]
    pub as_of: Option<String>,
    #[serde(default)]
    pub window_days: Option<i64>,
}

async fn load(
    state: &AppState,
    patient_id: &str,
    code: &str,
    req: &ReviewRequest,
) -> Result<DriftReport, ApiError> {
    let params = AnalysisParams {
        as_of: req.as_of.clone(),
        window_days: req.window_days.map(|w| w.to_string()),
    };
    let options = params.options(state.config.window_days)?;
    Ok(reports::series_report(state, patient_id, code, options)
        .await?
        .report)
}

/// Reason rules: optional for acknowledge, required (3..=500, no
/// identifiers) for annotate, dismiss and reopen.
fn reason_for(action: ReviewAction, raw: Option<&str>) -> Result<Option<String>, ApiError> {
    match (action, raw.map(str::trim).filter(|r| !r.is_empty())) {
        (ReviewAction::Acknowledge, None) => Ok(None),
        (_, Some(text)) => validate::reason(text).map(Some),
        (other, None) => Err(ApiError::BadRequest(format!(
            "a reason is required to {}",
            other.as_str()
        ))),
    }
}

pub async fn create(
    State(state): State<AppState>,
    Path((patient_id, code)): Path<(String, String)>,
    Json(req): Json<ReviewRequest>,
) -> Result<(StatusCode, Json<ReviewCreatedView>), ApiError> {
    let code = code.to_uppercase();
    let reason = reason_for(req.action, req.reason.as_deref())?;
    let report = load(&state, &patient_id, &code, &req).await?;
    let signal = report
        .signals
        .iter()
        .find(|s| review::rule_name(s) == req.rule && s.t == req.t)
        .ok_or_else(|| {
            ApiError::StaleSignal(format!(
                "no {} signal at {} in the current analysis of {patient_id} / {code} — reload and review again",
                req.rule, req.t
            ))
        })?;
    let event = state
        .store
        .append_review(NewReviewEvent {
            patient_id: patient_id.clone(),
            code: code.clone(),
            rule: req.rule.clone(),
            signal_t: req.t,
            action: req.action,
            reason,
            snapshot: review::snapshot(&report, signal),
        })
        .await?;
    tracing::info!(
        patient_id,
        code,
        rule = req.rule,
        action = req.action.as_str(),
        event = event.id,
        "signal reviewed"
    );
    let events = state.store.reviews(&patient_id, Some(&code)).await?;
    let review = review::signal_review(&code, signal, &events);
    Ok((
        StatusCode::CREATED,
        Json(ReviewCreatedView { event, review }),
    ))
}

pub async fn history(
    State(state): State<AppState>,
    Path((patient_id, code)): Path<(String, String)>,
) -> Result<Json<HistoryView>, ApiError> {
    let code = code.to_uppercase();
    let history = state
        .store
        .reviews(&patient_id, Some(&code))
        .await?
        .into_iter()
        .rev()
        .collect();
    Ok(Json(HistoryView {
        patient_id,
        code,
        history,
    }))
}
