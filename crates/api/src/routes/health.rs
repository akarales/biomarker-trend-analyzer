//! Liveness endpoint.

use axum::Json;
use axum::extract::State;
use serde_json::json;

use crate::state::AppState;

pub async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "store": state.store.backend_name(),
    }))
}
