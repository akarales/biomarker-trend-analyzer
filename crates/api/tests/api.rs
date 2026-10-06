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

fn report<'a>(summary: &'a Value, code: &str) -> &'a Value {
    summary["reports"]
        .as_array()
        .expect("reports")
        .iter()
        .find(|r| r["code"] == code)
        .unwrap_or_else(|| panic!("{code} report present"))
}

fn rules(report: &Value) -> Vec<String> {
    report["signals"]
        .as_array()
        .expect("signals")
        .iter()
        .map(|s| {
            format!(
                "{}:{}",
                s["rule"].as_str().unwrap_or("?"),
                s["severity"].as_str().unwrap_or("?")
            )
        })
        .collect()
}

/// M2 acceptance: alice's HbA1c 5.6 → 7.0 % is flagged by the prRI and the
/// ADA diabetes threshold whatever the window (v1: `watch`, z 0.52 at 90 days).
#[tokio::test]
async fn alice_hba1c_step_alerts_for_every_window() {
    for window in [30, 90, 365] {
        let (status, body) = get(&format!(
            "/api/v1/patients/alice/summary?window_days={window}"
        ))
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["window_days"], window);
        let hba1c = report(&body, "HBA1C");
        assert_eq!(hba1c["status"], "alert", "window {window}");
        let fired = rules(hba1c);
        assert!(fired.contains(&"prri:alert".to_string()), "{fired:?}");
        assert!(fired.contains(&"threshold:alert".to_string()), "{fired:?}");
        assert_eq!(hba1c["analyte"]["loinc"], "4548-4");
        assert!(hba1c["change_point"]["t"].is_i64());
    }
}

/// M2 acceptance: bob's creatinine noise (≈ 3 %, below CVI 4.4 %) is normal
/// (v1: `watch`, z 2.7).
#[tokio::test]
async fn bob_creatinine_noise_is_normal() {
    let (_, body) = get("/api/v1/patients/bob/summary").await;
    let creat = report(&body, "CREAT");
    assert_eq!(creat["status"], "normal", "{:?}", rules(creat));
    assert!(creat["signals"].as_array().expect("signals").is_empty());
}

#[tokio::test]
async fn bob_ldl_rising_trend_detected() {
    let (_, body) = get("/api/v1/patients/bob/summary").await;
    let ldl = report(&body, "LDL");
    assert_eq!(
        ldl["trend"]["direction"], "rising",
        "gradual rise must be detected"
    );
    assert!(rules(ldl).contains(&"trend:watch".to_string()));
    assert_eq!(ldl["status"], "watch");
}

#[tokio::test]
async fn carol_control_has_no_alerts() {
    let (_, body) = get("/api/v1/patients/carol/summary").await;
    for r in body["reports"].as_array().expect("reports") {
        assert_ne!(r["status"], "alert", "{}: {:?}", r["code"], rules(r));
    }
}

/// M2 acceptance: results are identical whatever day the app runs — the
/// analysis date is explicit (default: the latest result).
#[tokio::test]
async fn as_of_is_explicit_and_excludes_later_results() {
    let (_, latest) = get("/api/v1/patients/alice/summary").await;
    assert!(latest["as_of"].is_null());
    let hba1c = report(&latest, "HBA1C");
    assert_eq!(hba1c["excluded"]["after_as_of"], 0);
    assert_eq!(hba1c["points"].as_array().expect("points").len(), 48);

    // before alice's step (2026-09-15) nothing alerts
    let (status, before) = get("/api/v1/patients/alice/summary?as_of=2026-08-31").await;
    assert_eq!(status, StatusCode::OK);
    assert!(before["as_of"].is_i64());
    let hba1c = report(&before, "HBA1C");
    assert_eq!(hba1c["status"], "normal", "{:?}", rules(hba1c));
    assert!(hba1c["excluded"]["after_as_of"].as_u64().expect("count") > 0);
}

#[tokio::test]
async fn invalid_analysis_params_are_400() {
    for query in ["window_days=1", "window_days=abc", "as_of=yesterday"] {
        let (status, body) = get(&format!("/api/v1/patients/alice/summary?{query}")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(body["code"], "bad_request");
    }
}

#[tokio::test]
async fn series_returns_observations_and_report() {
    let (status, body) = get("/api/v1/patients/alice/biomarkers/HBA1C").await;
    assert_eq!(status, StatusCode::OK);
    let observations = body["observations"].as_array().expect("observations");
    assert_eq!(observations.len(), 48);
    assert_eq!(observations[0]["taken_at"], "2026-01-06T00:00:00Z");
    assert_eq!(body["code"], "HBA1C");
    let report = &body["report"];
    assert!(report["baseline"]["prri_low"].as_f64().expect("prri") < 5.6);
    assert!(report["rcv"]["up"].as_f64().expect("rcv") > 0.0);
    assert_eq!(
        report["thresholds"].as_array().expect("thresholds").len(),
        2
    );
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
