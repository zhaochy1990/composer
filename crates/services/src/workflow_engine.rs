use crate::event_bus::EventBus;
use crate::session_service::SessionService;
use composer_api_types::*;
use composer_db::Database;
use std::sync::Arc;

/// The built-in "Feat-Common" workflow definition with review-fix loops:
/// 1. Plan → 2. Human review plan → 3. Implement + build/test + PR →
/// 4. PR review (separate session) → 5. Fix review findings → loop back to 4 (max 3 retries) →
/// 6. Human review PR → 7. Fix human comments → loop back to 6 → 8. Complete PR → Done
pub fn feat_common_definition() -> WorkflowDefinition {
    // NOTE: Prompt templates use {{step_N}} with hardcoded indices.
    // If steps are reordered, these references must be updated:
    //   - Step 4 ("Fix Review Findings") references {{step_3}} (Automated PR Review output)
    //   - Step 6 ("Fix Human Comments") references {{rejection}} (Human PR Review comments)
    WorkflowDefinition {
        steps: vec![
            WorkflowStepDefinition {
                step_type: WorkflowStepType::Agentic,
                name: "Plan".to_string(),
                prompt_template: Some("{{task}}\n\nInvestigate the existing codebase and create a detailed implementation plan. Do NOT implement yet. Only output the plan.{{rejection}}".to_string()),
                max_retries: None,
                loop_back_to: None,
                session_mode: Some(SessionMode::New),
            },
            WorkflowStepDefinition {
                step_type: WorkflowStepType::HumanGate,
                name: "Review Plan".to_string(),
                prompt_template: None,
                max_retries: None,
                loop_back_to: None,
                session_mode: None,
            },
            WorkflowStepDefinition {
                step_type: WorkflowStepType::Agentic,
                name: "Implement & Create PR".to_string(),
                prompt_template: Some("{{task}}\n\nThe plan has been approved. Implement it now. After implementation, run build, lint, and tests. Fix any failures. Then create a PR.\n\nApproved plan:\n{{step_0}}".to_string()),
                max_retries: None,
                loop_back_to: None,
                session_mode: Some(SessionMode::Resume),
            },
            WorkflowStepDefinition {
                step_type: WorkflowStepType::Agentic,
                name: "Automated PR Review".to_string(),
                prompt_template: Some("Review the changes in the current branch. Provide a thorough code review. List any bugs, logic errors, security issues, code quality problems, and suggestions for improvement. Be specific about file names and line numbers.".to_string()),
                max_retries: None,
                loop_back_to: None,
                session_mode: Some(SessionMode::Separate),
            },
            WorkflowStepDefinition {
                step_type: WorkflowStepType::Agentic,
                name: "Fix Review Findings".to_string(),
                prompt_template: Some("The PR has been reviewed. Fix these findings:\n{{step_3}}\n\nThen push the changes.".to_string()),
                max_retries: Some(3),
                loop_back_to: Some(3),
                session_mode: Some(SessionMode::Resume),
            },
            WorkflowStepDefinition {
                step_type: WorkflowStepType::HumanGate,
                name: "Human PR Review".to_string(),
                prompt_template: None,
                max_retries: None,
                loop_back_to: None,
                session_mode: None,
            },
            WorkflowStepDefinition {
                step_type: WorkflowStepType::Agentic,
                name: "Fix Human Comments".to_string(),
                prompt_template: Some("The reviewer left these comments on the PR:\n{{rejection}}\n\nFix them and push the changes.".to_string()),
                max_retries: None,
                loop_back_to: Some(5),
                session_mode: Some(SessionMode::Resume),
            },
            WorkflowStepDefinition {
                step_type: WorkflowStepType::Agentic,
                name: "Complete PR".to_string(),
                prompt_template: Some("The implementation is complete. Find the PR you created earlier and complete it:\n1. Check the CI status using `gh pr checks`. Wait for all checks to pass (poll every 30 seconds, up to 10 minutes).\n2. If there are merge conflicts, resolve them and push.\n3. Once all checks pass, merge the PR using `gh pr merge --squash --delete-branch`.\n4. Confirm the PR is merged successfully.".to_string()),
                max_retries: None,
                loop_back_to: None,
                session_mode: Some(SessionMode::Resume),
            },
        ],
    }
}

pub const FEAT_COMMON_NAME: &str = "Feat-Common";

/// Validate a workflow definition:
/// - Agentic steps must have a non-empty `prompt_template`
/// - `loop_back_to` references must be non-negative and point to a preceding step
pub fn validate_workflow_definition(def: &WorkflowDefinition) -> anyhow::Result<()> {
    for (i, step) in def.steps.iter().enumerate() {
        if matches!(step.step_type, WorkflowStepType::Agentic) {
            let has_prompt = step
                .prompt_template
                .as_ref()
                .is_some_and(|p| !p.trim().is_empty());
            if !has_prompt {
                return Err(anyhow::anyhow!(
                    "Step {} '{}' is Agentic but has no prompt_template",
                    i,
                    step.name,
                ));
            }
        }
        if let Some(target) = step.loop_back_to {
            if target < 0 {
                return Err(anyhow::anyhow!(
                    "Step {} '{}' has negative loop_back_to ({})",
                    i,
                    step.name,
                    target
                ));
            }
            if target >= i as i32 {
                return Err(anyhow::anyhow!(
                    "Step {} '{}' has loop_back_to ({}) that is not a preceding step",
                    i,
                    step.name,
                    target
                ));
            }
        }
    }
    Ok(())
}

#[derive(Clone)]
pub struct WorkflowEngine {
    db: Arc<Database>,
    event_bus: EventBus,
    session_service: SessionService,
}

impl WorkflowEngine {
    pub fn new(db: Arc<Database>, event_bus: EventBus, session_service: SessionService) -> Self {
        let engine = Self {
            db,
            event_bus,
            session_service,
        };

        engine.spawn_startup_recovery();

        engine
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    /// Ensure the built-in "Feat-Common" workflow exists globally.
    /// Also updates the definition if it's stale (e.g., missing loop_back_to fields).
    pub async fn ensure_builtin_workflow(&self) -> anyhow::Result<Workflow> {
        let canonical = feat_common_definition();
        validate_workflow_definition(&canonical)?;

        if let Some(wf) =
            composer_db::models::workflow::find_by_name(&self.db.pool, FEAT_COMMON_NAME).await?
        {
            // Auto-update if the stored definition differs from the canonical one,
            // but only if there are no active (running/paused) workflow runs using it.
            if wf.definition != canonical {
                let active_runs =
                    composer_db::models::workflow_run::find_running(&self.db.pool).await?;
                let has_active_runs = active_runs.iter().any(|r| r.workflow_id == wf.id);
                if !has_active_runs {
                    let updated = composer_db::models::workflow::update(
                        &self.db.pool,
                        &wf.id.to_string(),
                        None,
                        Some(&canonical),
                    )
                    .await?;
                    tracing::info!("Updated built-in workflow '{}'", FEAT_COMMON_NAME,);
                    return Ok(updated);
                } else {
                    tracing::info!(
                        "Skipping update of built-in workflow '{}' — active runs exist",
                        FEAT_COMMON_NAME,
                    );
                }
            }
            return Ok(wf);
        }

        let wf = composer_db::models::workflow::create(&self.db.pool, FEAT_COMMON_NAME, &canonical)
            .await?;
        tracing::info!("Created built-in workflow '{}'", FEAT_COMMON_NAME);
        Ok(wf)
    }

    /// On startup, recover workflow runs that were in "running" status.
    /// The agent process died with the server — mark the current step as failed,
    /// then set the workflow run status so it can be resumed from that step.
    fn spawn_startup_recovery(&self) {
        let engine = self.clone();
        tokio::spawn(async move {
            // Migrate existing workflow definitions from old step types
            if let Err(e) = engine.migrate_workflow_definitions().await {
                tracing::error!("Failed to migrate workflow definitions: {}", e);
            }
            // Seed built-in workflows
            if let Err(e) = engine.ensure_builtin_workflow().await {
                tracing::error!("Failed to seed built-in workflow: {}", e);
            }
            if let Err(e) = engine.recover_running_workflows().await {
                tracing::error!("Failed to recover workflow runs: {}", e);
            }
        });
    }

    /// Migrate existing workflow definitions from old step types to new consolidated types.
    async fn migrate_workflow_definitions(&self) -> anyhow::Result<()> {
        let rows: Vec<(String, String)> = sqlx::query_as("SELECT id, definition FROM workflows")
            .fetch_all(&self.db.pool)
            .await?;

        for (id, def_json) in rows {
            let needs_migration = def_json.contains("\"step_type\":\"plan\"")
                || def_json.contains("\"step_type\":\"implement\"")
                || def_json.contains("\"step_type\":\"pr_review\"")
                || def_json.contains("\"step_type\":\"human_review\"")
                || def_json.contains("\"step_type\":\"complete_pr\"")
                // Also match with space after colon (serde_json pretty-print)
                || def_json.contains("\"step_type\": \"plan\"")
                || def_json.contains("\"step_type\": \"implement\"")
                || def_json.contains("\"step_type\": \"pr_review\"")
                || def_json.contains("\"step_type\": \"human_review\"")
                || def_json.contains("\"step_type\": \"complete_pr\"");

            if !needs_migration {
                continue;
            }

            let parse_result = serde_json::from_str::<serde_json::Value>(&def_json);
            if let Err(ref e) = parse_result {
                tracing::warn!(
                    "Failed to parse workflow {} definition for migration: {}",
                    id,
                    e
                );
            }
            if let Ok(mut value) = parse_result {
                if let Some(steps) = value.get_mut("steps").and_then(|s| s.as_array_mut()) {
                    for step in steps {
                        if let Some(st) = step
                            .get("step_type")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                        {
                            let has_prompt = step
                                .get("prompt_template")
                                .and_then(|v| v.as_str())
                                .is_some_and(|s| !s.is_empty());
                            match st.as_str() {
                                "plan" => {
                                    step["step_type"] = serde_json::json!("agentic");
                                    step["session_mode"] = serde_json::json!("new");
                                    if !has_prompt {
                                        step["prompt_template"] = serde_json::json!(
                                            "{{task}}\n\nInvestigate the existing codebase and create a detailed implementation plan. Do NOT implement yet. Only output the plan.{{rejection}}"
                                        );
                                    }
                                }
                                "implement" => {
                                    step["step_type"] = serde_json::json!("agentic");
                                    step["session_mode"] = serde_json::json!("resume");
                                    if !has_prompt {
                                        step["prompt_template"] = serde_json::json!(
                                            "{{task}}\n\nThe plan has been approved. Implement it now. After implementation, run build, lint, and tests. Fix any failures. Then create a PR.\n\nApproved plan:\n{{step_0}}"
                                        );
                                    }
                                }
                                "complete_pr" => {
                                    step["step_type"] = serde_json::json!("agentic");
                                    step["session_mode"] = serde_json::json!("resume");
                                    if !has_prompt {
                                        step["prompt_template"] = serde_json::json!(
                                            "The implementation is complete. Find the PR you created earlier and complete it:\n1. Check the CI status using `gh pr checks`. Wait for all checks to pass (poll every 30 seconds, up to 10 minutes).\n2. If there are merge conflicts, resolve them and push.\n3. Once all checks pass, merge the PR using `gh pr merge --squash --delete-branch`.\n4. Confirm the PR is merged successfully."
                                        );
                                    }
                                }
                                "pr_review" => {
                                    step["step_type"] = serde_json::json!("agentic");
                                    step["session_mode"] = serde_json::json!("separate");
                                    if !has_prompt {
                                        step["prompt_template"] = serde_json::json!(
                                            "Review the changes in the current branch. Provide a thorough code review. List any bugs, logic errors, security issues, code quality problems, and suggestions for improvement. Be specific about file names and line numbers."
                                        );
                                    }
                                }
                                "human_review" => {
                                    step["step_type"] = serde_json::json!("human_gate");
                                }
                                _ => {}
                            }
                        }
                    }
                }

                let updated = serde_json::to_string(&value)?;
                sqlx::query("UPDATE workflows SET definition = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
                    .bind(&updated)
                    .bind(&id)
                    .execute(&self.db.pool)
                    .await?;
                tracing::info!("Migrated workflow {} definition to new step types", id);
            }
        }

        Ok(())
    }

    async fn recover_running_workflows(&self) -> anyhow::Result<()> {
        let running_runs = composer_db::models::workflow_run::find_running(&self.db.pool).await?;
        if running_runs.is_empty() {
            return Ok(());
        }
        tracing::warn!("Recovering {} orphaned workflow run(s)", running_runs.len());

        for run in running_runs {
            let run_id = run.id.to_string();

            // Mark the current step as failed (agent process is gone)
            if let Some(step_output) = composer_db::models::workflow_step_output::latest_for_step(
                &self.db.pool,
                &run_id,
                run.current_step_index,
            )
            .await?
            {
                if matches!(step_output.status, WorkflowStepStatus::Running) {
                    composer_db::models::workflow_step_output::update_status_and_output(
                        &self.db.pool,
                        &step_output.id.to_string(),
                        &WorkflowStepStatus::Failed,
                        Some("Server restarted while step was running"),
                    )
                    .await?;
                }
            }

            // Set workflow run to paused so it can be resumed
            composer_db::models::workflow_run::update_status(
                &self.db.pool,
                &run_id,
                &WorkflowRunStatus::Paused,
            )
            .await?;

            // Set task to waiting so the user knows action is needed
            composer_db::models::task::update_status(
                &self.db.pool,
                &run.task_id.to_string(),
                &TaskStatus::Waiting,
            )
            .await?;

            tracing::warn!(
                "Workflow run {} (task {}) paused at step {} for recovery",
                run_id,
                run.task_id,
                run.current_step_index
            );
        }

        Ok(())
    }

    /// Resume a workflow run that was paused due to server restart.
    /// Re-executes the current step from scratch (the agent session can be resumed
    /// since the worktree and Claude session ID are preserved).
    pub async fn resume_run(&self, run_id: &str) -> anyhow::Result<WorkflowRun> {
        let run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;

        if !matches!(
            run.status,
            WorkflowRunStatus::Paused | WorkflowRunStatus::Failed
        ) {
            return Err(anyhow::anyhow!(
                "Workflow run cannot be resumed from status {:?}",
                run.status
            ));
        }

        let workflow =
            composer_db::models::workflow::find_by_id(&self.db.pool, &run.workflow_id.to_string())
                .await?
                .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;

        let task = composer_db::models::task::find_by_id(&self.db.pool, &run.task_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        let agent_id = task
            .assigned_agent_id
            .ok_or_else(|| anyhow::anyhow!("Task has no assigned agent"))?;

        // Move task back to in_progress
        composer_db::models::task::update_status(
            &self.db.pool,
            &task.id.to_string(),
            &TaskStatus::InProgress,
        )
        .await?;

        // Re-execute the current step
        self.execute_step(run_id, &workflow, &task, agent_id, run.current_step_index)
            .await?;

        composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))
    }

    /// Start a workflow run for a task. Creates the run, then executes the first step.
    pub async fn start(&self, task_id: &str, workflow_id: &str) -> anyhow::Result<WorkflowRun> {
        let task = composer_db::models::task::find_by_id(&self.db.pool, task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        if !matches!(task.status, TaskStatus::Backlog) {
            return Err(anyhow::anyhow!(
                "Task must be in backlog to start a workflow"
            ));
        }

        let agent_id = task
            .assigned_agent_id
            .ok_or_else(|| anyhow::anyhow!("Task has no assigned agent"))?;

        let workflow = composer_db::models::workflow::find_by_id(&self.db.pool, workflow_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;

        if workflow.definition.steps.is_empty() {
            return Err(anyhow::anyhow!("Workflow has no steps"));
        }

        // Validate loop_back_to references
        validate_workflow_definition(&workflow.definition)?;

        // Create workflow run
        let run =
            composer_db::models::workflow_run::create(&self.db.pool, workflow_id, task_id).await?;

        // Link run to task
        composer_db::models::task::update_workflow_run_id(
            &self.db.pool,
            task_id,
            &run.id.to_string(),
        )
        .await?;

        // Transition task from Backlog to InProgress before executing steps
        composer_db::models::task::update_status(&self.db.pool, task_id, &TaskStatus::InProgress)
            .await?;
        self.event_bus.broadcast(WsEvent::TaskMoved {
            task_id: task.id,
            from_status: TaskStatus::Backlog,
            to_status: TaskStatus::InProgress,
        });

        self.event_bus
            .broadcast(WsEvent::WorkflowRunUpdated(run.clone()));

        // Execute the first step
        self.execute_step(&run.id.to_string(), &workflow, &task, agent_id, 0)
            .await?;

        // Re-fetch after step execution
        composer_db::models::workflow_run::find_by_id(&self.db.pool, &run.id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))
    }

    /// Execute a specific step in the workflow.
    async fn execute_step(
        &self,
        run_id: &str,
        workflow: &Workflow,
        task: &Task,
        agent_id: uuid::Uuid,
        step_index: i32,
    ) -> anyhow::Result<()> {
        let step_def = workflow
            .definition
            .steps
            .get(step_index as usize)
            .ok_or_else(|| anyhow::anyhow!("Step index {} out of bounds", step_index))?;

        composer_db::models::workflow_run::update_step_index(&self.db.pool, run_id, step_index)
            .await?;

        match step_def.step_type {
            WorkflowStepType::Agentic => {
                let mode = step_def.session_mode.clone().unwrap_or(SessionMode::Resume);
                match mode {
                    SessionMode::New => {
                        self.execute_agent_step(
                            run_id, task, agent_id, step_index, &step_def, true,
                        )
                        .await?;
                    }
                    SessionMode::Resume => {
                        self.execute_agent_step(
                            run_id, task, agent_id, step_index, &step_def, false,
                        )
                        .await?;
                    }
                    SessionMode::Separate => {
                        self.execute_pr_review(run_id, task, agent_id, step_index, &step_def)
                            .await?;
                    }
                }
            }
            WorkflowStepType::HumanGate => {
                self.execute_human_gate(run_id, task, step_index, &step_def)
                    .await?;
            }
        }

        Ok(())
    }

    /// Execute an agentic step. Uses the main session for New/Resume modes, or a separate session for Separate mode.
    async fn execute_agent_step(
        &self,
        run_id: &str,
        task: &Task,
        agent_id: uuid::Uuid,
        step_index: i32,
        step_def: &WorkflowStepDefinition,
        is_new_session: bool,
    ) -> anyhow::Result<()> {
        let prompt = self
            .build_prompt(run_id, task, step_index, step_def)
            .await?;

        // Create step output record
        let step_output = composer_db::models::workflow_step_output::create(
            &self.db.pool,
            run_id,
            step_index,
            &step_def.step_type,
            &WorkflowStepStatus::Running,
            None,
        )
        .await?;

        let run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;

        let session = if is_new_session || run.main_session_id.is_none() {
            // First step — create a new session.
            // Named "Plan & Implementation" because this session is reused (via --resume)
            // for both the initial plan step and subsequent implement/fix steps.
            let repo_path = self.get_repo_path(task).await?;
            let session = self
                .session_service
                .create_session(CreateSessionRequest {
                    agent_id,
                    task_id: task.id,
                    prompt,
                    repo_path,
                    name: Some("Plan & Implementation".to_string()),
                    auto_approve: Some(task.auto_approve),
                    exit_on_result: true,
                })
                .await?;
            composer_db::models::workflow_run::update_main_session(
                &self.db.pool,
                run_id,
                &session.id.to_string(),
            )
            .await?;
            session
        } else {
            // Resume existing main session
            let main_session_id = run.main_session_id.unwrap().to_string();
            self.session_service
                .resume_session(
                    &main_session_id,
                    ResumeSessionRequest {
                        prompt: Some(prompt),
                        exit_on_result: true,
                        continue_chat: false,
                    },
                )
                .await?
        };

        // Update step output with session_id
        composer_db::models::workflow_step_output::update_status_and_output(
            &self.db.pool,
            &step_output.id.to_string(),
            &WorkflowStepStatus::Running,
            None,
        )
        .await?;

        // Update run status
        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            run_id,
            &WorkflowRunStatus::Running,
        )
        .await?;

        let updated_run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;
        self.event_bus
            .broadcast(WsEvent::WorkflowRunUpdated(updated_run));
        self.event_bus.broadcast(WsEvent::WorkflowStepChanged {
            workflow_run_id: run.id,
            step: composer_db::models::workflow_step_output::find_by_id(
                &self.db.pool,
                &step_output.id.to_string(),
            )
            .await?
            .unwrap(),
        });

        let _ = session; // used above for session creation
        Ok(())
    }

    /// Pause the workflow for human review/approval.
    async fn execute_human_gate(
        &self,
        run_id: &str,
        task: &Task,
        step_index: i32,
        step_def: &WorkflowStepDefinition,
    ) -> anyhow::Result<()> {
        let step_output = composer_db::models::workflow_step_output::create(
            &self.db.pool,
            run_id,
            step_index,
            &step_def.step_type,
            &WorkflowStepStatus::WaitingForHuman,
            None,
        )
        .await?;

        // Pause the workflow run
        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            run_id,
            &WorkflowRunStatus::Paused,
        )
        .await?;

        // Move task to waiting
        composer_db::models::task::update_status(
            &self.db.pool,
            &task.id.to_string(),
            &TaskStatus::Waiting,
        )
        .await?;

        let run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;
        self.event_bus.broadcast(WsEvent::WorkflowRunUpdated(run));
        self.event_bus.broadcast(WsEvent::WorkflowWaitingForHuman {
            workflow_run_id: run_id.parse()?,
            task_id: task.id,
            step_index,
        });
        self.event_bus.broadcast(WsEvent::WorkflowStepChanged {
            workflow_run_id: run_id.parse()?,
            step: step_output,
        });
        self.event_bus.broadcast(WsEvent::TaskMoved {
            task_id: task.id,
            from_status: TaskStatus::InProgress,
            to_status: TaskStatus::Waiting,
        });

        Ok(())
    }

    /// Spawn a separate review session for the PR.
    async fn execute_pr_review(
        &self,
        run_id: &str,
        task: &Task,
        agent_id: uuid::Uuid,
        step_index: i32,
        step_def: &WorkflowStepDefinition,
    ) -> anyhow::Result<()> {
        let prompt = self
            .build_prompt(run_id, task, step_index, step_def)
            .await?;
        let repo_path = self.get_repo_path(task).await?;

        // Create a NEW session for the reviewer (not the main session)
        let session = self
            .session_service
            .create_session(CreateSessionRequest {
                agent_id,
                task_id: task.id,
                prompt,
                repo_path,
                name: Some("PR Review".to_string()),
                auto_approve: Some(task.auto_approve),
                exit_on_result: true,
            })
            .await?;

        let step_output = composer_db::models::workflow_step_output::create(
            &self.db.pool,
            run_id,
            step_index,
            &step_def.step_type,
            &WorkflowStepStatus::Running,
            Some(&session.id.to_string()),
        )
        .await?;

        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            run_id,
            &WorkflowRunStatus::Running,
        )
        .await?;

        let run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;
        self.event_bus.broadcast(WsEvent::WorkflowRunUpdated(run));
        self.event_bus.broadcast(WsEvent::WorkflowStepChanged {
            workflow_run_id: run_id.parse()?,
            step: step_output,
        });

        Ok(())
    }

    /// Called when a session completes. Determines if it belongs to a workflow and advances.
    pub async fn on_session_completed(
        &self,
        session_id: &str,
        result_summary: Option<&str>,
    ) -> anyhow::Result<bool> {
        // Check if this is the main session of a workflow run
        let run =
            composer_db::models::workflow_run::find_by_session(&self.db.pool, session_id).await?;

        // Or check if it's a review session referenced in step outputs
        let run = match run {
            Some(r) => Some(r),
            None => {
                composer_db::models::workflow_run::find_by_step_session(&self.db.pool, session_id)
                    .await?
            }
        };

        let run = match run {
            Some(r) => r,
            None => return Ok(false), // Not a workflow session
        };

        // Don't advance if workflow is already completed/failed
        if matches!(
            run.status,
            WorkflowRunStatus::Completed | WorkflowRunStatus::Failed
        ) {
            return Ok(true);
        }

        let run_id = run.id.to_string();
        let workflow =
            composer_db::models::workflow::find_by_id(&self.db.pool, &run.workflow_id.to_string())
                .await?
                .ok_or_else(|| anyhow::anyhow!("Workflow not found for run"))?;

        // Update current step output with the result
        if let Some(step_output) = composer_db::models::workflow_step_output::latest_for_step(
            &self.db.pool,
            &run_id,
            run.current_step_index,
        )
        .await?
        {
            composer_db::models::workflow_step_output::update_status_and_output(
                &self.db.pool,
                &step_output.id.to_string(),
                &WorkflowStepStatus::Completed,
                result_summary,
            )
            .await?;

            self.event_bus.broadcast(WsEvent::WorkflowStepChanged {
                workflow_run_id: run.id,
                step: composer_db::models::workflow_step_output::find_by_id(
                    &self.db.pool,
                    &step_output.id.to_string(),
                )
                .await?
                .unwrap(),
            });
        }

        // Advance to next step
        self.advance(&run_id, &workflow).await?;

        Ok(true)
    }

    /// Called when a session fails. Marks the workflow step as failed.
    pub async fn on_session_failed(&self, session_id: &str, error: &str) -> anyhow::Result<bool> {
        let run =
            composer_db::models::workflow_run::find_by_session(&self.db.pool, session_id).await?;
        let run = match run {
            Some(r) => r,
            None => {
                // Check review sessions
                match composer_db::models::workflow_run::find_by_step_session(
                    &self.db.pool,
                    session_id,
                )
                .await?
                {
                    Some(r) => r,
                    None => return Ok(false),
                }
            }
        };

        let run_id = run.id.to_string();

        if let Some(step_output) = composer_db::models::workflow_step_output::latest_for_step(
            &self.db.pool,
            &run_id,
            run.current_step_index,
        )
        .await?
        {
            composer_db::models::workflow_step_output::update_status_and_output(
                &self.db.pool,
                &step_output.id.to_string(),
                &WorkflowStepStatus::Failed,
                Some(error),
            )
            .await?;
        }

        // For now, fail the whole workflow run on step failure
        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            &run_id,
            &WorkflowRunStatus::Failed,
        )
        .await?;

        let updated_run = composer_db::models::workflow_run::find_by_id(&self.db.pool, &run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;
        self.event_bus
            .broadcast(WsEvent::WorkflowRunUpdated(updated_run));

        Ok(true)
    }

    /// Handle a human decision (approve/reject) on a human gate step.
    pub async fn submit_decision(
        &self,
        run_id: &str,
        approved: bool,
        comments: Option<&str>,
    ) -> anyhow::Result<WorkflowRun> {
        let run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;

        if !matches!(run.status, WorkflowRunStatus::Paused) {
            return Err(anyhow::anyhow!(
                "Workflow run is not paused (status: {:?})",
                run.status
            ));
        }

        let workflow =
            composer_db::models::workflow::find_by_id(&self.db.pool, &run.workflow_id.to_string())
                .await?
                .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;

        let step_def = workflow
            .definition
            .steps
            .get(run.current_step_index as usize)
            .ok_or_else(|| anyhow::anyhow!("Current step not found in workflow definition"))?;

        // Update step output
        if let Some(step_output) = composer_db::models::workflow_step_output::latest_for_step(
            &self.db.pool,
            run_id,
            run.current_step_index,
        )
        .await?
        {
            let status = if approved {
                WorkflowStepStatus::Completed
            } else {
                WorkflowStepStatus::Rejected
            };
            composer_db::models::workflow_step_output::update_status_and_output(
                &self.db.pool,
                &step_output.id.to_string(),
                &status,
                comments,
            )
            .await?;

            self.event_bus.broadcast(WsEvent::WorkflowStepChanged {
                workflow_run_id: run.id,
                step: composer_db::models::workflow_step_output::find_by_id(
                    &self.db.pool,
                    &step_output.id.to_string(),
                )
                .await?
                .unwrap(),
            });
        }

        if approved {
            // Move task back to in_progress before advancing
            composer_db::models::task::update_status(
                &self.db.pool,
                &run.task_id.to_string(),
                &TaskStatus::InProgress,
            )
            .await?;
            self.event_bus.broadcast(WsEvent::TaskMoved {
                task_id: run.task_id,
                from_status: TaskStatus::Waiting,
                to_status: TaskStatus::InProgress,
            });

            // Move to next step
            self.advance(run_id, &workflow).await?;
        } else {
            // Determine where to loop back: use step's loop_back_to if configured,
            // otherwise fall back to the closest preceding agent step.
            let loop_back_index = step_def.loop_back_to.unwrap_or_else(|| {
                self.find_preceding_agent_step(&workflow, run.current_step_index)
            });
            let task =
                composer_db::models::task::find_by_id(&self.db.pool, &run.task_id.to_string())
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
            let agent_id = task
                .assigned_agent_id
                .ok_or_else(|| anyhow::anyhow!("Task has no assigned agent"))?;

            composer_db::models::workflow_run::increment_iteration(&self.db.pool, run_id).await?;

            // Move task back to in_progress
            composer_db::models::task::update_status(
                &self.db.pool,
                &task.id.to_string(),
                &TaskStatus::InProgress,
            )
            .await?;

            self.event_bus.broadcast(WsEvent::TaskMoved {
                task_id: task.id,
                from_status: TaskStatus::Waiting,
                to_status: TaskStatus::InProgress,
            });

            // Re-execute the target step (execute_step handles step index update,
            // session mode dispatch, and step type dispatch properly)
            self.execute_step(run_id, &workflow, &task, agent_id, loop_back_index)
                .await?;
        }

        composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))
    }

    /// Advance the workflow to the next step after current step completes.
    async fn advance(&self, run_id: &str, workflow: &Workflow) -> anyhow::Result<()> {
        let run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;

        // Check if the current step has a loop_back_to target
        let next_index = if let Some(step_def) = workflow
            .definition
            .steps
            .get(run.current_step_index as usize)
        {
            if let Some(loop_target) = step_def.loop_back_to {
                if self
                    .should_loop(run_id, workflow, step_def, loop_target)
                    .await?
                {
                    tracing::info!(
                        "Workflow run {}: looping from step {} back to step {}",
                        run_id,
                        run.current_step_index,
                        loop_target
                    );
                    composer_db::models::workflow_run::increment_iteration(&self.db.pool, run_id)
                        .await?;
                    loop_target
                } else {
                    tracing::info!(
                        "Workflow run {}: max retries reached at step {}, advancing linearly",
                        run_id,
                        run.current_step_index
                    );
                    run.current_step_index + 1
                }
            } else {
                run.current_step_index + 1
            }
        } else {
            run.current_step_index + 1
        };

        if next_index as usize >= workflow.definition.steps.len() {
            // Workflow complete
            composer_db::models::workflow_run::update_status(
                &self.db.pool,
                run_id,
                &WorkflowRunStatus::Completed,
            )
            .await?;
            composer_db::models::task::update_status(
                &self.db.pool,
                &run.task_id.to_string(),
                &TaskStatus::Done,
            )
            .await?;

            // Clean up all session worktrees (main + review sessions)
            if let Some(main_sid) = &run.main_session_id {
                self.cleanup_worktree(&main_sid.to_string()).await;
            }
            // Clean up review session worktrees (stored in step outputs)
            let step_outputs =
                composer_db::models::workflow_step_output::list_by_run(&self.db.pool, run_id)
                    .await
                    .unwrap_or_default();
            for step in &step_outputs {
                if let Some(ref sid) = step.session_id {
                    let sid_str = sid.to_string();
                    // Skip main session (already cleaned up above)
                    if run.main_session_id.map(|m| m.to_string()) != Some(sid_str.clone()) {
                        self.cleanup_worktree(&sid_str).await;
                    }
                }
            }

            let updated_run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;
            self.event_bus.broadcast(WsEvent::WorkflowRunCompleted {
                workflow_run_id: run.id,
                task_id: run.task_id,
            });
            self.event_bus
                .broadcast(WsEvent::WorkflowRunUpdated(updated_run));
            self.event_bus.broadcast(WsEvent::TaskMoved {
                task_id: run.task_id,
                from_status: TaskStatus::InProgress,
                to_status: TaskStatus::Done,
            });
            return Ok(());
        }

        let task = composer_db::models::task::find_by_id(&self.db.pool, &run.task_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        let agent_id = task
            .assigned_agent_id
            .ok_or_else(|| anyhow::anyhow!("Task has no assigned agent"))?;

        self.execute_step(run_id, workflow, &task, agent_id, next_index)
            .await
    }

    /// Build the prompt for a step, injecting context from previous steps.
    /// Uses the step's `prompt_template` with variable substitution:
    /// - `{{task}}` — task title + description + project instructions
    /// - `{{step_N}}` — latest output from step N
    /// - `{{rejection}}` — latest rejected HumanGate output's comments
    async fn build_prompt(
        &self,
        run_id: &str,
        task: &Task,
        current_step_index: i32,
        step_def: &WorkflowStepDefinition,
    ) -> anyhow::Result<String> {
        // Human gates don't need prompts
        if step_def.step_type == WorkflowStepType::HumanGate {
            return Ok(String::new());
        }

        let template = step_def.prompt_template.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Agentic step '{}' requires a prompt_template",
                step_def.name
            )
        })?;

        let step_outputs =
            composer_db::models::workflow_step_output::list_by_run(&self.db.pool, run_id).await?;

        // Build {{task}} context
        let base_task_context = if let Some(ref desc) = task.description {
            format!("Task: {} - {}", task.title, desc)
        } else {
            format!("Task: {}", task.title)
        };

        let task_context = if let Some(ref pid) = task.project_id {
            let instructions = composer_db::models::project_instruction::list_by_project(
                &self.db.pool,
                &pid.to_string(),
            )
            .await?;
            match composer_db::models::project_instruction::format_instructions_block(&instructions)
            {
                Some(block) => format!("{}\n\n{}", block, base_task_context),
                None => base_task_context,
            }
        } else {
            base_task_context
        };

        let mut prompt = template.clone();
        prompt = prompt.replace("{{task}}", &task_context);

        // Inject {{step_N}} — resolves to latest completed/rejected output for step N.
        // Filter to terminal statuses to avoid picking up in-progress or failed outputs.
        let max_step = step_outputs.iter().map(|o| o.step_index).max().unwrap_or(0);
        for step_index in 0..=max_step {
            let key = format!("{{{{step_{}}}}}", step_index);
            if prompt.contains(&key) {
                let latest_output = step_outputs
                    .iter()
                    .filter(|o| {
                        o.step_index == step_index
                            && matches!(
                                o.status,
                                WorkflowStepStatus::Completed | WorkflowStepStatus::Rejected
                            )
                    })
                    .last()
                    .and_then(|o| o.output.as_deref())
                    .unwrap_or("");
                prompt = prompt.replace(&key, latest_output);
            }
        }

        // Inject {{rejection}} — most recent rejected HumanGate output scoped
        // to human gates that follow the current step (i.e., the gate that
        // triggered this re-execution). Falls back to global latest if none found.
        if prompt.contains("{{rejection}}") {
            let scoped = step_outputs
                .iter()
                .filter(|o| {
                    o.step_type == WorkflowStepType::HumanGate
                        && o.status == WorkflowStepStatus::Rejected
                        && o.step_index > current_step_index
                })
                .last();
            let fallback = step_outputs
                .iter()
                .filter(|o| {
                    o.step_type == WorkflowStepType::HumanGate
                        && o.status == WorkflowStepStatus::Rejected
                })
                .last();
            let rejection_text = scoped
                .or(fallback)
                .and_then(|o| o.output.as_deref())
                .unwrap_or("");
            if rejection_text.is_empty() {
                prompt = prompt.replace("{{rejection}}", "");
            } else {
                prompt = prompt.replace("{{rejection}}",
                    &format!("\n\nThe previous output was rejected. Feedback: {}\nPlease revise based on this feedback.", rejection_text));
            }
        }

        Ok(prompt.trim().to_string())
    }

    /// Find the preceding agent step to loop back to on rejection.
    fn find_preceding_agent_step(&self, workflow: &Workflow, current_index: i32) -> i32 {
        for i in (0..current_index).rev() {
            let step = &workflow.definition.steps[i as usize];
            if matches!(step.step_type, WorkflowStepType::Agentic) {
                return i;
            }
        }
        0 // fallback to first step
    }

    /// Determine whether a step with `loop_back_to` should actually loop back
    /// or if the loop should terminate (max retries exhausted, or human approved).
    pub async fn should_loop(
        &self,
        run_id: &str,
        workflow: &Workflow,
        step_def: &WorkflowStepDefinition,
        loop_target: i32,
    ) -> anyhow::Result<bool> {
        // If the loop target is a HumanGate step, check if the most recent
        // output for that step was approved (Completed).
        // If approved, don't loop — the human is satisfied.
        if let Some(target_step_def) = workflow.definition.steps.get(loop_target as usize) {
            if matches!(target_step_def.step_type, WorkflowStepType::HumanGate) {
                // Use latest_for_step for deterministic ordering (ORDER BY attempt DESC LIMIT 1)
                let latest_target_output =
                    composer_db::models::workflow_step_output::latest_for_step(
                        &self.db.pool,
                        run_id,
                        loop_target,
                    )
                    .await?;
                if let Some(output) = latest_target_output {
                    if output.status == WorkflowStepStatus::Completed {
                        // Human approved at the target step — don't loop back
                        return Ok(false);
                    }
                }
            }
        }

        // For automated loops, respect max_retries by counting completions
        // of the loop target step (review step). The first execution is not a
        // retry, so we stop when (completions - 1) >= max_retries.
        if let Some(max) = step_def.max_retries {
            let step_outputs =
                composer_db::models::workflow_step_output::list_by_run(&self.db.pool, run_id)
                    .await?;
            let target_completed_count = step_outputs
                .iter()
                .filter(|o| {
                    o.step_index == loop_target && matches!(o.status, WorkflowStepStatus::Completed)
                })
                .count() as i32;

            let retries_done = (target_completed_count - 1).max(0);
            if retries_done >= max {
                return Ok(false);
            }
        }

        // No limit reached: loop back
        Ok(true)
    }

    /// Get the repo path for a task's project.
    async fn get_repo_path(&self, task: &Task) -> anyhow::Result<String> {
        let project_id = task
            .project_id
            .ok_or_else(|| anyhow::anyhow!("Task has no project assigned"))?;
        let repos = composer_db::models::project_repository::list_by_project(
            &self.db.pool,
            &project_id.to_string(),
        )
        .await?;
        let primary_repo = repos
            .iter()
            .find(|r| r.role == RepositoryRole::Primary)
            .or_else(|| repos.first())
            .ok_or_else(|| anyhow::anyhow!("Project has no repositories configured"))?;
        Ok(primary_repo.local_path.clone())
    }

    /// Clean up a session's worktree.
    async fn cleanup_worktree(&self, session_id: &str) {
        if let Ok(Some(session)) =
            composer_db::models::session::find_by_id(&self.db.pool, session_id).await
        {
            if let Some(wt_id) = &session.worktree_id {
                let wt_id_str = wt_id.to_string();
                if let Ok(Some(wt)) =
                    composer_db::models::worktree::find_by_id(&self.db.pool, &wt_id_str).await
                {
                    if wt.status != WorktreeStatus::Deleted {
                        let _ = composer_git::worktree::remove_worktree(
                            std::path::Path::new(&wt.repo_path),
                            std::path::Path::new(&wt.worktree_path),
                            &wt.branch_name,
                        )
                        .await;
                        let _ = composer_db::models::worktree::update_status(
                            &self.db.pool,
                            &wt_id_str,
                            &WorktreeStatus::Deleted,
                        )
                        .await;
                    }
                }
            }
        }
    }

    /// Get the workflow run with all step outputs for a task.
    pub async fn get_run_with_steps(
        &self,
        run_id: &str,
    ) -> anyhow::Result<(WorkflowRun, Vec<WorkflowStepOutput>)> {
        let run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;
        let steps =
            composer_db::models::workflow_step_output::list_by_run(&self.db.pool, run_id).await?;
        Ok((run, steps))
    }
}
