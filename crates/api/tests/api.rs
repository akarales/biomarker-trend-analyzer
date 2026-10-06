//! Integration tests: full axum router over a MemoryStore seeded with the
//! synthetic fixture — no Postgres, no network.

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use biomarker_api::config::{Config, StoreBackend};
use biomarker_api::routes;
use biomarker_api::state::AppState;
use biomarker_api::store::{Store, memory::MemoryStore};

fn fixture_csv() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo_labs.csv");
    std::fs::read_to_string(&path).expect("fixture exists")
}

async fn test_app() -> axum::Router {
    let store = Store::Memory(MemoryStore::default());
    let csv = fixture_csv();
    let observations = biomarker_ingest::parse_csv_bytes(csv.as_bytes()).expect("fixture parses");
    let report = store
        .insert_observations(&observations)
        .await
        .expect("seed inserts");
    assert_eq!(report.inserted, 576);
    let config = Config {
        store: StoreBackend::Memory,
        database_url: None,
        seed_demo_data: true,
        demo_csv: PathBuf::new(),
        port: 0,
        window_days: 365,
    };
    routes::router(AppState {
        store: Arc::new(store),
        config: Arc::new(config),
    })
}

async fn get(path: &str) -> (StatusCode, Value) {
    let app = test_app().await;
    let response = app
        .oneshot(
            Request::get(path)
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("in-process request");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body reads");
    let body: Value = serde_json::from_slice(&bytes).expect("json body");
    (status, body)
}

async fn post_text_to(app: &axum::Router, path: &str, body: String) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::post(path)
                .header("content-type", "text/csv")
                .body(Body::from(body))
                .expect("request builds"),
        )
        .await
        .expect("in-process request");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body reads");
    let body: Value = serde_json::from_slice(&bytes).expect("json body");
    (status, body)
}

async fn post_text(path: &str, body: String) -> (StatusCode, Value) {
    post_text_to(&test_app().await, path, body).await
}

#[tokio::test]
async fn health_reports_memory_store() {
    let (status, body) = get("/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["store"], "memory");
}

#[tokio::test]
async fn patients_listing_has_three() {
    let (status, body) = get("/api/v1/patients").await;
    assert_eq!(status, StatusCode::OK);
    let patients = body["patients"].as_array().expect("patients array");
    assert_eq!(patients.len(), 3);
    assert!(patients.iter().all(|p| p["observations"] == 192));
}

#[tokio::test]
async fn alice_hba1c_step_is_an_alert() {
    let (status, body) = get("/api/v1/patients/alice/summary").await;
    assert_eq!(status, StatusCode::OK);
    let reports = body["reports"].as_array().expect("reports");
    let hba1c = reports
        .iter()
        .find(|r| r["code"] == "HBA1C")
        .expect("HBA1C report present");
    assert_eq!(hba1c["status"], "alert", "injected step-up must alert");
    assert!(hba1c["latest_z"].as_f64().unwrap() >= 3.0);
}

#[tokio::test]
async fn bob_ldl_rising_trend_detected() {
    let (status, body) = get("/api/v1/patients/bob/summary").await;
    assert_eq!(status, StatusCode::OK);
    let ldl = body["reports"]
        .as_array()
        .expect("reports")
        .iter()
        .find(|r| r["code"] == "LDL")
        .expect("LDL report present");
    assert_eq!(ldl["trend"], "rising", "gradual rise must be detected");
}

#[tokio::test]
async fn carol_stable_series_is_normal() {
    let (status, body) = get("/api/v1/patients/carol/summary").await;
    assert_eq!(status, StatusCode::OK);
    let reports = body["reports"].as_array().expect("reports");
    assert!(
        reports
            .iter()
            .all(|r| r["status"] == "normal" || r["status"] == "watch"),
        "no alerts for the control patient"
    );
}

#[tokio::test]
async fn series_returns_observations_and_report() {
    let (status, body) = get("/api/v1/patients/alice/biomarkers/HBA1C").await;
    assert_eq!(status, StatusCode::OK);
    let observations = body["observations"].as_array().expect("observations");
    assert_eq!(observations.len(), 48);
    assert_eq!(body["code"], "HBA1C");
    assert!(body["report"]["baseline"].is_object());
}

#[tokio::test]
async fn unknown_patient_404() {
    let (status, body) = get("/api/v1/patients/ghost/summary").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"].as_str().unwrap().contains("ghost"));
    assert_eq!(body["code"], "not_found");
}

#[tokio::test]
async fn unknown_series_404() {
    let (status, body) = get("/api/v1/patients/alice/biomarkers/ZZZ").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");
}

#[tokio::test]
async fn upload_is_idempotent() {
    let app = test_app().await;
    let csv = "\
patient_id,code,value,unit,taken_at,source\
\ndave,HBA1C,7.2,%,2026-09-01,upload\
\ndave,HBA1C,7.2,%,2026-09-01,upload\n";
    let (status, body) = post_text_to(&app, "/api/v1/observations", csv.to_string()).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["inserted"], 1);
    assert_eq!(body["duplicates"], 1, "repeat within one batch");
    assert_eq!(body["patients"], 1);

    // Same file again on the same instance: nothing new is stored.
    let (status, body) = post_text_to(&app, "/api/v1/observations", csv.to_string()).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["inserted"], 0);
    assert_eq!(body["duplicates"], 2);

    // Re-uploading the whole demo fixture (a restart re-seed) changes nothing.
    let (_, body) = post_text_to(&app, "/api/v1/observations", fixture_csv()).await;
    assert_eq!(body["inserted"], 0);
    assert_eq!(body["duplicates"], 576);
    let response = app
        .oneshot(
            Request::get("/api/v1/patients")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("in-process request");
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body reads");
    let body: Value = serde_json::from_slice(&bytes).expect("json body");
    let patients = body["patients"].as_array().expect("patients array");
    assert_eq!(patients.len(), 4);
    let dave = patients
        .iter()
        .find(|p| p["patient_id"] == "dave")
        .expect("dave listed");
    assert_eq!(dave["observations"], 1);
    assert!(
        patients
            .iter()
            .filter(|p| p["patient_id"] != "dave")
            .all(|p| p["observations"] == 192)
    );
}

#[tokio::test]
async fn responses_carry_a_request_id() {
    let app = test_app().await;
    let generated = app
        .clone()
        .oneshot(
            Request::get("/api/v1/patients/ghost/summary")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("in-process request");
    assert!(generated.headers().contains_key("x-request-id"));

    let echoed = app
        .oneshot(
            Request::get("/health")
                .header("x-request-id", "client-abc_123")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("in-process request");
    assert_eq!(echoed.headers()["x-request-id"], "client-abc_123");
}

#[tokio::test]
async fn upload_rejects_bad_csv() {
    // Header mismatch -> 422 with the schema-mismatch message.
    let (status, body) = post_text("/api/v1/observations", "garbage,header\n".to_string()).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["error"].as_str().unwrap().contains("schema mismatch"));
    assert_eq!(body["code"], "invalid_csv");

    // A valid header with zero data rows -> 400 "no observations".
    let empty = "patient_id,code,value,unit,taken_at,source\n";
    let (status, body) = post_text("/api/v1/observations", empty.to_string()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("no observations"));
    assert_eq!(body["code"], "bad_request");

    // A non-numeric value against the enforced schema -> 422 CSV error.
    let bad = "\
patient_id,code,value,unit,taken_at,source\
\ndave,HBA1C,not-a-number,%,2026-09-01,upload\n";
    let (status, body) = post_text("/api/v1/observations", bad.to_string()).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["error"].is_string());
}
