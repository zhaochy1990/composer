use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use composer_api_types::*;
use crate::AppState;
use crate::error::ServiceError;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/worktrees", get(list_worktrees))
        .route("/worktrees/{id}/cleanup", post(cleanup_worktree))
}

async fn list_worktrees(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Worktree>>, ServiceError> {
    let worktrees = state.services.worktrees.list_all().await?;
    Ok(Json(worktrees))
}

async fn cleanup_worktree(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<(), ServiceError> {
    tracing::info!(worktree_id = %id, "API: cleanup worktree");
    state.services.worktrees.cleanup(&id).await?;
    Ok(())
}
