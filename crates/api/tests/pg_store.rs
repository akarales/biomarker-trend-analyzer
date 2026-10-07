//! Postgres store against a real database (CI job with a postgres:17
//! service; locally `docker compose up -d db`). Run with:
//!   DATABASE_URL=postgresql://app:app@127.0.0.1:5435/biomarkers \
//!     cargo test -p biomarker-api --features pg-tests --test pg_store
//! `#[sqlx::test]` creates a fresh database per test and applies the migrations.
#![cfg(feature = "pg-tests")]

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

use biomarker_api::config::{Config, StoreBackend};
use biomarker_api::routes;
use biomarker_api::state::AppState;
use biomarker_api::store::{NewReviewEvent, ReviewAction, Store, postgres::PgStore};
use biomarker_ingest::Observation;

fn fixture() -> Vec<Observation> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo_labs.csv");
    let csv = std::fs::read_to_string(&path).expect("fixture exists");
    biomarker_ingest::parse_csv_bytes(csv.as_bytes()).expect("fixture parses")
}

fn app(store: Store) -> axum::Router {
    routes::router(AppState::new(
        Arc::new(store),
        Config {
            store: StoreBackend::Postgres,
            database_url: None,
            seed_demo_data: true,
            demo_data: PathBuf::new(),
            port: 0,
            window_days: 365,
            llm: Default::default(),
        },
    ))
}

async fn get(app: &axum::Router, path: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(Request::get(path).body(Body::empty()).expect("request"))
        .await
        .expect("in-process request");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body reads");
    (status, serde_json::from_slice(&bytes).expect("json body"))
}

#[sqlx::test(migrations = "./migrations")]
async fn reseeding_is_idempotent(pool: PgPool) {
    let store = Store::Postgres(PgStore::from_pool(pool));
    let observations = fixture();

    let first = store
        .insert_observations(&observations)
        .await
        .expect("seed");
    assert_eq!((first.inserted, first.duplicates), (576, 0));
    // a restart re-runs the seed: identical counts, nothing duplicated
    let second = store
        .insert_observations(&observations)
        .await
        .expect("reseed");
    assert_eq!((second.inserted, second.duplicates), (0, 576));

    let listing = store.listing().await.expect("listing");
    assert_eq!(listing.len(), 3);
    assert!(
        listing
            .iter()
            .all(|p| p.observations == 192 && p.biomarkers == 4)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn series_round_trips_timestamps(pool: PgPool) {
    let store = Store::Postgres(PgStore::from_pool(pool));
    let observations = fixture();
    store
        .insert_observations(&observations)
        .await
        .expect("seed");

    let series = store.series("alice", "HBA1C").await.expect("series");
    let expected: Vec<_> = observations
        .iter()
        .filter(|o| o.patient_id == "alice" && o.code == "HBA1C")
        .collect();
    assert_eq!(series.len(), expected.len());
    for (got, want) in series.iter().zip(expected) {
        assert_eq!(got.taken_at, want.taken_at);
        assert_eq!(got.value, want.value);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn api_summary_works_over_postgres(pool: PgPool) {
    let store = Store::Postgres(PgStore::from_pool(pool));
    store.insert_observations(&fixture()).await.expect("seed");
    let app = app(store);

    let (status, body) = get(&app, "/api/v1/patients/alice/summary").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let hba1c = body["reports"]
        .as_array()
        .expect("reports")
        .iter()
        .find(|r| r["code"] == "HBA1C")
        .expect("HBA1C report");
    assert_eq!(hba1c["status"], "alert");

    let (status, body) = get(&app, "/api/v1/patients/alice/biomarkers/HBA1C").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["observations"].as_array().expect("obs").len(), 48);

    let (status, body) = get(&app, "/api/v1/patients/ghost/summary").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");
}

#[sqlx::test(migrations = "./migrations")]
async fn review_events_round_trip_and_are_append_only(pool: PgPool) {
    let store = PgStore::from_pool(pool.clone());
    let snapshot =
        serde_json::json!({ "signal": { "rule": "prri", "severity": "alert" }, "status": "alert" });
    let stored = store
        .append_review(NewReviewEvent {
            patient_id: "alice".into(),
            code: "HBA1C".into(),
            rule: "prri".into(),
            signal_t: 1_790_000_000,
            action: ReviewAction::Dismiss,
            reason: Some("haemolysed sample, repeat normal".into()),
            snapshot: snapshot.clone(),
        })
        .await
        .expect("append");
    assert_eq!(stored.actor, "demo-clinician");
    let read = store.reviews("alice", Some("HBA1C")).await.expect("read");
    assert_eq!(read, vec![stored.clone()]);
    assert_eq!(read[0].snapshot, snapshot);
    assert_eq!(read[0].signal_t, 1_790_000_000);
    assert!(
        store
            .reviews("alice", Some("LDL"))
            .await
            .expect("read")
            .is_empty()
    );

    for statement in [
        "UPDATE review_events SET reason = 'edited later'",
        "DELETE FROM review_events",
        "TRUNCATE review_events",
    ] {
        let err = sqlx::query(statement)
            .execute(&pool)
            .await
            .expect_err(statement);
        assert!(
            err.to_string().contains("append-only"),
            "{statement}: {err}"
        );
    }
    // the database itself refuses a dismissal without a reason
    let err = sqlx::query(
        "INSERT INTO review_events (patient_id, code, rule, signal_t, action, snapshot) \
         VALUES ('alice', 'HBA1C', 'prri', now(), 'dismiss', '{}')",
    )
    .execute(&pool)
    .await
    .expect_err("reason required");
    assert!(
        err.to_string().contains("review_events_reason_required"),
        "{err}"
    );
    assert_eq!(store.reviews("alice", None).await.expect("read").len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn api_review_works_over_postgres(pool: PgPool) {
    let store = Store::Postgres(PgStore::from_pool(pool));
    store.insert_observations(&fixture()).await.expect("seed");
    let app = app(store);
    let (_, series) = get(&app, "/api/v1/patients/alice/biomarkers/HBA1C").await;
    let signal = &series["report"]["signals"][0];
    let request = Request::post("/api/v1/patients/alice/biomarkers/HBA1C/reviews")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({ "rule": signal["rule"], "t": signal["t"], "action": "acknowledge" }).to_string(),
        ))
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("request");
    assert_eq!(response.status(), StatusCode::CREATED);
    let (_, series) = get(&app, "/api/v1/patients/alice/biomarkers/HBA1C").await;
    assert_eq!(series["reviews"][0]["state"], "acknowledged");
    assert_eq!(series["history"][0]["snapshot"]["status"], "alert");
    let (_, listing) = get(&app, "/api/v1/patients").await;
    assert!(
        listing["patients"]
            .as_array()
            .expect("patients")
            .iter()
            .all(|p| p["unreviewed"].is_u64())
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn demographics_upsert_and_round_trip(pool: PgPool) {
    use biomarker_api::store::Demographics;
    let store = PgStore::from_pool(pool.clone());
    let row = |gender: Option<&str>, year: Option<i32>| Demographics {
        patient_id: "SYN-01".into(),
        gender: gender.map(str::to_string),
        birth_year: year,
    };
    store
        .upsert_demographics(&[row(Some("female"), Some(1963))])
        .await
        .expect("insert");
    assert_eq!(
        store.demographics("SYN-01").await.expect("read"),
        Some(row(Some("female"), Some(1963)))
    );
    store
        .upsert_demographics(&[row(None, Some(1964))])
        .await
        .expect("update");
    assert_eq!(
        store.demographics("SYN-01").await.expect("read"),
        Some(row(None, Some(1964)))
    );
    assert_eq!(store.demographics("nobody").await.expect("read"), None);
    let err =
        sqlx::query("INSERT INTO patient_demographics (patient_id, gender) VALUES ('X', 'robot')")
            .execute(&pool)
            .await
            .expect_err("gender check");
    assert!(
        err.to_string()
            .contains("patient_demographics_gender_check"),
        "{err}"
    );
}
