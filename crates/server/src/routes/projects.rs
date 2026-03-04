use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use std::sync::Arc;
use composer_api_types::*;
use crate::AppState;
use crate::error::ServiceError;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/projects", get(list_projects).post(create_project))
        .route("/projects/{id}", get(get_project).put(update_project).delete(delete_project))
        .route("/projects/{id}/repositories", get(list_repositories).post(add_repository))
        .route("/projects/{id}/repositories/{repo_id}", put(update_repository).delete(remove_repository))
        .route("/projects/{id}/tasks", get(list_project_tasks))
}

async fn list_projects(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Project>>, ServiceError> {
    let projects = state.services.projects.list_all().await?;
    Ok(Json(projects))
}

async fn create_project(State(state): State<Arc<AppState>>, Json(req): Json<CreateProjectRequest>) -> Result<Json<Project>, ServiceError> {
    let project = state.services.projects.create(req).await?;
    Ok(Json(project))
}

async fn get_project(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Project>, ServiceError> {
    let project = state.services.projects.get(&id).await?
        .ok_or_else(|| ServiceError::NotFound(format!("Project {} not found", id)))?;
    Ok(Json(project))
}

async fn update_project(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<UpdateProjectRequest>) -> Result<Json<Project>, ServiceError> {
    let project = state.services.projects.update(&id, req).await?;
    Ok(Json(project))
}

async fn delete_project(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<(), ServiceError> {
    state.services.projects.delete(&id).await?;
    Ok(())
}

async fn list_repositories(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Vec<ProjectRepository>>, ServiceError> {
    let repos = state.services.projects.list_repositories(&id).await?;
    Ok(Json(repos))
}

async fn add_repository(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<AddProjectRepositoryRequest>) -> Result<Json<ProjectRepository>, ServiceError> {
    let repo = state.services.projects.add_repository(&id, req).await?;
    Ok(Json(repo))
}

async fn update_repository(
    State(state): State<Arc<AppState>>,
    Path((id, repo_id)): Path<(String, String)>,
    Json(req): Json<UpdateProjectRepositoryRequest>,
) -> Result<Json<ProjectRepository>, ServiceError> {
    state.services.projects.get(&id).await?
        .ok_or_else(|| ServiceError::NotFound(format!("Project {} not found", id)))?;
    let repo = state.services.projects.update_repository(&id, &repo_id, req).await?;
    Ok(Json(repo))
}

async fn remove_repository(
    State(state): State<Arc<AppState>>,
    Path((id, repo_id)): Path<(String, String)>,
) -> Result<(), ServiceError> {
    state.services.projects.remove_repository(&id, &repo_id).await?;
    Ok(())
}

async fn list_project_tasks(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Vec<Task>>, ServiceError> {
    state.services.projects.get(&id).await?
        .ok_or_else(|| ServiceError::NotFound(format!("Project {} not found", id)))?;
    let tasks = state.services.projects.list_tasks(&id).await?;
    Ok(Json(tasks))
}
