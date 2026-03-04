use composer_api_types::*;

// ---------------------------------------------------------------------------
// Enum serde roundtrips (from crates/api-types/src/lib.rs)
// ---------------------------------------------------------------------------

#[test]
fn agent_type_serde_roundtrip() {
    let json = serde_json::to_string(&AgentType::ClaudeCode).unwrap();
    assert_eq!(json, r#""claude_code""#);
    let parsed: AgentType = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, AgentType::ClaudeCode));
}

#[test]
fn agent_status_serde_roundtrip() {
    for (variant, expected) in [
        (AgentStatus::Idle, "idle"),
        (AgentStatus::Busy, "busy"),
        (AgentStatus::Error, "error"),
        (AgentStatus::Offline, "offline"),
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{expected}\""));
        let _: AgentStatus = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn auth_status_serde_roundtrip() {
    for (variant, expected) in [
        (AuthStatus::Unknown, "unknown"),
        (AuthStatus::Authenticated, "authenticated"),
        (AuthStatus::Unauthenticated, "unauthenticated"),
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{expected}\""));
        let _: AuthStatus = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn task_status_serde_snake_case() {
    let json = serde_json::to_string(&TaskStatus::InProgress).unwrap();
    assert_eq!(json, r#""in_progress""#);
    let parsed: TaskStatus = serde_json::from_str(r#""in_progress""#).unwrap();
    assert!(matches!(parsed, TaskStatus::InProgress));
}

#[test]
fn task_status_all_variants() {
    for (variant, expected) in [
        (TaskStatus::Backlog, "backlog"),
        (TaskStatus::InProgress, "in_progress"),
        (TaskStatus::Waiting, "waiting"),
        (TaskStatus::Done, "done"),
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{expected}\""));
    }
}

#[test]
fn session_status_serde_roundtrip() {
    for (variant, expected) in [
        (SessionStatus::Created, "created"),
        (SessionStatus::Running, "running"),
        (SessionStatus::Paused, "paused"),
        (SessionStatus::Completed, "completed"),
        (SessionStatus::Failed, "failed"),
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{expected}\""));
    }
}

#[test]
fn worktree_status_serde_roundtrip() {
    for (variant, expected) in [
        (WorktreeStatus::Active, "active"),
        (WorktreeStatus::Stale, "stale"),
        (WorktreeStatus::Deleted, "deleted"),
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{expected}\""));
    }
}

#[test]
fn log_type_serde_roundtrip() {
    for (variant, expected) in [
        (LogType::Stdout, "stdout"),
        (LogType::Stderr, "stderr"),
        (LogType::Control, "control"),
        (LogType::Status, "status"),
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{expected}\""));
    }
}

// --- Request type deserialization ---

#[test]
fn create_task_request_minimal() {
    let json = r#"{"title": "My Task"}"#;
    let req: CreateTaskRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.title, "My Task");
    assert!(req.description.is_none());
    assert!(req.priority.is_none());
    assert!(req.status.is_none());
}

#[test]
fn create_task_request_full() {
    let json = r#"{"title": "T", "description": "Desc", "priority": 2, "status": "in_progress"}"#;
    let req: CreateTaskRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.title, "T");
    assert_eq!(req.description.as_deref(), Some("Desc"));
    assert_eq!(req.priority, Some(2));
    assert!(matches!(req.status, Some(TaskStatus::InProgress)));
}

#[test]
fn create_session_request_deser() {
    let json = r#"{
        "agent_id": "00000000-0000-0000-0000-000000000001",
        "task_id": "00000000-0000-0000-0000-000000000002",
        "prompt": "Fix the bug",
        "repo_path": "/repo"
    }"#;
    let req: CreateSessionRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.prompt, "Fix the bug");
    assert!(req.auto_approve.is_none());
}

#[test]
fn create_agent_request_optional_type() {
    let json = r#"{"name": "My Agent"}"#;
    let req: CreateAgentRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.name, "My Agent");
    assert!(req.agent_type.is_none());
}

#[test]
fn move_task_request_deser() {
    let json = r#"{"status": "done"}"#;
    let req: MoveTaskRequest = serde_json::from_str(json).unwrap();
    assert!(matches!(req.status, TaskStatus::Done));
    assert!(req.position.is_none());
}

#[test]
fn agent_health_optional_version() {
    let health = AgentHealth {
        agent_id: uuid::Uuid::nil(),
        is_installed: true,
        is_authenticated: false,
        version: None,
    };
    let json = serde_json::to_string(&health).unwrap();
    assert!(json.contains("\"version\":null"));
    let parsed: AgentHealth = serde_json::from_str(&json).unwrap();
    assert!(parsed.version.is_none());
}

// ---------------------------------------------------------------------------
// WsEvent / WsCommand tests (from crates/api-types/src/events.rs)
// ---------------------------------------------------------------------------

#[test]
fn ws_event_tagged_serialization_task_created() {
    let task = Task {
        id: uuid::Uuid::nil(),
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
    };
    let event = WsEvent::TaskCreated(task);
    let json: serde_json::Value = serde_json::to_value(&event).unwrap();
    assert_eq!(json["type"], "TaskCreated");
    assert!(json["payload"].is_object());
    assert_eq!(json["payload"]["title"], "Test");
}

#[test]
fn ws_event_tagged_serialization_agent_status() {
    let event = WsEvent::AgentStatusChanged {
        agent_id: uuid::Uuid::nil(),
        status: AgentStatus::Busy,
    };
    let json: serde_json::Value = serde_json::to_value(&event).unwrap();
    assert_eq!(json["type"], "AgentStatusChanged");
    assert_eq!(json["payload"]["status"], "busy");
}

#[test]
fn ws_event_session_output_shape() {
    let event = WsEvent::SessionOutput {
        session_id: uuid::Uuid::nil(),
        log_type: LogType::Stdout,
        content: "hello".to_string(),
    };
    let json: serde_json::Value = serde_json::to_value(&event).unwrap();
    assert_eq!(json["type"], "SessionOutput");
    assert_eq!(json["payload"]["content"], "hello");
    assert_eq!(json["payload"]["log_type"], "stdout");
}

#[test]
fn ws_command_subscribe_shape() {
    let cmd = WsCommand::SubscribeSession {
        session_id: uuid::Uuid::nil(),
    };
    let json: serde_json::Value = serde_json::to_value(&cmd).unwrap();
    assert_eq!(json["type"], "SubscribeSession");
}

#[test]
fn ws_command_ping_no_payload() {
    let cmd = WsCommand::Ping;
    let json: serde_json::Value = serde_json::to_value(&cmd).unwrap();
    assert_eq!(json["type"], "Ping");
    // Ping has no content, so payload should not exist
    assert!(json.get("payload").is_none());
}

#[test]
fn ws_event_roundtrip() {
    let event = WsEvent::TaskDeleted {
        task_id: uuid::Uuid::nil(),
    };
    let json = serde_json::to_string(&event).unwrap();
    let parsed: WsEvent = serde_json::from_str(&json).unwrap();
    match parsed {
        WsEvent::TaskDeleted { task_id } => assert_eq!(task_id, uuid::Uuid::nil()),
        _ => panic!("Wrong variant"),
    }
}
