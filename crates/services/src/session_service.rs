use std::sync::Arc;
use once_cell::sync::Lazy;
use regex::Regex;
use composer_api_types::*;
use composer_db::Database;
use composer_executors::process_manager::{AgentProcessManager, SpawnOptions};
use crate::event_bus::EventBus;
use crate::worktree_service::WorktreeService;

/// Matches PR URLs from GitHub, Azure DevOps, GitLab, and self-hosted instances.
static PR_URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"https?://[^\s"<>]+/(?:pull|pullrequest|merge_requests)/\d+"#).unwrap()
});

#[derive(Clone)]
pub struct SessionService {
    db: Arc<Database>,
    event_bus: EventBus,
    process_manager: Arc<AgentProcessManager>,
    worktree_service: WorktreeService,
}

impl SessionService {
    pub fn new(db: Arc<Database>, event_bus: EventBus, process_manager: Arc<AgentProcessManager>) -> Self {
        let worktree_service = WorktreeService::new(db.clone());

        let service = Self {
            db,
            event_bus,
            process_manager,
            worktree_service,
        };

        // Recover orphaned sessions/agents from previous unclean shutdown
        service.spawn_startup_recovery();

        // Spawn background task to persist session events to DB
        service.spawn_event_listener();

        service
    }

    /// On startup, mark any "running" sessions as "failed" and reset "busy" agents to "idle".
    /// These are orphaned from a previous server process that exited without cleanup.
    fn spawn_startup_recovery(&self) {
        let db = self.db.clone();
        tokio::spawn(async move {
            match composer_db::models::session::fail_orphaned_running(&db.pool).await {
                Ok(n) if n > 0 => tracing::warn!("Recovered {} orphaned running session(s) → failed", n),
                Ok(_) => {}
                Err(e) => tracing::error!("Failed to recover orphaned sessions: {}", e),
            }
            match composer_db::models::agent::reset_all_busy_to_idle(&db.pool).await {
                Ok(n) if n > 0 => tracing::warn!("Reset {} busy agent(s) → idle", n),
                Ok(_) => {}
                Err(e) => tracing::error!("Failed to reset busy agents: {}", e),
            }
        });
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
                        // Detect PR URLs in session output
                        Self::extract_and_save_pr_urls(&db, &session_id_str, content).await;
                    }
                    WsEvent::SessionCompleted { session_id, ref result_summary, ref claude_session_id } => {
                        let id_str = session_id.to_string();
                        if let Err(e) = composer_db::models::session::update_status(
                            &db.pool,
                            &id_str,
                            &SessionStatus::Completed,
                        ).await {
                            tracing::error!("Failed to update session status to completed: {}", e);
                        }
                        // Persist Claude Code's session_id from event payload for --resume support
                        if let Err(e) = composer_db::models::session::update_result(
                            &db.pool,
                            &id_str,
                            result_summary.as_deref(),
                            claude_session_id.as_deref(),
                        ).await {
                            tracing::error!("Failed to update session result: {}", e);
                        }
                        // Also scan result_summary for PR URLs
                        if let Some(summary) = result_summary {
                            Self::extract_and_save_pr_urls(&db, &id_str, summary).await;
                        }
                        // Reset agent to idle and cleanup worktree
                        Self::reset_agent_and_cleanup_worktree(&db, &id_str).await;
                    }
                    WsEvent::SessionFailed { session_id, ref error, ref claude_session_id } => {
                        let id_str = session_id.to_string();
                        if let Err(e) = composer_db::models::session::update_status(
                            &db.pool,
                            &id_str,
                            &SessionStatus::Failed,
                        ).await {
                            tracing::error!("Failed to update session status to failed: {}", e);
                        }
                        // Persist Claude Code's session_id from event payload for --resume
                        if let Err(e) = composer_db::models::session::update_result(
                            &db.pool,
                            &id_str,
                            Some(error.as_str()),
                            claude_session_id.as_deref(),
                        ).await {
                            tracing::error!("Failed to update session error result: {}", e);
                        }
                        // Only reset agent to idle — preserve worktree so retry/resume is possible
                        Self::reset_agent_only(&db, &id_str).await;
                    }
                    _ => {}
                }
            } // end loop
        });
    }

    /// Resets the session's agent to idle and cleans up the associated worktree.
    async fn reset_agent_and_cleanup_worktree(db: &Database, session_id: &str) {
        if let Ok(Some(session)) =
            composer_db::models::session::find_by_id(&db.pool, session_id).await
        {
            let agent_id_str = session.agent_id.to_string();
            let _ = composer_db::models::agent::update_status(
                &db.pool,
                &agent_id_str,
                &AgentStatus::Idle,
            )
            .await;

            // Cleanup worktree
            if let Some(wt_id) = &session.worktree_id {
                let wt_id_str = wt_id.to_string();
                if let Ok(Some(wt)) =
                    composer_db::models::worktree::find_by_id(&db.pool, &wt_id_str).await
                {
                    if wt.status != WorktreeStatus::Deleted {
                        let remove_result = composer_git::worktree::remove_worktree(
                            std::path::Path::new(&wt.repo_path),
                            std::path::Path::new(&wt.worktree_path),
                            &wt.branch_name,
                        )
                        .await;
                        if let Err(e) = &remove_result {
                            tracing::warn!(
                                "Failed to cleanup worktree {} on session end: {}",
                                wt_id_str,
                                e
                            );
                        }
                        let _ = composer_db::models::worktree::update_status(
                            &db.pool,
                            &wt_id_str,
                            &WorktreeStatus::Deleted,
                        )
                        .await;
                    }
                }
            }
        }
    }

    /// Extracts PR URLs from text content and saves them to the session's task.
    async fn extract_and_save_pr_urls(db: &Database, session_id: &str, content: &str) {
        let urls: Vec<String> = PR_URL_RE
            .find_iter(content)
            .map(|m| m.as_str().to_string())
            .collect();
        if urls.is_empty() {
            return;
        }
        // Look up the task_id for this session
        let task_id = match composer_db::models::session::find_by_id(&db.pool, session_id).await {
            Ok(Some(session)) => session.task_id,
            _ => None,
        };
        if let Some(task_id) = task_id {
            let task_id_str = task_id.to_string();
            match composer_db::models::task::append_pr_urls(&db.pool, &task_id_str, &urls).await {
                Ok(true) => {
                    tracing::info!("Saved PR URLs for task {}: {:?}", task_id_str, urls);
                }
                Ok(false) => {} // All URLs already existed
                Err(e) => {
                    tracing::warn!("Failed to save PR URLs for task {}: {}", task_id_str, e);
                }
            }
        }
    }

    /// Resets only the session's agent to idle, preserving the worktree for retry.
    async fn reset_agent_only(db: &Database, session_id: &str) {
        if let Ok(Some(session)) =
            composer_db::models::session::find_by_id(&db.pool, session_id).await
        {
            let agent_id_str = session.agent_id.to_string();
            let _ = composer_db::models::agent::update_status(
                &db.pool,
                &agent_id_str,
                &AgentStatus::Idle,
            )
            .await;
        }
    }

    pub async fn create_session(&self, req: CreateSessionRequest) -> anyhow::Result<Session> {
        // Validate repo_path: must be absolute, must exist, must be a git repo
        let repo_path = std::path::Path::new(&req.repo_path);
        if !repo_path.is_absolute() {
            return Err(anyhow::anyhow!("repo_path must be an absolute path"));
        }
        let canonical = std::fs::canonicalize(repo_path)
            .map_err(|_| anyhow::anyhow!("repo_path does not exist: {}", req.repo_path))?;
        if !canonical.join(".git").exists() {
            return Err(anyhow::anyhow!("repo_path is not a git repository: {}", req.repo_path));
        }
        // Strip Windows UNC prefix (\\?\) that canonicalize produces — git doesn't understand it
        let canonical_str = canonical.to_string_lossy().to_string();
        let validated_repo_path = canonical_str
            .strip_prefix(r"\\?\")
            .unwrap_or(&canonical_str)
            .to_string();

        let agent =
            composer_db::models::agent::find_by_id(&self.db.pool, &req.agent_id.to_string())
                .await?
                .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;

        // Generate session UUID upfront so worktree + DB share the same ID
        let session_uuid = uuid::Uuid::new_v4();
        let session_id_str = session_uuid.to_string();

        // Create worktree for isolated work
        let worktree = self
            .worktree_service
            .create_for_session(
                &validated_repo_path,
                &agent.name,
                &agent.id.to_string(),
                &session_id_str,
            )
            .await?;

        // Wrap all remaining operations — any failure after worktree creation triggers cleanup
        let result = async {
            // Create session in DB directly with Running status (fix #24)
            let session = composer_db::models::session::create_with_status(
                &self.db.pool,
                &session_id_str,
                &agent.id.to_string(),
                Some(&req.task_id.to_string()),
                Some(&worktree.id.to_string()),
                &req.prompt,
                &SessionStatus::Running,
            )
            .await?;

            composer_db::models::agent::update_status(
                &self.db.pool,
                &agent.id.to_string(),
                &AgentStatus::Busy,
            )
            .await?;

            composer_db::models::task::update_status(
                &self.db.pool,
                &req.task_id.to_string(),
                &TaskStatus::InProgress,
            )
            .await?;

            self.process_manager
                .spawn(SpawnOptions {
                    session_id: session.id,
                    agent_id: agent.id,
                    task_id: Some(req.task_id),
                    prompt: req.prompt.clone(),
                    working_dir: worktree.worktree_path.clone(),
                    auto_approve: req.auto_approve.unwrap_or(false),
                    resume_session_id: None,
                    resume_at_message_id: None,
                })
                .await
                .map_err(|e| anyhow::anyhow!("Failed to spawn agent: {}", e))?;

            Ok::<_, anyhow::Error>(session)
        }
        .await;

        match result {
            Ok(session) => Ok(session),
            Err(e) => {
                // Rollback: set session to Failed, agent to Idle, cleanup worktree
                let _ = composer_db::models::session::update_status(
                    &self.db.pool,
                    &session_id_str,
                    &SessionStatus::Failed,
                )
                .await;
                let _ = composer_db::models::agent::update_status(
                    &self.db.pool,
                    &agent.id.to_string(),
                    &AgentStatus::Idle,
                )
                .await;
                let _ = self
                    .worktree_service
                    .cleanup(&worktree.id.to_string())
                    .await;
                Err(e)
            }
        }
    }

    pub async fn resume_session(&self, id: &str, req: ResumeSessionRequest) -> anyhow::Result<Session> {
        let session = composer_db::models::session::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        // Fix #25: don't allow resuming completed sessions
        if !matches!(session.status, SessionStatus::Paused | SessionStatus::Failed) {
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

        // Use explicit prompt, or consume a queued message, or fallback to default
        let prompt = req.prompt
            .or_else(|| self.process_manager.take_queued_message(&session.id))
            .unwrap_or_else(|| "Continue from where you left off.".to_string());

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

        // Spawn agent process with --resume flag — rollback on failure (fix #2)
        if let Err(e) = self.process_manager
            .spawn(SpawnOptions {
                session_id: session.id,
                agent_id: agent.id,
                task_id: session.task_id,
                prompt,
                working_dir,
                auto_approve: true,
                resume_session_id: Some(resume_id),
                resume_at_message_id: None,
            })
            .await
        {
            // Rollback: set session to Failed, agent to Idle
            let _ = composer_db::models::session::update_status(
                &self.db.pool, id, &SessionStatus::Failed,
            ).await;
            let _ = composer_db::models::agent::update_status(
                &self.db.pool, &agent.id.to_string(), &AgentStatus::Idle,
            ).await;
            return Err(anyhow::anyhow!("Failed to resume agent: {}", e));
        }

        composer_db::models::session::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))
    }

    pub async fn retry_session(&self, id: &str, req: ResumeSessionRequest) -> anyhow::Result<Session> {
        let session = composer_db::models::session::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        if !matches!(session.status, SessionStatus::Failed) {
            return Err(anyhow::anyhow!(
                "Only failed sessions can be retried, current status: {:?}",
                session.status
            ));
        }

        // Check if worktree is still usable for resume
        let worktree_usable = if let Some(wt_id) = &session.worktree_id {
            if let Ok(Some(wt)) =
                composer_db::models::worktree::find_by_id(&self.db.pool, &wt_id.to_string()).await
            {
                matches!(wt.status, WorktreeStatus::Active)
                    && std::path::Path::new(&wt.worktree_path).exists()
            } else {
                false
            }
        } else {
            false
        };

        // Save prompt before potentially moving req
        let saved_prompt = req.prompt.clone();

        if worktree_usable {
            // Try to resume the failed session (reuses existing worktree + conversation context)
            match self.resume_session(id, req).await {
                Ok(session) => return Ok(session),
                Err(e) => {
                    tracing::warn!("Resume failed during retry, falling back to new session: {}", e);
                    // Fall through to create a new session
                }
            }
        }

        // Fallback: clean up old worktree and create a brand new session
        let task_id = session
            .task_id
            .ok_or_else(|| anyhow::anyhow!("Session has no task_id, cannot retry"))?;
        let prompt = saved_prompt
            .or(session.prompt.clone())
            .unwrap_or_else(|| "Continue the task.".to_string());

        // Get repo_path from worktree before cleaning up
        let repo_path = if let Some(wt_id) = &session.worktree_id {
            let wt_id_str = wt_id.to_string();
            let repo = composer_db::models::worktree::find_by_id(&self.db.pool, &wt_id_str)
                .await?
                .map(|wt| wt.repo_path);
            // Clean up old worktree
            let _ = self.worktree_service.cleanup(&wt_id_str).await;
            repo
        } else {
            None
        };

        let repo_path = repo_path
            .ok_or_else(|| anyhow::anyhow!("Cannot determine repo_path for retry"))?;

        self.create_session(CreateSessionRequest {
            agent_id: session.agent_id,
            task_id,
            prompt,
            repo_path,
            auto_approve: Some(true),
        })
        .await
    }

    pub async fn list_all(&self) -> anyhow::Result<Vec<Session>> {
        composer_db::models::session::list_all(&self.db.pool).await
    }

    pub async fn list_by_task(&self, task_id: &str) -> anyhow::Result<Vec<Session>> {
        composer_db::models::session::list_by_task(&self.db.pool, task_id).await
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
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> anyhow::Result<Vec<SessionLog>> {
        composer_db::models::session_log::list_by_session(&self.db.pool, session_id, since, limit, offset).await
    }

    pub async fn send_input(&self, id: &str, message: String) -> anyhow::Result<()> {
        let session = composer_db::models::session::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        if !matches!(session.status, SessionStatus::Running) {
            return Err(anyhow::anyhow!("Session is not running (status: {:?})", session.status));
        }

        // Broadcast the user input as session output so it gets persisted and shown in the UI
        self.event_bus.broadcast(WsEvent::SessionOutput {
            session_id: session.id,
            log_type: LogType::UserInput,
            content: message.clone(),
        });

        // Send the message to the running Claude Code process
        self.process_manager
            .send_input(session.id, message)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send input: {}", e))?;

        Ok(())
    }

    pub fn process_manager(&self) -> &AgentProcessManager {
        &self.process_manager
    }
}
