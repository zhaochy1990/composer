pub mod tasks;
pub mod agents;
pub mod sessions;
pub mod worktrees;
pub mod ws;
pub mod frontend;
pub mod health;

use std::sync::Arc;
use axum::Router;
use crate::AppState;

pub fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .merge(health::router())
        .merge(tasks::router())
        .merge(agents::router())
        .merge(sessions::router())
        .merge(worktrees::router())
        .merge(ws::router())
}
