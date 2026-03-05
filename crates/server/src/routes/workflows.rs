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
        .route("/workflows", get(list_workflows).post(create_workflow))
        .route("/workflows/{id}", get(get_workflow).put(update_workflow).delete(delete_workflow))
        .route("/tasks/{id}/start-workflow", post(start_workflow))
        .route("/workflow-runs/{id}", get(get_workflow_run))
        .route("/workflow-runs/{id}/decision", post(submit_decision))
        .route("/workflow-runs/{id}/resume", post(resume_workflow_run))
        .route("/workflow-runs/{id}/steps", get(list_step_outputs))
}

async fn create_workflow(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateWorkflowRequest>,
) -> Result<Json<Workflow>, ServiceError> {
    let workflow = composer_db::models::workflow::create(
        &state.services.workflows.db().pool,
        &req.name,
        &req.definition,
    ).await?;
    Ok(Json(workflow))
}

async fn list_workflows(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Workflow>>, ServiceError> {
    let workflows = composer_db::models::workflow::list_all(
        &state.services.workflows.db().pool,
    ).await?;
    Ok(Json(workflows))
}

async fn get_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Workflow>, ServiceError> {
    let workflow = composer_db::models::workflow::find_by_id(
        &state.services.workflows.db().pool,
        &id,
    ).await?
    .ok_or_else(|| ServiceError::NotFound(format!("Workflow {} not found", id)))?;
    Ok(Json(workflow))
}

async fn update_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateWorkflowRequest>,
) -> Result<Json<Workflow>, ServiceError> {
    let workflow = composer_db::models::workflow::update(
        &state.services.workflows.db().pool,
        &id,
        req.name.as_deref(),
        req.definition.as_ref(),
    ).await?;
    Ok(Json(workflow))
}

async fn delete_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<(), ServiceError> {
    composer_db::models::workflow::delete(
        &state.services.workflows.db().pool,
        &id,
    ).await?;
    Ok(())
}

async fn start_workflow(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
    Json(req): Json<StartWorkflowRequest>,
) -> Result<Json<WorkflowRun>, ServiceError> {
    let task = state.services.tasks.get(&task_id).await?
        .ok_or_else(|| ServiceError::NotFound(format!("Task {} not found", task_id)))?;
    if !matches!(task.status, TaskStatus::Backlog) {
        return Err(ServiceError::BadRequest("Task must be in backlog to start a workflow".into()));
    }
    if task.assigned_agent_id.is_none() {
        return Err(ServiceError::BadRequest("Task has no assigned agent".into()));
    }
    if task.project_id.is_none() {
        return Err(ServiceError::BadRequest("Task has no project assigned".into()));
    }

    let run = state.services.workflows.start(&task_id, &req.workflow_id.to_string()).await?;
    Ok(Json(run))
}

async fn get_workflow_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowRun>, ServiceError> {
    let (run, _steps) = state.services.workflows.get_run_with_steps(&id).await?;
    Ok(Json(run))
}

async fn submit_decision(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<WorkflowDecisionRequest>,
) -> Result<Json<WorkflowRun>, ServiceError> {
    let run = state.services.workflows.submit_decision(
        &id,
        req.approved,
        req.comments.as_deref(),
    ).await?;
    Ok(Json(run))
}

async fn resume_workflow_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowRun>, ServiceError> {
    let run = state.services.workflows.resume_run(&id).await?;
    Ok(Json(run))
}

async fn list_step_outputs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<WorkflowStepOutput>>, ServiceError> {
    let (_run, steps) = state.services.workflows.get_run_with_steps(&id).await?;
    Ok(Json(steps))
}
