use axum::{
    extract::{Path, State, Query},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use composer_api_types::*;
use crate::AppState;
use crate::error::AppError;
use serde::Deserialize;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sessions", get(list_sessions).post(create_session))
        .route("/sessions/{id}", get(get_session))
        .route("/sessions/{id}/interrupt", post(interrupt_session))
        .route("/sessions/{id}/resume", post(resume_session))
        .route("/sessions/{id}/logs", get(get_session_logs))
}

async fn list_sessions(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Session>>, AppError> {
    let sessions = state.services.sessions.list_all().await?;
    Ok(Json(sessions))
}

async fn create_session(State(state): State<Arc<AppState>>, Json(req): Json<CreateSessionRequest>) -> Result<Json<Session>, AppError> {
    let session = state.services.sessions.create_session(req).await?;
    Ok(Json(session))
}

async fn get_session(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Session>, AppError> {
    let session = state.services.sessions.get(&id).await?
        .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
    Ok(Json(session))
}

async fn interrupt_session(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Session>, AppError> {
    let session = state.services.sessions.interrupt(&id).await?;
    Ok(Json(session))
}

async fn resume_session(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<ResumeSessionRequest>) -> Result<Json<Session>, AppError> {
    let session = state.services.sessions.resume_session(&id, req).await?;
    Ok(Json(session))
}

#[derive(Deserialize)]
struct LogsQuery {
    since: Option<String>,
}

async fn get_session_logs(State(state): State<Arc<AppState>>, Path(id): Path<String>, Query(query): Query<LogsQuery>) -> Result<Json<Vec<SessionLog>>, AppError> {
    let logs = state.services.sessions.get_logs(&id, query.since.as_deref()).await?;
    Ok(Json(logs))
}
