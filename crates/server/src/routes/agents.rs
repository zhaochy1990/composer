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
        .route("/agents", get(list_agents).post(create_agent))
        .route("/agents/{id}", get(get_agent).delete(delete_agent))
        .route("/agents/{id}/health", get(agent_health))
        .route("/agents/discover", post(discover_agents))
}

async fn list_agents(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Agent>>, ServiceError> {
    let agents = state.services.agents.list_all().await?;
    Ok(Json(agents))
}

async fn create_agent(State(state): State<Arc<AppState>>, Json(req): Json<CreateAgentRequest>) -> Result<Json<Agent>, ServiceError> {
    let agent = state.services.agents.create(req).await?;
    Ok(Json(agent))
}

async fn get_agent(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Agent>, ServiceError> {
    let agent = state.services.agents.get(&id).await?
        .ok_or_else(|| ServiceError::NotFound(format!("Agent {} not found", id)))?;
    Ok(Json(agent))
}

async fn delete_agent(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<(), ServiceError> {
    state.services.agents.delete(&id).await?;
    Ok(())
}

async fn agent_health(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<AgentHealth>, ServiceError> {
    let health = state.services.agents.health_check(&id).await?;
    Ok(Json(health))
}

async fn discover_agents(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Agent>>, ServiceError> {
    let agents = state.services.agents.discover().await?;
    Ok(Json(agents))
}
