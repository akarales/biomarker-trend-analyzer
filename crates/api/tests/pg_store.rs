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
use biomarker_api::store::{Store, postgres::PgStore};
use biomarker_ingest::Observation;

fn fixture() -> Vec<Observation> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo_labs.csv");
    let csv = std::fs::read_to_string(&path).expect("fixture exists");
    biomarker_ingest::parse_csv_bytes(csv.as_bytes()).expect("fixture parses")
}

fn app(store: Store) -> axum::Router {
    routes::router(AppState {
        store: Arc::new(store),
        config: Arc::new(Config {
            store: StoreBackend::Postgres,
            database_url: None,
            seed_demo_data: true,
            demo_csv: PathBuf::new(),
            port: 0,
            window_days: 365,
        }),
    })
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
