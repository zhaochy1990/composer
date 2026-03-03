use std::sync::Arc;
use composer_api_types::*;
use composer_db::Database;
use composer_executors::discovery;
use crate::event_bus::EventBus;

#[derive(Clone)]
pub struct AgentService {
    db: Arc<Database>,
    event_bus: EventBus,
}

impl AgentService {
    pub fn new(db: Arc<Database>, event_bus: EventBus) -> Self {
        Self { db, event_bus }
    }

    pub async fn create(&self, req: CreateAgentRequest) -> anyhow::Result<Agent> {
        let agent_type = req.agent_type.unwrap_or(AgentType::ClaudeCode);
        composer_db::models::agent::create(&self.db.pool, &req.name, &agent_type, None).await
    }

    pub async fn list_all(&self) -> anyhow::Result<Vec<Agent>> {
        composer_db::models::agent::list_all(&self.db.pool).await
    }

    pub async fn get(&self, id: &str) -> anyhow::Result<Option<Agent>> {
        composer_db::models::agent::find_by_id(&self.db.pool, id).await
    }

    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        composer_db::models::agent::delete(&self.db.pool, id).await
    }

    pub async fn discover(&self) -> anyhow::Result<Vec<Agent>> {
        let discovered = discovery::discover_agents().await;
        let mut agents = Vec::new();
        for d in discovered {
            let agent = composer_db::models::agent::create(
                &self.db.pool,
                &d.name,
                &d.agent_type,
                Some(&d.executable_path),
            )
            .await?;
            if d.is_authenticated {
                composer_db::models::agent::update_auth_status(
                    &self.db.pool,
                    &agent.id.to_string(),
                    &AuthStatus::Authenticated,
                )
                .await?;
            }
            let updated =
                composer_db::models::agent::find_by_id(&self.db.pool, &agent.id.to_string())
                    .await?
                    .unwrap_or(agent);
            agents.push(updated);
        }
        Ok(agents)
    }

    pub async fn health_check(&self, id: &str) -> anyhow::Result<AgentHealth> {
        let agent = composer_db::models::agent::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;
        Ok(AgentHealth {
            agent_id: agent.id,
            is_installed: agent.executable_path.is_some(),
            is_authenticated: matches!(agent.auth_status, AuthStatus::Authenticated),
            version: None,
        })
    }

    pub async fn update_status(&self, id: &str, status: &AgentStatus) -> anyhow::Result<()> {
        composer_db::models::agent::update_status(&self.db.pool, id, status).await?;
        let uuid: uuid::Uuid = id.parse()?;
        self.event_bus.broadcast(WsEvent::AgentStatusChanged {
            agent_id: uuid,
            status: status.clone(),
        });
        Ok(())
    }
}
