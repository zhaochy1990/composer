use axum::{Router, routing::get, Json};
use std::sync::Arc;
use crate::AppState;
use serde_json::json;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health_check))
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
