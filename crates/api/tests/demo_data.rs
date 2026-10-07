//! The committed demo data (`demo/`: Synthea subset + hand-made edge
//! cases) seeded the way `main.rs` does it, through the real router.
//! Pins what the demo must show: every detector fires for at least one
//! patient, the edge cases behave as documented in demo/PROVENANCE.md,
//! and re-seeding is a no-op.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use biomarker_api::config::{Config, StoreBackend};
use biomarker_api::import;
use biomarker_api::routes;
use biomarker_api::state::AppState;
use biomarker_api::store::{Store, memory::MemoryStore};

fn demo_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../demo")
}

async fn seeded() -> (axum::Router, Arc<Store>) {
    let store = Arc::new(Store::Memory(MemoryStore::default()));
    for file in import::seed_files(&demo_dir()).expect("demo dir") {
        let parsed = import::parse_file(&file).expect("demo file parses");
        store
            .insert_observations(&parsed.observations)
            .await
            .expect("seed");
    }
    let config = Config {
        store: StoreBackend::Memory,
        database_url: None,
        seed_demo_data: true,
        demo_data: demo_dir(),
        port: 0,
        window_days: 1095,
    };
    let app = routes::router(AppState {
        store: store.clone(),
        config: Arc::new(config),
    });
    (app, store)
}

async fn get(app: &axum::Router, path: &str) -> Value {
    let response = app
        .clone()
        .oneshot(Request::get(path).body(Body::empty()).expect("request"))
        .await
        .expect("in-process request");
    assert_eq!(response.status(), StatusCode::OK, "{path}");
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("json")
}

fn report<'a>(summary: &'a Value, code: &str) -> &'a Value {
    summary["reports"]
        .as_array()
        .expect("reports")
        .iter()
        .find(|r| r["code"] == code)
        .expect("report")
}

fn rules(r: &Value) -> Vec<String> {
    r["signals"]
        .as_array()
        .expect("signals")
        .iter()
        .map(|s| s["rule"].as_str().unwrap_or("?").to_string())
        .collect()
}

#[test]
fn demo_files_are_fhir_bundles_with_provenance() {
    let files = import::seed_files(&demo_dir()).expect("demo dir");
    let names: Vec<_> = files
        .iter()
        .filter_map(|f| f.file_name()?.to_str().map(str::to_owned))
        .collect();
    assert_eq!(names, ["edge-cases.json", "synthea-subset.json"]);
    assert!(demo_dir().join("PROVENANCE.md").is_file());
}

#[tokio::test]
async fn every_detector_fires_somewhere_in_the_demo() {
    let (app, _) = seeded().await;
    let listing = get(&app, "/api/v1/patients").await;
    let ids: Vec<String> = listing["patients"]
        .as_array()
        .expect("patients")
        .iter()
        .map(|p| p["patient_id"].as_str().expect("id").to_string())
        .collect();
    assert_eq!(ids.len(), 10, "{ids:?}");
    let mut fired = BTreeSet::new();
    let mut statuses = BTreeSet::new();
    for id in &ids {
        let summary = get(&app, &format!("/api/v1/patients/{id}/summary")).await;
        for r in summary["reports"].as_array().expect("reports") {
            fired.extend(rules(r));
            statuses.insert(r["status"].as_str().expect("status").to_string());
        }
    }
    let expected: BTreeSet<String> = [
        "prri",
        "rcv",
        "shift",
        "ewma",
        "trend",
        "threshold",
        "population",
    ]
    .map(String::from)
    .into();
    assert_eq!(fired, expected);
    assert_eq!(
        statuses,
        ["alert", "normal", "watch"].map(String::from).into()
    );
}

#[tokio::test]
async fn edge_cases_behave_as_documented() {
    let (app, _) = seeded().await;

    // EDGE-01: TSH rising into the subclinical range
    let tsh = report(&get(&app, "/api/v1/patients/EDGE-01/summary").await, "TSH").clone();
    assert_eq!(tsh["status"], "alert");
    assert!(rules(&tsh).contains(&"threshold".into()) && rules(&tsh).contains(&"prri".into()));

    // EDGE-02: 8 results in µmol/L normalised to mg/dL; trend detected
    let creat = report(
        &get(&app, "/api/v1/patients/EDGE-02/summary").await,
        "CREAT",
    )
    .clone();
    assert_eq!(creat["unit"], "mg/dL");
    assert_eq!(creat["points"].as_array().expect("points").len(), 24);
    assert_eq!(creat["excluded"]["unit_unknown"], 0);
    assert!(
        creat["points"]
            .as_array()
            .expect("points")
            .iter()
            .all(|p| p["v"].as_f64().expect("v") < 2.0)
    );
    assert_eq!(creat["trend"]["direction"], "rising");
    assert!(rules(&creat).contains(&"trend".into()));
    let series = get(&app, "/api/v1/patients/EDGE-02/biomarkers/CREAT").await;
    let units: BTreeSet<&str> = series["observations"]
        .as_array()
        .expect("obs")
        .iter()
        .filter_map(|o| o["unit"].as_str())
        .collect();
    assert_eq!(units, ["mg/dL", "umol/L"].into(), "stored as recorded");

    // EDGE-03: spike repeated twice → quiet latest, no shift, jumps visible,
    // the entered-in-error duplicate was not imported
    let creat = report(
        &get(&app, "/api/v1/patients/EDGE-03/summary").await,
        "CREAT",
    )
    .clone();
    assert_eq!(creat["status"], "normal", "{:?}", rules(&creat));
    assert!(creat["change_point"].is_null());
    assert_eq!(creat["rcv_jumps"].as_array().expect("jumps").len(), 2);
    assert_eq!(creat["points"].as_array().expect("points").len(), 12);

    // EDGE-04: two HbA1c results — no personal baseline, said so
    let hba1c = report(
        &get(&app, "/api/v1/patients/EDGE-04/summary").await,
        "HBA1C",
    )
    .clone();
    let not: Vec<&str> = hba1c["not_assessed"]
        .as_array()
        .expect("na")
        .iter()
        .filter_map(|n| n["rule"].as_str())
        .collect();
    assert!(not.contains(&"prri"), "{not:?}");
    assert_eq!(
        hba1c["status"], "watch",
        "prediabetes threshold still applies"
    );
}

#[tokio::test]
async fn synthea_patients_tell_their_stories() {
    let (app, _) = seeded().await;
    let hba1c = report(&get(&app, "/api/v1/patients/SYN-01/summary").await, "HBA1C").clone();
    assert_eq!(hba1c["status"], "alert");
    assert!(rules(&hba1c).contains(&"threshold".into()));
    let ldl = report(&get(&app, "/api/v1/patients/SYN-03/summary").await, "LDL").clone();
    let prri = &ldl["signals"].as_array().expect("signals")[0];
    assert!(
        prri["explanation"]
            .as_str()
            .expect("text")
            .contains("below"),
        "statin response is below the prRI"
    );
    let creat = report(&get(&app, "/api/v1/patients/SYN-05/summary").await, "CREAT").clone();
    assert_eq!(creat["status"], "normal", "stable CKD is personally normal");
    assert_eq!(
        rules(&creat),
        ["population"],
        "…but outside the population interval (info)"
    );
}

#[tokio::test]
async fn reseeding_the_demo_changes_nothing() {
    let (_, store) = seeded().await;
    for file in import::seed_files(&demo_dir()).expect("demo dir") {
        let parsed = import::parse_file(&file).expect("parse");
        let report = store
            .insert_observations(&parsed.observations)
            .await
            .expect("reseed");
        assert_eq!(report.inserted, 0, "{}", file.display());
    }
}
