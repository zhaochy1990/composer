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
