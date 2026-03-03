use std::sync::Arc;
use composer_api_types::*;
use composer_db::Database;
use composer_executors::process_manager::{AgentProcessManager, SpawnOptions};
use crate::event_bus::EventBus;
use crate::worktree_service::WorktreeService;

#[derive(Clone)]
pub struct SessionService {
    db: Arc<Database>,
    event_bus: EventBus,
    process_manager: Arc<AgentProcessManager>,
    worktree_service: WorktreeService,
}

impl SessionService {
    pub fn new(db: Arc<Database>, event_bus: EventBus) -> Self {
        let process_manager = Arc::new(AgentProcessManager::new(event_bus.sender()));
        let worktree_service = WorktreeService::new(db.clone());

        let service = Self {
            db,
            event_bus,
            process_manager,
            worktree_service,
        };

        // Spawn background task to persist session events to DB
        service.spawn_event_listener();

        service
    }

    /// Listens for WsEvents and persists session output/completion/failure to the database.
    fn spawn_event_listener(&self) {
        let mut rx = self.event_bus.subscribe();
        let db = self.db.clone();

        tokio::spawn(async move {
            loop {
                let event = match rx.recv().await {
                    Ok(event) => event,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Event listener lagged, dropped {} events", n);
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                };
                match event {
                    WsEvent::SessionOutput { session_id, log_type, ref content } => {
                        let session_id_str = session_id.to_string();
                        if let Err(e) = composer_db::models::session_log::append(
                            &db.pool,
                            &session_id_str,
                            &log_type,
                            content,
                        ).await {
                            tracing::warn!("Failed to persist session log: {}", e);
                        }
                    }
                    WsEvent::SessionCompleted { session_id, ref result_summary } => {
                        let id_str = session_id.to_string();
                        if let Err(e) = composer_db::models::session::update_status(
                            &db.pool,
                            &id_str,
                            &SessionStatus::Completed,
                        ).await {
                            tracing::error!("Failed to update session status to completed: {}", e);
                        }
                        if let Err(e) = composer_db::models::session::update_result(
                            &db.pool,
                            &id_str,
                            result_summary.as_deref(),
                            None,
                        ).await {
                            tracing::error!("Failed to update session result: {}", e);
                        }
                        // Reset agent to idle
                        if let Ok(Some(session)) = composer_db::models::session::find_by_id(&db.pool, &id_str).await {
                            let agent_id_str = session.agent_id.to_string();
                            let _ = composer_db::models::agent::update_status(
                                &db.pool,
                                &agent_id_str,
                                &AgentStatus::Idle,
                            ).await;
                        }
                    }
                    WsEvent::SessionFailed { session_id, ref error } => {
                        let id_str = session_id.to_string();
                        if let Err(e) = composer_db::models::session::update_status(
                            &db.pool,
                            &id_str,
                            &SessionStatus::Failed,
                        ).await {
                            tracing::error!("Failed to update session status to failed: {}", e);
                        }
                        if let Err(e) = composer_db::models::session::update_result(
                            &db.pool,
                            &id_str,
                            Some(error.as_str()),
                            None,
                        ).await {
                            tracing::error!("Failed to update session error result: {}", e);
                        }
                        // Reset agent to idle
                        if let Ok(Some(session)) = composer_db::models::session::find_by_id(&db.pool, &id_str).await {
                            let agent_id_str = session.agent_id.to_string();
                            let _ = composer_db::models::agent::update_status(
                                &db.pool,
                                &agent_id_str,
                                &AgentStatus::Idle,
                            ).await;
                        }
                    }
                    _ => {}
                }
            } // end loop
        });
    }

    pub async fn create_session(&self, req: CreateSessionRequest) -> anyhow::Result<Session> {
        let agent =
            composer_db::models::agent::find_by_id(&self.db.pool, &req.agent_id.to_string())
                .await?
                .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;

        // Create worktree for isolated work
        let worktree = self
            .worktree_service
            .create_for_session(
                &req.repo_path,
                &agent.name,
                &agent.id.to_string(),
                &uuid::Uuid::new_v4().to_string(),
            )
            .await?;

        // Create session in DB
        let session = composer_db::models::session::create(
            &self.db.pool,
            &agent.id.to_string(),
            req.task_id.map(|id| id.to_string()).as_deref(),
            Some(&worktree.id.to_string()),
            &req.prompt,
        )
        .await?;

        // Update statuses
        composer_db::models::session::update_status(
            &self.db.pool,
            &session.id.to_string(),
            &SessionStatus::Running,
        )
        .await?;
        composer_db::models::agent::update_status(
            &self.db.pool,
            &agent.id.to_string(),
            &AgentStatus::Busy,
        )
        .await?;

        if let Some(task_id) = req.task_id {
            composer_db::models::task::update_status(
                &self.db.pool,
                &task_id.to_string(),
                &TaskStatus::InProgress,
            )
            .await?;
        }

        // Spawn agent process
        self.process_manager
            .spawn(SpawnOptions {
                session_id: session.id,
                agent_id: agent.id,
                task_id: req.task_id,
                prompt: req.prompt.clone(),
                working_dir: worktree.worktree_path.clone(),
                auto_approve: req.auto_approve.unwrap_or(true),
                resume_session_id: None,
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to spawn agent: {}", e))?;

        Ok(session)
    }

    pub async fn resume_session(&self, id: &str, req: ResumeSessionRequest) -> anyhow::Result<Session> {
        let session = composer_db::models::session::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        if !matches!(session.status, SessionStatus::Paused | SessionStatus::Completed | SessionStatus::Failed) {
            return Err(anyhow::anyhow!("Session cannot be resumed from status {:?}", session.status));
        }

        let agent = composer_db::models::agent::find_by_id(&self.db.pool, &session.agent_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;

        // Determine working directory from worktree or fallback
        let working_dir = if let Some(wt_id) = &session.worktree_id {
            let wt = composer_db::models::worktree::find_by_id(&self.db.pool, &wt_id.to_string())
                .await?
                .ok_or_else(|| anyhow::anyhow!("Worktree not found"))?;
            wt.worktree_path
        } else {
            return Err(anyhow::anyhow!("Session has no worktree, cannot resume"));
        };

        let prompt = req.prompt.unwrap_or_else(|| "Continue from where you left off.".to_string());

        // The resume_session_id is the Claude Code session ID, which we may have stored
        // from the original session. For now, use the session's own ID as a reference.
        let resume_id = session.resume_session_id.clone().unwrap_or_else(|| session.id.to_string());

        // Update session status back to Running
        composer_db::models::session::update_status(&self.db.pool, id, &SessionStatus::Running).await?;
        composer_db::models::agent::update_status(
            &self.db.pool,
            &agent.id.to_string(),
            &AgentStatus::Busy,
        ).await?;

        // Spawn agent process with --resume flag
        self.process_manager
            .spawn(SpawnOptions {
                session_id: session.id,
                agent_id: agent.id,
                task_id: session.task_id,
                prompt,
                working_dir,
                auto_approve: true,
                resume_session_id: Some(resume_id),
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to resume agent: {}", e))?;

        composer_db::models::session::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))
    }

    pub async fn list_all(&self) -> anyhow::Result<Vec<Session>> {
        composer_db::models::session::list_all(&self.db.pool).await
    }

    pub async fn get(&self, id: &str) -> anyhow::Result<Option<Session>> {
        composer_db::models::session::find_by_id(&self.db.pool, id).await
    }

    pub async fn interrupt(&self, id: &str) -> anyhow::Result<Session> {
        let session = composer_db::models::session::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        self.process_manager
            .interrupt(session.id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to interrupt: {}", e))?;
        composer_db::models::session::update_status(&self.db.pool, id, &SessionStatus::Paused)
            .await?;
        composer_db::models::agent::update_status(
            &self.db.pool,
            &session.agent_id.to_string(),
            &AgentStatus::Idle,
        )
        .await?;
        self.event_bus.broadcast(WsEvent::SessionPaused {
            session_id: session.id,
        });
        composer_db::models::session::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))
    }

    pub async fn get_logs(
        &self,
        session_id: &str,
        since: Option<&str>,
    ) -> anyhow::Result<Vec<SessionLog>> {
        composer_db::models::session_log::list_by_session(&self.db.pool, session_id, since).await
    }

    pub fn process_manager(&self) -> &AgentProcessManager {
        &self.process_manager
    }
}
