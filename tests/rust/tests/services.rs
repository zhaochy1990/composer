use std::sync::Arc;
use composer_api_types::*;
use composer_db::Database;
use composer_services::event_bus::EventBus;
use composer_services::task_service::TaskService;
use composer_services::agent_service::AgentService;
use composer_services::session_service::SessionService;
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
        let svc = TaskService::new(Arc::new(db), event_bus);
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

    async fn setup() -> SessionService {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.run_migrations().await.unwrap();
        let event_bus = EventBus::new();
        let pm = Arc::new(AgentProcessManager::new(event_bus.sender()));
        SessionService::new(Arc::new(db), event_bus, pm)
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
}
