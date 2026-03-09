use crate::event_bus::EventBus;
use crate::worktree_service::WorktreeService;
use composer_api_types::*;
use composer_db::Database;
use composer_executors::process_manager::{AgentProcessManager, SpawnOptions};
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::mpsc;

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
    workflow_engine: Arc<OnceLock<crate::workflow_engine::WorkflowEngine>>,
}

impl SessionService {
    pub fn new(
        db: Arc<Database>,
        event_bus: EventBus,
        process_manager: Arc<AgentProcessManager>,
        persist_rx: mpsc::UnboundedReceiver<WsEvent>,
    ) -> Self {
        let worktree_service = WorktreeService::new(db.clone());

        let service = Self {
            db,
            event_bus,
            process_manager,
            worktree_service,
            workflow_engine: Arc::new(OnceLock::new()),
        };

        // Recover orphaned sessions/agents from previous unclean shutdown
        service.spawn_startup_recovery();

        // Spawn background task to persist session events to DB
        service.spawn_event_listener(persist_rx);

        service
    }

    /// On startup, mark any "running" sessions as "failed" and reset "busy" agents to "idle".
    /// These are orphaned from a previous server process that exited without cleanup.
    fn spawn_startup_recovery(&self) {
        let db = self.db.clone();
        tokio::spawn(async move {
            match composer_db::models::session::fail_orphaned_running(&db.pool).await {
                Ok(n) if n > 0 => {
                    tracing::warn!("Recovered {} orphaned running session(s) → failed", n)
                }
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

    /// Listens for WsEvents via a dedicated mpsc channel and persists session
    /// output/completion/failure to the database. The mpsc channel cannot lag or
    /// drop messages, unlike the broadcast channel used for WebSocket fan-out.
    fn spawn_event_listener(&self, mut rx: mpsc::UnboundedReceiver<WsEvent>) {
        let db = self.db.clone();
        let workflow_engine = self.workflow_engine.clone();
        let process_manager = self.process_manager.clone();
        let event_bus = self.event_bus.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    WsEvent::SessionOutput {
                        session_id,
                        log_type,
                        ref content,
                    } => {
                        let session_id_str = session_id.to_string();
                        if let Err(e) = composer_db::models::session_log::append(
                            &db.pool,
                            &session_id_str,
                            &log_type,
                            content,
                        )
                        .await
                        {
                            tracing::warn!("Failed to persist session log: {}", e);
                        }
                        // Detect PR URLs in session output
                        Self::extract_and_save_pr_urls(&db, &session_id_str, content).await;
                    }
                    WsEvent::SessionCompleted {
                        session_id,
                        ref result_summary,
                        ref claude_session_id,
                    } => {
                        let id_str = session_id.to_string();
                        if let Err(e) = composer_db::models::session::update_status(
                            &db.pool,
                            &id_str,
                            &SessionStatus::Completed,
                        )
                        .await
                        {
                            tracing::error!("Failed to update session status to completed: {}", e);
                        }
                        // Persist Claude Code's session_id from event payload for --resume support
                        if let Err(e) = composer_db::models::session::update_result(
                            &db.pool,
                            &id_str,
                            result_summary.as_deref(),
                            claude_session_id.as_deref(),
                        )
                        .await
                        {
                            tracing::error!("Failed to update session result: {}", e);
                        }
                        // Also scan result_summary for PR URLs
                        if let Some(summary) = result_summary {
                            Self::extract_and_save_pr_urls(&db, &id_str, summary).await;
                        }

                        // Check if this session belongs to a workflow run
                        let handled_by_workflow = if let Some(engine) = workflow_engine.get() {
                            match engine
                                .on_session_completed(&id_str, result_summary.as_deref())
                                .await
                            {
                                Ok(true) => {
                                    // Workflow handled it — only reset agent, don't cleanup worktree
                                    Self::reset_agent_only(&db, &id_str).await;
                                    true
                                }
                                Ok(false) => false,
                                Err(e) => {
                                    tracing::error!(
                                        "Workflow engine error on session completed: {}",
                                        e
                                    );
                                    false
                                }
                            }
                        } else {
                            false
                        };

                        if !handled_by_workflow {
                            // Regular (non-workflow) session — reset agent and cleanup worktree
                            Self::reset_agent_and_cleanup_worktree(&db, &id_str).await;
                        }
                    }
                    WsEvent::SessionResumeIdCaptured {
                        session_id,
                        ref claude_session_id,
                    } => {
                        let id_str = session_id.to_string();
                        if let Err(e) = composer_db::models::session::update_resume_session_id(
                            &db.pool,
                            &id_str,
                            claude_session_id,
                        )
                        .await
                        {
                            tracing::warn!("Failed to eagerly persist resume_session_id: {}", e);
                        }
                    }
                    WsEvent::SessionFailed {
                        session_id,
                        ref error,
                        ref claude_session_id,
                    } => {
                        let id_str = session_id.to_string();
                        if let Err(e) = composer_db::models::session::update_status(
                            &db.pool,
                            &id_str,
                            &SessionStatus::Failed,
                        )
                        .await
                        {
                            tracing::error!("Failed to update session status to failed: {}", e);
                        }
                        // Persist Claude Code's session_id from event payload for --resume
                        if let Err(e) = composer_db::models::session::update_result(
                            &db.pool,
                            &id_str,
                            Some(error.as_str()),
                            claude_session_id.as_deref(),
                        )
                        .await
                        {
                            tracing::error!("Failed to update session error result: {}", e);
                        }

                        // Notify workflow engine if applicable
                        if let Some(engine) = workflow_engine.get() {
                            if let Err(e) = engine.on_session_failed(&id_str, error).await {
                                tracing::error!("Workflow engine error on session failed: {}", e);
                            }
                        }

                        // Only reset agent to idle — preserve worktree so retry/resume is possible
                        Self::reset_agent_only(&db, &id_str).await;
                    }
                    WsEvent::UserQuestionRequested {
                        session_id,
                        ref request_id,
                        ref questions,
                        ref plan_content,
                    } => {
                        // Skip already-enriched events (plan_content is Some) to avoid infinite loop
                        if plan_content.is_some() {
                            continue;
                        }

                        // Try multiple sources for plan content:
                        // 1. Captured content from Write tool_use input
                        // 2. Plan file path from Write detection → read from disk
                        // 3. Glob .claude/plans/*.md in working directory
                        let plan = process_manager
                            .get_plan_content(&session_id)
                            .or_else(|| {
                                process_manager
                                    .get_plan_file_path(&session_id)
                                    .and_then(|path| std::fs::read_to_string(&path).ok())
                            })
                            .or_else(|| process_manager.find_plan_file(&session_id))
                            .unwrap_or_default();
                        event_bus.broadcast(WsEvent::UserQuestionRequested {
                            session_id,
                            request_id: request_id.clone(),
                            questions: questions.clone(),
                            plan_content: Some(plan),
                        });

                        // Move task to Waiting while blocked on user answer
                        let id_str = session_id.to_string();
                        if let Ok(Some(session)) =
                            composer_db::models::session::find_by_id(&db.pool, &id_str).await
                        {
                            if let Some(task_id) = session.task_id {
                                let task_id_str = task_id.to_string();
                                if let Err(e) = composer_db::models::task::update_status(
                                    &db.pool,
                                    &task_id_str,
                                    &TaskStatus::Waiting,
                                )
                                .await
                                {
                                    tracing::warn!(
                                        "Failed to set task to Waiting on user question: {}",
                                        e
                                    );
                                }
                                // Pause the workflow run if applicable
                                if let Ok(Some(run)) =
                                    composer_db::models::workflow_run::find_by_step_session(
                                        &db.pool, &id_str,
                                    )
                                    .await
                                {
                                    let run_id_str = run.id.to_string();
                                    let _ = composer_db::models::workflow_run::update_status(
                                        &db.pool,
                                        &run_id_str,
                                        &WorkflowRunStatus::Paused,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                    WsEvent::UserQuestionAnswered { session_id } => {
                        // Move task back to InProgress after user answers
                        let id_str = session_id.to_string();
                        if let Ok(Some(session)) =
                            composer_db::models::session::find_by_id(&db.pool, &id_str).await
                        {
                            if let Some(task_id) = session.task_id {
                                let task_id_str = task_id.to_string();
                                if let Err(e) = composer_db::models::task::update_status(
                                    &db.pool,
                                    &task_id_str,
                                    &TaskStatus::InProgress,
                                )
                                .await
                                {
                                    tracing::warn!(
                                        "Failed to set task to InProgress after answer: {}",
                                        e
                                    );
                                }
                                // Resume the workflow run if applicable
                                if let Ok(Some(run)) =
                                    composer_db::models::workflow_run::find_by_step_session(
                                        &db.pool, &id_str,
                                    )
                                    .await
                                {
                                    let run_id_str = run.id.to_string();
                                    let _ = composer_db::models::workflow_run::update_status(
                                        &db.pool,
                                        &run_id_str,
                                        &WorkflowRunStatus::Running,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                    WsEvent::PlanCompleted {
                        session_id,
                        ref plan_content,
                    } => {
                        tracing::info!(
                            "Session {} PlanCompleted event received. plan_content present: {}, len: {}",
                            session_id,
                            plan_content.is_some(),
                            plan_content.as_ref().map(|c| c.len()).unwrap_or(0)
                        );
                        // Plan content from executor, with multiple fallbacks.
                        let plan = match plan_content {
                            Some(content) if !content.is_empty() => Some(content.clone()),
                            _ => {
                                tracing::info!(
                                    "Session {} PlanCompleted: trying fallbacks",
                                    session_id
                                );
                                process_manager
                                    .get_plan_content(&session_id)
                                    .or_else(|| {
                                        process_manager
                                            .get_plan_file_path(&session_id)
                                            .and_then(|path| std::fs::read_to_string(&path).ok())
                                    })
                                    .or_else(|| process_manager.find_plan_file(&session_id))
                            }
                        };

                        tracing::info!(
                            "Session {} PlanCompleted final plan len: {}",
                            session_id,
                            plan.as_ref().map(|c| c.len()).unwrap_or(0)
                        );

                        if let Some(ref content) = plan {
                            // Eagerly store plan content in the workflow step output
                            let id_str = session_id.to_string();
                            if let Ok(Some(step_output)) =
                                composer_db::models::workflow_step_output::find_by_session(
                                    &db.pool, &id_str,
                                )
                                .await
                            {
                                if matches!(step_output.status, WorkflowStepStatus::Running) {
                                    if let Err(e) =
                                        composer_db::models::workflow_step_output::update_output(
                                            &db.pool,
                                            &step_output.id.to_string(),
                                            content,
                                        )
                                        .await
                                    {
                                        tracing::warn!(
                                            "Failed to eagerly store plan content: {}",
                                            e
                                        );
                                    } else {
                                        // Broadcast step change so frontend picks up the update
                                        if let Ok(Some(refreshed)) =
                                            composer_db::models::workflow_step_output::find_by_id(
                                                &db.pool,
                                                &step_output.id.to_string(),
                                            )
                                            .await
                                        {
                                            event_bus.broadcast(WsEvent::WorkflowStepChanged {
                                                workflow_run_id: step_output.workflow_run_id,
                                                step: refreshed,
                                            });
                                        }
                                    }
                                }
                            }
                        }
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

    /// Set the workflow engine reference (called after construction to break circular dependency).
    pub fn set_workflow_engine(&self, engine: crate::workflow_engine::WorkflowEngine) {
        let _ = self.workflow_engine.set(engine);
    }

    pub async fn create_session(&self, req: CreateSessionRequest) -> anyhow::Result<Session> {
        // Validate name length
        if let Some(ref name) = req.name {
            if name.len() > 255 {
                return Err(anyhow::anyhow!(
                    "Session name must be 255 characters or fewer"
                ));
            }
        }

        // Validate repo_path: must be absolute, must exist, must be a git repo
        let repo_path = std::path::Path::new(&req.repo_path);
        if !repo_path.is_absolute() {
            return Err(anyhow::anyhow!("repo_path must be an absolute path"));
        }
        let canonical = std::fs::canonicalize(repo_path)
            .map_err(|_| anyhow::anyhow!("repo_path does not exist: {}", req.repo_path))?;
        if !canonical.join(".git").exists() {
            return Err(anyhow::anyhow!(
                "repo_path is not a git repository: {}",
                req.repo_path
            ));
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
                req.name.as_deref(),
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
                    exit_on_result: req.exit_on_result,
                })
                .await
                .map_err(|e| anyhow::anyhow!("Failed to spawn agent: {}", e))?;

            // Broadcast the initial prompt so it appears in session logs
            self.event_bus.broadcast(WsEvent::SessionOutput {
                session_id: session.id,
                log_type: LogType::UserInput,
                content: req.prompt.clone(),
            });

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

    pub async fn resume_session(
        &self,
        id: &str,
        req: ResumeSessionRequest,
    ) -> anyhow::Result<Session> {
        let session = composer_db::models::session::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        // Fix #25: only allow resuming sessions that are in a terminal or paused state.
        // Completed sessions can be resumed for workflow multi-step execution (the same
        // Claude Code session is reused across plan → implement → fix steps via --resume).
        if !matches!(
            session.status,
            SessionStatus::Paused | SessionStatus::Failed | SessionStatus::Completed
        ) {
            return Err(anyhow::anyhow!(
                "Session cannot be resumed from status {:?}",
                session.status
            ));
        }

        let agent =
            composer_db::models::agent::find_by_id(&self.db.pool, &session.agent_id.to_string())
                .await?
                .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;

        // Determine working directory from worktree, recreating if deleted
        let working_dir = if let Some(wt_id) = &session.worktree_id {
            let wt = composer_db::models::worktree::find_by_id(&self.db.pool, &wt_id.to_string())
                .await?
                .ok_or_else(|| anyhow::anyhow!("Worktree not found"))?;

            // If the worktree was deleted (e.g., after workflow completion), recreate it
            if wt.status == WorktreeStatus::Deleted
                || !std::path::Path::new(&wt.worktree_path).exists()
            {
                let new_wt = self
                    .worktree_service
                    .create_for_session(
                        &wt.repo_path,
                        &agent.name,
                        &agent.id.to_string(),
                        &session.id.to_string(),
                    )
                    .await?;
                composer_db::models::session::update_worktree_id(
                    &self.db.pool,
                    id,
                    &new_wt.id.to_string(),
                )
                .await?;
                new_wt.worktree_path
            } else {
                wt.worktree_path
            }
        } else {
            return Err(anyhow::anyhow!("Session has no worktree, cannot resume"));
        };

        // Use explicit prompt, or consume a queued message, or fallback to default
        let prompt = req
            .prompt
            .or_else(|| self.process_manager.take_queued_message(&session.id))
            .unwrap_or_else(|| "Continue from where you left off.".to_string());

        // The resume_session_id is the Claude Code session ID, which we may have stored
        // from the original session.
        let resume_id = match &session.resume_session_id {
            Some(id) => {
                tracing::info!(
                    "Resuming session {} with Claude Code session {}",
                    session.id,
                    id
                );
                id.clone()
            }
            None => {
                tracing::warn!(
                    "Session {} has no resume_session_id — Claude Code history will be lost. \
                     This may happen if the server crashed before the ID was captured.",
                    session.id
                );
                session.id.to_string()
            }
        };

        // Update session status back to Running
        composer_db::models::session::update_status(&self.db.pool, id, &SessionStatus::Running)
            .await?;
        composer_db::models::agent::update_status(
            &self.db.pool,
            &agent.id.to_string(),
            &AgentStatus::Busy,
        )
        .await?;

        // Save prompt for broadcasting after spawn succeeds (prompt is moved into SpawnOptions)
        let prompt_for_log = prompt.clone();

        // Spawn agent process with --resume flag — rollback on failure (fix #2)
        if let Err(e) = self
            .process_manager
            .spawn(SpawnOptions {
                session_id: session.id,
                agent_id: agent.id,
                task_id: session.task_id,
                prompt,
                working_dir,
                auto_approve: true,
                resume_session_id: Some(resume_id),
                resume_at_message_id: None,
                // continue_chat overrides exit_on_result to keep the session alive
                exit_on_result: if req.continue_chat {
                    false
                } else {
                    req.exit_on_result
                },
            })
            .await
        {
            // Rollback: set session to Failed, agent to Idle
            let _ = composer_db::models::session::update_status(
                &self.db.pool,
                id,
                &SessionStatus::Failed,
            )
            .await;
            let _ = composer_db::models::agent::update_status(
                &self.db.pool,
                &agent.id.to_string(),
                &AgentStatus::Idle,
            )
            .await;
            return Err(anyhow::anyhow!("Failed to resume agent: {}", e));
        }

        // Broadcast the resume prompt so it appears in session logs (only after spawn succeeds)
        self.event_bus.broadcast(WsEvent::SessionOutput {
            session_id: session.id,
            log_type: LogType::UserInput,
            content: prompt_for_log,
        });

        composer_db::models::session::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))
    }

    pub async fn retry_session(
        &self,
        id: &str,
        req: ResumeSessionRequest,
    ) -> anyhow::Result<Session> {
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

        if worktree_usable && session.resume_session_id.is_some() {
            // Only attempt resume if we have a Claude Code session ID (needed for --resume).
            // If resume_session_id is NULL (e.g., server crashed before it was persisted),
            // skip resume and create a fresh session instead.
            match self.resume_session(id, req).await {
                Ok(session) => return Ok(session),
                Err(e) => {
                    tracing::warn!(
                        "Resume failed during retry, falling back to new session: {}",
                        e
                    );
                    // Fall through to create a new session
                }
            }
        } else if worktree_usable {
            tracing::info!(
                "Session {} has usable worktree but no resume_session_id, creating fresh session",
                id
            );
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

        let repo_path =
            repo_path.ok_or_else(|| anyhow::anyhow!("Cannot determine repo_path for retry"))?;

        self.create_session(CreateSessionRequest {
            agent_id: session.agent_id,
            task_id,
            prompt,
            repo_path,
            name: None,
            auto_approve: Some(true),
            exit_on_result: false,
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

    /// Answer a pending AskUserQuestion by sending a control response back to Claude Code.
    /// Also reads the plan file from disk (if available) and enriches the event with plan content.
    pub async fn answer_question(
        &self,
        session_id: &str,
        request_id: String,
        answers: serde_json::Value,
    ) -> anyhow::Result<()> {
        let session = composer_db::models::session::find_by_id(&self.db.pool, session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        if !matches!(session.status, SessionStatus::Running) {
            anyhow::bail!("Session is not running (status: {:?})", session.status);
        }

        tracing::info!(session_id = %session_id, request_id = %request_id, "Answering user question");

        // Build the control response with the user's answers
        let response = composer_executors::types::SDKControlResponse::new(
            composer_executors::types::ControlResponsePayload::Success {
                request_id: request_id.clone(),
                response: Some(serde_json::json!({
                    "behavior": "allow",
                    "updatedInput": answers,
                })),
            },
        );

        self.process_manager
            .send_control_response(session.id, response)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send control response: {}", e))?;

        // Log the answer as user input
        self.event_bus.broadcast(WsEvent::SessionOutput {
            session_id: session.id,
            log_type: LogType::UserInput,
            content: format!(
                "User answered question: {}",
                serde_json::to_string(&answers).unwrap_or_default()
            ),
        });

        // Notify that the question was answered (triggers task status transition)
        self.event_bus.broadcast(WsEvent::UserQuestionAnswered {
            session_id: session.id,
        });

        Ok(())
    }

    /// Read the plan file content for a running session, if available.
    pub fn get_plan_content(&self, session_id: &str) -> Option<String> {
        let uuid = session_id.parse::<uuid::Uuid>().ok()?;
        let path = self.process_manager.get_plan_file_path(&uuid)?;
        std::fs::read_to_string(&path).ok()
    }

    /// Gracefully complete a running session by closing stdin.
    /// The process exits naturally, emitting `SessionCompleted` which the
    /// workflow engine handles to advance to the next step.
    pub async fn complete_session(&self, id: &str) -> anyhow::Result<Session> {
        let session = composer_db::models::session::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        if !matches!(session.status, SessionStatus::Running) {
            anyhow::bail!("Session is not running (status: {:?})", session.status);
        }
        tracing::info!(session_id = %id, "Completing session by closing stdin");
        self.process_manager
            .close_stdin(session.id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to close stdin: {}", e))?;
        // Don't update status here — the monitor task will emit SessionCompleted
        // when the process exits, and the event listener will handle the status update.
        composer_db::models::session::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))
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
        composer_db::models::session_log::list_by_session(
            &self.db.pool,
            session_id,
            since,
            limit,
            offset,
        )
        .await
    }

    pub async fn get_logs_cursor(
        &self,
        session_id: &str,
        before: Option<i64>,
        limit: i64,
    ) -> anyhow::Result<Vec<SessionLog>> {
        composer_db::models::session_log::list_by_session_cursor(
            &self.db.pool,
            session_id,
            before,
            limit,
        )
        .await
    }

    pub async fn get_log_count(&self, session_id: &str) -> anyhow::Result<i64> {
        composer_db::models::session_log::count_by_session(&self.db.pool, session_id).await
    }

    pub async fn has_logs_before(&self, session_id: &str, before_id: i64) -> anyhow::Result<bool> {
        composer_db::models::session_log::has_logs_before(&self.db.pool, session_id, before_id)
            .await
    }

    pub async fn send_input(&self, id: &str, message: String) -> anyhow::Result<()> {
        let session = composer_db::models::session::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        if !matches!(session.status, SessionStatus::Running) {
            return Err(anyhow::anyhow!(
                "Session is not running (status: {:?})",
                session.status
            ));
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
