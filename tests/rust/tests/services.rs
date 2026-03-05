use std::sync::Arc;
use composer_api_types::*;
use composer_db::Database;
use composer_services::event_bus::EventBus;
use composer_services::task_service::TaskService;
use composer_services::agent_service::AgentService;
use composer_services::session_service::SessionService;
use composer_services::worktree_service::WorktreeService;
use composer_executors::process_manager::AgentProcessManager;

// ---------------------------------------------------------------------------
// EventBus tests (from crates/services/src/event_bus.rs)
// ---------------------------------------------------------------------------

mod event_bus_tests {
    use super::*;

    fn make_task_created_event() -> WsEvent {
        WsEvent::TaskCreated(Task {
            id: uuid::Uuid::new_v4(),
            title: "Test".to_string(),
            description: None,
            status: TaskStatus::Backlog,
            priority: 0,
            assigned_agent_id: None,
            project_id: None,
            auto_approve: true,
            position: 1.0,
            task_number: 0,
            simple_id: String::new(),
            pr_urls: vec![],
            workflow_run_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    #[tokio::test]
    async fn broadcast_and_subscribe() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        bus.broadcast(make_task_created_event());
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, WsEvent::TaskCreated(_)));
    }

    #[tokio::test]
    async fn multiple_subscribers() {
        let bus = EventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        bus.broadcast(WsEvent::TaskDeleted { task_id: uuid::Uuid::nil() });
        assert!(rx1.recv().await.is_ok());
        assert!(rx2.recv().await.is_ok());
    }

    #[test]
    fn no_subscribers_no_panic() {
        let bus = EventBus::new();
        // Should not panic even with no subscribers
        bus.broadcast(make_task_created_event());
    }
}

// ---------------------------------------------------------------------------
// TaskService tests (from crates/services/src/task_service.rs)
// ---------------------------------------------------------------------------

mod task_service_tests {
    use super::*;

    async fn setup() -> (TaskService, tokio::sync::broadcast::Receiver<WsEvent>) {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.run_migrations().await.unwrap();
        let event_bus = EventBus::new();
        let rx = event_bus.subscribe();
        let db = Arc::new(db);
        let process_manager = Arc::new(AgentProcessManager::new(event_bus.sender()));
        let session_service = SessionService::new(db.clone(), event_bus.clone(), process_manager);
        let svc = TaskService::new(db, event_bus, session_service);
        (svc, rx)
    }

    #[tokio::test]
    async fn create_task_broadcasts_event() {
        let (svc, mut rx) = setup().await;
        let task = svc
            .create(CreateTaskRequest {
                title: "New Task".to_string(),
                description: None,
                priority: None,
                status: None,
                project_id: None,
                assigned_agent_id: None,

            })
            .await
            .unwrap();
        assert_eq!(task.title, "New Task");
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, WsEvent::TaskCreated(_)));
    }

    #[tokio::test]
    async fn list_all_empty() {
        let (svc, _rx) = setup().await;
        let tasks = svc.list_all().await.unwrap();
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn get_existing_task() {
        let (svc, _rx) = setup().await;
        let task = svc
            .create(CreateTaskRequest {
                title: "T".to_string(),
                description: None,
                priority: None,
                status: None,
                project_id: None,
                assigned_agent_id: None,

            })
            .await
            .unwrap();
        let found = svc.get(&task.id.to_string()).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn get_nonexistent_task() {
        let (svc, _rx) = setup().await;
        let found = svc
            .get("00000000-0000-0000-0000-000000000000")
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn update_task_broadcasts() {
        let (svc, mut rx) = setup().await;
        let task = svc
            .create(CreateTaskRequest {
                title: "Old".to_string(),
                description: None,
                priority: None,
                status: None,
                project_id: None,
                assigned_agent_id: None,

            })
            .await
            .unwrap();
        let _ = rx.recv().await; // consume TaskCreated

        let updated = svc
            .update(
                &task.id.to_string(),
                UpdateTaskRequest {
                    title: Some("New".to_string()),
                    description: None,
                    priority: None,
                    status: None,
                    position: None,
                    project_id: None,
                    assigned_agent_id: None,
    
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.title, "New");
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, WsEvent::TaskUpdated(_)));
    }

    #[tokio::test]
    async fn delete_task_broadcasts() {
        let (svc, mut rx) = setup().await;
        let task = svc
            .create(CreateTaskRequest {
                title: "Del".to_string(),
                description: None,
                priority: None,
                status: None,
                project_id: None,
                assigned_agent_id: None,

            })
            .await
            .unwrap();
        let _ = rx.recv().await; // consume TaskCreated

        svc.delete(&task.id.to_string()).await.unwrap();
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, WsEvent::TaskDeleted { .. }));
    }

    #[tokio::test]
    async fn move_task_changes_status() {
        let (svc, mut rx) = setup().await;
        let task = svc
            .create(CreateTaskRequest {
                title: "Move".to_string(),
                description: None,
                priority: None,
                status: None,
                project_id: None,
                assigned_agent_id: None,

            })
            .await
            .unwrap();
        let _ = rx.recv().await; // consume TaskCreated

        let moved = svc
            .move_task(
                &task.id.to_string(),
                MoveTaskRequest {
                    status: TaskStatus::Done,
                    position: None,
                },
            )
            .await
            .unwrap();
        assert!(matches!(moved.status, TaskStatus::Done));
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, WsEvent::TaskMoved { .. }));
    }
}

// ---------------------------------------------------------------------------
// AgentService tests (from crates/services/src/agent_service.rs)
// ---------------------------------------------------------------------------

mod agent_service_tests {
    use super::*;

    async fn setup() -> (AgentService, tokio::sync::broadcast::Receiver<WsEvent>) {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.run_migrations().await.unwrap();
        let event_bus = EventBus::new();
        let rx = event_bus.subscribe();
        let pm = Arc::new(AgentProcessManager::new(event_bus.sender()));
        let svc = AgentService::new(Arc::new(db), event_bus, pm);
        (svc, rx)
    }

    #[tokio::test]
    async fn create_and_list() {
        let (svc, _rx) = setup().await;
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
        let (svc, _rx) = setup().await;
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
        let (svc, mut rx) = setup().await;
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

// ---------------------------------------------------------------------------
// SessionService tests (from crates/services/src/session_service.rs)
// ---------------------------------------------------------------------------

mod session_service_tests {
    use super::*;
    use composer_db::models::{agent, session, worktree};

    async fn setup() -> SessionService {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.run_migrations().await.unwrap();
        let event_bus = EventBus::new();
        let pm = Arc::new(AgentProcessManager::new(event_bus.sender()));
        SessionService::new(Arc::new(db), event_bus, pm)
    }

    /// Setup that returns the DB and event bus for event-driven tests.
    async fn setup_with_internals() -> (SessionService, Arc<Database>, EventBus) {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.run_migrations().await.unwrap();
        let db = Arc::new(db);
        let event_bus = EventBus::new();
        let pm = Arc::new(AgentProcessManager::new(event_bus.sender()));
        let svc = SessionService::new(db.clone(), event_bus.clone(), pm);
        (svc, db, event_bus)
    }

    #[tokio::test]
    async fn list_all_empty() {
        let svc = setup().await;
        let sessions = svc.list_all().await.unwrap();
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn get_nonexistent() {
        let svc = setup().await;
        let found = svc
            .get("00000000-0000-0000-0000-000000000000")
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn session_completed_resets_agent_and_cleans_worktree() {
        let (_svc, db, event_bus) = setup_with_internals().await;

        // Create agent in Busy state
        let ag = agent::create(&db.pool, "Test Agent", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        let agent_id = ag.id.to_string();
        agent::update_status(&db.pool, &agent_id, &AgentStatus::Busy)
            .await
            .unwrap();

        // Create session with a worktree record
        let session_id = uuid::Uuid::new_v4().to_string();
        let wt = worktree::create(
            &db.pool, &agent_id, &session_id,
            "/nonexistent/repo", "/nonexistent/repo/wt", "branch",
        )
        .await
        .unwrap();
        let wt_id = wt.id.to_string();

        let sess = session::create_with_status(
            &db.pool, &session_id, &agent_id, None, Some(&wt_id),
            "do work", &SessionStatus::Running,
        )
        .await
        .unwrap();

        // Broadcast SessionCompleted event
        event_bus.broadcast(WsEvent::SessionCompleted {
            session_id: sess.id,
            result_summary: Some("done".to_string()),
            claude_session_id: None,
        });

        // Give the background event listener time to process
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Verify agent is reset to Idle
        let ag_after = agent::find_by_id(&db.pool, &agent_id).await.unwrap().unwrap();
        assert_eq!(ag_after.status, AgentStatus::Idle);

        // Verify worktree is marked Deleted in DB
        let wt_after = worktree::find_by_id(&db.pool, &wt_id).await.unwrap().unwrap();
        assert_eq!(wt_after.status, WorktreeStatus::Deleted);
    }

    #[tokio::test]
    async fn session_failed_resets_agent_but_preserves_worktree() {
        let (_svc, db, event_bus) = setup_with_internals().await;

        // Create agent in Busy state
        let ag = agent::create(&db.pool, "Test Agent", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        let agent_id = ag.id.to_string();
        agent::update_status(&db.pool, &agent_id, &AgentStatus::Busy)
            .await
            .unwrap();

        // Create session with a worktree record
        let session_id = uuid::Uuid::new_v4().to_string();
        let wt = worktree::create(
            &db.pool, &agent_id, &session_id,
            "/nonexistent/repo", "/nonexistent/repo/wt", "branch",
        )
        .await
        .unwrap();
        let wt_id = wt.id.to_string();

        let sess = session::create_with_status(
            &db.pool, &session_id, &agent_id, None, Some(&wt_id),
            "do work", &SessionStatus::Running,
        )
        .await
        .unwrap();

        // Broadcast SessionFailed event
        event_bus.broadcast(WsEvent::SessionFailed {
            session_id: sess.id,
            error: "something broke".to_string(),
            claude_session_id: None,
        });

        // Give the background event listener time to process
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Verify agent is reset to Idle
        let ag_after = agent::find_by_id(&db.pool, &agent_id).await.unwrap().unwrap();
        assert_eq!(ag_after.status, AgentStatus::Idle);

        // Verify worktree is still Active (preserved for retry)
        let wt_after = worktree::find_by_id(&db.pool, &wt_id).await.unwrap().unwrap();
        assert_eq!(wt_after.status, WorktreeStatus::Active);
    }

    #[tokio::test]
    async fn retry_non_failed_session_is_rejected() {
        let (_svc, db, _event_bus) = setup_with_internals().await;

        // Create agent
        let ag = agent::create(&db.pool, "Test Agent", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        let agent_id = ag.id.to_string();

        // Create session in Running status
        let session_id = uuid::Uuid::new_v4().to_string();
        let sess = session::create_with_status(
            &db.pool, &session_id, &agent_id, None, None,
            "do work", &SessionStatus::Running,
        )
        .await
        .unwrap();

        // Try to retry a running session — should fail
        let result = _svc
            .retry_session(&sess.id.to_string(), ResumeSessionRequest { prompt: None })
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Only failed sessions can be retried"));
    }
}

// ---------------------------------------------------------------------------
// WorktreeService tests
// ---------------------------------------------------------------------------

mod worktree_service_tests {
    use super::*;
    use composer_db::models::{agent, session, worktree};

    async fn setup() -> (WorktreeService, sqlx::SqlitePool) {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.run_migrations().await.unwrap();
        let db = Arc::new(db);
        let svc = WorktreeService::new(db.clone());
        (svc, db.pool.clone())
    }

    async fn setup_agent(pool: &sqlx::SqlitePool) -> String {
        let a = agent::create(pool, "Agent", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        a.id.to_string()
    }

    async fn setup_session(pool: &sqlx::SqlitePool, agent_id: &str) -> String {
        let s = session::create(pool, agent_id, None, None, "test")
            .await
            .unwrap();
        s.id.to_string()
    }

    #[tokio::test]
    async fn list_all_empty() {
        let (svc, _pool) = setup().await;
        let all = svc.list_all().await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn list_all_returns_worktrees() {
        let (svc, pool) = setup().await;
        let agent_id = setup_agent(&pool).await;
        let s1 = setup_session(&pool, &agent_id).await;
        let s2 = setup_session(&pool, &agent_id).await;
        worktree::create(&pool, &agent_id, &s1, "/repo", "/repo/wt1", "b1")
            .await
            .unwrap();
        worktree::create(&pool, &agent_id, &s2, "/repo", "/repo/wt2", "b2")
            .await
            .unwrap();
        let all = svc.list_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn cleanup_nonexistent_worktree_returns_error() {
        let (svc, _pool) = setup().await;
        let result = svc.cleanup("00000000-0000-0000-0000-000000000000").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Worktree not found"));
    }

    #[tokio::test]
    async fn cleanup_marks_db_deleted_even_when_git_removal_fails() {
        // Fix 3: cleanup() should always update DB status to Deleted,
        // even if the git worktree removal fails (e.g. path doesn't exist on disk).
        let (svc, pool) = setup().await;
        let agent_id = setup_agent(&pool).await;
        let session_id = setup_session(&pool, &agent_id).await;
        let wt = worktree::create(
            &pool, &agent_id, &session_id,
            "/nonexistent/repo", "/nonexistent/repo/wt", "branch",
        )
        .await
        .unwrap();
        let wt_id = wt.id.to_string();

        // cleanup should return error (git removal fails on nonexistent path)
        // but DB status should still be updated to Deleted
        let result = svc.cleanup(&wt_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to remove worktree"));

        // Verify DB status is Deleted despite the error
        let found = worktree::find_by_id(&pool, &wt_id).await.unwrap().unwrap();
        assert_eq!(found.status, WorktreeStatus::Deleted);
    }

    #[tokio::test]
    async fn worktree_status_partial_eq() {
        // Verify the PartialEq derive works correctly
        assert_eq!(WorktreeStatus::Active, WorktreeStatus::Active);
        assert_eq!(WorktreeStatus::Deleted, WorktreeStatus::Deleted);
        assert_ne!(WorktreeStatus::Active, WorktreeStatus::Deleted);
        assert_ne!(WorktreeStatus::Active, WorktreeStatus::Stale);
    }
}

// ---------------------------------------------------------------------------
// Workflow engine tests
// ---------------------------------------------------------------------------

mod workflow_tests {
    use super::*;
    use composer_db::models::{workflow, workflow_run, workflow_step_output, project, task};
    use composer_services::workflow_engine::{self, WorkflowEngine, FEAT_COMMON_NAME};

    async fn setup() -> (WorkflowEngine, Arc<Database>, EventBus) {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.run_migrations().await.unwrap();
        let db = Arc::new(db);
        let event_bus = EventBus::new();
        let pm = Arc::new(AgentProcessManager::new(event_bus.sender()));
        let session_service = SessionService::new(db.clone(), event_bus.clone(), pm);
        let engine = WorkflowEngine::new(db.clone(), event_bus.clone(), session_service);
        (engine, db, event_bus)
    }

    async fn create_project(pool: &sqlx::SqlitePool) -> String {
        let p = project::create(pool, "Test Project", None).await.unwrap();
        p.id.to_string()
    }

    async fn create_workflow_in_db(pool: &sqlx::SqlitePool, project_id: &str) -> String {
        let def = workflow_engine::feat_common_definition();
        let wf = workflow::create(pool, project_id, FEAT_COMMON_NAME, &def).await.unwrap();
        wf.id.to_string()
    }

    #[test]
    fn feat_common_definition_has_expected_steps() {
        let def = workflow_engine::feat_common_definition();
        assert_eq!(def.steps.len(), 7);
        assert_eq!(def.steps[0].step_type, WorkflowStepType::Plan);
        assert_eq!(def.steps[1].step_type, WorkflowStepType::HumanGate);
        assert_eq!(def.steps[2].step_type, WorkflowStepType::Implement);
        assert_eq!(def.steps[3].step_type, WorkflowStepType::PrReview);
        assert_eq!(def.steps[4].step_type, WorkflowStepType::Implement);
        assert_eq!(def.steps[5].step_type, WorkflowStepType::HumanReview);
        assert_eq!(def.steps[6].step_type, WorkflowStepType::Implement);
    }

    #[test]
    fn feat_common_step_names() {
        let def = workflow_engine::feat_common_definition();
        assert_eq!(def.steps[0].name, "Plan");
        assert_eq!(def.steps[1].name, "Review Plan");
        assert_eq!(def.steps[2].name, "Implement & Create PR");
        assert_eq!(def.steps[3].name, "Automated PR Review");
        assert_eq!(def.steps[4].name, "Fix Review Findings");
        assert_eq!(def.steps[5].name, "Human PR Review");
        assert_eq!(def.steps[6].name, "Fix Human Comments");
    }

    #[tokio::test]
    async fn ensure_builtin_workflow_creates_once() {
        let (engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;

        // First call creates
        let wf1 = engine.ensure_builtin_workflow(&project_id).await.unwrap();
        assert_eq!(wf1.name, FEAT_COMMON_NAME);

        // Second call returns same
        let wf2 = engine.ensure_builtin_workflow(&project_id).await.unwrap();
        assert_eq!(wf1.id, wf2.id);

        // Only one workflow exists
        let all = workflow::list_by_project(&db.pool, &project_id).await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn workflow_crud() {
        let (_engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;

        let def = WorkflowDefinition { steps: vec![
            WorkflowStepDefinition {
                step_type: WorkflowStepType::Plan,
                name: "Plan".to_string(),
                prompt_template: None,
                max_retries: None,
            },
        ]};

        let wf = workflow::create(&db.pool, &project_id, "Test WF", &def).await.unwrap();
        assert_eq!(wf.name, "Test WF");
        assert_eq!(wf.definition.steps.len(), 1);

        let found = workflow::find_by_id(&db.pool, &wf.id.to_string()).await.unwrap();
        assert!(found.is_some());

        let updated = workflow::update(&db.pool, &wf.id.to_string(), Some("Updated"), None).await.unwrap();
        assert_eq!(updated.name, "Updated");

        workflow::delete(&db.pool, &wf.id.to_string()).await.unwrap();
        let gone = workflow::find_by_id(&db.pool, &wf.id.to_string()).await.unwrap();
        assert!(gone.is_none());
    }

    #[tokio::test]
    async fn workflow_run_crud() {
        let (_engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool, &project_id).await;

        let task_obj = task::create(&db.pool, "Test Task", None, None, None, Some(&project_id), None).await.unwrap();
        let task_id = task_obj.id.to_string();

        let run = workflow_run::create(&db.pool, &wf_id, &task_id).await.unwrap();
        assert_eq!(run.status, WorkflowRunStatus::Running);
        assert_eq!(run.current_step_index, 0);
        assert_eq!(run.iteration_count, 0);
        assert!(run.main_session_id.is_none());

        // Update step index
        workflow_run::update_step_index(&db.pool, &run.id.to_string(), 3).await.unwrap();
        let updated = workflow_run::find_by_id(&db.pool, &run.id.to_string()).await.unwrap().unwrap();
        assert_eq!(updated.current_step_index, 3);

        // Update status
        workflow_run::update_status(&db.pool, &run.id.to_string(), &WorkflowRunStatus::Paused).await.unwrap();
        let updated = workflow_run::find_by_id(&db.pool, &run.id.to_string()).await.unwrap().unwrap();
        assert_eq!(updated.status, WorkflowRunStatus::Paused);

        // Increment iteration
        workflow_run::increment_iteration(&db.pool, &run.id.to_string()).await.unwrap();
        let updated = workflow_run::find_by_id(&db.pool, &run.id.to_string()).await.unwrap().unwrap();
        assert_eq!(updated.iteration_count, 1);

        // Find by task
        let by_task = workflow_run::find_by_task(&db.pool, &task_id).await.unwrap();
        assert!(by_task.is_some());
        assert_eq!(by_task.unwrap().id, run.id);
    }

    #[tokio::test]
    async fn workflow_step_output_crud() {
        let (_engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool, &project_id).await;
        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None).await.unwrap();
        let run = workflow_run::create(&db.pool, &wf_id, &task_obj.id.to_string()).await.unwrap();
        let run_id = run.id.to_string();

        // Create step output
        let step = workflow_step_output::create(
            &db.pool, &run_id, 0,
            &WorkflowStepType::Plan, &WorkflowStepStatus::Running, None,
        ).await.unwrap();
        assert_eq!(step.step_index, 0);
        assert_eq!(step.attempt, 1);
        assert_eq!(step.status, WorkflowStepStatus::Running);

        // Update status and output
        workflow_step_output::update_status_and_output(
            &db.pool, &step.id.to_string(), &WorkflowStepStatus::Completed, Some("Plan text here"),
        ).await.unwrap();
        let updated = workflow_step_output::find_by_id(&db.pool, &step.id.to_string()).await.unwrap().unwrap();
        assert_eq!(updated.status, WorkflowStepStatus::Completed);
        assert_eq!(updated.output.as_deref(), Some("Plan text here"));

        // Create another attempt for same step
        let step2 = workflow_step_output::create(
            &db.pool, &run_id, 0,
            &WorkflowStepType::Plan, &WorkflowStepStatus::Running, None,
        ).await.unwrap();
        assert_eq!(step2.attempt, 2);

        // latest_for_step returns the highest attempt
        let latest = workflow_step_output::latest_for_step(&db.pool, &run_id, 0).await.unwrap().unwrap();
        assert_eq!(latest.attempt, 2);

        // List all for the run
        let all = workflow_step_output::list_by_run(&db.pool, &run_id).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    use composer_db::models::{agent, session};

    async fn create_agent_and_session(pool: &sqlx::SqlitePool) -> String {
        let ag = agent::create(pool, "Test Agent", &AgentType::ClaudeCode, None).await.unwrap();
        let sess = session::create(pool, &ag.id.to_string(), None, None, "test").await.unwrap();
        sess.id.to_string()
    }

    #[tokio::test]
    async fn workflow_run_find_by_session() {
        let (_engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool, &project_id).await;
        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None).await.unwrap();
        let run = workflow_run::create(&db.pool, &wf_id, &task_obj.id.to_string()).await.unwrap();
        let run_id = run.id.to_string();

        // Create a real session for the FK
        let session_id = create_agent_and_session(&db.pool).await;
        workflow_run::update_main_session(&db.pool, &run_id, &session_id).await.unwrap();

        let found = workflow_run::find_by_session(&db.pool, &session_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, run.id);
    }

    #[tokio::test]
    async fn workflow_run_find_by_step_session() {
        let (_engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool, &project_id).await;
        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None).await.unwrap();
        let run = workflow_run::create(&db.pool, &wf_id, &task_obj.id.to_string()).await.unwrap();
        let run_id = run.id.to_string();

        // Create a real session for the FK, then reference it in a step output
        let review_session_id = create_agent_and_session(&db.pool).await;
        workflow_step_output::create(
            &db.pool, &run_id, 3,
            &WorkflowStepType::PrReview, &WorkflowStepStatus::Running, Some(&review_session_id),
        ).await.unwrap();

        let found = workflow_run::find_by_step_session(&db.pool, &review_session_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, run.id);
    }

    #[tokio::test]
    async fn startup_recovery_pauses_running_workflows() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.run_migrations().await.unwrap();
        let db = Arc::new(db);
        let event_bus = EventBus::new();

        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool, &project_id).await;
        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None).await.unwrap();
        let task_id = task_obj.id.to_string();

        // Simulate a running workflow run that was interrupted by server crash
        let run = workflow_run::create(&db.pool, &wf_id, &task_id).await.unwrap();
        let run_id = run.id.to_string();
        workflow_run::update_step_index(&db.pool, &run_id, 2).await.unwrap();

        // Create a running step output
        workflow_step_output::create(
            &db.pool, &run_id, 2,
            &WorkflowStepType::Implement, &WorkflowStepStatus::Running, None,
        ).await.unwrap();

        // Set task to in_progress
        task::update_status(&db.pool, &task_id, &TaskStatus::InProgress).await.unwrap();

        // Now create the WorkflowEngine — triggers startup recovery
        let pm = Arc::new(AgentProcessManager::new(event_bus.sender()));
        let session_service = SessionService::new(db.clone(), event_bus.clone(), pm);
        let _engine = WorkflowEngine::new(db.clone(), event_bus.clone(), session_service);

        // Give recovery time to run
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Workflow run should be paused
        let recovered = workflow_run::find_by_id(&db.pool, &run_id).await.unwrap().unwrap();
        assert_eq!(recovered.status, WorkflowRunStatus::Paused);

        // Task should be in waiting
        let task_after = task::find_by_id(&db.pool, &task_id).await.unwrap().unwrap();
        assert_eq!(task_after.status, TaskStatus::Waiting);

        // Step should be failed
        let step = workflow_step_output::latest_for_step(&db.pool, &run_id, 2).await.unwrap().unwrap();
        assert_eq!(step.status, WorkflowStepStatus::Failed);
    }

    #[tokio::test]
    async fn task_workflow_run_id_link() {
        let (_engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool, &project_id).await;
        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None).await.unwrap();
        let task_id = task_obj.id.to_string();

        assert!(task_obj.workflow_run_id.is_none());

        let run = workflow_run::create(&db.pool, &wf_id, &task_id).await.unwrap();
        task::update_workflow_run_id(&db.pool, &task_id, &run.id.to_string()).await.unwrap();

        let updated_task = task::find_by_id(&db.pool, &task_id).await.unwrap().unwrap();
        assert_eq!(updated_task.workflow_run_id, Some(run.id));
    }

    #[tokio::test]
    async fn workflow_run_find_running() {
        let (_engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool, &project_id).await;

        let t1 = task::create(&db.pool, "T1", None, None, None, Some(&project_id), None).await.unwrap();
        let t2 = task::create(&db.pool, "T2", None, None, None, Some(&project_id), None).await.unwrap();

        let run1 = workflow_run::create(&db.pool, &wf_id, &t1.id.to_string()).await.unwrap();
        let run2 = workflow_run::create(&db.pool, &wf_id, &t2.id.to_string()).await.unwrap();

        // run1 stays running, run2 is paused
        workflow_run::update_status(&db.pool, &run2.id.to_string(), &WorkflowRunStatus::Paused).await.unwrap();

        let running = workflow_run::find_running(&db.pool).await.unwrap();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].id, run1.id);
    }

    #[tokio::test]
    async fn submit_decision_on_non_paused_workflow_fails() {
        let (engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool, &project_id).await;
        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None).await.unwrap();

        let run = workflow_run::create(&db.pool, &wf_id, &task_obj.id.to_string()).await.unwrap();
        // Run is in "running" status, not "paused"

        let result = engine.submit_decision(&run.id.to_string(), true, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not paused"));
    }

    #[tokio::test]
    async fn start_workflow_on_non_backlog_task_fails() {
        let (engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool, &project_id).await;

        let task_obj = task::create(
            &db.pool, "Test", None, None, Some(&TaskStatus::InProgress), Some(&project_id), None,
        ).await.unwrap();

        let result = engine.start(&task_obj.id.to_string(), &wf_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("backlog"));
    }

    #[tokio::test]
    async fn start_workflow_without_agent_fails() {
        let (engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool, &project_id).await;

        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None).await.unwrap();
        // No agent assigned

        let result = engine.start(&task_obj.id.to_string(), &wf_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no assigned agent"));
    }
}
