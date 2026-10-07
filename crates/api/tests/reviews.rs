//! Review workflow through the real router (memory store): acknowledge /
//! annotate / dismiss / reopen, server-side snapshots, validation, stale
//! signals, and triage by unreviewed signals.

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use biomarker_api::config::{Config, StoreBackend};
use biomarker_api::routes;
use biomarker_api::state::AppState;
use biomarker_api::store::{Store, memory::MemoryStore};

async fn app() -> axum::Router {
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
        llm: Default::default(),
    };
    routes::router(AppState::new(Arc::new(store), config))
}

async fn call(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.expect("request");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn get(app: &axum::Router, path: &str) -> Value {
    let (status, body) = call(
        app,
        Request::get(path).body(Body::empty()).expect("request"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{path}: {body}");
    body
}

async fn review(app: &axum::Router, patient: &str, code: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::post(format!(
        "/api/v1/patients/{patient}/biomarkers/{code}/reviews"
    ))
    .header(header::CONTENT_TYPE, "application/json")
    .body(Body::from(body.to_string()))
    .expect("request");
    call(app, request).await
}

/// (rule, t) of every watch/alert signal of a series.
async fn open_signals(app: &axum::Router, patient: &str, code: &str) -> Vec<(String, i64)> {
    let series = get(
        app,
        &format!("/api/v1/patients/{patient}/biomarkers/{code}"),
    )
    .await;
    series["report"]["signals"]
        .as_array()
        .expect("signals")
        .iter()
        .filter(|s| s["severity"] != "info")
        .map(|s| {
            (
                s["rule"].as_str().expect("rule").to_string(),
                s["t"].as_i64().expect("t"),
            )
        })
        .collect()
}

#[tokio::test]
async fn acknowledge_stores_a_server_snapshot_and_shows_in_the_series() {
    let app = app().await;
    let (rule, t) = open_signals(&app, "alice", "HBA1C").await.remove(0);
    // a client-supplied "snapshot" or severity is ignored — only rule + t name the signal
    let (status, body) =
        review(&app, "alice", "hba1c", json!({ "rule": rule, "t": t, "action": "acknowledge", "snapshot": { "status": "normal" } })).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let event = &body["event"];
    assert_eq!(
        (event["action"].as_str(), event["actor"].as_str()),
        (Some("acknowledge"), Some("demo-clinician"))
    );
    assert_eq!(
        event["snapshot"]["status"], "alert",
        "server-computed snapshot"
    );
    assert_eq!(event["snapshot"]["signal"]["rule"], rule.as_str());
    assert!(event["snapshot"]["signal"]["explanation"].is_string());
    assert_eq!(body["review"]["state"], "acknowledged");

    let series = get(&app, "/api/v1/patients/alice/biomarkers/HBA1C").await;
    let first = &series["reviews"][0];
    assert_eq!(
        (first["rule"].as_str(), first["state"].as_str()),
        (Some(rule.as_str()), Some("acknowledged"))
    );
    assert_eq!(
        series["reviews"].as_array().map(Vec::len),
        series["report"]["signals"].as_array().map(Vec::len)
    );
    assert_eq!(series["history"][0]["id"], event["id"]);
    let summary = get(&app, "/api/v1/patients/alice/summary").await;
    assert_eq!(summary["reviews"]["HBA1C"][0]["state"], "acknowledged");
}

#[tokio::test]
async fn notes_dismissal_and_reopen_follow_the_latest_decision() {
    let app = app().await;
    let (rule, t) = open_signals(&app, "alice", "HBA1C").await.remove(0);
    let body = |action: &str, reason: &str| json!({ "rule": rule, "t": t, "action": action, "reason": reason });
    assert_eq!(
        review(
            &app,
            "alice",
            "HBA1C",
            body("annotate", "repeat HbA1c booked")
        )
        .await
        .1["review"]["state"],
        "unreviewed"
    );
    let (_, dismissed) = review(
        &app,
        "alice",
        "HBA1C",
        body("dismiss", "steroid course explains the rise"),
    )
    .await;
    assert_eq!(dismissed["review"]["state"], "dismissed");
    assert_eq!(dismissed["review"]["notes"], 1);
    assert_eq!(
        dismissed["review"]["reason"],
        "steroid course explains the rise"
    );
    let (_, reopened) = review(
        &app,
        "alice",
        "HBA1C",
        body("reopen", "steroids stopped, still high"),
    )
    .await;
    assert_eq!(reopened["review"]["state"], "unreviewed");

    let history = get(&app, "/api/v1/patients/alice/biomarkers/HBA1C/reviews").await;
    let actions: Vec<&str> = history["history"]
        .as_array()
        .expect("history")
        .iter()
        .filter_map(|e| e["action"].as_str())
        .collect();
    assert_eq!(
        actions,
        ["reopen", "dismiss", "annotate"],
        "append-only, newest first"
    );
}

#[tokio::test]
async fn invalid_reviews_are_rejected_with_codes() {
    let app = app().await;
    let (rule, t) = open_signals(&app, "alice", "HBA1C").await.remove(0);
    let cases = [
        (
            json!({ "rule": rule, "t": t, "action": "dismiss" }),
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (
            json!({ "rule": rule, "t": t, "action": "annotate", "reason": "x" }),
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (
            json!({ "rule": rule, "t": t, "action": "dismiss", "reason": "patient MRN 12345678" }),
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (
            json!({ "rule": rule, "t": t - 86_400, "action": "acknowledge" }),
            StatusCode::CONFLICT,
            "stale_signal",
        ),
        (
            json!({ "rule": "ewma", "t": 0, "action": "acknowledge" }),
            StatusCode::CONFLICT,
            "stale_signal",
        ),
    ];
    for (body, status, code) in cases {
        let (got, response) = review(&app, "alice", "HBA1C", body.clone()).await;
        assert_eq!(
            (got, response["code"].as_str()),
            (status, Some(code)),
            "{body}"
        );
    }
    let (status, _) = review(
        &app,
        "nobody",
        "HBA1C",
        json!({ "rule": rule, "t": t, "action": "acknowledge" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let history = get(&app, "/api/v1/patients/alice/biomarkers/HBA1C/reviews").await;
    assert_eq!(
        history["history"],
        json!([]),
        "nothing stored for rejected reviews"
    );
}

#[tokio::test]
async fn triage_moves_a_fully_reviewed_patient_down_but_keeps_its_status() {
    let app = app().await;
    let listing = get(&app, "/api/v1/patients").await;
    assert_eq!(listing["patients"][0]["patient_id"], "alice");
    let before = listing["patients"][0]["unreviewed"]
        .as_u64()
        .expect("unreviewed");
    assert!(before > 0);

    for code in ["HBA1C", "LDL", "TSH", "CREAT"] {
        for (rule, t) in open_signals(&app, "alice", code).await {
            let (status, _) = review(
                &app,
                "alice",
                code,
                json!({ "rule": rule, "t": t, "action": "acknowledge" }),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED);
        }
    }
    let listing = get(&app, "/api/v1/patients").await;
    let rows = listing["patients"].as_array().expect("patients");
    let alice = rows
        .iter()
        .find(|p| p["patient_id"] == "alice")
        .expect("alice");
    assert_eq!(alice["status"], "alert", "the finding stays visible");
    assert_eq!(
        (alice["unreviewed"].as_u64(), alice["top_signal"].is_null()),
        (Some(0), true)
    );
    assert_ne!(
        rows[0]["patient_id"], "alice",
        "patients with open signals come first"
    );
    assert_eq!(rows[0]["patient_id"], "bob");
}

#[tokio::test]
async fn reviews_apply_to_the_signal_seen_in_an_as_of_view() {
    let app = app().await;
    let series = get(
        &app,
        "/api/v1/patients/bob/biomarkers/LDL?as_of=2026-10-31&window_days=365",
    )
    .await;
    let signal = series["report"]["signals"]
        .as_array()
        .expect("signals")
        .iter()
        .find(|s| s["severity"] != "info")
        .cloned()
        .expect("bob's LDL has a watch signal");
    let (status, body) = review(
        &app,
        "bob",
        "LDL",
        json!({ "rule": signal["rule"], "t": signal["t"], "action": "acknowledge", "as_of": "2026-10-31", "window_days": 365 }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["event"]["snapshot"]["window_days"], 365);
}
