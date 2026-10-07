//! `POST /explain/stream` — the same grounded draft, streamed as NDJSON so
//! text appears while the model writes (pipeline ported from app #1):
//!
//! ```text
//! {"type":"start", …computed facts, model, disclaimer}   first line, before any model text
//! {"type":"delta","field":"summary","text":"…"}           repeated
//! {"type":"done", …same body as /explain: validated, computed status + signals re-asserted}
//! {"type":"error","code":"llm_upstream","error":"…"}      instead of done
//! ```
//!
//! Validation (unknown patient/biomarker, unavailable model) fails with a
//! normal JSON error BEFORE streaming. The model call runs in a task that
//! holds an explain permit for the whole stream; when the client goes away
//! (Stop), the next send fails, the task returns and drops the upstream
//! stream — closing the model request.

use std::convert::Infallible;

use axum::Json;
use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use biomarker_drift::DriftReport;
use futures_util::stream::{self, StreamExt};
use serde_json::{Value, json};
use tokio::sync::{OwnedSemaphorePermit, mpsc};
use tracing::Instrument;

use super::explain::{ExplainRequest, load_report, resolve_target, response_body, response_meta};
use crate::error::ApiError;
use crate::explain::context;
use crate::llm::{self, TextStream, extract::FieldExtractor};
use crate::state::AppState;

pub async fn explain_stream(
    State(state): State<AppState>,
    Json(request): Json<ExplainRequest>,
) -> Result<Response, ApiError> {
    let report = load_report(&state, &request).await?;
    let (provider, model) = resolve_target(&state, &request).await?;
    // at most EXPLAIN_CONCURRENCY model calls in flight; the permit lives as long as the stream
    let permit = state
        .explain_permits
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| ApiError::Internal)?;
    let upstream = llm::explain_stream(
        &state.http,
        &state.config.llm,
        provider,
        &model,
        &report,
        &context(&report),
    )
    .await?;

    let meta = response_meta(&request, &report, provider, &model);
    let (tx, rx) = mpsc::channel::<String>(64);
    tokio::spawn(pump(upstream, tx, permit, report, meta).instrument(tracing::Span::current()));

    let body = Body::from_stream(stream::unfold(rx, |mut rx| async move {
        rx.recv()
            .await
            .map(|line| (Ok::<_, Infallible>(format!("{line}\n")), rx))
    }));
    Ok((
        [
            (header::CONTENT_TYPE, "application/x-ndjson"),
            (header::CACHE_CONTROL, "no-cache"),
            (header::HeaderName::from_static("x-accel-buffering"), "no"),
        ],
        body,
    )
        .into_response())
}

fn error_line(err: &ApiError) -> String {
    json!({ "type": "error", "code": err.code(), "error": err.to_string() }).to_string()
}

fn typed(mut value: Value, kind: &str) -> String {
    value["type"] = json!(kind);
    value.to_string()
}

async fn pump(
    mut upstream: TextStream,
    tx: mpsc::Sender<String>,
    _permit: OwnedSemaphorePermit,
    report: DriftReport,
    meta: Value,
) {
    let gone =
        || tracing::info!("explain stream: client disconnected — upstream model request dropped");
    if tx.send(typed(meta.clone(), "start")).await.is_err() {
        return gone();
    }
    let mut extractor = FieldExtractor::new();
    let mut raw = String::new();
    while let Some(fragment) = upstream.next().await {
        let text = match fragment {
            Ok(text) => text,
            Err(err) => {
                let _ = tx.send(error_line(&err)).await;
                return;
            }
        };
        raw.push_str(&text);
        for delta in extractor.feed(&text) {
            let line =
                json!({ "type": "delta", "field": delta.field, "text": delta.text }).to_string();
            if tx.send(line).await.is_err() {
                return gone();
            }
        }
    }
    let last = match llm::parse_explanation(&raw) {
        Ok(explanation) => typed(response_body(meta, &report, explanation), "done"),
        Err(err) => error_line(&err),
    };
    let _ = tx.send(last).await;
}

/// `GET /llm/models` — the chooser: providers, models, availability, default.
pub async fn models(State(state): State<AppState>) -> Json<Value> {
    let (provider, model) = state.config.llm.default_target();
    Json(json!({
        "default": { "provider": provider, "model": model },
        "providers": llm::list_models(&state.http, &state.config.llm).await,
    }))
}
