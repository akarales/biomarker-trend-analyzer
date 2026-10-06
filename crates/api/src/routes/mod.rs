//! Router assembly.

pub mod biomarkers;
pub mod health;
pub mod observations;

use axum::Router;
use axum::middleware;
use axum::routing::{get, post};

use crate::request_id::request_id;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/v1/observations", post(observations::upload))
        .route("/api/v1/patients", get(biomarkers::patients))
        .route(
            "/api/v1/patients/{patient_id}/summary",
            get(biomarkers::patient_summary),
        )
        .route(
            "/api/v1/patients/{patient_id}/biomarkers/{code}",
            get(biomarkers::biomarker_series),
        )
        .with_state(state)
        .layer(middleware::from_fn(request_id))
}
