//! Liveness endpoint.

use axum::Json;
use axum::extract::State;

use super::views::HealthView;
use crate::state::AppState;

pub async fn health(State(state): State<AppState>) -> Json<HealthView> {
    Json(HealthView {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        store: state.store.backend_name(),
    })
}
