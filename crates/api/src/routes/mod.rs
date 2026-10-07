//! Router assembly. One module per resource; JSON shaping in `views`.

pub mod biomarkers;
pub mod explain;
pub mod explain_stream;
pub mod health;
pub mod observations;
pub mod patients;
pub mod reviews;
mod views;

use axum::Router;
use axum::middleware;
use axum::routing::{get, post};

use crate::request_id::request_id;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/v1/observations", post(observations::upload))
        .route("/api/v1/llm/models", get(explain_stream::models))
        .route("/api/v1/explain", post(explain::explain))
        .route(
            "/api/v1/explain/stream",
            post(explain_stream::explain_stream),
        )
        .route("/api/v1/patients", get(patients::list))
        .route(
            "/api/v1/patients/{patient_id}/summary",
            get(patients::summary),
        )
        .route(
            "/api/v1/patients/{patient_id}/biomarkers/{code}",
            get(biomarkers::series),
        )
        .route(
            "/api/v1/patients/{patient_id}/biomarkers/{code}/reviews",
            get(reviews::history).post(reviews::create),
        )
        .with_state(state)
        .layer(middleware::from_fn(request_id))
}
