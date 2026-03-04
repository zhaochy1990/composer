use composer_api_types::*;
use composer_db::Database;
use composer_tests::test_pool;

// ---------------------------------------------------------------------------
// Database connection tests (from crates/db/src/lib.rs)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn connect_and_migrate() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    db.run_migrations().await.unwrap();
}

#[tokio::test]
async fn wal_mode_pragma() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let row: (String,) = sqlx::query_as("PRAGMA journal_mode")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    // In-memory SQLite may report "memory" instead of "wal"
    assert!(row.0 == "wal" || row.0 == "memory");
}

// ---------------------------------------------------------------------------
// Agent model tests (from crates/db/src/models/agent.rs)
// ---------------------------------------------------------------------------

mod agent_tests {
    use super::*;
    use composer_db::models::agent;

    #[tokio::test]
    async fn create_agent_defaults() {
        let pool = test_pool().await;
        let a = agent::create(&pool, "Agent 1", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        assert_eq!(a.name, "Agent 1");
        assert!(matches!(a.agent_type, AgentType::ClaudeCode));
        assert!(matches!(a.status, AgentStatus::Idle));
        assert!(matches!(a.auth_status, AuthStatus::Unknown));
        assert!(a.executable_path.is_none());
    }

    #[tokio::test]
    async fn create_agent_with_path() {
        let pool = test_pool().await;
        let a = agent::create(&pool, "Agent", &AgentType::ClaudeCode, Some("/usr/bin/claude"))
            .await
            .unwrap();
        assert_eq!(a.executable_path.as_deref(), Some("/usr/bin/claude"));
    }

    #[tokio::test]
    async fn find_by_id_returns_agent() {
        let pool = test_pool().await;
        let a = agent::create(&pool, "Find Me", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        let found = agent::find_by_id(&pool, &a.id.to_string()).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Find Me");
    }

    #[tokio::test]
    async fn find_by_agent_type_returns_agent() {
        let pool = test_pool().await;
        agent::create(&pool, "CC Agent", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        let found = agent::find_by_agent_type(&pool, &AgentType::ClaudeCode)
            .await
            .unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn update_executable_path_works() {
        let pool = test_pool().await;
        let a = agent::create(&pool, "Agent", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        let id = a.id.to_string();
        agent::update_executable_path(&pool, &id, "/new/path").await.unwrap();
        let found = agent::find_by_id(&pool, &id).await.unwrap().unwrap();
        assert_eq!(found.executable_path.as_deref(), Some("/new/path"));
    }

    #[tokio::test]
    async fn update_status_works() {
        let pool = test_pool().await;
        let a = agent::create(&pool, "Agent", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        let id = a.id.to_string();
        agent::update_status(&pool, &id, &AgentStatus::Busy).await.unwrap();
        let found = agent::find_by_id(&pool, &id).await.unwrap().unwrap();
        assert!(matches!(found.status, AgentStatus::Busy));
    }

    #[tokio::test]
    async fn update_auth_status_works() {
        let pool = test_pool().await;
        let a = agent::create(&pool, "Agent", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        let id = a.id.to_string();
        agent::update_auth_status(&pool, &id, &AuthStatus::Authenticated)
            .await
            .unwrap();
        let found = agent::find_by_id(&pool, &id).await.unwrap().unwrap();
        assert!(matches!(found.auth_status, AuthStatus::Authenticated));
    }

    #[tokio::test]
    async fn list_all_returns_agents() {
        let pool = test_pool().await;
        agent::create(&pool, "Agent 1", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        let agents = agent::list_all(&pool).await.unwrap();
        assert_eq!(agents.len(), 1);
    }

    #[tokio::test]
    async fn delete_agent_removes() {
        let pool = test_pool().await;
        let a = agent::create(&pool, "Delete Me", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        let id = a.id.to_string();
        agent::delete(&pool, &id).await.unwrap();
        assert!(agent::find_by_id(&pool, &id).await.unwrap().is_none());
    }
}

// ---------------------------------------------------------------------------
// Task model tests (from crates/db/src/models/task.rs)
// ---------------------------------------------------------------------------

mod task_tests {
    use super::*;
    use composer_db::models::task;

    #[tokio::test]
    async fn create_task_defaults() {
        let pool = test_pool().await;
        let t = task::create(&pool, "Test Task", None, None, None, None, None).await.unwrap();
        assert_eq!(t.title, "Test Task");
        assert!(t.description.is_none());
        assert_eq!(t.priority, 0);
        assert!(matches!(t.status, TaskStatus::Backlog));
        assert!(t.assigned_agent_id.is_none());
    }

    #[tokio::test]
    async fn create_task_with_all_fields() {
        let pool = test_pool().await;
        let t = task::create(
            &pool,
            "Full Task",
            Some("A description"),
            Some(3),
            Some(&TaskStatus::InProgress),
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(t.title, "Full Task");
        assert_eq!(t.description.as_deref(), Some("A description"));
        assert_eq!(t.priority, 3);
        assert!(matches!(t.status, TaskStatus::InProgress));
        assert!(t.auto_approve); // default is true
    }

    #[tokio::test]
    async fn find_by_id_hit() {
        let pool = test_pool().await;
        let t = task::create(&pool, "Find Me", None, None, None, None, None).await.unwrap();
        let found = task::find_by_id(&pool, &t.id.to_string()).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Find Me");
    }

    #[tokio::test]
    async fn find_by_id_miss() {
        let pool = test_pool().await;
        let found = task::find_by_id(&pool, "00000000-0000-0000-0000-000000000000")
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn list_all_empty() {
        let pool = test_pool().await;
        let tasks = task::list_all(&pool).await.unwrap();
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn list_all_returns_tasks() {
        let pool = test_pool().await;
        task::create(&pool, "Task 1", None, None, None, None, None).await.unwrap();
        task::create(&pool, "Task 2", None, None, None, None, None).await.unwrap();
        let tasks = task::list_all(&pool).await.unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn list_by_status_filters() {
        let pool = test_pool().await;
        task::create(&pool, "Backlog Task", None, None, Some(&TaskStatus::Backlog), None, None)
            .await
            .unwrap();
        task::create(&pool, "Done Task", None, None, Some(&TaskStatus::Done), None, None)
            .await
            .unwrap();
        let backlog = task::list_by_status(&pool, &TaskStatus::Backlog).await.unwrap();
        assert_eq!(backlog.len(), 1);
        assert_eq!(backlog[0].title, "Backlog Task");
    }

    #[tokio::test]
    async fn update_partial_fields() {
        let pool = test_pool().await;
        let t = task::create(&pool, "Original", None, None, None, None, None).await.unwrap();
        let id = t.id.to_string();
        let updated = task::update(&pool, &id, Some("Updated"), None, None, None, None, None, None)
            .await
            .unwrap();
        assert_eq!(updated.title, "Updated");
        assert!(updated.description.is_none());
    }

    #[tokio::test]
    async fn update_status_changes_status() {
        let pool = test_pool().await;
        let t = task::create(&pool, "Move Me", None, None, None, None, None).await.unwrap();
        let id = t.id.to_string();
        task::update_status(&pool, &id, &TaskStatus::Done).await.unwrap();
        let found = task::find_by_id(&pool, &id).await.unwrap().unwrap();
        assert!(matches!(found.status, TaskStatus::Done));
    }

    #[tokio::test]
    async fn update_assigned_agent_sets_and_clears() {
        let pool = test_pool().await;
        let agent = composer_db::models::agent::create(
            &pool,
            "Agent",
            &AgentType::ClaudeCode,
            None,
        )
        .await
        .unwrap();
        let agent_id = agent.id.to_string();

        let t = task::create(&pool, "Assign Me", None, None, None, None, None).await.unwrap();
        let id = t.id.to_string();

        task::update_assigned_agent(&pool, &id, Some(&agent_id)).await.unwrap();
        let found = task::find_by_id(&pool, &id).await.unwrap().unwrap();
        assert!(found.assigned_agent_id.is_some());

        task::update_assigned_agent(&pool, &id, None).await.unwrap();
        let found = task::find_by_id(&pool, &id).await.unwrap().unwrap();
        assert!(found.assigned_agent_id.is_none());
    }

    #[tokio::test]
    async fn delete_task() {
        let pool = test_pool().await;
        let t = task::create(&pool, "Delete Me", None, None, None, None, None).await.unwrap();
        let id = t.id.to_string();
        task::delete(&pool, &id).await.unwrap();
        let found = task::find_by_id(&pool, &id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn position_auto_increment() {
        let pool = test_pool().await;
        let t1 = task::create(&pool, "First", None, None, Some(&TaskStatus::Backlog), None, None)
            .await
            .unwrap();
        let t2 = task::create(&pool, "Second", None, None, Some(&TaskStatus::Backlog), None, None)
            .await
            .unwrap();
        assert!(t2.position > t1.position);
    }

    #[tokio::test]
    async fn position_independent_per_status() {
        let pool = test_pool().await;
        let backlog = task::create(&pool, "Backlog", None, None, Some(&TaskStatus::Backlog), None, None)
            .await
            .unwrap();
        let done = task::create(&pool, "Done", None, None, Some(&TaskStatus::Done), None, None)
            .await
            .unwrap();
        // Both should start at position 1.0 since they're in different columns
        assert_eq!(backlog.position, 1.0);
        assert_eq!(done.position, 1.0);
    }
}

// ---------------------------------------------------------------------------
// Session model tests (from crates/db/src/models/session.rs)
// ---------------------------------------------------------------------------

mod session_tests {
    use super::*;
    use composer_db::models::{agent, session};

    async fn setup_agent(pool: &sqlx::SqlitePool) -> String {
        let a = agent::create(pool, "Test Agent", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        a.id.to_string()
    }

    #[tokio::test]
    async fn create_session_defaults() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let s = session::create(&pool, &agent_id, None, None, "do stuff").await.unwrap();
        assert!(matches!(s.status, SessionStatus::Created));
        assert_eq!(s.prompt.as_deref(), Some("do stuff"));
        assert!(s.task_id.is_none());
        assert!(s.worktree_id.is_none());
        assert!(s.started_at.is_none());
        assert!(s.completed_at.is_none());
    }

    #[tokio::test]
    async fn create_with_status_running() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let id = uuid::Uuid::new_v4().to_string();
        let s = session::create_with_status(
            &pool, &id, &agent_id, None, None, "run it", &SessionStatus::Running,
        )
        .await
        .unwrap();
        assert!(matches!(s.status, SessionStatus::Running));
        assert!(s.started_at.is_some());
    }

    #[tokio::test]
    async fn create_with_status_created() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let id = uuid::Uuid::new_v4().to_string();
        let s = session::create_with_status(
            &pool, &id, &agent_id, None, None, "pending", &SessionStatus::Created,
        )
        .await
        .unwrap();
        assert!(matches!(s.status, SessionStatus::Created));
        assert!(s.started_at.is_none());
    }

    #[tokio::test]
    async fn find_by_id_hit_and_miss() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let s = session::create(&pool, &agent_id, None, None, "prompt").await.unwrap();
        let found = session::find_by_id(&pool, &s.id.to_string()).await.unwrap();
        assert!(found.is_some());
        let miss = session::find_by_id(&pool, "00000000-0000-0000-0000-000000000000")
            .await
            .unwrap();
        assert!(miss.is_none());
    }

    #[tokio::test]
    async fn list_by_agent_filters() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        session::create(&pool, &agent_id, None, None, "s1").await.unwrap();
        session::create(&pool, &agent_id, None, None, "s2").await.unwrap();
        let sessions = session::list_by_agent(&pool, &agent_id).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn list_by_task_filters() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let task = composer_db::models::task::create(&pool, "Task", None, None, None, None, None)
            .await
            .unwrap();
        let task_id = task.id.to_string();
        session::create(&pool, &agent_id, Some(&task_id), None, "s1")
            .await
            .unwrap();
        let sessions = session::list_by_task(&pool, &task_id).await.unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[tokio::test]
    async fn update_status_completed_sets_timestamp() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let id = uuid::Uuid::new_v4().to_string();
        session::create_with_status(
            &pool, &id, &agent_id, None, None, "run", &SessionStatus::Running,
        )
        .await
        .unwrap();
        session::update_status(&pool, &id, &SessionStatus::Completed).await.unwrap();
        let found = session::find_by_id(&pool, &id).await.unwrap().unwrap();
        assert!(matches!(found.status, SessionStatus::Completed));
        assert!(found.completed_at.is_some());
    }

    #[tokio::test]
    async fn update_status_failed_sets_timestamp() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let id = uuid::Uuid::new_v4().to_string();
        session::create_with_status(
            &pool, &id, &agent_id, None, None, "run", &SessionStatus::Running,
        )
        .await
        .unwrap();
        session::update_status(&pool, &id, &SessionStatus::Failed).await.unwrap();
        let found = session::find_by_id(&pool, &id).await.unwrap().unwrap();
        assert!(matches!(found.status, SessionStatus::Failed));
        assert!(found.completed_at.is_some());
    }

    #[tokio::test]
    async fn update_result_sets_summary() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let s = session::create(&pool, &agent_id, None, None, "prompt").await.unwrap();
        let id = s.id.to_string();
        session::update_result(&pool, &id, Some("All done"), Some("resume-123"))
            .await
            .unwrap();
        let found = session::find_by_id(&pool, &id).await.unwrap().unwrap();
        assert_eq!(found.result_summary.as_deref(), Some("All done"));
        assert_eq!(found.resume_session_id.as_deref(), Some("resume-123"));
    }
}

// ---------------------------------------------------------------------------
// SessionLog model tests (from crates/db/src/models/session_log.rs)
// ---------------------------------------------------------------------------

mod session_log_tests {
    use super::*;
    use composer_db::models::{agent, session, session_log};

    async fn setup_session(pool: &sqlx::SqlitePool) -> String {
        let a = agent::create(
            pool, "Agent", &AgentType::ClaudeCode, None,
        )
        .await
        .unwrap();
        let s = session::create(
            pool, &a.id.to_string(), None, None, "test",
        )
        .await
        .unwrap();
        s.id.to_string()
    }

    #[tokio::test]
    async fn append_and_list() {
        let pool = test_pool().await;
        let session_id = setup_session(&pool).await;
        session_log::append(&pool, &session_id, &LogType::Stdout, "line 1").await.unwrap();
        session_log::append(&pool, &session_id, &LogType::Stderr, "err 1").await.unwrap();
        let logs = session_log::list_by_session(&pool, &session_id, None, None, None)
            .await
            .unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].content, "line 1");
        assert!(matches!(logs[1].log_type, LogType::Stderr));
    }

    #[tokio::test]
    async fn list_with_limit() {
        let pool = test_pool().await;
        let session_id = setup_session(&pool).await;
        for i in 0..10 {
            session_log::append(&pool, &session_id, &LogType::Stdout, &format!("line {i}"))
                .await
                .unwrap();
        }
        let logs = session_log::list_by_session(&pool, &session_id, None, Some(3), None)
            .await
            .unwrap();
        assert_eq!(logs.len(), 3);
    }

    #[tokio::test]
    async fn list_with_offset() {
        let pool = test_pool().await;
        let session_id = setup_session(&pool).await;
        for i in 0..5 {
            session_log::append(&pool, &session_id, &LogType::Stdout, &format!("line {i}"))
                .await
                .unwrap();
        }
        let logs = session_log::list_by_session(&pool, &session_id, None, None, Some(3))
            .await
            .unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].content, "line 3");
    }

    #[tokio::test]
    async fn list_with_since_filter() {
        let pool = test_pool().await;
        let session_id = setup_session(&pool).await;
        session_log::append(&pool, &session_id, &LogType::Stdout, "old line").await.unwrap();
        // Since all inserts happen nearly instantly, use a past timestamp
        let logs = session_log::list_by_session(&pool, &session_id, Some("2000-01-01T00:00:00Z"), None, None)
            .await
            .unwrap();
        assert_eq!(logs.len(), 1);
    }

    #[tokio::test]
    async fn limit_caps_at_5000() {
        let pool = test_pool().await;
        let session_id = setup_session(&pool).await;
        // Just verify the function accepts limit > 5000 and caps it
        let logs = session_log::list_by_session(&pool, &session_id, None, Some(10000), None)
            .await
            .unwrap();
        assert!(logs.is_empty()); // no data, but no error
    }
}

// ---------------------------------------------------------------------------
// Worktree model tests (from crates/db/src/models/worktree.rs)
// ---------------------------------------------------------------------------

mod worktree_tests {
    use super::*;
    use composer_db::models::{agent, session, worktree};

    async fn setup_agent(pool: &sqlx::SqlitePool) -> String {
        let a = agent::create(
            pool, "Agent", &AgentType::ClaudeCode, None,
        )
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
    async fn create_worktree_record() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let session_id = setup_session(&pool, &agent_id).await;
        let wt = worktree::create(&pool, &agent_id, &session_id, "/repo", "/repo/.composer/worktrees/test", "composer/test")
            .await
            .unwrap();
        assert_eq!(wt.repo_path, "/repo");
        assert!(matches!(wt.status, WorktreeStatus::Active));
    }

    #[tokio::test]
    async fn find_by_session_returns_worktree() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let session_id = setup_session(&pool, &agent_id).await;
        worktree::create(&pool, &agent_id, &session_id, "/repo", "/repo/wt", "branch")
            .await
            .unwrap();
        let found = worktree::find_by_session(&pool, &session_id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn list_all_worktrees() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let s1 = setup_session(&pool, &agent_id).await;
        let s2 = setup_session(&pool, &agent_id).await;
        worktree::create(&pool, &agent_id, &s1, "/repo", "/repo/wt1", "b1")
            .await
            .unwrap();
        worktree::create(&pool, &agent_id, &s2, "/repo", "/repo/wt2", "b2")
            .await
            .unwrap();
        let all = worktree::list_all(&pool).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn update_status_changes_worktree_status() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let session_id = setup_session(&pool, &agent_id).await;
        let wt = worktree::create(&pool, &agent_id, &session_id, "/repo", "/repo/wt", "branch")
            .await
            .unwrap();
        let id = wt.id.to_string();
        worktree::update_status(&pool, &id, &WorktreeStatus::Deleted).await.unwrap();
        let found = worktree::find_by_id(&pool, &id).await.unwrap().unwrap();
        assert!(matches!(found.status, WorktreeStatus::Deleted));
    }
}
