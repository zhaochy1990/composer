use composer_api_types::*;
use composer_db::Database;
use composer_executors::process_manager::AgentProcessManager;
use composer_services::agent_service::AgentService;
use composer_services::event_bus::EventBus;
use composer_services::session_service::SessionService;
use composer_services::task_service::TaskService;
use composer_services::worktree_service::WorktreeService;
use std::sync::Arc;

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
            workflow_id: None,
            completed_at: None,
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
        bus.broadcast(WsEvent::TaskDeleted {
            task_id: uuid::Uuid::nil(),
        });
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
                workflow_id: None,
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
                workflow_id: None,
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
                workflow_id: None,
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
                    workflow_id: None,
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
                workflow_id: None,
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
                workflow_id: None,
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
            &db.pool,
            &agent_id,
            &session_id,
            "/nonexistent/repo",
            "/nonexistent/repo/wt",
            "branch",
        )
        .await
        .unwrap();
        let wt_id = wt.id.to_string();

        let sess = session::create_with_status(
            &db.pool, &session_id, &agent_id, None, Some(&wt_id),
            "do work", &SessionStatus::Running, None,
        )
        .await
        .unwrap();

        // Broadcast SessionCompleted event
        event_bus.broadcast(WsEvent::SessionCompleted {
            session_id: sess.id,
            result_summary: Some("done".to_string()),
            claude_session_id: None,
        });

        // Poll until the background event listener has processed (up to 2s)
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            let ag_after = agent::find_by_id(&db.pool, &agent_id).await.unwrap().unwrap();
            if ag_after.status == AgentStatus::Idle {
                break;
            }
            if std::time::Instant::now() > deadline {
                panic!("Timed out waiting for agent to reset to Idle");
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        // Poll until worktree is marked Deleted in DB (cleanup may be async)
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            let wt_after = worktree::find_by_id(&db.pool, &wt_id).await.unwrap().unwrap();
            if wt_after.status == WorktreeStatus::Deleted {
                break;
            }
            if std::time::Instant::now() > deadline {
                panic!("Timed out waiting for worktree to be marked Deleted");
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
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
            &db.pool,
            &agent_id,
            &session_id,
            "/nonexistent/repo",
            "/nonexistent/repo/wt",
            "branch",
        )
        .await
        .unwrap();
        let wt_id = wt.id.to_string();

        let sess = session::create_with_status(
            &db.pool, &session_id, &agent_id, None, Some(&wt_id),
            "do work", &SessionStatus::Running, None,
        )
        .await
        .unwrap();

        // Broadcast SessionFailed event
        event_bus.broadcast(WsEvent::SessionFailed {
            session_id: sess.id,
            error: "something broke".to_string(),
            claude_session_id: None,
        });

        // Poll until the background event listener has processed (up to 2s)
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            let ag_after = agent::find_by_id(&db.pool, &agent_id).await.unwrap().unwrap();
            if ag_after.status == AgentStatus::Idle {
                break;
            }
            if std::time::Instant::now() > deadline {
                panic!("Timed out waiting for agent to reset to Idle");
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        // Verify worktree is still Active (preserved for retry)
        let wt_after = worktree::find_by_id(&db.pool, &wt_id)
            .await
            .unwrap()
            .unwrap();
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
            "do work", &SessionStatus::Running, None,
        )
        .await
        .unwrap();

        // Try to retry a running session — should fail
        let result = _svc
            .retry_session(
                &sess.id.to_string(),
                ResumeSessionRequest {
                    prompt: None,
                    exit_on_result: false,
                    continue_chat: false,
                },
            )
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Only failed sessions can be retried"));
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
        let s = session::create(pool, agent_id, None, None, "test", None)
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Worktree not found"));
    }

    #[tokio::test]
    async fn cleanup_marks_db_deleted_even_when_git_removal_fails() {
        // Fix 3: cleanup() should always update DB status to Deleted,
        // even if the git worktree removal fails (e.g. path doesn't exist on disk).
        let (svc, pool) = setup().await;
        let agent_id = setup_agent(&pool).await;
        let session_id = setup_session(&pool, &agent_id).await;
        let wt = worktree::create(
            &pool,
            &agent_id,
            &session_id,
            "/nonexistent/repo",
            "/nonexistent/repo/wt",
            "branch",
        )
        .await
        .unwrap();
        let wt_id = wt.id.to_string();

        // cleanup should return error (git removal fails on nonexistent path)
        // but DB status should still be updated to Deleted
        let result = svc.cleanup(&wt_id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to remove worktree"));

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
    use composer_db::models::{project, task, workflow, workflow_run, workflow_step_output};
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

    async fn create_workflow_in_db(pool: &sqlx::SqlitePool) -> String {
        let def = workflow_engine::feat_common_definition();
        let wf = workflow::create(pool, FEAT_COMMON_NAME, &def).await.unwrap();
        wf.id.to_string()
    }

    fn find_step<'a>(def: &'a WorkflowDefinition, id: &str) -> &'a WorkflowStepDefinition {
        def.steps.iter().find(|s| s.id == id).unwrap()
    }

    #[test]
    fn feat_common_definition_has_expected_steps() {
        let def = workflow_engine::feat_common_definition();
        assert_eq!(def.steps.len(), 7);
        assert_eq!(find_step(&def, "plan").step_type, WorkflowStepType::Agentic);
        assert_eq!(find_step(&def, "plan").session_mode, Some(SessionMode::New));
        assert_eq!(find_step(&def, "review_plan").step_type, WorkflowStepType::HumanGate);
        assert_eq!(find_step(&def, "implement").step_type, WorkflowStepType::Agentic);
        assert_eq!(find_step(&def, "implement").session_mode, Some(SessionMode::Resume));
        assert_eq!(find_step(&def, "auto_review").step_type, WorkflowStepType::Agentic);
        assert_eq!(find_step(&def, "auto_review").session_mode, Some(SessionMode::Separate));
        assert_eq!(find_step(&def, "fix_review").step_type, WorkflowStepType::Agentic);
        assert_eq!(find_step(&def, "human_review").step_type, WorkflowStepType::HumanGate);
        assert_eq!(find_step(&def, "complete_pr").step_type, WorkflowStepType::Agentic);
    }

    #[test]
    fn feat_common_step_names() {
        let def = workflow_engine::feat_common_definition();
        assert_eq!(find_step(&def, "plan").name, "Plan");
        assert_eq!(find_step(&def, "review_plan").name, "Review Plan");
        assert_eq!(find_step(&def, "implement").name, "Implement & Create PR");
        assert_eq!(find_step(&def, "auto_review").name, "Automated PR Review");
        assert_eq!(find_step(&def, "fix_review").name, "Fix Review Findings");
        assert_eq!(find_step(&def, "human_review").name, "Human PR Review");
        assert_eq!(find_step(&def, "complete_pr").name, "Complete PR");
    }

    #[tokio::test]
    async fn ensure_builtin_workflow_creates_once() {
        let (engine, db, _) = setup().await;

        // Wait for startup seeding to complete
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let wf1 = engine.ensure_builtin_workflow().await.unwrap();
        assert_eq!(wf1.name, FEAT_COMMON_NAME);
        assert!(wf1.is_template);

        let wf2 = engine.ensure_builtin_workflow().await.unwrap();
        assert_eq!(wf1.id, wf2.id);

        let all = workflow::list_all(&db.pool).await.unwrap();
        let feat_common_count = all.iter().filter(|w| w.name == FEAT_COMMON_NAME).count();
        assert_eq!(feat_common_count, 1);
    }

    #[tokio::test]
    async fn workflow_crud() {
        let (_engine, db, _) = setup().await;

        let def = WorkflowDefinition {
            steps: vec![WorkflowStepDefinition {
                id: "plan".to_string(),
                step_type: WorkflowStepType::Agentic,
                name: "Plan".to_string(),
                prompt_template: Some("{{task}}\n\nCreate a plan.".to_string()),
                depends_on: vec![],
                on_approve: None,
                on_reject: None,
                max_retries: None,
                loop_back_to: None,
                session_mode: Some(SessionMode::New),
                interactive: None,
            }],
        };

        let wf = workflow::create(&db.pool, "Test WF", &def).await.unwrap();
        assert_eq!(wf.name, "Test WF");
        assert!(!wf.is_template);
        assert_eq!(wf.definition.steps.len(), 1);

        let found = workflow::find_by_id(&db.pool, &wf.id.to_string())
            .await
            .unwrap();
        assert!(found.is_some());

        let updated = workflow::update(&db.pool, &wf.id.to_string(), Some("Updated"), None)
            .await
            .unwrap();
        assert_eq!(updated.name, "Updated");

        workflow::delete(&db.pool, &wf.id.to_string())
            .await
            .unwrap();
        let gone = workflow::find_by_id(&db.pool, &wf.id.to_string())
            .await
            .unwrap();
        assert!(gone.is_none());
    }

    #[tokio::test]
    async fn workflow_clone() {
        let (_engine, db, _) = setup().await;

        let def = workflow_engine::feat_common_definition();
        let template = workflow::create_with_template(&db.pool, "Template", &def, true).await.unwrap();
        assert!(template.is_template);

        let cloned = workflow::clone_workflow(&db.pool, &template.id.to_string(), "My Copy").await.unwrap();
        assert!(!cloned.is_template);
        assert_eq!(cloned.name, "My Copy");
        assert_eq!(cloned.definition, template.definition);
        assert_ne!(cloned.id, template.id);
    }

    #[tokio::test]
    async fn workflow_run_crud() {
        let (_engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool).await;

        let task_obj = task::create(&db.pool, "Test Task", None, None, None, Some(&project_id), None, None)
            .await
            .unwrap();
        let task_id = task_obj.id.to_string();

        let run = workflow_run::create(&db.pool, &wf_id, &task_id)
            .await
            .unwrap();
        assert_eq!(run.status, WorkflowRunStatus::Running);
        assert_eq!(run.iteration_count, 0);
        assert!(run.activated_steps.is_empty());

        // Add activated step
        workflow_run::add_activated_step(&db.pool, &run.id.to_string(), "implement").await.unwrap();
        let updated = workflow_run::find_by_id(&db.pool, &run.id.to_string()).await.unwrap().unwrap();
        assert_eq!(updated.activated_steps, vec!["implement".to_string()]);

        // Update status
        workflow_run::update_status(&db.pool, &run.id.to_string(), &WorkflowRunStatus::Paused)
            .await
            .unwrap();
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
        let wf_id = create_workflow_in_db(&db.pool).await;
        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None, None).await.unwrap();
        let run = workflow_run::create(&db.pool, &wf_id, &task_obj.id.to_string()).await.unwrap();
        let run_id = run.id.to_string();

        // Create step output
        let step = workflow_step_output::create(
            &db.pool, &run_id, "plan", &WorkflowStepType::Agentic,
            &WorkflowStepStatus::Running, None,
        ).await.unwrap();
        assert_eq!(step.step_id, "plan");
        assert_eq!(step.attempt, 1);
        assert_eq!(step.status, WorkflowStepStatus::Running);

        // Update status and output
        workflow_step_output::update_status_and_output(
            &db.pool, &step.id.to_string(),
            &WorkflowStepStatus::Completed, Some("Plan text here"),
        ).await.unwrap();
        let updated = workflow_step_output::find_by_id(&db.pool, &step.id.to_string()).await.unwrap().unwrap();
        assert_eq!(updated.status, WorkflowStepStatus::Completed);
        assert_eq!(updated.output.as_deref(), Some("Plan text here"));

        // Create another attempt for same step
        let step2 = workflow_step_output::create(
            &db.pool, &run_id, "plan", &WorkflowStepType::Agentic,
            &WorkflowStepStatus::Running, None,
        ).await.unwrap();
        assert_eq!(step2.attempt, 2);

        // latest_for_step returns the highest attempt
        let latest = workflow_step_output::latest_for_step(&db.pool, &run_id, "plan").await.unwrap().unwrap();
        assert_eq!(latest.attempt, 2);

        // List all for the run
        let all = workflow_step_output::list_by_run(&db.pool, &run_id).await.unwrap();
        assert_eq!(all.len(), 2);

        // Find completed step IDs
        workflow_step_output::update_status(&db.pool, &step.id.to_string(), &WorkflowStepStatus::Completed).await.unwrap();
        let completed = workflow_step_output::find_completed_step_ids(&db.pool, &run_id).await.unwrap();
        assert!(completed.contains(&"plan".to_string()));
    }

    use composer_db::models::{agent, session};

    async fn create_agent_and_session(pool: &sqlx::SqlitePool) -> String {
        let ag = agent::create(pool, "Test Agent", &AgentType::ClaudeCode, None).await.unwrap();
        let sess = session::create(pool, &ag.id.to_string(), None, None, "test", None).await.unwrap();
        sess.id.to_string()
    }

    #[tokio::test]
    async fn workflow_run_find_by_step_session() {
        let (_engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool).await;
        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None, None).await.unwrap();
        let run = workflow_run::create(&db.pool, &wf_id, &task_obj.id.to_string()).await.unwrap();
        let run_id = run.id.to_string();

        let review_session_id = create_agent_and_session(&db.pool).await;
        workflow_step_output::create(
            &db.pool, &run_id, "auto_review", &WorkflowStepType::Agentic,
            &WorkflowStepStatus::Running, Some(&review_session_id),
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
        let wf_id = create_workflow_in_db(&db.pool).await;
        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None, None).await.unwrap();
        let task_id = task_obj.id.to_string();

        let run = workflow_run::create(&db.pool, &wf_id, &task_id).await.unwrap();
        let run_id = run.id.to_string();

        // Create a running step output
        workflow_step_output::create(
            &db.pool, &run_id, "implement", &WorkflowStepType::Agentic,
            &WorkflowStepStatus::Running, None,
        ).await.unwrap();

        task::update_status(&db.pool, &task_id, &TaskStatus::InProgress).await.unwrap();

        // Create the WorkflowEngine — triggers startup recovery
        let pm = Arc::new(AgentProcessManager::new(event_bus.sender()));
        let session_service = SessionService::new(db.clone(), event_bus.clone(), pm);
        let _engine = WorkflowEngine::new(db.clone(), event_bus.clone(), session_service);

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let recovered = workflow_run::find_by_id(&db.pool, &run_id).await.unwrap().unwrap();
        assert_eq!(recovered.status, WorkflowRunStatus::Paused);

        let task_after = task::find_by_id(&db.pool, &task_id).await.unwrap().unwrap();
        assert_eq!(task_after.status, TaskStatus::Waiting);

        let step = workflow_step_output::latest_for_step(&db.pool, &run_id, "implement").await.unwrap().unwrap();
        assert_eq!(step.status, WorkflowStepStatus::Failed);
    }

    #[tokio::test]
    async fn task_workflow_run_id_link() {
        let (_engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool).await;
        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None, None).await.unwrap();
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
        let wf_id = create_workflow_in_db(&db.pool).await;

        let t1 = task::create(&db.pool, "T1", None, None, None, Some(&project_id), None, None).await.unwrap();
        let t2 = task::create(&db.pool, "T2", None, None, None, Some(&project_id), None, None).await.unwrap();

        let run1 = workflow_run::create(&db.pool, &wf_id, &t1.id.to_string()).await.unwrap();
        let run2 = workflow_run::create(&db.pool, &wf_id, &t2.id.to_string()).await.unwrap();

        workflow_run::update_status(&db.pool, &run2.id.to_string(), &WorkflowRunStatus::Paused).await.unwrap();

        let running = workflow_run::find_running(&db.pool).await.unwrap();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].id, run1.id);
    }

    #[tokio::test]
    async fn submit_decision_on_non_paused_workflow_fails() {
        let (engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool).await;
        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None, None).await.unwrap();

        let run = workflow_run::create(&db.pool, &wf_id, &task_obj.id.to_string()).await.unwrap();

        let result = engine
            .submit_decision(&run.id.to_string(), "review_plan", true, None)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not paused"));
    }

    #[tokio::test]
    async fn start_workflow_on_non_backlog_task_fails() {
        let (engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool).await;

        let task_obj = task::create(
            &db.pool, "Test", None, None, Some(&TaskStatus::InProgress),
            Some(&project_id), None, None,
        ).await.unwrap();

        let result = engine.start(&task_obj.id.to_string(), &wf_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("backlog"));
    }

    #[tokio::test]
    async fn start_workflow_without_agent_fails() {
        let (engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool).await;

        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None, None).await.unwrap();

        let result = engine.start(&task_obj.id.to_string(), &wf_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no assigned agent"));
    }

    #[test]
    fn feat_common_dag_structure() {
        let def = workflow_engine::feat_common_definition();

        // plan has no dependencies (entry step)
        let plan = find_step(&def, "plan");
        assert!(plan.depends_on.is_empty());
        assert_eq!(plan.loop_back_to, None);

        // review_plan depends on plan, branches
        let review = find_step(&def, "review_plan");
        assert_eq!(review.depends_on, vec!["plan"]);
        assert_eq!(review.on_approve, Some("implement".to_string()));
        assert_eq!(review.on_reject, Some("plan".to_string()));

        // implement depends on review_plan
        let implement = find_step(&def, "implement");
        assert_eq!(implement.depends_on, vec!["review_plan"]);

        // fix_review loops back to auto_review with max 3 retries
        let fix = find_step(&def, "fix_review");
        assert_eq!(fix.loop_back_to, Some("auto_review".to_string()));
        assert_eq!(fix.max_retries, Some(3));

        // human_review branches to complete_pr or implement
        let hr = find_step(&def, "human_review");
        assert_eq!(hr.on_approve, Some("complete_pr".to_string()));
        assert_eq!(hr.on_reject, Some("implement".to_string()));

        // complete_pr depends on human_review
        let cpr = find_step(&def, "complete_pr");
        assert_eq!(cpr.depends_on, vec!["human_review"]);
    }

    #[test]
    fn validate_dag_accepts_valid() {
        let def = workflow_engine::feat_common_definition();
        assert!(workflow_engine::validate_dag(&def).is_ok());
    }

    #[test]
    fn validate_dag_rejects_missing_reference() {
        let def = WorkflowDefinition {
            steps: vec![
                WorkflowStepDefinition {
                    id: "plan".to_string(),
                    step_type: WorkflowStepType::Agentic,
                    name: "Plan".to_string(),
                    prompt_template: Some("{{task}}\n\nCreate a plan.".to_string()),
                    depends_on: vec!["nonexistent".to_string()],
                    on_approve: None,
                    on_reject: None,
                    max_retries: None,
                    loop_back_to: None,
                    session_mode: Some(SessionMode::New),
                    interactive: None,
                },
            ],
        };
        let result = workflow_engine::validate_dag(&def);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("non-existent")));
    }

    #[test]
    fn validate_dag_rejects_duplicate_ids() {
        let def = WorkflowDefinition {
            steps: vec![
                WorkflowStepDefinition {
                    id: "plan".to_string(),
                    step_type: WorkflowStepType::Agentic,
                    name: "Plan 1".to_string(),
                    prompt_template: Some("{{task}}".to_string()),
                    depends_on: vec![],
                    on_approve: None,
                    on_reject: None,
                    max_retries: None,
                    loop_back_to: None,
                    session_mode: Some(SessionMode::New),
                    interactive: None,
                },
                WorkflowStepDefinition {
                    id: "plan".to_string(),
                    step_type: WorkflowStepType::Agentic,
                    name: "Plan 2".to_string(),
                    prompt_template: Some("{{task}}".to_string()),
                    depends_on: vec![],
                    on_approve: None,
                    on_reject: None,
                    max_retries: None,
                    loop_back_to: None,
                    session_mode: Some(SessionMode::Resume),
                    interactive: None,
                },
            ],
        };
        let result = workflow_engine::validate_dag(&def);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("Duplicate")));
    }

    #[test]
    fn validate_dag_rejects_cycle() {
        let def = WorkflowDefinition {
            steps: vec![
                WorkflowStepDefinition {
                    id: "a".to_string(),
                    step_type: WorkflowStepType::Agentic,
                    name: "A".to_string(),
                    prompt_template: Some("{{task}}".to_string()),
                    depends_on: vec!["b".to_string()],
                    on_approve: None,
                    on_reject: None,
                    max_retries: None,
                    loop_back_to: None,
                    session_mode: Some(SessionMode::New),
                    interactive: None,
                },
                WorkflowStepDefinition {
                    id: "b".to_string(),
                    step_type: WorkflowStepType::Agentic,
                    name: "B".to_string(),
                    prompt_template: Some("{{task}}".to_string()),
                    depends_on: vec!["a".to_string()],
                    on_approve: None,
                    on_reject: None,
                    max_retries: None,
                    loop_back_to: None,
                    session_mode: Some(SessionMode::Resume),
                    interactive: None,
                },
            ],
        };
        let result = workflow_engine::validate_dag(&def);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("cycle")));
    }

    /// Routing cycles via on_approve/on_reject are intentionally allowed (for reject→redo loops),
    /// but a mutual on_approve cycle between two HumanGate steps with no `depends_on` links
    /// would still pass the depends_on-only Kahn's sort. This test documents that on_approve/on_reject
    /// back-edges are legitimate and don't fail validation, since runtime guards (max_retries,
    /// activation tracking) prevent infinite loops.
    #[test]
    fn validate_dag_allows_on_approve_back_edges() {
        // This mimics the Feat-Common pattern: review_plan rejects back to plan
        let def = WorkflowDefinition {
            steps: vec![
                WorkflowStepDefinition {
                    id: "plan".to_string(),
                    step_type: WorkflowStepType::Agentic,
                    name: "Plan".to_string(),
                    prompt_template: Some("{{task}}".to_string()),
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
                    on_reject: Some("plan".to_string()), // back-edge to plan
                    max_retries: None,
                    loop_back_to: None,
                    session_mode: None,
                    interactive: None,
                },
                WorkflowStepDefinition {
                    id: "implement".to_string(),
                    step_type: WorkflowStepType::Agentic,
                    name: "Implement".to_string(),
                    prompt_template: Some("{{task}}".to_string()),
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
        // This should pass — on_reject back-edges are intentional
        let result = workflow_engine::validate_dag(&def);
        assert!(result.is_ok(), "on_approve/on_reject back-edges should be allowed: {:?}", result);
    }

    #[test]
    fn validate_dag_rejects_missing_human_gate_on_approve() {
        let def = WorkflowDefinition {
            steps: vec![
                WorkflowStepDefinition {
                    id: "plan".to_string(),
                    step_type: WorkflowStepType::Agentic,
                    name: "Plan".to_string(),
                    prompt_template: Some("{{task}}".to_string()),
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
                    on_approve: None, // Missing!
                    on_reject: None,
                    max_retries: None,
                    loop_back_to: None,
                    session_mode: None,
                    interactive: None,
                },
            ],
        };
        let result = workflow_engine::validate_dag(&def);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("on_approve")));
    }

    #[tokio::test]
    async fn should_loop_stops_after_max_retries() {
        let (engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;
        let wf_id = create_workflow_in_db(&db.pool).await;
        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None, None).await.unwrap();
        let run = workflow_run::create(&db.pool, &wf_id, &task_obj.id.to_string()).await.unwrap();
        let run_id = run.id.to_string();

        let wf = workflow::find_by_id(&db.pool, &wf_id).await.unwrap().unwrap();

        let step_def = find_step(&wf.definition, "fix_review");
        assert_eq!(step_def.loop_back_to, Some("auto_review".to_string()));
        assert_eq!(step_def.max_retries, Some(3));

        // No completions of target step yet → should NOT loop (crash recovery guard)
        let result = engine.should_loop(&run_id, step_def, "auto_review").await.unwrap();
        assert!(!result, "should not loop when target has no completions");

        // Simulate completions
        for _ in 0..3 {
            workflow_step_output::create(
                &db.pool, &run_id, "auto_review", &WorkflowStepType::Agentic,
                &WorkflowStepStatus::Completed, None,
            ).await.unwrap();
        }
        let result = engine.should_loop(&run_id, step_def, "auto_review").await.unwrap();
        assert!(result, "should loop after 3 completions (2 retries done, max is 3)");

        // 4th completion → should NOT loop
        workflow_step_output::create(
            &db.pool, &run_id, "auto_review", &WorkflowStepType::Agentic,
            &WorkflowStepStatus::Completed, None,
        ).await.unwrap();
        let result = engine.should_loop(&run_id, step_def, "auto_review").await.unwrap();
        assert!(!result, "should stop looping after 4 completions (3 retries = max)");
    }

    #[tokio::test]
    async fn start_workflow_rejects_invalid_dag() {
        let (engine, db, _) = setup().await;
        let project_id = create_project(&db.pool).await;

        let bad_def = WorkflowDefinition {
            steps: vec![
                WorkflowStepDefinition {
                    id: "plan".to_string(),
                    step_type: WorkflowStepType::Agentic,
                    name: "Plan".to_string(),
                    prompt_template: Some("{{task}}".to_string()),
                    depends_on: vec!["nonexistent".to_string()],
                    on_approve: None,
                    on_reject: None,
                    max_retries: None,
                    loop_back_to: None,
                    session_mode: Some(SessionMode::New),
                    interactive: None,
                },
            ],
        };
        let wf = workflow::create(&db.pool, "Bad WF", &bad_def).await.unwrap();

        let ag = agent::create(&db.pool, "Agent", &AgentType::ClaudeCode, None).await.unwrap();
        let task_obj = task::create(&db.pool, "Test", None, None, None, Some(&project_id), None, None).await.unwrap();
        task::update_assigned_agent(&db.pool, &task_obj.id.to_string(), Some(&ag.id.to_string())).await.unwrap();

        let result = engine.start(&task_obj.id.to_string(), &wf.id.to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-existent"));
    }
}
