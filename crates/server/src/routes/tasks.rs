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
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/{id}", get(get_task).put(update_task).delete(delete_task))
        .route("/tasks/{id}/assign", post(assign_task))
        .route("/tasks/{id}/move", post(move_task))
        .route("/tasks/{id}/start", post(start_task))
        .route("/tasks/{id}/sessions", get(list_task_sessions))
}

async fn list_tasks(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Task>>, ServiceError> {
    let tasks = state.services.tasks.list_all().await?;
    Ok(Json(tasks))
}

async fn create_task(State(state): State<Arc<AppState>>, Json(req): Json<CreateTaskRequest>) -> Result<Json<Task>, ServiceError> {
    let task = state.services.tasks.create(req).await?;
    Ok(Json(task))
}

async fn get_task(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Task>, ServiceError> {
    let task = state.services.tasks.get(&id).await?
        .ok_or_else(|| ServiceError::NotFound(format!("Task {} not found", id)))?;
    Ok(Json(task))
}

async fn update_task(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<UpdateTaskRequest>) -> Result<Json<Task>, ServiceError> {
    let task = state.services.tasks.update(&id, req).await?;
    Ok(Json(task))
}

async fn delete_task(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<(), ServiceError> {
    state.services.tasks.delete(&id).await?;
    Ok(())
}

async fn assign_task(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<AssignTaskRequest>) -> Result<Json<Task>, ServiceError> {
    let task = state.services.tasks.assign_agent(&id, &req.agent_id.to_string()).await?;
    Ok(Json(task))
}

async fn move_task(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<MoveTaskRequest>) -> Result<Json<Task>, ServiceError> {
    let task = state.services.tasks.move_task(&id, req).await?;
    Ok(Json(task))
}

async fn start_task(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<StartTaskResponse>, ServiceError> {
    // Validate preconditions with proper HTTP status codes
    let task = state.services.tasks.get(&id).await?
        .ok_or_else(|| ServiceError::NotFound(format!("Task {} not found", id)))?;
    if !matches!(task.status, TaskStatus::Backlog) {
        return Err(ServiceError::BadRequest("Task must be in backlog to start".into()));
    }
    if task.assigned_agent_id.is_none() {
        return Err(ServiceError::BadRequest("Task has no assigned agent".into()));
    }
    if task.repo_path.is_none() {
        return Err(ServiceError::BadRequest("Task has no repo_path configured".into()));
    }

    let response = state.services.tasks.start_task(&id).await?;
    Ok(Json(response))
}

async fn list_task_sessions(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Session>>, ServiceError> {
    state.services.tasks.get(&id).await?
        .ok_or_else(|| ServiceError::NotFound(format!("Task {} not found", id)))?;
    let sessions = state.services.sessions.list_by_task(&id).await?;
    Ok(Json(sessions))
}
