use std::sync::Arc;
use composer_api_types::*;
use composer_db::Database;
use crate::event_bus::EventBus;
use crate::session_service::SessionService;

/// The built-in "Feat-Common" workflow definition matching the 10-step feature development flow:
/// 1. Plan → 2. Human review plan → 3. Implement + build/test + PR →
/// 4. PR review (separate session) → 5. Fix review findings →
/// 6. Loop 4-5 → 7. Human review PR → 8. Fix human comments →
/// 9. Loop 7-8 → 10. Done
pub fn feat_common_definition() -> WorkflowDefinition {
    WorkflowDefinition {
        steps: vec![
            WorkflowStepDefinition {
                step_type: WorkflowStepType::Plan,
                name: "Plan".to_string(),
                prompt_template: None,
                max_retries: None,
            },
            WorkflowStepDefinition {
                step_type: WorkflowStepType::HumanGate,
                name: "Review Plan".to_string(),
                prompt_template: None,
                max_retries: None,
            },
            WorkflowStepDefinition {
                step_type: WorkflowStepType::Implement,
                name: "Implement & Create PR".to_string(),
                prompt_template: None,
                max_retries: None,
            },
            WorkflowStepDefinition {
                step_type: WorkflowStepType::PrReview,
                name: "Automated PR Review".to_string(),
                prompt_template: None,
                max_retries: None,
            },
            WorkflowStepDefinition {
                step_type: WorkflowStepType::Implement,
                name: "Fix Review Findings".to_string(),
                prompt_template: None,
                max_retries: None,
            },
            WorkflowStepDefinition {
                step_type: WorkflowStepType::HumanReview,
                name: "Human PR Review".to_string(),
                prompt_template: None,
                max_retries: None,
            },
            WorkflowStepDefinition {
                step_type: WorkflowStepType::Implement,
                name: "Fix Human Comments".to_string(),
                prompt_template: None,
                max_retries: None,
            },
        ],
    }
}

pub const FEAT_COMMON_NAME: &str = "Feat-Common";

#[derive(Clone)]
pub struct WorkflowEngine {
    db: Arc<Database>,
    event_bus: EventBus,
    session_service: SessionService,
}

impl WorkflowEngine {
    pub fn new(db: Arc<Database>, event_bus: EventBus, session_service: SessionService) -> Self {
        let engine = Self { db, event_bus, session_service };

        engine.spawn_startup_recovery();

        engine
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    /// Ensure the built-in "Feat-Common" workflow exists for a given project.
    /// Called when a project is created or on first use. Returns the workflow id.
    pub async fn ensure_builtin_workflow(&self, project_id: &str) -> anyhow::Result<Workflow> {
        // Check if it already exists for this project
        let existing = composer_db::models::workflow::list_by_project(&self.db.pool, project_id).await?;
        if let Some(wf) = existing.into_iter().find(|w| w.name == FEAT_COMMON_NAME) {
            return Ok(wf);
        }

        // Create it
        let definition = feat_common_definition();
        let wf = composer_db::models::workflow::create(
            &self.db.pool,
            project_id,
            FEAT_COMMON_NAME,
            &definition,
        ).await?;
        tracing::info!("Created built-in workflow '{}' for project {}", FEAT_COMMON_NAME, project_id);
        Ok(wf)
    }

    /// On startup, recover workflow runs that were in "running" status.
    /// The agent process died with the server — mark the current step as failed,
    /// then set the workflow run status so it can be resumed from that step.
    fn spawn_startup_recovery(&self) {
        let engine = self.clone();
        tokio::spawn(async move {
            if let Err(e) = engine.recover_running_workflows().await {
                tracing::error!("Failed to recover workflow runs: {}", e);
            }
        });
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
            ).await? {
                if matches!(step_output.status, WorkflowStepStatus::Running) {
                    composer_db::models::workflow_step_output::update_status_and_output(
                        &self.db.pool,
                        &step_output.id.to_string(),
                        &WorkflowStepStatus::Failed,
                        Some("Server restarted while step was running"),
                    ).await?;
                }
            }

            // Set workflow run to paused so it can be resumed
            composer_db::models::workflow_run::update_status(
                &self.db.pool,
                &run_id,
                &WorkflowRunStatus::Paused,
            ).await?;

            // Set task to waiting so the user knows action is needed
            composer_db::models::task::update_status(
                &self.db.pool,
                &run.task_id.to_string(),
                &TaskStatus::Waiting,
            ).await?;

            tracing::warn!(
                "Workflow run {} (task {}) paused at step {} for recovery",
                run_id, run.task_id, run.current_step_index
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

        if !matches!(run.status, WorkflowRunStatus::Paused | WorkflowRunStatus::Failed) {
            return Err(anyhow::anyhow!("Workflow run cannot be resumed from status {:?}", run.status));
        }

        let workflow = composer_db::models::workflow::find_by_id(&self.db.pool, &run.workflow_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;

        let task = composer_db::models::task::find_by_id(&self.db.pool, &run.task_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        let agent_id = task.assigned_agent_id
            .ok_or_else(|| anyhow::anyhow!("Task has no assigned agent"))?;

        // Move task back to in_progress
        composer_db::models::task::update_status(
            &self.db.pool,
            &task.id.to_string(),
            &TaskStatus::InProgress,
        ).await?;

        // Re-execute the current step
        self.execute_step(run_id, &workflow, &task, agent_id, run.current_step_index).await?;

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
            return Err(anyhow::anyhow!("Task must be in backlog to start a workflow"));
        }

        let agent_id = task.assigned_agent_id
            .ok_or_else(|| anyhow::anyhow!("Task has no assigned agent"))?;

        let workflow = composer_db::models::workflow::find_by_id(&self.db.pool, workflow_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;

        if workflow.definition.steps.is_empty() {
            return Err(anyhow::anyhow!("Workflow has no steps"));
        }

        // Create workflow run
        let run = composer_db::models::workflow_run::create(
            &self.db.pool,
            workflow_id,
            task_id,
        ).await?;

        // Link run to task
        composer_db::models::task::update_workflow_run_id(
            &self.db.pool,
            task_id,
            &run.id.to_string(),
        ).await?;

        // Transition task from Backlog to InProgress before executing steps
        composer_db::models::task::update_status(
            &self.db.pool,
            task_id,
            &TaskStatus::InProgress,
        ).await?;
        self.event_bus.broadcast(WsEvent::TaskMoved {
            task_id: task.id,
            from_status: TaskStatus::Backlog,
            to_status: TaskStatus::InProgress,
        });

        self.event_bus.broadcast(WsEvent::WorkflowRunUpdated(run.clone()));

        // Execute the first step
        self.execute_step(&run.id.to_string(), &workflow, &task, agent_id, 0).await?;

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
        let step_def = workflow.definition.steps.get(step_index as usize)
            .ok_or_else(|| anyhow::anyhow!("Step index {} out of bounds", step_index))?;

        composer_db::models::workflow_run::update_step_index(&self.db.pool, run_id, step_index).await?;

        match step_def.step_type {
            WorkflowStepType::Plan => {
                self.execute_agent_step(run_id, task, agent_id, step_index, &step_def, true).await?;
            }
            WorkflowStepType::Implement => {
                self.execute_agent_step(run_id, task, agent_id, step_index, &step_def, false).await?;
            }
            WorkflowStepType::HumanGate | WorkflowStepType::HumanReview => {
                self.execute_human_gate(run_id, task, step_index, &step_def).await?;
            }
            WorkflowStepType::PrReview => {
                self.execute_pr_review(run_id, task, agent_id, step_index, &step_def).await?;
            }
        }

        Ok(())
    }

    /// Execute an agent task step (plan or implement). Uses the main session.
    async fn execute_agent_step(
        &self,
        run_id: &str,
        task: &Task,
        agent_id: uuid::Uuid,
        step_index: i32,
        step_def: &WorkflowStepDefinition,
        is_new_session: bool,
    ) -> anyhow::Result<()> {
        let prompt = self.build_prompt(run_id, task, step_def).await?;

        // Create step output record
        let step_output = composer_db::models::workflow_step_output::create(
            &self.db.pool,
            run_id,
            step_index,
            &step_def.step_type,
            &WorkflowStepStatus::Running,
            None,
        ).await?;

        let run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;

        let session = if is_new_session || run.main_session_id.is_none() {
            // First step — create a new session
            let repo_path = self.get_repo_path(task).await?;
            let session = self.session_service.create_session(CreateSessionRequest {
                agent_id,
                task_id: task.id,
                prompt,
                repo_path,
                auto_approve: Some(task.auto_approve),
            }).await?;
            composer_db::models::workflow_run::update_main_session(
                &self.db.pool,
                run_id,
                &session.id.to_string(),
            ).await?;
            session
        } else {
            // Resume existing main session
            let main_session_id = run.main_session_id.unwrap().to_string();
            self.session_service.resume_session(&main_session_id, ResumeSessionRequest {
                prompt: Some(prompt),
            }).await?
        };

        // Update step output with session_id
        composer_db::models::workflow_step_output::update_status_and_output(
            &self.db.pool,
            &step_output.id.to_string(),
            &WorkflowStepStatus::Running,
            None,
        ).await?;

        // Update run status
        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            run_id,
            &WorkflowRunStatus::Running,
        ).await?;

        let updated_run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;
        self.event_bus.broadcast(WsEvent::WorkflowRunUpdated(updated_run));
        self.event_bus.broadcast(WsEvent::WorkflowStepChanged {
            workflow_run_id: run.id,
            step: composer_db::models::workflow_step_output::find_by_id(
                &self.db.pool,
                &step_output.id.to_string(),
            ).await?.unwrap(),
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
        ).await?;

        // Pause the workflow run
        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            run_id,
            &WorkflowRunStatus::Paused,
        ).await?;

        // Move task to waiting
        composer_db::models::task::update_status(
            &self.db.pool,
            &task.id.to_string(),
            &TaskStatus::Waiting,
        ).await?;

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
        let prompt = self.build_prompt(run_id, task, step_def).await?;
        let repo_path = self.get_repo_path(task).await?;

        // Create a NEW session for the reviewer (not the main session)
        let session = self.session_service.create_session(CreateSessionRequest {
            agent_id,
            task_id: task.id,
            prompt,
            repo_path,
            auto_approve: Some(task.auto_approve),
        }).await?;

        let step_output = composer_db::models::workflow_step_output::create(
            &self.db.pool,
            run_id,
            step_index,
            &step_def.step_type,
            &WorkflowStepStatus::Running,
            Some(&session.id.to_string()),
        ).await?;

        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            run_id,
            &WorkflowRunStatus::Running,
        ).await?;

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
    pub async fn on_session_completed(&self, session_id: &str, result_summary: Option<&str>) -> anyhow::Result<bool> {
        // Check if this is the main session of a workflow run
        let run = composer_db::models::workflow_run::find_by_session(&self.db.pool, session_id).await?;

        // Or check if it's a review session referenced in step outputs
        let run = match run {
            Some(r) => Some(r),
            None => composer_db::models::workflow_run::find_by_step_session(&self.db.pool, session_id).await?,
        };

        let run = match run {
            Some(r) => r,
            None => return Ok(false), // Not a workflow session
        };

        // Don't advance if workflow is already completed/failed
        if matches!(run.status, WorkflowRunStatus::Completed | WorkflowRunStatus::Failed) {
            return Ok(true);
        }

        let run_id = run.id.to_string();
        let workflow = composer_db::models::workflow::find_by_id(&self.db.pool, &run.workflow_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found for run"))?;

        // Update current step output with the result
        if let Some(step_output) = composer_db::models::workflow_step_output::latest_for_step(
            &self.db.pool,
            &run_id,
            run.current_step_index,
        ).await? {
            composer_db::models::workflow_step_output::update_status_and_output(
                &self.db.pool,
                &step_output.id.to_string(),
                &WorkflowStepStatus::Completed,
                result_summary,
            ).await?;

            self.event_bus.broadcast(WsEvent::WorkflowStepChanged {
                workflow_run_id: run.id,
                step: composer_db::models::workflow_step_output::find_by_id(
                    &self.db.pool,
                    &step_output.id.to_string(),
                ).await?.unwrap(),
            });
        }

        // Advance to next step
        self.advance(&run_id, &workflow).await?;

        Ok(true)
    }

    /// Called when a session fails. Marks the workflow step as failed.
    pub async fn on_session_failed(&self, session_id: &str, error: &str) -> anyhow::Result<bool> {
        let run = composer_db::models::workflow_run::find_by_session(&self.db.pool, session_id).await?;
        let run = match run {
            Some(r) => r,
            None => {
                // Check review sessions
                match composer_db::models::workflow_run::find_by_step_session(&self.db.pool, session_id).await? {
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
        ).await? {
            composer_db::models::workflow_step_output::update_status_and_output(
                &self.db.pool,
                &step_output.id.to_string(),
                &WorkflowStepStatus::Failed,
                Some(error),
            ).await?;
        }

        // For now, fail the whole workflow run on step failure
        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            &run_id,
            &WorkflowRunStatus::Failed,
        ).await?;

        let updated_run = composer_db::models::workflow_run::find_by_id(&self.db.pool, &run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;
        self.event_bus.broadcast(WsEvent::WorkflowRunUpdated(updated_run));

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
            return Err(anyhow::anyhow!("Workflow run is not paused (status: {:?})", run.status));
        }

        let workflow = composer_db::models::workflow::find_by_id(&self.db.pool, &run.workflow_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;

        let _step_def = workflow.definition.steps.get(run.current_step_index as usize)
            .ok_or_else(|| anyhow::anyhow!("Current step not found in workflow definition"))?;

        // Update step output
        if let Some(step_output) = composer_db::models::workflow_step_output::latest_for_step(
            &self.db.pool,
            run_id,
            run.current_step_index,
        ).await? {
            let status = if approved { WorkflowStepStatus::Completed } else { WorkflowStepStatus::Rejected };
            composer_db::models::workflow_step_output::update_status_and_output(
                &self.db.pool,
                &step_output.id.to_string(),
                &status,
                comments,
            ).await?;

            self.event_bus.broadcast(WsEvent::WorkflowStepChanged {
                workflow_run_id: run.id,
                step: composer_db::models::workflow_step_output::find_by_id(
                    &self.db.pool,
                    &step_output.id.to_string(),
                ).await?.unwrap(),
            });
        }

        if approved {
            // Move task back to in_progress before advancing
            composer_db::models::task::update_status(
                &self.db.pool,
                &run.task_id.to_string(),
                &TaskStatus::InProgress,
            ).await?;
            self.event_bus.broadcast(WsEvent::TaskMoved {
                task_id: run.task_id,
                from_status: TaskStatus::Waiting,
                to_status: TaskStatus::InProgress,
            });

            // Move to next step
            self.advance(run_id, &workflow).await?;
        } else {
            // Find the preceding agent step to loop back to
            let loop_back_index = self.find_preceding_agent_step(&workflow, run.current_step_index);
            let task = composer_db::models::task::find_by_id(&self.db.pool, &run.task_id.to_string())
                .await?
                .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
            let agent_id = task.assigned_agent_id
                .ok_or_else(|| anyhow::anyhow!("Task has no assigned agent"))?;

            composer_db::models::workflow_run::increment_iteration(&self.db.pool, run_id).await?;
            composer_db::models::workflow_run::update_step_index(&self.db.pool, run_id, loop_back_index).await?;

            // Move task back to in_progress
            composer_db::models::task::update_status(
                &self.db.pool,
                &task.id.to_string(),
                &TaskStatus::InProgress,
            ).await?;

            self.event_bus.broadcast(WsEvent::TaskMoved {
                task_id: task.id,
                from_status: TaskStatus::Waiting,
                to_status: TaskStatus::InProgress,
            });

            // Re-execute the agent step with rejection context
            let rejection_step_def = workflow.definition.steps.get(loop_back_index as usize).unwrap();
            self.execute_agent_step(
                run_id,
                &task,
                agent_id,
                loop_back_index,
                rejection_step_def,
                false, // resume existing session
            ).await?;
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

        let next_index = run.current_step_index + 1;

        if next_index as usize >= workflow.definition.steps.len() {
            // Workflow complete
            composer_db::models::workflow_run::update_status(
                &self.db.pool,
                run_id,
                &WorkflowRunStatus::Completed,
            ).await?;
            composer_db::models::task::update_status(
                &self.db.pool,
                &run.task_id.to_string(),
                &TaskStatus::Done,
            ).await?;

            // Clean up all session worktrees (main + review sessions)
            if let Some(main_sid) = &run.main_session_id {
                self.cleanup_worktree(&main_sid.to_string()).await;
            }
            // Clean up review session worktrees (stored in step outputs)
            let step_outputs = composer_db::models::workflow_step_output::list_by_run(
                &self.db.pool, run_id,
            ).await.unwrap_or_default();
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
            self.event_bus.broadcast(WsEvent::WorkflowRunUpdated(updated_run));
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
        let agent_id = task.assigned_agent_id
            .ok_or_else(|| anyhow::anyhow!("Task has no assigned agent"))?;

        self.execute_step(run_id, workflow, &task, agent_id, next_index).await
    }

    /// Build the prompt for a step, injecting context from previous steps.
    async fn build_prompt(
        &self,
        run_id: &str,
        task: &Task,
        step_def: &WorkflowStepDefinition,
    ) -> anyhow::Result<String> {
        let step_outputs = composer_db::models::workflow_step_output::list_by_run(
            &self.db.pool,
            run_id,
        ).await?;

        let task_context = if let Some(ref desc) = task.description {
            format!("Task: {} - {}", task.title, desc)
        } else {
            format!("Task: {}", task.title)
        };

        // Use custom template if provided, otherwise use defaults
        if let Some(ref template) = step_def.prompt_template {
            let mut prompt = template.clone();
            prompt = prompt.replace("{{task}}", &task_context);

            // Inject outputs from prior steps
            for output in &step_outputs {
                if let Some(ref text) = output.output {
                    let key = format!("{{{{step_{}}}}}", output.step_index);
                    prompt = prompt.replace(&key, text);
                }
            }
            return Ok(prompt);
        }

        // Default prompts by step type
        match step_def.step_type {
            WorkflowStepType::Plan => {
                // Check if this is a retry (rejection happened)
                let rejections: Vec<&WorkflowStepOutput> = step_outputs.iter()
                    .filter(|o| o.step_type == WorkflowStepType::HumanGate
                        && o.status == WorkflowStepStatus::Rejected)
                    .collect();

                if let Some(last_rejection) = rejections.last() {
                    let feedback = last_rejection.output.as_deref().unwrap_or("No specific feedback provided.");
                    Ok(format!(
                        "{} - The previous plan was rejected. Feedback: {} - Please revise the plan based on this feedback. Do NOT implement yet, only output the revised plan.",
                        task_context, feedback
                    ))
                } else {
                    Ok(format!(
                        "{} - Investigate the existing codebase and create a detailed implementation plan. Do NOT implement yet. Only output the plan.",
                        task_context
                    ))
                }
            }
            WorkflowStepType::Implement => {
                // Find the approved plan
                let plan_output = step_outputs.iter()
                    .filter(|o| o.step_type == WorkflowStepType::Plan && o.status == WorkflowStepStatus::Completed)
                    .last()
                    .and_then(|o| o.output.as_deref());

                // Check for PR review findings to fix
                let review_findings: Vec<&str> = step_outputs.iter()
                    .filter(|o| o.step_type == WorkflowStepType::PrReview && o.status == WorkflowStepStatus::Completed)
                    .filter_map(|o| o.output.as_deref())
                    .collect();

                // Check for human review comments to fix
                let human_comments: Vec<&str> = step_outputs.iter()
                    .filter(|o| o.step_type == WorkflowStepType::HumanReview && o.status == WorkflowStepStatus::Rejected)
                    .filter_map(|o| o.output.as_deref())
                    .collect();

                if !human_comments.is_empty() {
                    let latest = human_comments.last().unwrap();
                    Ok(format!(
                        "The reviewer left these comments on the PR: {} - Fix them and push the changes.",
                        latest
                    ))
                } else if !review_findings.is_empty() {
                    let latest = review_findings.last().unwrap();
                    Ok(format!(
                        "The PR has been reviewed. Fix these findings: {} - Then push the changes.",
                        latest
                    ))
                } else if let Some(plan) = plan_output {
                    Ok(format!(
                        "{} - The plan has been approved. Implement it now. After implementation, run build, lint, and tests. Fix any failures. Then create a PR. - Approved plan: {}",
                        task_context, plan
                    ))
                } else {
                    Ok(format!(
                        "{} - Implement this task. Run build, lint, and tests. Fix any failures. Then create a PR.",
                        task_context
                    ))
                }
            }
            WorkflowStepType::PrReview => {
                // Find the PR URLs from the task
                let pr_urls = &task.pr_urls;
                let pr_context = if pr_urls.is_empty() {
                    "Review the changes made in the current branch.".to_string()
                } else {
                    format!("Review this PR: {}", pr_urls.join(", "))
                };
                Ok(format!(
                    "{} - Provide a thorough code review. List any bugs, logic errors, security issues, code quality problems, and suggestions for improvement. Be specific about file names and line numbers.",
                    pr_context
                ))
            }
            WorkflowStepType::HumanGate | WorkflowStepType::HumanReview => {
                // Human gates don't need prompts — they pause the workflow
                Ok(String::new())
            }
        }
    }

    /// Find the preceding agent step to loop back to on rejection.
    fn find_preceding_agent_step(&self, workflow: &Workflow, current_index: i32) -> i32 {
        for i in (0..current_index).rev() {
            let step = &workflow.definition.steps[i as usize];
            if matches!(step.step_type, WorkflowStepType::Plan | WorkflowStepType::Implement) {
                return i;
            }
        }
        0 // fallback to first step
    }

    /// Get the repo path for a task's project.
    async fn get_repo_path(&self, task: &Task) -> anyhow::Result<String> {
        let project_id = task.project_id
            .ok_or_else(|| anyhow::anyhow!("Task has no project assigned"))?;
        let repos = composer_db::models::project_repository::list_by_project(
            &self.db.pool,
            &project_id.to_string(),
        ).await?;
        let primary_repo = repos.iter()
            .find(|r| r.role == RepositoryRole::Primary)
            .or_else(|| repos.first())
            .ok_or_else(|| anyhow::anyhow!("Project has no repositories configured"))?;
        Ok(primary_repo.local_path.clone())
    }

    /// Clean up a session's worktree.
    async fn cleanup_worktree(&self, session_id: &str) {
        if let Ok(Some(session)) = composer_db::models::session::find_by_id(&self.db.pool, session_id).await {
            if let Some(wt_id) = &session.worktree_id {
                let wt_id_str = wt_id.to_string();
                if let Ok(Some(wt)) = composer_db::models::worktree::find_by_id(&self.db.pool, &wt_id_str).await {
                    if wt.status != WorktreeStatus::Deleted {
                        let _ = composer_git::worktree::remove_worktree(
                            std::path::Path::new(&wt.repo_path),
                            std::path::Path::new(&wt.worktree_path),
                            &wt.branch_name,
                        ).await;
                        let _ = composer_db::models::worktree::update_status(
                            &self.db.pool,
                            &wt_id_str,
                            &WorktreeStatus::Deleted,
                        ).await;
                    }
                }
            }
        }
    }

    /// Get the workflow run with all step outputs for a task.
    pub async fn get_run_with_steps(&self, run_id: &str) -> anyhow::Result<(WorkflowRun, Vec<WorkflowStepOutput>)> {
        let run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;
        let steps = composer_db::models::workflow_step_output::list_by_run(&self.db.pool, run_id).await?;
        Ok((run, steps))
    }
}
