//! Long-running scenario tests that exercise the full workflow lifecycle
//! with real Claude Code agent sessions.
//!
//! These tests are `#[ignore]` by default — they require:
//! - A real Claude Code installation (`npx @anthropic-ai/claude-code`)
//! - Valid API credentials (ANTHROPIC_API_KEY)
//! - A git repository to work against
//!
//! Run with: `cargo test -p composer-tests --test workflow_scenarios -- --ignored --nocapture`
//! Timeout: expect 3-10 minutes per test depending on agent response time.

use composer_api_types::*;
use composer_db::Database;
use composer_executors::process_manager::AgentProcessManager;
use composer_services::event_bus::EventBus;
use composer_services::session_service::SessionService;
use composer_services::workflow_engine::WorkflowEngine;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

fn test_repo_path() -> String {
    std::env::var("TEST_REPO_PATH").unwrap_or_else(|_| {
        let cwd = std::env::current_dir().expect("Failed to get current directory");
        cwd.to_string_lossy().to_string()
    })
}
const STEP_TIMEOUT: Duration = Duration::from_secs(300); // 5 min per step

struct TestHarness {
    db: Arc<Database>,
    #[allow(dead_code)]
    event_bus: EventBus,
    engine: WorkflowEngine,
    #[allow(dead_code)]
    session_service: SessionService,
    rx: broadcast::Receiver<WsEvent>,
}

async fn setup() -> TestHarness {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    db.run_migrations().await.unwrap();
    let db = Arc::new(db);
    let (event_bus, persist_rx) = EventBus::new();
    let rx = event_bus.subscribe();
    let pm = Arc::new(AgentProcessManager::new(event_bus.sender(), event_bus.persist_sender()));
    let session_service = SessionService::new(db.clone(), event_bus.clone(), pm, persist_rx);
    let engine = WorkflowEngine::new(db.clone(), event_bus.clone(), session_service.clone());
    session_service.set_workflow_engine(engine.clone());

    TestHarness {
        db,
        event_bus,
        engine,
        session_service,
        rx,
    }
}

async fn create_project_with_repo(db: &Database) -> String {
    let project = composer_db::models::project::create(&db.pool, "Scenario Test", None)
        .await
        .unwrap();
    let pid = project.id.to_string();
    composer_db::models::project_repository::create(
        &db.pool,
        &pid,
        &test_repo_path(),
        None,
        Some(&RepositoryRole::Primary),
        None,
    )
    .await
    .unwrap();
    pid
}

async fn create_agent(db: &Database) -> String {
    let agent = composer_db::models::agent::create(
        &db.pool,
        "Scenario Agent",
        &AgentType::ClaudeCode,
        None,
    )
    .await
    .unwrap();
    agent.id.to_string()
}

/// Wait for a specific event, collecting all events along the way.
async fn wait_for_event(
    rx: &mut broadcast::Receiver<WsEvent>,
    timeout: Duration,
    predicate: impl Fn(&WsEvent) -> bool,
) -> Vec<WsEvent> {
    let mut events = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        match tokio::time::timeout_at(deadline, rx.recv()).await {
            Ok(Ok(event)) => {
                let matched = predicate(&event);
                events.push(event);
                if matched {
                    return events;
                }
            }
            Ok(Err(broadcast::error::RecvError::Lagged(n))) => {
                eprintln!("Warning: lagged {} events", n);
            }
            Ok(Err(_)) => panic!("Event channel closed"),
            Err(_) => panic!(
                "Timed out after {:?} waiting for event. Received {} events: {:#?}",
                timeout,
                events.len(),
                events.iter().map(event_summary).collect::<Vec<_>>()
            ),
        }
    }
}

fn event_summary(e: &WsEvent) -> String {
    match e {
        WsEvent::SessionStarted { session_id, .. } => {
            format!("SessionStarted({})", &session_id.to_string()[..8])
        }
        WsEvent::SessionCompleted { session_id, .. } => {
            format!("SessionCompleted({})", &session_id.to_string()[..8])
        }
        WsEvent::SessionFailed {
            session_id, error, ..
        } => format!(
            "SessionFailed({}, {})",
            &session_id.to_string()[..8],
            &error[..error.len().min(50)]
        ),
        WsEvent::SessionPaused { session_id } => {
            format!("SessionPaused({})", &session_id.to_string()[..8])
        }
        WsEvent::SessionOutput {
            session_id,
            log_type,
            content,
            ..
        } => {
            let preview = if content.len() > 100 {
                &content[..100]
            } else {
                content.as_str()
            };
            format!(
                "SessionOutput({}, {:?}, {})",
                &session_id.to_string()[..8],
                log_type,
                preview
            )
        }
        WsEvent::WorkflowRunUpdated(run) => format!(
            "WorkflowRunUpdated(status={:?})",
            run.status
        ),
        WsEvent::WorkflowStepChanged { step, .. } => format!(
            "WorkflowStepChanged(step={}, status={:?})",
            step.step_id, step.status
        ),
        WsEvent::WorkflowWaitingForHuman { step_id, .. } => {
            format!("WorkflowWaitingForHuman(step={})", step_id)
        }
        WsEvent::WorkflowRunCompleted { .. } => "WorkflowRunCompleted".to_string(),
        WsEvent::TaskMoved {
            from_status,
            to_status,
            ..
        } => format!("TaskMoved({:?} → {:?})", from_status, to_status),
        other => format!("{:?}", std::mem::discriminant(other)),
    }
}

// ---------------------------------------------------------------------------
// Scenario: Plan step completes and workflow advances to human gate
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore] // Requires real Claude Code + API key (~2-5 min)
async fn scenario_plan_step_completes_and_pauses_at_human_gate() {
    let mut h = setup().await;
    let project_id = create_project_with_repo(&h.db).await;
    let agent_id = create_agent(&h.db).await;

    // Create a simple workflow: Plan → Human Gate
    let def = WorkflowDefinition {
        steps: vec![
            WorkflowStepDefinition {
                id: "plan".to_string(),
                step_type: WorkflowStepType::Agentic,
                name: "Plan".to_string(),
                prompt_template: Some("List the files in the root directory of this project. Keep your response under 5 lines. Do NOT implement anything.".to_string()),
                depends_on: vec![],
                on_approve: None,
                on_reject: None,
                max_retries: None,
                loop_back_to: None,
                session_mode: Some(SessionMode::New),
                interactive: None,
            },
            WorkflowStepDefinition {
                id: "review".to_string(),
                step_type: WorkflowStepType::HumanGate,
                name: "Review Plan".to_string(),
                prompt_template: None,
                depends_on: vec!["plan".to_string()],
                on_approve: Some("plan".to_string()), // loops back for simplicity
                on_reject: None,
                max_retries: None,
                loop_back_to: None,
                session_mode: None,
                interactive: None,
            },
        ],
    };
    let workflow = composer_db::models::workflow::create(
        &h.db.pool, "Test Plan", &def,
    ).await.unwrap();

    let task = composer_db::models::task::create(
        &h.db.pool,
        "Scenario: plan then gate",
        None,
        None,
        None,
        Some(&project_id),
        Some(&agent_id),
        None,
    )
    .await
    .unwrap();
    let task_id = task.id.to_string();

    eprintln!("Starting workflow...");
    let run = h
        .engine
        .start(&task_id, &workflow.id.to_string())
        .await
        .unwrap();
    let run_id = run.id.to_string();
    eprintln!(
        "Workflow run {} started, waiting for Plan step to complete...",
        &run_id[..8]
    );

    // Wait for the workflow to pause at the human gate
    wait_for_event(&mut h.rx, STEP_TIMEOUT, |e| {
        matches!(e, WsEvent::WorkflowWaitingForHuman { step_id, .. } if step_id == "review")
    })
    .await;

    eprintln!("Workflow paused at human gate (review) — verifying state...");

    let run = composer_db::models::workflow_run::find_by_id(&h.db.pool, &run_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(run.status, WorkflowRunStatus::Paused);

    let task = composer_db::models::task::find_by_id(&h.db.pool, &task_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(task.status, TaskStatus::Waiting);

    let steps = composer_db::models::workflow_step_output::list_by_run(&h.db.pool, &run_id)
        .await
        .unwrap();
    let plan_step = steps.iter().find(|s| s.step_id == "plan").unwrap();
    assert_eq!(plan_step.status, WorkflowStepStatus::Completed);
    assert!(plan_step.output.is_some());

    eprintln!("PASSED: Plan step completed and workflow paused at human gate");
}

// ---------------------------------------------------------------------------
// Scenario: Full plan → approve → implement cycle
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore] // Requires real Claude Code + API key (~5-10 min)
async fn scenario_plan_approve_implement() {
    let mut h = setup().await;
    let project_id = create_project_with_repo(&h.db).await;
    let agent_id = create_agent(&h.db).await;

    let def = WorkflowDefinition {
        steps: vec![
            WorkflowStepDefinition {
                id: "plan".to_string(),
                step_type: WorkflowStepType::Agentic,
                name: "Plan".to_string(),
                prompt_template: Some("Describe how you would add a comment '// scenario test' to the top of CLAUDE.md. Keep it under 3 lines. Do NOT implement.".to_string()),
                depends_on: vec![],
                on_approve: None,
                on_reject: None,
                max_retries: None,
                loop_back_to: None,
                session_mode: Some(SessionMode::New),
                interactive: None,
            },
            WorkflowStepDefinition {
                id: "review".to_string(),
                step_type: WorkflowStepType::HumanGate,
                name: "Review".to_string(),
                prompt_template: None,
                depends_on: vec!["plan".to_string()],
                on_approve: Some("implement".to_string()),
                on_reject: Some("plan".to_string()),
                max_retries: None,
                loop_back_to: None,
                session_mode: None,
                interactive: None,
            },
            WorkflowStepDefinition {
                id: "implement".to_string(),
                step_type: WorkflowStepType::Agentic,
                name: "Implement".to_string(),
                prompt_template: Some("The plan is approved. Just reply with 'Implementation complete.' and do NOT modify any files.".to_string()),
                depends_on: vec!["review".to_string()],
                on_approve: None,
                on_reject: None,
                max_retries: None,
                loop_back_to: None,
                session_mode: Some(SessionMode::Resume),
                interactive: None,
            },
        ],
    };
    let workflow = composer_db::models::workflow::create(
        &h.db.pool, "Test Full Cycle", &def,
    ).await.unwrap();

    let task = composer_db::models::task::create(
        &h.db.pool,
        "Scenario: full cycle",
        None,
        None,
        None,
        Some(&project_id),
        Some(&agent_id),
        None,
    )
    .await
    .unwrap();
    let task_id = task.id.to_string();

    eprintln!("Step 1: Starting workflow (Plan step)...");
    let run = h
        .engine
        .start(&task_id, &workflow.id.to_string())
        .await
        .unwrap();
    let run_id = run.id.to_string();

    wait_for_event(&mut h.rx, STEP_TIMEOUT, |e| {
        matches!(e, WsEvent::WorkflowWaitingForHuman { step_id, .. } if step_id == "review")
    })
    .await;
    eprintln!("Step 1 complete — Plan done, paused at human gate");

    eprintln!("Step 2: Approving plan...");
    h.engine.submit_decision(&run_id, "review", true, None).await.unwrap();

    wait_for_event(&mut h.rx, STEP_TIMEOUT, |e| {
        matches!(e, WsEvent::WorkflowRunCompleted { .. })
    })
    .await;
    eprintln!("Step 3 complete — Implement done, workflow completed");

    let run = composer_db::models::workflow_run::find_by_id(&h.db.pool, &run_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(run.status, WorkflowRunStatus::Completed);

    let task = composer_db::models::task::find_by_id(&h.db.pool, &task_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(task.status, TaskStatus::Done);

    eprintln!("PASSED: Full plan → approve → implement cycle completed");
}

// ---------------------------------------------------------------------------
// Scenario: Plan rejection loops back
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore] // Requires real Claude Code + API key (~5-10 min)
async fn scenario_plan_rejection_loops_back() {
    let mut h = setup().await;
    let project_id = create_project_with_repo(&h.db).await;
    let agent_id = create_agent(&h.db).await;

    let def = WorkflowDefinition {
        steps: vec![
            WorkflowStepDefinition {
                id: "plan".to_string(),
                step_type: WorkflowStepType::Agentic,
                name: "Plan".to_string(),
                prompt_template: Some("Reply with exactly: 'Plan v1'. Nothing else.".to_string()),
                depends_on: vec![],
                on_approve: None,
                on_reject: None,
                max_retries: None,
                loop_back_to: None,
                session_mode: Some(SessionMode::New),
                interactive: None,
            },
            WorkflowStepDefinition {
                id: "review".to_string(),
                step_type: WorkflowStepType::HumanGate,
                name: "Review".to_string(),
                prompt_template: None,
                depends_on: vec!["plan".to_string()],
                on_approve: Some("plan".to_string()),
                on_reject: Some("plan".to_string()),
                max_retries: None,
                loop_back_to: None,
                session_mode: None,
                interactive: None,
            },
        ],
    };
    let workflow = composer_db::models::workflow::create(
        &h.db.pool, "Test Rejection", &def,
    ).await.unwrap();

    let task = composer_db::models::task::create(
        &h.db.pool,
        "Scenario: rejection",
        None,
        None,
        None,
        Some(&project_id),
        Some(&agent_id),
        None,
    )
    .await
    .unwrap();
    let task_id = task.id.to_string();

    eprintln!("Starting workflow...");
    let run = h
        .engine
        .start(&task_id, &workflow.id.to_string())
        .await
        .unwrap();
    let run_id = run.id.to_string();

    // Wait for first human gate
    wait_for_event(&mut h.rx, STEP_TIMEOUT, |e| {
        matches!(e, WsEvent::WorkflowWaitingForHuman { step_id, .. } if step_id == "review")
    })
    .await;
    eprintln!("First plan done, rejecting...");

    // Reject with feedback
    h.engine
        .submit_decision(&run_id, "review", false, Some("Please add more detail"))
        .await
        .unwrap();

    // Should loop back to plan and then pause at human gate again
    wait_for_event(&mut h.rx, STEP_TIMEOUT, |e| {
        matches!(e, WsEvent::WorkflowWaitingForHuman { step_id, .. } if step_id == "review")
    })
    .await;
    eprintln!("Second plan done after rejection feedback");

    let run = composer_db::models::workflow_run::find_by_id(&h.db.pool, &run_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(run.iteration_count, 1);

    let steps = composer_db::models::workflow_step_output::list_by_run(&h.db.pool, &run_id)
        .await
        .unwrap();
    let plan_attempts: Vec<_> = steps.iter().filter(|s| s.step_id == "plan").collect();
    assert_eq!(plan_attempts.len(), 2);

    let task = composer_db::models::task::find_by_id(&h.db.pool, &task_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(task.status, TaskStatus::Waiting);

    eprintln!("PASSED: Plan rejection correctly looped back with feedback");
}

// ---------------------------------------------------------------------------
// Scenario: exit_on_result causes session to complete
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore] // Requires real Claude Code + API key (~1-2 min)
async fn scenario_exit_on_result_completes_session() {
    let (event_bus, _persist_rx) = EventBus::new();
    let mut rx = event_bus.subscribe();
    let pm = Arc::new(AgentProcessManager::new(event_bus.sender(), event_bus.persist_sender()));

    let session_id = uuid::Uuid::new_v4();
    let agent_id = uuid::Uuid::new_v4();

    eprintln!("Spawning session with exit_on_result=true...");
    pm.spawn(composer_executors::process_manager::SpawnOptions {
        session_id,
        agent_id,
        task_id: None,
        prompt: "Reply with exactly: EXIT_TEST. Nothing else.".to_string(),
        working_dir: test_repo_path(),
        auto_approve: true,
        resume_session_id: None,
        resume_at_message_id: None,
        exit_on_result: true,
    })
    .await
    .expect("Failed to spawn");

    let events = wait_for_event(&mut rx, Duration::from_secs(120), |e| {
        matches!(e, WsEvent::SessionCompleted { .. })
    })
    .await;

    let completed = events
        .iter()
        .find(|e| matches!(e, WsEvent::SessionCompleted { .. }));
    assert!(completed.is_some());

    if let Some(WsEvent::SessionCompleted { result_summary, .. }) = completed {
        eprintln!("Session completed with summary: {:?}", result_summary);
    }

    assert!(!pm.is_running(&session_id));
    eprintln!("PASSED: exit_on_result correctly closed stdin and completed session");
}
