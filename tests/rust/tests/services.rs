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
            repo_path: None,
            auto_approve: true,
            position: 1.0,
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
                assigned_agent_id: None,
                repo_path: None,
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
                assigned_agent_id: None,
                repo_path: None,
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
                assigned_agent_id: None,
                repo_path: None,
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
                    assigned_agent_id: None,
                    repo_path: None,
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
                assigned_agent_id: None,
                repo_path: None,
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
                assigned_agent_id: None,
                repo_path: None,
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
    async fn session_failed_resets_agent_and_cleans_worktree() {
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
