use composer_executors::types::{
    CliMessage, UserMessage, SDKControlRequest, SDKControlRequestType,
    SDKControlResponse, ControlResponsePayload, ControlRequestPayload,
};
use composer_executors::process_manager::AgentProcessManager;
use tokio::sync::broadcast;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// CLI message type tests (from crates/executors/src/types.rs)
// ---------------------------------------------------------------------------

#[test]
fn cli_message_system_deser() {
    let json = r#"{"type": "system", "data": "init"}"#;
    let msg: CliMessage = serde_json::from_str(json).unwrap();
    assert!(matches!(msg, CliMessage::System(_)));
}

#[test]
fn cli_message_user_deser() {
    let json = r#"{"type": "user", "uuid": "u1", "message": {"role": "user"}, "session_id": "s1"}"#;
    let msg: CliMessage = serde_json::from_str(json).unwrap();
    match msg {
        CliMessage::User(u) => {
            assert_eq!(u.uuid.as_deref(), Some("u1"));
            assert_eq!(u.session_id.as_deref(), Some("s1"));
        }
        _ => panic!("Expected User variant"),
    }
}

#[test]
fn cli_message_assistant_deser() {
    let json = r#"{"type": "assistant", "uuid": "a1", "message": {"role": "assistant"}, "session_id": "abc"}"#;
    let msg: CliMessage = serde_json::from_str(json).unwrap();
    match msg {
        CliMessage::Assistant(m) => {
            assert_eq!(m.session_id.as_deref(), Some("abc"));
            assert_eq!(m.uuid.as_deref(), Some("a1"));
        }
        _ => panic!("Expected Assistant variant"),
    }
}

#[test]
fn cli_message_result_success() {
    let json = r#"{"type": "result", "result": "All done", "session_id": "s1"}"#;
    let msg: CliMessage = serde_json::from_str(json).unwrap();
    match msg {
        CliMessage::Result(r) => {
            assert_eq!(r.result.as_deref(), Some("All done"));
            assert_eq!(r.is_error, None);
        }
        _ => panic!("Expected Result variant"),
    }
}

#[test]
fn cli_message_result_error() {
    let json = r#"{"type": "result", "result": "Failed", "is_error": true}"#;
    let msg: CliMessage = serde_json::from_str(json).unwrap();
    match msg {
        CliMessage::Result(r) => {
            assert_eq!(r.is_error, Some(true));
        }
        _ => panic!("Expected Result variant"),
    }
}

#[test]
fn cli_message_unknown_type() {
    let json = r#"{"type": "something_new", "data": 42}"#;
    let msg: CliMessage = serde_json::from_str(json).unwrap();
    assert!(matches!(msg, CliMessage::Other));
}

#[test]
fn cli_message_result_optional_fields() {
    let json = r#"{"type": "result"}"#;
    let msg: CliMessage = serde_json::from_str(json).unwrap();
    match msg {
        CliMessage::Result(r) => {
            assert!(r.result.is_none());
            assert!(r.session_id.is_none());
            assert!(r.is_error.is_none());
        }
        _ => panic!("Expected Result variant"),
    }
}

#[test]
fn cli_message_assistant_defaults() {
    let json = r#"{"type": "assistant"}"#;
    let msg: CliMessage = serde_json::from_str(json).unwrap();
    match msg {
        CliMessage::Assistant(m) => {
            assert!(m.session_id.is_none());
            assert!(m.uuid.is_none());
        }
        _ => panic!("Expected Assistant variant"),
    }
}

// --- Control protocol messages ---

#[test]
fn cli_message_control_request_can_use_tool() {
    let json = r#"{"type": "control_request", "request_id": "r1", "request": {"subtype": "can_use_tool", "tool_name": "Bash", "input": {"command": "ls"}}}"#;
    let msg: CliMessage = serde_json::from_str(json).unwrap();
    match msg {
        CliMessage::ControlRequest { request_id, request } => {
            assert_eq!(request_id, "r1");
            match request {
                ControlRequestPayload::CanUseTool { tool_name, .. } => {
                    assert_eq!(tool_name, "Bash");
                }
                _ => panic!("Expected CanUseTool"),
            }
        }
        _ => panic!("Expected ControlRequest variant"),
    }
}

#[test]
fn cli_message_control_request_hook_callback() {
    let json = r#"{"type": "control_request", "request_id": "r2", "request": {"subtype": "hook_callback", "callback_id": "cb1", "input": {}}}"#;
    let msg: CliMessage = serde_json::from_str(json).unwrap();
    match msg {
        CliMessage::ControlRequest { request_id, request } => {
            assert_eq!(request_id, "r2");
            match request {
                ControlRequestPayload::HookCallback { callback_id, .. } => {
                    assert_eq!(callback_id, "cb1");
                }
                _ => panic!("Expected HookCallback"),
            }
        }
        _ => panic!("Expected ControlRequest variant"),
    }
}

#[test]
fn cli_message_control_response_deser() {
    let json = r#"{"type": "control_response", "response": {"subtype": "success", "request_id": "r1"}}"#;
    let msg: CliMessage = serde_json::from_str(json).unwrap();
    assert!(matches!(msg, CliMessage::ControlResponse { .. }));
}

#[test]
fn cli_message_control_cancel_request_deser() {
    let json = r#"{"type": "control_cancel_request", "request_id": "r1"}"#;
    let msg: CliMessage = serde_json::from_str(json).unwrap();
    match msg {
        CliMessage::ControlCancelRequest { request_id } => {
            assert_eq!(request_id, "r1");
        }
        _ => panic!("Expected ControlCancelRequest variant"),
    }
}

// --- Outgoing message tests ---

#[test]
fn user_message_new() {
    let msg = UserMessage::new("Hello".to_string());
    assert_eq!(msg.message_type, "user");
    assert_eq!(msg.message.role, "user");
    assert_eq!(msg.message.content, "Hello");
}

#[test]
fn user_message_serialization() {
    let msg = UserMessage::new("Fix bug".to_string());
    let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
    assert_eq!(json["type"], "user");
    assert_eq!(json["message"]["role"], "user");
    assert_eq!(json["message"]["content"], "Fix bug");
    // Must NOT have a top-level "content" field
    assert!(json.get("content").is_none());
}

#[test]
fn sdk_control_request_interrupt_serialization() {
    let req = SDKControlRequest::new(SDKControlRequestType::Interrupt {});
    let json: serde_json::Value = serde_json::to_value(&req).unwrap();
    assert_eq!(json["type"], "control_request");
    assert!(json["request_id"].is_string());
    assert_eq!(json["request"]["subtype"], "interrupt");
}

#[test]
fn sdk_control_response_success_serialization() {
    let resp = SDKControlResponse::new(ControlResponsePayload::Success {
        request_id: "r1".to_string(),
        response: Some(serde_json::json!({"behavior": "allow"})),
    });
    let json: serde_json::Value = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["type"], "control_response");
    assert_eq!(json["response"]["subtype"], "success");
    assert_eq!(json["response"]["request_id"], "r1");
}

#[test]
fn sdk_control_response_error_serialization() {
    let resp = SDKControlResponse::new(ControlResponsePayload::Error {
        request_id: "r2".to_string(),
        error: Some("denied".to_string()),
    });
    let json: serde_json::Value = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["type"], "control_response");
    assert_eq!(json["response"]["subtype"], "error");
    assert_eq!(json["response"]["error"], "denied");
}

// ---------------------------------------------------------------------------
// Process manager tests (from crates/executors/src/process_manager.rs)
// ---------------------------------------------------------------------------

fn make_manager() -> AgentProcessManager {
    let (tx, _) = broadcast::channel(16);
    AgentProcessManager::new(tx)
}

#[test]
fn is_running_false_for_unknown() {
    let mgr = make_manager();
    assert!(!mgr.is_running(&Uuid::new_v4()));
}

#[test]
fn running_count_starts_at_zero() {
    let mgr = make_manager();
    assert_eq!(mgr.running_count(), 0);
}

#[tokio::test]
async fn interrupt_nonexistent_session_ok() {
    let mgr = make_manager();
    // Should not panic or error
    mgr.interrupt(Uuid::new_v4()).await.unwrap();
}

#[tokio::test]
async fn send_input_nonexistent_session_errors() {
    let mgr = make_manager();
    let result = mgr.send_input(Uuid::new_v4(), "hello".to_string()).await;
    assert!(result.is_err());
}

#[test]
fn queue_and_take_message() {
    let mgr = make_manager();
    let sid = Uuid::new_v4();
    assert!(mgr.take_queued_message(&sid).is_none());
    mgr.queue_message(sid, "hello".to_string());
    assert_eq!(mgr.take_queued_message(&sid).as_deref(), Some("hello"));
    // Consumed — should be gone
    assert!(mgr.take_queued_message(&sid).is_none());
}

#[test]
fn queue_replaces_previous_message() {
    let mgr = make_manager();
    let sid = Uuid::new_v4();
    mgr.queue_message(sid, "first".to_string());
    mgr.queue_message(sid, "second".to_string());
    assert_eq!(mgr.take_queued_message(&sid).as_deref(), Some("second"));
}

// ---------------------------------------------------------------------------
// E2E test: real Claude Code process against Q:/src/tests
// ---------------------------------------------------------------------------
// These tests spawn a real Claude Code process. They require:
// - Claude Code CLI installed (npx @anthropic-ai/claude-code)
// - ANTHROPIC_API_KEY set in environment
// - Q:/src/tests to be a valid git repo
// Run with: cargo test --test executors -- e2e --ignored

use composer_executors::process_manager::SpawnOptions;
use composer_api_types::{WsEvent, LogType};

/// Helper: collect WsEvents from a broadcast receiver until a condition is met or timeout.
async fn collect_events_until(
    rx: &mut broadcast::Receiver<WsEvent>,
    timeout: std::time::Duration,
    mut predicate: impl FnMut(&WsEvent) -> bool,
) -> Vec<WsEvent> {
    let mut events = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok(event)) => {
                let done = predicate(&event);
                events.push(event);
                if done {
                    break;
                }
            }
            Ok(Err(broadcast::error::RecvError::Lagged(n))) => {
                eprintln!("Warning: lagged {} events", n);
            }
            _ => break,
        }
    }
    events
}

#[tokio::test]
#[ignore] // Requires real Claude Code + API key
async fn e2e_spawn_send_message_and_receive_output() {
    let (tx, mut rx) = broadcast::channel(1024);
    let mgr = AgentProcessManager::new(tx);
    let session_id = Uuid::new_v4();

    // Spawn Claude Code against the test repo
    mgr.spawn(SpawnOptions {
        session_id,
        agent_id: Uuid::new_v4(),
        task_id: None,
        prompt: "Reply with exactly: HELLO_E2E_TEST. Nothing else.".to_string(),
        working_dir: "Q:/src/tests".to_string(),
        auto_approve: true,
        resume_session_id: None,
        resume_at_message_id: None,
    })
    .await
    .expect("Failed to spawn Claude Code");

    assert!(mgr.is_running(&session_id));

    // Wait for SessionStarted + at least one SessionOutput with stdout content
    let events = collect_events_until(
        &mut rx,
        std::time::Duration::from_secs(120),
        |e| matches!(e, WsEvent::SessionOutput { log_type: LogType::Stdout, content, .. } if content.contains("HELLO_E2E_TEST")),
    )
    .await;

    // Verify we received SessionStarted
    assert!(
        events.iter().any(|e| matches!(e, WsEvent::SessionStarted { .. })),
        "Expected SessionStarted event"
    );

    // Verify we got stdout output containing our test string
    let has_output = events.iter().any(|e| {
        matches!(e, WsEvent::SessionOutput { log_type: LogType::Stdout, content, .. } if content.contains("HELLO_E2E_TEST"))
    });
    assert!(has_output, "Expected output containing HELLO_E2E_TEST, got events: {:#?}",
        events.iter().filter(|e| matches!(e, WsEvent::SessionOutput { .. })).collect::<Vec<_>>());

    // Give async session_id capture a moment to propagate
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify Claude Code session_id was captured (may not be available if output format differs)
    let claude_sid = mgr.get_claude_session_id(&session_id);
    if claude_sid.is_some() {
        eprintln!("Captured Claude Code session_id: {:?}", claude_sid);
    } else {
        eprintln!("Warning: Claude Code session_id not captured (may not be in output)");
    }

    // Session should still be running (multi-turn)
    assert!(mgr.is_running(&session_id), "Session should still be running for multi-turn");

    // Interrupt and verify cleanup
    mgr.interrupt(session_id).await.unwrap();
    assert!(!mgr.is_running(&session_id), "Session should be stopped after interrupt");
}

#[tokio::test]
#[ignore] // Requires real Claude Code + API key
async fn e2e_multi_turn_send_follow_up_message() {
    let (tx, mut rx) = broadcast::channel(1024);
    let mgr = AgentProcessManager::new(tx);
    let session_id = Uuid::new_v4();

    // Spawn with initial prompt
    mgr.spawn(SpawnOptions {
        session_id,
        agent_id: Uuid::new_v4(),
        task_id: None,
        prompt: "Reply with exactly: TURN_ONE. Nothing else.".to_string(),
        working_dir: "Q:/src/tests".to_string(),
        auto_approve: true,
        resume_session_id: None,
        resume_at_message_id: None,
    })
    .await
    .expect("Failed to spawn");

    // Wait for first turn to complete (Result message stored internally)
    let _events = collect_events_until(
        &mut rx,
        std::time::Duration::from_secs(120),
        |e| matches!(e, WsEvent::SessionOutput { log_type: LogType::Stdout, content, .. } if content.contains("TURN_ONE")),
    )
    .await;

    // Session should still be running
    assert!(mgr.is_running(&session_id), "Session should still be running after first turn");

    // Send a follow-up message
    mgr.send_input(session_id, "Now reply with exactly: TURN_TWO. Nothing else.".to_string())
        .await
        .expect("Failed to send follow-up message");

    // Wait for second turn output
    let events2 = collect_events_until(
        &mut rx,
        std::time::Duration::from_secs(120),
        |e| matches!(e, WsEvent::SessionOutput { log_type: LogType::Stdout, content, .. } if content.contains("TURN_TWO")),
    )
    .await;

    let has_turn2 = events2.iter().any(|e| {
        matches!(e, WsEvent::SessionOutput { log_type: LogType::Stdout, content, .. } if content.contains("TURN_TWO"))
    });
    assert!(has_turn2, "Expected TURN_TWO in follow-up output");

    // Clean up
    mgr.interrupt(session_id).await.unwrap();
}
