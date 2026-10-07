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
        demo_data: PathBuf::new(),
        port: 0,
        window_days: 365,
        llm: Default::default(),
    };
    routes::router(AppState::new(Arc::new(store), config))
}

async fn get(path: &str) -> (StatusCode, Value) {
    get_from(&test_app().await, path).await
}

async fn get_from(app: &axum::Router, path: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
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
    post_typed(app, path, "text/csv", body).await
}

async fn post_typed(
    app: &axum::Router,
    path: &str,
    content_type: &str,
    body: String,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::post(path)
                .header("content-type", content_type)
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
async fn listing_is_triaged_worst_first_and_honours_as_of() {
    let (status, body) = get("/api/v1/patients").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["window_days"], 365);
    let first = &body["patients"][0];
    assert_eq!(first["patient_id"], "alice");
    assert_eq!(first["status"], "alert");
    assert_eq!(first["alerts"], 1);
    assert_eq!(first["observations"], 192);
    assert_eq!(first["top_signal"]["code"], "HBA1C");
    assert_eq!(first["top_signal"]["severity"], "alert");
    assert!(
        first["top_signal"]["explanation"]
            .as_str()
            .expect("text")
            .contains("Hemoglobin A1c")
    );

    // before alice's step she is not alerting
    let (_, before) = get("/api/v1/patients?as_of=2026-08-31").await;
    let alice = before["patients"]
        .as_array()
        .expect("patients")
        .iter()
        .find(|p| p["patient_id"] == "alice")
        .expect("alice");
    assert_ne!(alice["status"], "alert");
    let (status, _) = get("/api/v1/patients?window_days=2").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
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

fn fhir_bundle(observations: &[(&str, &str, f64, &str, &str)]) -> String {
    let entries: Vec<Value> = observations
        .iter()
        .map(|&(loinc, status, value, unit, when)| {
            serde_json::json!({ "resource": {
                "resourceType": "Observation", "status": status,
                "code": { "coding": [{ "system": "http://loinc.org", "code": loinc }] },
                "subject": { "reference": "Patient/FHIR-1" },
                "effectiveDateTime": when,
                "valueQuantity": { "value": value, "unit": unit, "system": "http://unitsofmeasure.org", "code": unit }
            }})
        })
        .collect();
    let mut all =
        vec![serde_json::json!({ "resource": { "resourceType": "Patient", "id": "FHIR-1" } })];
    all.extend(entries);
    serde_json::json!({ "resourceType": "Bundle", "type": "collection", "entry": all }).to_string()
}

#[tokio::test]
async fn fhir_bundle_upload_is_mapped_filtered_and_idempotent() {
    let app = test_app().await;
    let bundle = fhir_bundle(&[
        ("4548-4", "final", 6.1, "%", "2026-03-01T09:00:00Z"),
        ("4548-4", "final", 44.0, "mmol/mol", "2026-06-01T09:00:00Z"),
        ("2345-7", "final", 99.0, "mg/dL", "2026-03-01T09:00:00Z"),
        (
            "4548-4",
            "entered-in-error",
            9.9,
            "%",
            "2026-04-01T09:00:00Z",
        ),
    ]);
    let (status, body) = post_typed(
        &app,
        "/api/v1/observations",
        "application/fhir+json",
        bundle.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["format"], "fhir");
    assert_eq!(body["inserted"], 2);
    assert_eq!(body["skipped"], 2);
    assert_eq!(body["biomarkers"], serde_json::json!(["HBA1C"]));
    let reasons: Vec<&str> = body["skipped_reasons"]
        .as_array()
        .expect("reasons")
        .iter()
        .filter_map(|r| r["reason"].as_str())
        .collect();
    assert!(
        reasons.contains(&"no analyte profile for LOINC 2345-7"),
        "{reasons:?}"
    );
    assert!(
        reasons.iter().any(|r| r.contains("entered-in-error")),
        "{reasons:?}"
    );

    // same bundle again (content sniffed without a JSON content type)
    let (_, again) = post_typed(&app, "/api/v1/observations", "text/plain", bundle).await;
    assert_eq!(
        (again["format"].as_str(), again["inserted"].as_u64()),
        (Some("fhir"), Some(0))
    );

    // stored under the profile code, IFCC unit normalised by the engine
    let response = app
        .oneshot(
            Request::get("/api/v1/patients/FHIR-1/biomarkers/HBA1C")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("in-process request");
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let series: Value = serde_json::from_slice(&bytes).expect("json");
    let points = series["report"]["points"].as_array().expect("points");
    assert_eq!(points.len(), 2);
    assert!(
        (points[1]["v"].as_f64().expect("v") - 6.18).abs() < 0.02,
        "44 mmol/mol ≈ 6.2 %"
    );
}

#[tokio::test]
async fn fhir_upload_errors_have_codes() {
    let (status, body) = post_typed(
        &test_app().await,
        "/api/v1/observations",
        "application/json",
        "{not json".into(),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "invalid_fhir");

    let only_untracked = fhir_bundle(&[("2345-7", "final", 99.0, "mg/dL", "2026-03-01")]);
    let (status, body) = post_typed(
        &test_app().await,
        "/api/v1/observations",
        "application/fhir+json",
        only_untracked,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"]
            .as_str()
            .expect("msg")
            .contains("1 skipped: no analyte profile for LOINC 2345-7"),
        "{body}"
    );
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

#[tokio::test]
async fn csv_only_patients_say_why_kidney_staging_is_missing() {
    let app = test_app().await;
    let (_, summary) = get_from(&app, "/api/v1/patients/alice/summary").await;
    assert!(summary["demographics"].is_null());
    let reports = summary["reports"].as_array().expect("reports");
    assert!(
        reports.iter().all(|r| r["code"] != "EGFR"),
        "no eGFR without sex and birth year"
    );
    let creat = reports
        .iter()
        .find(|r| r["code"] == "CREAT")
        .expect("creat");
    let reason = creat["not_assessed"]
        .as_array()
        .expect("na")
        .iter()
        .find(|n| n["rule"] == "threshold")
        .and_then(|n| n["reason"].as_str())
        .expect("reason");
    assert!(
        reason.contains("needs the patient's sex and birth year"),
        "{reason}"
    );
    let (status, _) = get_from(&app, "/api/v1/patients/alice/biomarkers/EGFR").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn fhir_patient_demographics_enable_egfr_and_can_be_corrected() {
    let app = test_app().await;
    let bundle = |gender: &str| {
        let mut entries = vec![serde_json::json!({ "resource": {
            "resourceType": "Patient", "id": "KID-1", "gender": gender, "birthDate": "1960-06-01" } })];
        for (i, scr) in [1.0, 1.05, 1.1, 1.2, 1.4, 1.7].iter().enumerate() {
            entries.push(serde_json::json!({ "resource": {
                "resourceType": "Observation", "status": "final",
                "code": { "coding": [{ "system": "http://loinc.org", "code": "2160-0" }] },
                "subject": { "reference": "Patient/KID-1" },
                "effectiveDateTime": format!("202{}-03-01", i + 1),
                "valueQuantity": { "value": scr, "unit": "mg/dL", "system": "http://unitsofmeasure.org", "code": "mg/dL" }
            }}));
        }
        serde_json::json!({ "resourceType": "Bundle", "type": "collection", "entry": entries })
            .to_string()
    };
    let (status, body) = post_typed(
        &app,
        "/api/v1/observations",
        "application/fhir+json",
        bundle("female"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["demographics"], 1);
    let (_, female) = get_from(&app, "/api/v1/patients/KID-1/biomarkers/EGFR").await;
    let latest_f = female["report"]["latest"]["v"].as_f64().expect("eGFR");
    // 2026: 66 years, female, Scr 1.7 → CKD-EPI 2021
    assert!((latest_f - 32.9).abs() < 0.2, "{latest_f}");

    // re-import with a corrected gender: same observations (duplicates), new demographics
    let (_, body) = post_typed(
        &app,
        "/api/v1/observations",
        "application/fhir+json",
        bundle("male"),
    )
    .await;
    assert_eq!(
        (body["inserted"].as_u64(), body["demographics"].as_u64()),
        (Some(0), Some(1))
    );
    let (_, male) = get_from(&app, "/api/v1/patients/KID-1/biomarkers/EGFR").await;
    assert!(
        male["report"]["latest"]["v"].as_f64().expect("eGFR") > latest_f,
        "male eGFR is higher at the same creatinine"
    );
    assert_eq!(male["derived"]["gender"], "male");
}
