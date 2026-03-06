use axum::{
    extract::{Path, State, Query},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use composer_api_types::*;
use crate::AppState;
use crate::error::ServiceError;
use serde::Deserialize;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sessions", get(list_sessions).post(create_session))
        .route("/sessions/{id}", get(get_session))
        .route("/sessions/{id}/interrupt", post(interrupt_session))
        .route("/sessions/{id}/resume", post(resume_session))
        .route("/sessions/{id}/input", post(send_session_input))
        .route("/sessions/{id}/retry", post(retry_session))
        .route("/sessions/{id}/logs", get(get_session_logs))
}

async fn list_sessions(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Session>>, ServiceError> {
    let sessions = state.services.sessions.list_all().await?;
    Ok(Json(sessions))
}

async fn create_session(State(state): State<Arc<AppState>>, Json(req): Json<CreateSessionRequest>) -> Result<Json<Session>, ServiceError> {
    tracing::info!(task_id = %req.task_id, agent_id = %req.agent_id, "API: create session");
    let session = state.services.sessions.create_session(req).await?;
    Ok(Json(session))
}

async fn get_session(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Session>, ServiceError> {
    let session = state.services.sessions.get(&id).await?
        .ok_or_else(|| ServiceError::NotFound(format!("Session {} not found", id)))?;
    Ok(Json(session))
}

async fn interrupt_session(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Session>, ServiceError> {
    tracing::info!(session_id = %id, "API: interrupt session");
    let session = state.services.sessions.interrupt(&id).await?;
    Ok(Json(session))
}

async fn resume_session(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<ResumeSessionRequest>) -> Result<Json<Session>, ServiceError> {
    tracing::info!(session_id = %id, "API: resume session");
    let session = state.services.sessions.resume_session(&id, req).await?;
    Ok(Json(session))
}

async fn send_session_input(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<SendSessionInputRequest>) -> Result<(), ServiceError> {
    state.services.sessions.send_input(&id, req.message).await?;
    Ok(())
}

async fn retry_session(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<ResumeSessionRequest>) -> Result<Json<Session>, ServiceError> {
    tracing::info!(session_id = %id, "API: retry session");
    let session = state.services.sessions.retry_session(&id, req).await?;
    Ok(Json(session))
}

#[derive(Deserialize)]
struct LogsQuery {
    since: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn get_session_logs(State(state): State<Arc<AppState>>, Path(id): Path<String>, Query(query): Query<LogsQuery>) -> Result<Json<Vec<SessionLog>>, ServiceError> {
    let logs = state.services.sessions.get_logs(&id, query.since.as_deref(), query.limit, query.offset).await?;
    Ok(Json(logs))
}
