//! "Explain this drift" through the real router on the stub provider (no
//! network: Ollama points at a closed port, no Anthropic key).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use futures_util::StreamExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use biomarker_api::config::{Config, LlmConfig, StoreBackend};
use biomarker_api::routes;
use biomarker_api::state::{AppState, EXPLAIN_CONCURRENCY};
use biomarker_api::store::{Store, memory::MemoryStore};

async fn state() -> AppState {
    let store = Store::Memory(MemoryStore::default());
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo_labs.csv");
    let csv = std::fs::read_to_string(path).expect("fixture");
    store
        .insert_observations(&biomarker_ingest::parse_csv_bytes(csv.as_bytes()).expect("parse"))
        .await
        .expect("seed");
    let config = Config {
        store: StoreBackend::Memory,
        database_url: None,
        seed_demo_data: true,
        demo_data: PathBuf::new(),
        port: 0,
        window_days: 365,
        llm: LlmConfig {
            ollama_url: "http://127.0.0.1:9".into(),
            ..LlmConfig::default()
        },
    };
    AppState::new(Arc::new(store), config)
}

fn post(path: &str, body: Value) -> Request<Body> {
    Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

async fn ndjson(state: &AppState, body: Value) -> (StatusCode, Option<String>, Vec<Value>) {
    let response = routes::router(state.clone())
        .oneshot(post("/api/v1/explain/stream", body))
        .await
        .expect("request");
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap_or("").to_string());
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let text = String::from_utf8(bytes.to_vec()).expect("utf8");
    let lines = text
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or(json!(l)))
        .collect();
    (status, content_type, lines)
}

#[tokio::test]
async fn streams_start_deltas_done_with_the_computed_facts_reasserted() {
    let state = state().await;
    let (status, content_type, events) =
        ndjson(&state, json!({ "patient_id": "alice", "code": "hba1c" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("application/x-ndjson"));

    let kinds: Vec<&str> = events
        .iter()
        .map(|e| e["type"].as_str().unwrap_or("?"))
        .collect();
    assert_eq!(kinds.first(), Some(&"start"));
    assert_eq!(kinds.last(), Some(&"done"));
    assert!(
        kinds.iter().filter(|k| **k == "delta").count() > 5,
        "text arrives progressively"
    );

    let start = &events[0];
    assert!(
        start["disclaimer"]
            .as_str()
            .expect("disclaimer")
            .contains("clinician review required"),
        "from the first line"
    );
    assert_eq!(
        (
            start["computed_status"].as_str(),
            start["provider"].as_str(),
            start["stub"].as_bool()
        ),
        (Some("alert"), Some("stub"), Some(true))
    );

    let mut streamed: BTreeMap<String, String> = BTreeMap::new();
    for e in events.iter().filter(|e| e["type"] == "delta") {
        streamed
            .entry(e["field"].as_str().expect("field").to_string())
            .or_default()
            .push_str(e["text"].as_str().expect("text"));
    }
    let done = events.last().expect("done");
    for field in [
        "summary",
        "interpretation",
        "follow_up",
        "limitations",
        "status",
    ] {
        assert_eq!(
            streamed.get(field).map(String::as_str),
            done["explanation"][field].as_str(),
            "{field}: deltas == validated text"
        );
    }
    assert_eq!(done["explanation"]["status"], "alert");
    assert_eq!(done["status_overridden"], false);
    let signals = done["signals"].as_array().expect("signals re-asserted");
    assert!(
        signals
            .iter()
            .any(|s| s["rule"] == "prri" && s["severity"] == "alert")
    );
    assert!(
        done["explanation"]["summary"]
            .as_str()
            .expect("summary")
            .contains("personal reference interval")
    );
}

#[tokio::test]
async fn validation_errors_are_json_before_any_stream() {
    let state = state().await;
    let (status, _, events) =
        ndjson(&state, json!({ "patient_id": "nobody", "code": "HBA1C" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(events[0]["code"], "not_found");

    let (status, _, events) = ndjson(
        &state,
        json!({ "patient_id": "alice", "code": "HBA1C", "provider": "anthropic" }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "no API key → not selectable"
    );
    assert_eq!(events[0]["code"], "bad_request");

    let (status, _, _) = ndjson(
        &state,
        json!({ "patient_id": "alice", "code": "HBA1C", "window_days": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn as_of_changes_what_is_explained() {
    let state = state().await;
    let (_, _, events) = ndjson(
        &state,
        json!({ "patient_id": "alice", "code": "HBA1C", "as_of": "2026-08-31" }),
    )
    .await;
    assert_ne!(events[0]["computed_status"], "alert");
    assert_eq!(
        events.last().expect("done")["explanation"]["status"],
        events[0]["computed_status"]
    );
}

#[tokio::test]
async fn a_client_that_goes_away_releases_its_permit() {
    let state = state().await;
    let response = routes::router(state.clone())
        .oneshot(post(
            "/api/v1/explain/stream",
            json!({ "patient_id": "alice", "code": "HBA1C" }),
        ))
        .await
        .expect("request");
    let mut body = response.into_body().into_data_stream();
    let first = body.next().await.expect("first chunk").expect("bytes");
    assert!(String::from_utf8_lossy(&first).contains("\"type\":\"start\""));
    assert!(
        state.explain_permits.available_permits() < EXPLAIN_CONCURRENCY,
        "held while streaming"
    );
    drop(body);
    for _ in 0..100 {
        if state.explain_permits.available_permits() == EXPLAIN_CONCURRENCY {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("permit not released after the client disconnected");
}

#[tokio::test]
async fn non_streaming_fallback_and_model_list() {
    let state = state().await;
    let response = routes::router(state.clone())
        .oneshot(post(
            "/api/v1/explain",
            json!({ "patient_id": "bob", "code": "LDL" }),
        ))
        .await
        .expect("request");
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");
    assert_eq!(body["explanation"]["status"], body["computed_status"]);
    assert!(body["disclaimer"].is_string() && body["signals"].is_array());

    let response = routes::router(state)
        .oneshot(
            Request::get("/api/v1/llm/models")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("request");
    let models: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");
    assert_eq!(
        models["default"],
        json!({ "provider": "stub", "model": "stub" })
    );
    let by_provider: BTreeMap<&str, bool> = models["providers"]
        .as_array()
        .expect("providers")
        .iter()
        .map(|p| {
            (
                p["provider"].as_str().unwrap_or("?"),
                p["available"].as_bool().unwrap_or(false),
            )
        })
        .collect();
    assert_eq!(
        by_provider,
        BTreeMap::from([("anthropic", false), ("ollama", false), ("stub", true)])
    );
}
