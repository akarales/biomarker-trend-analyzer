//! `POST /explain` — a grounded "explain this drift" draft for one
//! biomarker of one patient (stub by default). The streamed variant is in
//! `explain_stream.rs`; both share request parsing, target resolution and
//! the response shape (computed status + signals re-asserted).

use axum::Json;
use axum::extract::State;
use biomarker_drift::DriftReport;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::analysis::{self, AnalysisParams};
use crate::config::LlmProvider;
use crate::error::ApiError;
use crate::explain::{DISCLAIMER, context};
use crate::llm::{self, Explanation};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ExplainRequest {
    pub patient_id: String,
    pub code: String,
    #[serde(default)]
    pub as_of: Option<String>,
    #[serde(default)]
    pub window_days: Option<i64>,
    /// Model chooser: `stub` | `ollama` | `anthropic`; server default when absent.
    #[serde(default)]
    pub provider: Option<LlmProvider>,
    /// Model id within the provider; the provider's default when absent.
    #[serde(default)]
    pub model: Option<String>,
}

/// The report the explanation is about (same options as the chart).
pub(super) async fn load_report(
    state: &AppState,
    request: &ExplainRequest,
) -> Result<DriftReport, ApiError> {
    let params = AnalysisParams {
        as_of: request.as_of.clone(),
        window_days: request.window_days.map(|w| w.to_string()),
    };
    let options = params.options(state.config.window_days)?;
    let code = request.code.to_uppercase();
    let series = state.store.series(&request.patient_id, &code).await?;
    analysis::report(&series, &code, options).ok_or_else(|| {
        ApiError::NotFound(format!(
            "no observations for {} / {code}",
            request.patient_id
        ))
    })
}

/// Provider + model (server defaults when absent), validated against what
/// is actually available.
pub(super) async fn resolve_target(
    state: &AppState,
    request: &ExplainRequest,
) -> Result<(LlmProvider, String), ApiError> {
    let llm_config = &state.config.llm;
    let (default_provider, default_model) = llm_config.default_target();
    let provider = request.provider.unwrap_or(default_provider);
    let model = match (&request.model, provider) {
        (Some(model), _) => model.clone(),
        (None, p) if p == default_provider => default_model,
        (None, LlmProvider::Stub) => "stub".to_string(),
        (None, LlmProvider::Ollama) => llm_config.ollama_model.clone(),
        (None, LlmProvider::Anthropic) => llm_config.anthropic_model.clone(),
    };
    llm::validate_choice(&state.http, llm_config, provider, &model).await?;
    Ok((provider, model))
}

fn wire<T: serde::Serialize>(v: T) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

/// Computed facts + model identity: the stream's first line (`start`) and
/// the base of the final body.
pub(super) fn response_meta(
    request: &ExplainRequest,
    report: &DriftReport,
    provider: LlmProvider,
    model: &str,
) -> Value {
    json!({
        "patient_id": request.patient_id,
        "code": report.code,
        "as_of": report.as_of,
        "window_days": report.window_days,
        "computed_status": wire(report.status),
        "provider": provider,
        "model": model,
        "stub": provider == LlmProvider::Stub,
        "disclaimer": DISCLAIMER,
    })
}

/// Final body: meta + the validated draft with the computed status
/// asserted, plus the computed signals (re-asserted next to the text).
pub(super) fn response_body(
    meta: Value,
    report: &DriftReport,
    mut explanation: Explanation,
) -> Value {
    let overridden = explanation.assert_status(wire(report.status).as_str().unwrap_or("normal"));
    let mut body = meta;
    body["explanation"] = json!(explanation);
    body["status_overridden"] = json!(overridden);
    body["signals"] = json!(report.signals);
    body["not_assessed"] = json!(report.not_assessed);
    body
}

pub async fn explain(
    State(state): State<AppState>,
    Json(request): Json<ExplainRequest>,
) -> Result<Json<Value>, ApiError> {
    let report = load_report(&state, &request).await?;
    let (provider, model) = resolve_target(&state, &request).await?;
    let _permit = state
        .explain_permits
        .acquire()
        .await
        .map_err(|_| ApiError::Internal)?;
    let explanation = llm::explain(
        &state.http,
        &state.config.llm,
        provider,
        &model,
        &report,
        &context(&report),
    )
    .await?;
    let meta = response_meta(&request, &report, provider, &model);
    Ok(Json(response_body(meta, &report, explanation)))
}

#[cfg(test)]
mod tests {
    use biomarker_drift::{AnalysisInput, Reading, analyze};

    use super::*;

    #[test]
    fn the_engine_status_and_signals_win_over_the_model() {
        let mut readings: Vec<Reading> = (0..8)
            .map(|i| Reading {
                t: i * 30 * 86_400,
                value: 5.6,
                unit: "%".into(),
            })
            .collect();
        readings.push(Reading {
            t: 8 * 30 * 86_400,
            value: 7.1,
            unit: "%".into(),
        });
        let report = analyze(&AnalysisInput {
            code: "HBA1C",
            readings: &readings,
            as_of: None,
            trend_window_days: 365,
        });
        let request = ExplainRequest {
            patient_id: "p".into(),
            code: "HBA1C".into(),
            as_of: None,
            window_days: None,
            provider: None,
            model: None,
        };
        let model_says = Explanation {
            summary: "looks fine".into(),
            interpretation: "i".into(),
            follow_up: "f".into(),
            limitations: "l".into(),
            status: "normal".into(),
        };
        let body = response_body(
            response_meta(&request, &report, LlmProvider::Stub, "stub"),
            &report,
            model_says,
        );
        assert_eq!(body["explanation"]["status"], "alert");
        assert_eq!(body["computed_status"], "alert");
        assert_eq!(body["status_overridden"], true);
        assert_eq!(
            body["signals"].as_array().expect("signals").len(),
            report.signals.len()
        );
        assert!(
            body["disclaimer"]
                .as_str()
                .expect("disclaimer")
                .contains("clinician review required")
        );
    }
}
