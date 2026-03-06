use crate::event_bus::EventBus;
use crate::session_service::SessionService;
use composer_api_types::*;
use composer_db::Database;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::Mutex;

/// Result of evaluating whether a workflow step should loop back.
enum LoopDecision {
    /// Continue looping — re-run the loop target step.
    Loop,
    /// No issues found in the loop target output — advance to the next step.
    NoIssuesFound,
    /// Max retries exhausted — pause the workflow for user decision.
    MaxRetriesExhausted,
}

// ---------------------------------------------------------------------------
// Built-in workflow definition helpers
// ---------------------------------------------------------------------------

fn agentic_step(
    id: &str,
    name: &str,
    mode: SessionMode,
    prompt: &str,
) -> WorkflowStepDefinition {
    WorkflowStepDefinition {
        id: id.to_string(),
        step_type: WorkflowStepType::Agentic,
        name: name.to_string(),
        prompt_template: Some(prompt.to_string()),
        depends_on: vec![],
        on_approve: None,
        on_reject: None,
        max_retries: None,
        loop_back_to: None,
        session_mode: Some(mode),
    }
}

fn human_gate_step(id: &str, name: &str) -> WorkflowStepDefinition {
    WorkflowStepDefinition {
        id: id.to_string(),
        step_type: WorkflowStepType::HumanGate,
        name: name.to_string(),
        prompt_template: None,
        depends_on: vec![],
        on_approve: None,
        on_reject: None,
        max_retries: None,
        loop_back_to: None,
        session_mode: None,
    }
}

pub fn feat_common_definition() -> WorkflowDefinition {
    WorkflowDefinition {
        steps: vec![
            agentic_step("start", "Start", SessionMode::New,
                "You are about to work on a task. Read the following instructions and task context carefully.\n\n\
                {{task}}\n\n\
                IMPORTANT: Do NOT start implementing or planning yet. \
                Acknowledge that you understand the task and instructions, then wait for further directions."),
            {
                let mut s = agentic_step("plan", "Plan", SessionMode::Resume,
                    "Investigate the existing codebase and create a detailed implementation plan. Do NOT implement yet. Only output the plan.{{rejection}}");
                s.depends_on = vec!["start".to_string()];
                s
            },
            {
                let mut s = human_gate_step("review_plan", "Review Plan");
                s.depends_on = vec!["plan".to_string()];
                s.on_approve = Some("implement".to_string());
                s.on_reject = Some("plan".to_string());
                s
            },
            {
                let mut s = agentic_step("implement", "Implement & Create PR", SessionMode::Resume,
                    "The plan has been approved. Implement it now. After implementation, run build, lint, and tests. Fix any failures. Then create a PR.\n\nApproved plan:\n{{step:plan}}{{rejection}}");
                s.depends_on = vec!["review_plan".to_string()];
                s
            },
            {
                let mut s = agentic_step("auto_review", "Automated PR Review", SessionMode::Resume,
                    "Review the PR on the current branch. Follow these rules in order:\n\n\
                    1. First, check if the PR has any merge conflicts with its target branch. \
                    If there are conflicts, STOP immediately, report the conflicts, and do NOT proceed to code review.\n\n\
                    2. Only if there are no merge conflicts, use the /code-review:code-review skill to perform code review of the PR. \
                    This is a force re-review — the PR may have been reviewed before, but we need a fresh review of the latest changes.\n\n\
                    IMPORTANT: If you find NO issues at all (no conflicts and no code review findings), you MUST include the exact marker [NO_ISSUES_FOUND] in your response. Only use this marker if there are truly zero issues to report.");
                s.depends_on = vec!["implement".to_string()];
                s
            },
            {
                let mut s = agentic_step("fix_review", "Fix Review Findings", SessionMode::Resume,
                    "The PR has been reviewed. Fix these findings:\n{{step:auto_review}}\n\nThen push the changes.");
                s.depends_on = vec!["auto_review".to_string()];
                s.max_retries = Some(3);
                s.loop_back_to = Some("auto_review".to_string());
                s
            },
            {
                let mut s = human_gate_step("human_review", "Human PR Review");
                s.depends_on = vec!["fix_review".to_string()];
                s.on_approve = Some("complete_pr".to_string());
                s.on_reject = Some("implement".to_string());
                s
            },
            {
                let mut s = agentic_step("complete_pr", "Complete PR", SessionMode::Resume,
                    "The implementation is complete. Find the PR you created earlier and complete it:\n\
                    1. Squash all commits on this branch into a single clean commit. The commit message should summarize what the PR accomplishes — do NOT include intermediate review/fix cycle details.\n\
                    2. Wait for CI checks to pass.\n\
                    3. Resolve any merge conflicts if needed.\n\
                    4. Merge the PR and delete the branch.\n\
                    5. Confirm the PR is merged successfully.");
                s.depends_on = vec!["human_review".to_string()];
                s
            },
        ],
    }
}

pub const FEAT_COMMON_NAME: &str = "Feat-Common";

// ---------------------------------------------------------------------------
// DAG validation
// ---------------------------------------------------------------------------

pub fn validate_dag(def: &WorkflowDefinition) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let step_ids: HashSet<&str> = def.steps.iter().map(|s| s.id.as_str()).collect();

    // Check for duplicate IDs
    {
        let mut seen = HashSet::new();
        for step in &def.steps {
            if !seen.insert(&step.id) {
                errors.push(format!("Duplicate step ID: '{}'", step.id));
            }
        }
    }

    for step in &def.steps {
        // Agentic steps require prompt_template
        if step.step_type == WorkflowStepType::Agentic {
            let has_prompt = step.prompt_template.as_ref().map_or(false, |t| !t.trim().is_empty());
            if !has_prompt {
                errors.push(format!("Step '{}' is agentic but has no prompt_template", step.id));
            }
        }

        // HumanGate steps must have on_approve
        if step.step_type == WorkflowStepType::HumanGate && step.on_approve.is_none() {
            errors.push(format!("HumanGate step '{}' is missing on_approve", step.id));
        }

        // Check depends_on references
        for dep in &step.depends_on {
            if !step_ids.contains(dep.as_str()) {
                errors.push(format!("Step '{}' depends_on non-existent step '{}'", step.id, dep));
            }
        }

        // Check on_approve reference
        if let Some(ref target) = step.on_approve {
            if !step_ids.contains(target.as_str()) {
                errors.push(format!("Step '{}' on_approve references non-existent step '{}'", step.id, target));
            }
        }

        // Check on_reject reference
        if let Some(ref target) = step.on_reject {
            if !step_ids.contains(target.as_str()) {
                errors.push(format!("Step '{}' on_reject references non-existent step '{}'", step.id, target));
            }
        }

        // Check loop_back_to reference
        if let Some(ref target) = step.loop_back_to {
            if !step_ids.contains(target.as_str()) {
                errors.push(format!("Step '{}' loop_back_to references non-existent step '{}'", step.id, target));
            }
        }
    }

    // Cycle detection via topological sort
    if errors.is_empty() {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for step in &def.steps {
            in_degree.entry(step.id.as_str()).or_insert(0);
            adj.entry(step.id.as_str()).or_default();
            for dep in &step.depends_on {
                adj.entry(dep.as_str()).or_default().push(step.id.as_str());
                *in_degree.entry(step.id.as_str()).or_insert(0) += 1;
            }
        }
        let mut queue: VecDeque<&str> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut visited = 0;
        while let Some(node) = queue.pop_front() {
            visited += 1;
            if let Some(neighbors) = adj.get(node) {
                for &n in neighbors {
                    let deg = in_degree.get_mut(n).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(n);
                    }
                }
            }
        }
        if visited != def.steps.len() {
            errors.push("Workflow definition contains a cycle".to_string());
        }
    }

    // Check for orphaned steps (no path from any entry point)
    if errors.is_empty() {
        let entry_steps: Vec<&str> = def.steps.iter()
            .filter(|s| s.depends_on.is_empty())
            .map(|s| s.id.as_str())
            .collect();

        if entry_steps.is_empty() && !def.steps.is_empty() {
            errors.push("No entry steps found (all steps have dependencies)".to_string());
        } else {
            // BFS from entry steps following depends_on edges, on_approve, on_reject, loop_back_to
            let mut reachable = HashSet::new();
            let mut queue: VecDeque<&str> = entry_steps.into_iter().collect();

            // Build reverse-depends + branch target adjacency
            let mut forward: HashMap<&str, Vec<&str>> = HashMap::new();
            for step in &def.steps {
                // Steps that depend on this step
                for dep in &step.depends_on {
                    forward.entry(dep.as_str()).or_default().push(step.id.as_str());
                }
                // Branch targets
                if let Some(ref t) = step.on_approve {
                    forward.entry(step.id.as_str()).or_default().push(t.as_str());
                }
                if let Some(ref t) = step.on_reject {
                    forward.entry(step.id.as_str()).or_default().push(t.as_str());
                }
                if let Some(ref t) = step.loop_back_to {
                    forward.entry(step.id.as_str()).or_default().push(t.as_str());
                }
            }

            while let Some(node) = queue.pop_front() {
                if !reachable.insert(node) {
                    continue;
                }
                if let Some(neighbors) = forward.get(node) {
                    for &n in neighbors {
                        if !reachable.contains(n) {
                            queue.push_back(n);
                        }
                    }
                }
            }

            for step in &def.steps {
                if !reachable.contains(step.id.as_str()) {
                    errors.push(format!("Step '{}' is orphaned (not reachable from any entry point)", step.id));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ---------------------------------------------------------------------------
// Workflow Engine
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct WorkflowEngine {
    db: Arc<Database>,
    event_bus: EventBus,
    session_service: SessionService,
    /// Per-run mutex to serialize state transitions within a single workflow run
    run_locks: Arc<DashMap<String, Arc<Mutex<()>>>>,
}

impl WorkflowEngine {
    pub fn new(db: Arc<Database>, event_bus: EventBus, session_service: SessionService) -> Self {
        let engine = Self {
            db,
            event_bus,
            session_service,
            run_locks: Arc::new(DashMap::new()),
        };

        engine.spawn_startup_recovery();

        engine
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    fn run_lock(&self, run_id: &str) -> Arc<Mutex<()>> {
        self.run_locks
            .entry(run_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .value()
            .clone()
    }

    /// Remove the per-run lock when a workflow run reaches a terminal state.
    fn cleanup_run_lock(&self, run_id: &str) {
        self.run_locks.remove(run_id);
    }

    /// Ensure the built-in "Feat-Common" workflow exists as a template.
    pub async fn ensure_builtin_workflow(&self) -> anyhow::Result<Workflow> {
        let canonical = feat_common_definition();
        validate_dag(&canonical).map_err(|errs| anyhow::anyhow!("Built-in workflow invalid: {}", errs.join(", ")))?;

        if let Some(wf) = composer_db::models::workflow::find_by_name(&self.db.pool, FEAT_COMMON_NAME).await? {
            if wf.definition != canonical {
                let active_runs =
                    composer_db::models::workflow_run::find_active(&self.db.pool).await?;
                let has_active_runs = active_runs.iter().any(|r| r.workflow_id == wf.id);
                if !has_active_runs {
                    let updated = composer_db::models::workflow::update(
                        &self.db.pool,
                        &wf.id.to_string(),
                        None,
                        Some(&canonical),
                    )
                    .await?;
                    tracing::info!("Updated built-in workflow '{}'", FEAT_COMMON_NAME);
                    return Ok(updated);
                }
            }
            return Ok(wf);
        }

        let wf = composer_db::models::workflow::create_with_template(
            &self.db.pool,
            FEAT_COMMON_NAME,
            &canonical,
            true,
        ).await?;
        tracing::info!("Created built-in workflow template '{}'", FEAT_COMMON_NAME);
        Ok(wf)
    }

    fn spawn_startup_recovery(&self) {
        let engine = self.clone();
        tokio::spawn(async move {
            if let Err(e) = engine.ensure_builtin_workflow().await {
                tracing::error!("Failed to seed built-in workflow: {}", e);
            }
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

            // Mark all running step outputs as failed
            let running_steps = composer_db::models::workflow_step_output::find_running_steps(
                &self.db.pool,
                &run_id,
            ).await?;
            for step_output in running_steps {
                composer_db::models::workflow_step_output::update_status_and_output(
                    &self.db.pool,
                    &step_output.id.to_string(),
                    &WorkflowStepStatus::Failed,
                    Some("Server restarted while step was running"),
                ).await?;
            }

            composer_db::models::workflow_run::update_status(
                &self.db.pool,
                &run_id,
                &WorkflowRunStatus::Paused,
            ).await?;

            composer_db::models::task::update_status(
                &self.db.pool,
                &run.task_id.to_string(),
                &TaskStatus::Waiting,
            ).await?;

            tracing::warn!(
                "Workflow run {} (task {}) paused for recovery",
                run_id,
                run.task_id
            );
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Start a workflow run for a task.
    pub async fn start(&self, task_id: &str, workflow_id: &str) -> anyhow::Result<WorkflowRun> {
        let task = composer_db::models::task::find_by_id(&self.db.pool, task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        if !matches!(task.status, TaskStatus::Backlog) {
            return Err(anyhow::anyhow!("Task must be in backlog to start a workflow"));
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

        validate_dag(&workflow.definition)
            .map_err(|errs| anyhow::anyhow!("Workflow validation failed: {}", errs.join(", ")))?;

        let run = composer_db::models::workflow_run::create(&self.db.pool, workflow_id, task_id).await?;

        // Atomic conditional update: claim the task for this workflow run.
        // If another concurrent start() already moved this task out of backlog,
        // affected_rows will be 0 and we bail out.
        let result = sqlx::query(
            "UPDATE tasks SET workflow_run_id = ?, status = 'in_progress', \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
             WHERE id = ? AND status = 'backlog' AND workflow_run_id IS NULL"
        )
        .bind(&run.id.to_string())
        .bind(task_id)
        .execute(&self.db.pool)
        .await?;

        if result.rows_affected() == 0 {
            // Another call won the race — clean up orphaned run
            sqlx::query("DELETE FROM workflow_runs WHERE id = ?")
                .bind(&run.id.to_string())
                .execute(&self.db.pool)
                .await?;
            return Err(anyhow::anyhow!("Task was already claimed by another workflow run"));
        }
        self.event_bus.broadcast(WsEvent::TaskMoved {
            task_id: task.id,
            from_status: TaskStatus::Backlog,
            to_status: TaskStatus::InProgress,
        });
        self.event_bus.broadcast(WsEvent::WorkflowRunUpdated(run.clone()));

        // Advance frontier to kick off entry steps
        let run_id = run.id.to_string();
        {
            let lock = self.run_lock(&run_id);
            let _guard = lock.lock().await;
            self.advance_frontier(&run_id, &workflow, &task, agent_id).await?;
        }

        composer_db::models::workflow_run::find_by_id(&self.db.pool, &run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))
    }

    /// Resume a paused/failed workflow run.
    pub async fn resume_run(&self, run_id: &str, req: &WorkflowResumeRequest) -> anyhow::Result<WorkflowRun> {
        let lock = self.run_lock(run_id);
        let _guard = lock.lock().await;

        let run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;

        if !matches!(run.status, WorkflowRunStatus::Paused | WorkflowRunStatus::Failed) {
            return Err(anyhow::anyhow!(
                "Workflow run cannot be resumed from status {:?}",
                run.status
            ));
        }

        let workflow = composer_db::models::workflow::find_by_id(&self.db.pool, &run.workflow_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;

        let task = composer_db::models::task::find_by_id(&self.db.pool, &run.task_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        let agent_id = task
            .assigned_agent_id
            .ok_or_else(|| anyhow::anyhow!("Task has no assigned agent"))?;

        // Handle retry-exhaustion resume actions
        if let (Some(step_id), Some(action)) = (&req.step_id, &req.action) {
            let step_def = workflow.definition.steps.iter()
                .find(|s| s.id == *step_id)
                .ok_or_else(|| anyhow::anyhow!("Step '{}' not found in workflow definition", step_id))?;

            match action {
                WorkflowResumeAction::ContinueLoop => {
                    tracing::info!("Continuing loop for step '{}' in run {}", step_id, run_id);

                    // Reset the fix step from Failed back to Pending so the loop can re-enter
                    if let Some(step_output) = composer_db::models::workflow_step_output::latest_for_step(
                        &self.db.pool, run_id, step_id,
                    ).await? {
                        composer_db::models::workflow_step_output::update_status(
                            &self.db.pool,
                            &step_output.id.to_string(),
                            &WorkflowStepStatus::Pending,
                        ).await?;
                    }

                    // Create a new Pending output for the loop target step so
                    // compute_ready_steps picks it up for re-execution
                    if let Some(ref loop_target) = step_def.loop_back_to {
                        let target_def = workflow.definition.steps.iter()
                            .find(|s| s.id == *loop_target);
                        if let Some(td) = target_def {
                            composer_db::models::workflow_step_output::create(
                                &self.db.pool,
                                run_id,
                                loop_target,
                                &td.step_type,
                                &WorkflowStepStatus::Pending,
                                None,
                            ).await?;
                        }
                    }
                }
                WorkflowResumeAction::SkipToNext => {
                    // Mark the step as skipped and advance
                    if let Some(step_output) = composer_db::models::workflow_step_output::latest_for_step(
                        &self.db.pool, run_id, step_id,
                    ).await? {
                        composer_db::models::workflow_step_output::update_status_and_output(
                            &self.db.pool,
                            &step_output.id.to_string(),
                            &WorkflowStepStatus::Skipped,
                            Some("Skipped by user after retry exhaustion"),
                        ).await?;
                    }

                    // Also mark the loop target as completed/skipped so dependencies can proceed
                    if let Some(ref loop_target) = step_def.loop_back_to {
                        if let Some(target_output) = composer_db::models::workflow_step_output::latest_for_step(
                            &self.db.pool, run_id, loop_target,
                        ).await? {
                            if !matches!(target_output.status, WorkflowStepStatus::Completed) {
                                composer_db::models::workflow_step_output::update_status(
                                    &self.db.pool,
                                    &target_output.id.to_string(),
                                    &WorkflowStepStatus::Completed,
                                ).await?;
                            }
                        }
                    }
                }
            }
        }

        // Move task back to in_progress
        composer_db::models::task::update_status(
            &self.db.pool,
            &task.id.to_string(),
            &TaskStatus::InProgress,
        ).await?;

        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            run_id,
            &WorkflowRunStatus::Running,
        ).await?;

        self.advance_frontier(run_id, &workflow, &task, agent_id).await?;

        composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))
    }

    /// Handle a human decision (approve/reject) on a human gate step.
    pub async fn submit_decision(
        &self,
        run_id: &str,
        step_id: &str,
        approved: bool,
        comments: Option<&str>,
    ) -> anyhow::Result<WorkflowRun> {
        let lock = self.run_lock(run_id);
        let _guard = lock.lock().await;

        let run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;

        if !matches!(run.status, WorkflowRunStatus::Paused) {
            return Err(anyhow::anyhow!(
                "Workflow run is not paused (status: {:?})",
                run.status
            ));
        }

        let workflow = composer_db::models::workflow::find_by_id(&self.db.pool, &run.workflow_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;

        let step_def = workflow.definition.steps.iter()
            .find(|s| s.id == step_id)
            .ok_or_else(|| anyhow::anyhow!("Step '{}' not found in workflow definition", step_id))?;

        if step_def.step_type != WorkflowStepType::HumanGate {
            return Err(anyhow::anyhow!("Step '{}' is not a human gate", step_id));
        }

        // Update step output
        if let Some(step_output) = composer_db::models::workflow_step_output::latest_for_step(
            &self.db.pool, run_id, step_id,
        ).await? {
            if !matches!(step_output.status, WorkflowStepStatus::WaitingForHuman) {
                return Err(anyhow::anyhow!(
                    "Step '{}' is not waiting for human decision (status: {:?})",
                    step_id, step_output.status
                ));
            }
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
            ).await?;

            let refreshed_step = composer_db::models::workflow_step_output::find_by_id(
                &self.db.pool,
                &step_output.id.to_string(),
            ).await?
            .ok_or_else(|| anyhow::anyhow!("Step output not found after write"))?;
            self.event_bus.broadcast(WsEvent::WorkflowStepChanged {
                workflow_run_id: run.id,
                step: refreshed_step,
            });
        }

        // Activate branch target and create a new Pending output so
        // compute_ready_steps won't skip it if it already ran before.
        let branch_target = if approved {
            step_def.on_approve.as_deref()
        } else {
            step_def.on_reject.as_deref()
        };

        if let Some(target) = branch_target {
            composer_db::models::workflow_run::add_activated_step(&self.db.pool, run_id, target).await?;

            // If the target step already has a terminal output (Completed/Rejected/Failed),
            // create a new Pending output so it will be picked up by compute_ready_steps.
            if let Some(existing) = composer_db::models::workflow_step_output::latest_for_step(
                &self.db.pool, run_id, target,
            ).await? {
                if matches!(existing.status,
                    WorkflowStepStatus::Completed
                    | WorkflowStepStatus::Rejected
                    | WorkflowStepStatus::Failed
                ) {
                    let target_def = workflow.definition.steps.iter()
                        .find(|s| s.id == target);
                    if let Some(td) = target_def {
                        composer_db::models::workflow_step_output::create(
                            &self.db.pool,
                            run_id,
                            target,
                            &td.step_type,
                            &WorkflowStepStatus::Pending,
                            None,
                        ).await?;
                    }
                }
            }
        }

        // When rejected, also create a new Pending output for the human gate itself
        // so that it will be re-evaluated after the target step completes.
        // Without this, the gate's latest status stays Rejected and compute_ready_steps skips it.
        if !approved {
            composer_db::models::workflow_step_output::create(
                &self.db.pool,
                run_id,
                step_id,
                &step_def.step_type,
                &WorkflowStepStatus::Pending,
                None,
            ).await?;

            composer_db::models::workflow_run::increment_iteration(&self.db.pool, run_id).await?;
        }

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
        ).await?;
        self.event_bus.broadcast(WsEvent::TaskMoved {
            task_id: run.task_id,
            from_status: TaskStatus::Waiting,
            to_status: TaskStatus::InProgress,
        });

        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            run_id,
            &WorkflowRunStatus::Running,
        ).await?;

        self.advance_frontier(run_id, &workflow, &task, agent_id).await?;

        composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))
    }

    /// Called when a session completes. Determines if it belongs to a workflow and advances.
    pub async fn on_session_completed(
        &self,
        session_id: &str,
        result_summary: Option<&str>,
    ) -> anyhow::Result<bool> {
        // Find workflow run by step session
        let run = composer_db::models::workflow_run::find_by_step_session(&self.db.pool, session_id).await?;
        let run = match run {
            Some(r) => r,
            None => return Ok(false),
        };

        if matches!(run.status, WorkflowRunStatus::Completed | WorkflowRunStatus::Failed) {
            return Ok(true);
        }

        let run_id = run.id.to_string();
        let lock = self.run_lock(&run_id);
        let _guard = lock.lock().await;

        let workflow = composer_db::models::workflow::find_by_id(&self.db.pool, &run.workflow_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found for run"))?;

        // Find the step output for this session and mark it completed
        if let Some(step_output) = composer_db::models::workflow_step_output::find_by_session(
            &self.db.pool, session_id,
        ).await? {
            if matches!(step_output.status, WorkflowStepStatus::Running) {
                composer_db::models::workflow_step_output::update_status_and_output(
                    &self.db.pool,
                    &step_output.id.to_string(),
                    &WorkflowStepStatus::Completed,
                    result_summary,
                ).await?;

                let refreshed_step = composer_db::models::workflow_step_output::find_by_id(
                    &self.db.pool,
                    &step_output.id.to_string(),
                ).await?
                .ok_or_else(|| anyhow::anyhow!("Step output not found after write"))?;
                self.event_bus.broadcast(WsEvent::WorkflowStepChanged {
                    workflow_run_id: run.id,
                    step: refreshed_step,
                });

                // Check if this step has a loop_back_to and should loop
                let step_def = workflow.definition.steps.iter()
                    .find(|s| s.id == step_output.step_id);
                if step_def.is_none() {
                    tracing::warn!(
                        "Step definition '{}' not found in workflow for run {}",
                        step_output.step_id, run_id
                    );
                }
                if let Some(def) = step_def {
                    if let Some(ref loop_target) = def.loop_back_to {
                        match self.should_loop(&run_id, def, loop_target).await? {
                            LoopDecision::Loop => {
                                tracing::info!(
                                    "Workflow run {}: looping from step '{}' back to '{}'",
                                    run_id, step_output.step_id, loop_target
                                );
                                composer_db::models::workflow_run::increment_iteration(&self.db.pool, &run_id).await?;
                                // Create a new Pending output for the loop target so
                                // compute_ready_steps will pick it up (the previous
                                // Completed output would cause it to be skipped).
                                let target_def = workflow.definition.steps.iter()
                                    .find(|s| s.id == *loop_target);
                                if let Some(td) = target_def {
                                    composer_db::models::workflow_step_output::create(
                                        &self.db.pool,
                                        &run_id,
                                        loop_target,
                                        &td.step_type,
                                        &WorkflowStepStatus::Pending,
                                        None,
                                    ).await?;
                                }
                                // Also create a new Pending output for the current step
                                // so that downstream steps (which depend on this step)
                                // don't see it as Completed and advance prematurely.
                                composer_db::models::workflow_step_output::create(
                                    &self.db.pool,
                                    &run_id,
                                    &step_output.step_id,
                                    &def.step_type,
                                    &WorkflowStepStatus::Pending,
                                    None,
                                ).await?;
                            }
                            LoopDecision::NoIssuesFound => {
                                // Review found no issues — no need to loop, just
                                // let the workflow advance to the next step naturally.
                                tracing::info!(
                                    "Workflow run {}: no issues found at step '{}', advancing",
                                    run_id, step_output.step_id
                                );
                            }
                            LoopDecision::MaxRetriesExhausted => {
                                tracing::info!(
                                    "Workflow run {}: max retries reached at step '{}', pausing",
                                    run_id, step_output.step_id
                                );

                                // Mark as failed with retry exhaustion message
                                composer_db::models::workflow_step_output::update_status_and_output(
                                    &self.db.pool,
                                    &step_output.id.to_string(),
                                    &WorkflowStepStatus::Failed,
                                    Some("Max retries exceeded"),
                                ).await?;

                                composer_db::models::workflow_run::update_status(
                                    &self.db.pool,
                                    &run_id,
                                    &WorkflowRunStatus::Paused,
                                ).await?;

                                composer_db::models::task::update_status(
                                    &self.db.pool,
                                    &run.task_id.to_string(),
                                    &TaskStatus::Waiting,
                                ).await?;

                                self.event_bus.broadcast(WsEvent::WorkflowWaitingForHuman {
                                    workflow_run_id: run.id,
                                    task_id: run.task_id,
                                    step_id: step_output.step_id.clone(),
                                });

                                let updated_run = composer_db::models::workflow_run::find_by_id(&self.db.pool, &run_id).await?
                                    .ok_or_else(|| anyhow::anyhow!("Workflow run not found after update"))?;
                                self.event_bus.broadcast(WsEvent::WorkflowRunUpdated(updated_run));

                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }

        // Advance frontier
        let task = composer_db::models::task::find_by_id(&self.db.pool, &run.task_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        let agent_id = task
            .assigned_agent_id
            .ok_or_else(|| anyhow::anyhow!("Task has no assigned agent"))?;

        self.advance_frontier(&run_id, &workflow, &task, agent_id).await?;

        Ok(true)
    }

    /// Called when a session fails.
    pub async fn on_session_failed(&self, session_id: &str, error: &str) -> anyhow::Result<bool> {
        let run = composer_db::models::workflow_run::find_by_step_session(&self.db.pool, session_id).await?;
        let run = match run {
            Some(r) => r,
            None => return Ok(false),
        };

        let run_id = run.id.to_string();
        let lock = self.run_lock(&run_id);
        let _guard = lock.lock().await;

        if let Some(step_output) = composer_db::models::workflow_step_output::find_by_session(
            &self.db.pool, session_id,
        ).await? {
            composer_db::models::workflow_step_output::update_status_and_output(
                &self.db.pool,
                &step_output.id.to_string(),
                &WorkflowStepStatus::Failed,
                Some(error),
            ).await?;
        }

        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            &run_id,
            &WorkflowRunStatus::Failed,
        ).await?;

        let updated_run = composer_db::models::workflow_run::find_by_id(&self.db.pool, &run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;
        self.event_bus.broadcast(WsEvent::WorkflowRunUpdated(updated_run));

        self.cleanup_run_lock(&run_id);

        Ok(true)
    }

    /// Get the workflow run with all step outputs.
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

    // -----------------------------------------------------------------------
    // Core DAG engine
    // -----------------------------------------------------------------------

    /// Collect all branch target step IDs (steps referenced by on_approve/on_reject of HumanGate steps).
    fn collect_branch_targets(workflow: &Workflow) -> HashSet<String> {
        workflow.definition.steps.iter()
            .filter(|s| s.step_type == WorkflowStepType::HumanGate)
            .flat_map(|s| {
                let mut targets = Vec::new();
                if let Some(ref t) = s.on_approve { targets.push(t.clone()); }
                if let Some(ref t) = s.on_reject { targets.push(t.clone()); }
                targets
            })
            .collect()
    }

    /// Build a map of step_id -> latest output status from step outputs.
    fn build_latest_status(step_outputs: &[WorkflowStepOutput]) -> HashMap<&str, &WorkflowStepStatus> {
        let mut map: HashMap<&str, (&WorkflowStepOutput, i32)> = HashMap::new();
        for output in step_outputs {
            let entry = map.entry(output.step_id.as_str()).or_insert((output, output.attempt));
            if output.attempt > entry.1 {
                *entry = (output, output.attempt);
            }
        }
        map.into_iter().map(|(k, (v, _))| (k, &v.status)).collect()
    }

    /// Compute which steps are ready to execute.
    fn compute_ready_steps(
        &self,
        workflow: &Workflow,
        step_outputs: &[WorkflowStepOutput],
        activated_steps: &[String],
    ) -> Vec<String> {
        let branch_targets = Self::collect_branch_targets(workflow);
        let latest_status = Self::build_latest_status(step_outputs);

        let mut ready = Vec::new();

        for step in &workflow.definition.steps {
            let step_id = step.id.as_str();

            // Skip if already has a non-pending output (only Pending steps are re-executable)
            if let Some(status) = latest_status.get(step_id) {
                match status {
                    WorkflowStepStatus::Pending => {} // fall through — ready for execution
                    _ => continue, // Running, Completed, WaitingForHuman, Skipped, Failed, Rejected
                }
            }

            // Check reachability: if it's a branch target, it must be activated.
            // Exception: entry steps (no dependencies) are always reachable on first run.
            if branch_targets.contains(step_id)
                && !activated_steps.contains(&step.id)
                && !step.depends_on.is_empty()
            {
                continue;
            }

            // For entry steps that are also branch targets, only block them if they
            // already ran once (have any output) and haven't been re-activated.
            if branch_targets.contains(step_id)
                && !activated_steps.contains(&step.id)
                && step.depends_on.is_empty()
                && latest_status.contains_key(step_id)
            {
                continue;
            }

            // Check all dependencies are completed
            let deps_met = step.depends_on.iter().all(|dep| {
                latest_status.get(dep.as_str())
                    .map(|s| matches!(s, WorkflowStepStatus::Completed))
                    .unwrap_or(false)
            });

            if deps_met {
                ready.push(step.id.clone());
            }
        }

        ready
    }

    /// Advance the workflow frontier: compute ready steps and execute them.
    async fn advance_frontier(
        &self,
        run_id: &str,
        workflow: &Workflow,
        task: &Task,
        agent_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        let run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;

        let step_outputs = composer_db::models::workflow_step_output::list_by_run(&self.db.pool, run_id).await?;

        let ready_steps = self.compute_ready_steps(workflow, &step_outputs, &run.activated_steps);

        if ready_steps.is_empty() {
            // Check if workflow is complete
            if self.is_workflow_complete(workflow, &step_outputs, &run.activated_steps) {
                self.complete_workflow(run_id, &run, workflow, &step_outputs).await?;
            }
            return Ok(());
        }

        // Execute each ready step
        for step_id in ready_steps {
            let step_def = workflow.definition.steps.iter()
                .find(|s| s.id == step_id)
                .ok_or_else(|| anyhow::anyhow!("Step '{}' not found in workflow definition", step_id))?;

            match step_def.step_type {
                WorkflowStepType::Agentic => {
                    let mode = step_def.session_mode.clone().unwrap_or(SessionMode::Resume);
                    match mode {
                        SessionMode::New => {
                            self.execute_agent_step(run_id, task, agent_id, &step_def, true, workflow).await?;
                        }
                        SessionMode::Resume => {
                            self.execute_agent_step(run_id, task, agent_id, &step_def, false, workflow).await?;
                        }
                        SessionMode::Separate => {
                            self.execute_pr_review(run_id, task, agent_id, &step_def).await?;
                        }
                    }
                }
                WorkflowStepType::HumanGate => {
                    self.execute_human_gate(run_id, task, &step_def).await?;
                }
            }
        }

        Ok(())
    }

    /// Check if all reachable steps are in a terminal state.
    fn is_workflow_complete(
        &self,
        workflow: &Workflow,
        step_outputs: &[WorkflowStepOutput],
        activated_steps: &[String],
    ) -> bool {
        let branch_targets = Self::collect_branch_targets(workflow);
        let latest_status = Self::build_latest_status(step_outputs);

        for step in &workflow.definition.steps {
            let step_id = step.id.as_str();

            // Non-activated branch targets are considered unreachable (will be skipped),
            // unless they are entry steps (no dependencies) which are always reachable.
            if branch_targets.contains(step_id)
                && !activated_steps.contains(&step.id)
                && !step.depends_on.is_empty()
            {
                continue;
            }

            match latest_status.get(step_id) {
                Some(status) => {
                    if !matches!(status,
                        WorkflowStepStatus::Completed
                        | WorkflowStepStatus::Skipped
                        | WorkflowStepStatus::Rejected
                    ) {
                        return false;
                    }
                }
                None => return false, // No output at all — not done
            }
        }

        true
    }

    async fn complete_workflow(
        &self,
        run_id: &str,
        run: &WorkflowRun,
        workflow: &Workflow,
        step_outputs: &[WorkflowStepOutput],
    ) -> anyhow::Result<()> {
        // Mark unreachable branch targets as Skipped
        let branch_targets = Self::collect_branch_targets(workflow);

        for step in &workflow.definition.steps {
            if branch_targets.contains(&step.id)
                && !run.activated_steps.contains(&step.id)
                && !step.depends_on.is_empty()
            {
                // Auto-mark as skipped if no output exists
                let has_output = step_outputs.iter().any(|o| o.step_id == step.id);
                if !has_output {
                    let _ = composer_db::models::workflow_step_output::create(
                        &self.db.pool,
                        run_id,
                        &step.id,
                        &step.step_type,
                        &WorkflowStepStatus::Skipped,
                        None,
                    ).await;
                }
            }
        }

        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            run_id,
            &WorkflowRunStatus::Completed,
        ).await?;

        // Read current task status before updating, so the event has the correct from_status
        let task = composer_db::models::task::find_by_id(&self.db.pool, &run.task_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        let from_status = task.status;

        composer_db::models::task::update_status(
            &self.db.pool,
            &run.task_id.to_string(),
            &TaskStatus::Done,
        ).await?;

        // Clean up all session worktrees
        for step in step_outputs {
            if let Some(ref sid) = step.session_id {
                self.cleanup_worktree(&sid.to_string()).await;
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
            from_status,
            to_status: TaskStatus::Done,
        });

        self.cleanup_run_lock(run_id);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Step execution
    // -----------------------------------------------------------------------

    async fn execute_agent_step(
        &self,
        run_id: &str,
        task: &Task,
        agent_id: uuid::Uuid,
        step_def: &WorkflowStepDefinition,
        is_new_session: bool,
        workflow: &Workflow,
    ) -> anyhow::Result<()> {
        let prompt = self.build_prompt(run_id, task, step_def).await?;

        let step_output = composer_db::models::workflow_step_output::create(
            &self.db.pool,
            run_id,
            &step_def.id,
            &step_def.step_type,
            &WorkflowStepStatus::Running,
            None,
        ).await?;

        let session = if is_new_session {
            let repo_path = self.get_repo_path(task).await?;
            let session = self
                .session_service
                .create_session(CreateSessionRequest {
                    agent_id,
                    task_id: task.id,
                    prompt,
                    repo_path,
                    name: Some(format!("Workflow: {}", step_def.name)),
                    auto_approve: Some(task.auto_approve),
                    exit_on_result: true,
                })
                .await?;
            session
        } else {
            // Find the nearest ancestor session to resume
            let ancestor_session_id = self.find_ancestor_session(run_id, &step_def.id, workflow).await?;
            match ancestor_session_id {
                Some(sid) => {
                    self.session_service
                        .resume_session(
                            &sid,
                            ResumeSessionRequest {
                                prompt: Some(prompt),
                                exit_on_result: true,
                                continue_chat: false,
                            },
                        )
                        .await?
                }
                None => {
                    // No ancestor found, create a new session
                    let repo_path = self.get_repo_path(task).await?;
                    self.session_service
                        .create_session(CreateSessionRequest {
                            agent_id,
                            task_id: task.id,
                            prompt,
                            repo_path,
                            name: Some(format!("Workflow: {}", step_def.name)),
                            auto_approve: Some(task.auto_approve),
                            exit_on_result: true,
                        })
                        .await?
                }
            }
        };

        // Update step output with session_id
        composer_db::models::workflow_step_output::update_session_id(
            &self.db.pool,
            &step_output.id.to_string(),
            &session.id.to_string(),
        ).await?;

        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            run_id,
            &WorkflowRunStatus::Running,
        ).await?;

        let updated_run = composer_db::models::workflow_run::find_by_id(&self.db.pool, run_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;
        self.event_bus.broadcast(WsEvent::WorkflowRunUpdated(updated_run));

        let updated_step = composer_db::models::workflow_step_output::find_by_id(
            &self.db.pool,
            &step_output.id.to_string(),
        ).await?
        .ok_or_else(|| anyhow::anyhow!("Step output not found after write"))?;
        self.event_bus.broadcast(WsEvent::WorkflowStepChanged {
            workflow_run_id: run_id.parse()?,
            step: updated_step,
        });

        Ok(())
    }

    async fn execute_human_gate(
        &self,
        run_id: &str,
        task: &Task,
        step_def: &WorkflowStepDefinition,
    ) -> anyhow::Result<()> {
        let step_output = composer_db::models::workflow_step_output::create(
            &self.db.pool,
            run_id,
            &step_def.id,
            &step_def.step_type,
            &WorkflowStepStatus::WaitingForHuman,
            None,
        ).await?;

        composer_db::models::workflow_run::update_status(
            &self.db.pool,
            run_id,
            &WorkflowRunStatus::Paused,
        ).await?;

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
            step_id: step_def.id.clone(),
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

    async fn execute_pr_review(
        &self,
        run_id: &str,
        task: &Task,
        agent_id: uuid::Uuid,
        step_def: &WorkflowStepDefinition,
    ) -> anyhow::Result<()> {
        let prompt = self.build_prompt(run_id, task, step_def).await?;
        let repo_path = self.get_repo_path(task).await?;

        // Create step output BEFORE starting the session so that
        // on_session_completed can always find the matching record.
        let step_output = composer_db::models::workflow_step_output::create(
            &self.db.pool,
            run_id,
            &step_def.id,
            &step_def.step_type,
            &WorkflowStepStatus::Running,
            None,
        ).await?;

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

        // Update step output with the session_id
        composer_db::models::workflow_step_output::update_session_id(
            &self.db.pool,
            &step_output.id.to_string(),
            &session.id.to_string(),
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

        let updated_step = composer_db::models::workflow_step_output::find_by_id(
            &self.db.pool,
            &step_output.id.to_string(),
        ).await?
        .ok_or_else(|| anyhow::anyhow!("Step output not found after write"))?;
        self.event_bus.broadcast(WsEvent::WorkflowStepChanged {
            workflow_run_id: run_id.parse()?,
            step: updated_step,
        });

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Prompt building
    // -----------------------------------------------------------------------

    async fn build_prompt(
        &self,
        run_id: &str,
        task: &Task,
        step_def: &WorkflowStepDefinition,
    ) -> anyhow::Result<String> {
        if step_def.step_type == WorkflowStepType::HumanGate {
            return Ok(String::new());
        }

        let template = step_def.prompt_template.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Agentic step '{}' requires a prompt_template", step_def.name))?;

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
            ).await?;
            match composer_db::models::project_instruction::format_instructions_block(&instructions) {
                Some(block) => format!("{}\n\n{}", block, base_task_context),
                None => base_task_context,
            }
        } else {
            base_task_context
        };

        let mut prompt = template.clone();
        prompt = prompt.replace("{{task}}", &task_context);

        // Inject {{step:step_id}} — resolves to latest output for step
        static STEP_ID_PATTERN: std::sync::LazyLock<regex::Regex> =
            std::sync::LazyLock::new(|| regex::Regex::new(r"\{\{step:([\w-]+)\}\}").unwrap());
        let step_id_pattern = &*STEP_ID_PATTERN;
        let prompt_clone = prompt.clone();
        for cap in step_id_pattern.captures_iter(&prompt_clone) {
            let full_match = &cap[0];
            let ref_step_id = &cap[1];
            let latest_output = step_outputs.iter()
                .filter(|o| o.step_id == ref_step_id)
                .last()
                .and_then(|o| o.output.as_deref())
                .unwrap_or("");
            prompt = prompt.replace(full_match, latest_output);
        }

        // Inject {{rejection}} — latest rejected HumanGate output's comments
        if prompt.contains("{{rejection}}") {
            let rejection_text = step_outputs.iter()
                .filter(|o| o.step_type == WorkflowStepType::HumanGate
                    && o.status == WorkflowStepStatus::Rejected)
                .last()
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

    // -----------------------------------------------------------------------
    // Session resolution for DAG
    // -----------------------------------------------------------------------

    /// Walk the depends_on chain (BFS) to find the nearest ancestor agentic step
    /// with a completed session.
    async fn find_ancestor_session(
        &self,
        run_id: &str,
        step_id: &str,
        workflow: &Workflow,
    ) -> anyhow::Result<Option<String>> {
        let step_map: HashMap<&str, &WorkflowStepDefinition> = workflow.definition.steps.iter()
            .map(|s| (s.id.as_str(), s))
            .collect();

        let step_def = step_map.get(step_id)
            .ok_or_else(|| anyhow::anyhow!("Step '{}' not found", step_id))?;

        let mut queue: VecDeque<&str> = step_def.depends_on.iter()
            .map(|s| s.as_str())
            .collect();

        // Also check for HumanGate on_approve/on_reject that points to this step
        // and walk through the gate's dependencies
        for s in &workflow.definition.steps {
            if s.step_type == WorkflowStepType::HumanGate {
                if s.on_approve.as_deref() == Some(step_id) || s.on_reject.as_deref() == Some(step_id) {
                    queue.push_back(s.id.as_str());
                }
            }
        }

        let mut visited = HashSet::new();

        while let Some(dep_id) = queue.pop_front() {
            if !visited.insert(dep_id) {
                continue;
            }

            if let Some(dep_def) = step_map.get(dep_id) {
                if dep_def.step_type == WorkflowStepType::Agentic
                    && !matches!(dep_def.session_mode.as_ref(), Some(SessionMode::Separate))
                {
                    // Check if this step has a completed session
                    if let Some(output) = composer_db::models::workflow_step_output::latest_for_step(
                        &self.db.pool, run_id, dep_id,
                    ).await? {
                        if matches!(output.status, WorkflowStepStatus::Completed) {
                            if let Some(sid) = output.session_id {
                                return Ok(Some(sid.to_string()));
                            }
                        }
                    }
                }

                // Continue searching ancestors
                for parent in &dep_def.depends_on {
                    queue.push_back(parent.as_str());
                }
            }
        }

        Ok(None)
    }

    // -----------------------------------------------------------------------
    // Loop logic
    // -----------------------------------------------------------------------

    async fn should_loop(
        &self,
        run_id: &str,
        step_def: &WorkflowStepDefinition,
        loop_target: &str,
    ) -> anyhow::Result<LoopDecision> {
        let step_outputs =
            composer_db::models::workflow_step_output::list_by_run(&self.db.pool, run_id).await?;

        // Check if the loop target's latest output indicates no issues were found.
        // If the review step reported [NO_ISSUES_FOUND], there is nothing to fix
        // and we should skip the loop entirely, advancing to the next step.
        let target_latest = step_outputs.iter()
            .filter(|o| o.step_id == loop_target && matches!(o.status, WorkflowStepStatus::Completed))
            .max_by_key(|o| o.created_at);
        if let Some(target_output) = target_latest {
            if let Some(ref output) = target_output.output {
                if output.contains("[NO_ISSUES_FOUND]") {
                    tracing::info!(
                        "Workflow run {}: loop target '{}' reported no issues, skipping loop",
                        run_id, loop_target
                    );
                    return Ok(LoopDecision::NoIssuesFound);
                }
            }
        }

        if let Some(max) = step_def.max_retries {
            let target_completed_count = step_outputs.iter()
                .filter(|o| o.step_id == loop_target && matches!(o.status, WorkflowStepStatus::Completed))
                .count() as i32;

            if target_completed_count == 0 {
                return Ok(LoopDecision::Loop);
            }
            let retries_done = (target_completed_count - 1).max(0);
            if retries_done >= max {
                return Ok(LoopDecision::MaxRetriesExhausted);
            }
        }

        Ok(LoopDecision::Loop)
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    async fn get_repo_path(&self, task: &Task) -> anyhow::Result<String> {
        let project_id = task
            .project_id
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
}
