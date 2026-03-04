use std::sync::Arc;
use composer_api_types::*;
use composer_db::Database;
use composer_executors::process_manager::AgentProcessManager;
use composer_executors::discovery;
use crate::event_bus::EventBus;

#[derive(Clone)]
pub struct AgentService {
    db: Arc<Database>,
    event_bus: EventBus,
    process_manager: Arc<AgentProcessManager>,
}

impl AgentService {
    pub fn new(db: Arc<Database>, event_bus: EventBus, process_manager: Arc<AgentProcessManager>) -> Self {
        Self { db, event_bus, process_manager }
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
        // Interrupt any running sessions for this agent before deletion
        let sessions = composer_db::models::session::list_by_agent(&self.db.pool, id).await?;
        for session in &sessions {
            if matches!(session.status, SessionStatus::Running) {
                let _ = self.process_manager.interrupt(session.id).await;
            }
        }
        composer_db::models::agent::delete(&self.db.pool, id).await
    }

    pub async fn discover(&self) -> anyhow::Result<Vec<Agent>> {
        let discovered = discovery::discover_agents().await;
        let mut agents = Vec::new();
        for d in discovered {
            // Check if an agent of this type already exists — avoid duplicates
            let agent = if let Some(existing) =
                composer_db::models::agent::find_by_agent_type(&self.db.pool, &d.agent_type).await?
            {
                // Update executable path in case it changed
                composer_db::models::agent::update_executable_path(
                    &self.db.pool,
                    &existing.id.to_string(),
                    &d.executable_path,
                )
                .await?;
                existing
            } else {
                composer_db::models::agent::create(
                    &self.db.pool,
                    &d.name,
                    &d.agent_type,
                    Some(&d.executable_path),
                )
                .await?
            };

            // Always refresh auth status on discover
            let auth_status = if d.is_authenticated {
                AuthStatus::Authenticated
            } else {
                AuthStatus::Unauthenticated
            };
            composer_db::models::agent::update_auth_status(
                &self.db.pool,
                &agent.id.to_string(),
                &auth_status,
            )
            .await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::EventBus;

    async fn setup() -> AgentService {
        let db = composer_db::Database::connect("sqlite::memory:").await.unwrap();
        db.run_migrations().await.unwrap();
        let event_bus = EventBus::new();
        let pm = Arc::new(AgentProcessManager::new(event_bus.sender()));
        AgentService::new(Arc::new(db), event_bus, pm)
    }

    #[tokio::test]
    async fn create_and_list() {
        let svc = setup().await;
        svc.create(CreateAgentRequest {
            name: "Agent 1".to_string(),
            agent_type: None,
        })
        .await
        .unwrap();
        let agents = svc.list_all().await.unwrap();
        assert_eq!(agents.len(), 1);
    }

    #[tokio::test]
    async fn health_check_returns_info() {
        let svc = setup().await;
        let agent = svc
            .create(CreateAgentRequest {
                name: "HC Agent".to_string(),
                agent_type: None,
            })
            .await
            .unwrap();
        let health = svc.health_check(&agent.id.to_string()).await.unwrap();
        assert_eq!(health.agent_id, agent.id);
        assert!(!health.is_installed); // no executable_path set
        assert!(!health.is_authenticated);
    }

    #[tokio::test]
    async fn update_status_broadcasts() {
        let svc = setup().await;
        let mut rx = svc.event_bus.subscribe();
        let agent = svc
            .create(CreateAgentRequest {
                name: "Status Agent".to_string(),
                agent_type: None,
            })
            .await
            .unwrap();
        svc.update_status(&agent.id.to_string(), &AgentStatus::Error)
            .await
            .unwrap();
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, WsEvent::AgentStatusChanged { .. }));
    }
}
