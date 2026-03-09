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
        .route("/sessions/{id}/complete", post(complete_session))
        .route("/sessions/{id}/resume", post(resume_session))
        .route("/sessions/{id}/input", post(send_session_input))
        .route("/sessions/{id}/retry", post(retry_session))
        .route("/sessions/{id}/answer-question", post(answer_question))
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

async fn complete_session(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Session>, ServiceError> {
    tracing::info!(session_id = %id, "API: complete session (close stdin)");
    let session = state.services.sessions.complete_session(&id).await?;
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
struct AnswerQuestionRequest {
    request_id: String,
    answers: serde_json::Value,
}

async fn answer_question(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<AnswerQuestionRequest>) -> Result<(), ServiceError> {
    tracing::info!(session_id = %id, request_id = %req.request_id, "API: answer user question");
    state.services.sessions.answer_question(&id, req.request_id, req.answers).await?;
    Ok(())
}

#[derive(Deserialize)]
struct LogsQuery {
    before: Option<i64>,
    limit: Option<i64>,
}

async fn get_session_logs(State(state): State<Arc<AppState>>, Path(id): Path<String>, Query(query): Query<LogsQuery>) -> Result<Json<PaginatedSessionLogs>, ServiceError> {
    let limit = query.limit.unwrap_or(500).min(2000);
    let logs = state.services.sessions.get_logs_cursor(&id, query.before, limit).await?;
    let total_count = state.services.sessions.get_log_count(&id).await?;
    let oldest_id = logs.first().map(|l| l.id);
    let has_more = match oldest_id {
        Some(oid) => state.services.sessions.has_logs_before(&id, oid).await?,
        None => false,
    };
    Ok(Json(PaginatedSessionLogs {
        logs,
        has_more,
        oldest_id,
        total_count,
    }))
}
