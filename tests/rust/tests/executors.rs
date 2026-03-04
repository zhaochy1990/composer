use composer_executors::types::{CliMessage, UserMessage};
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
fn cli_message_assistant_deser() {
    let json = r#"{"type": "assistant", "message": {"role": "assistant"}, "session_id": "abc"}"#;
    let msg: CliMessage = serde_json::from_str(json).unwrap();
    match msg {
        CliMessage::Assistant(m) => {
            assert_eq!(m.session_id.as_deref(), Some("abc"));
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
        }
        _ => panic!("Expected Assistant variant"),
    }
}

#[test]
fn user_message_new() {
    let msg = UserMessage::new("Hello".to_string());
    assert_eq!(msg.message_type, "user");
    assert_eq!(msg.content, "Hello");
}

#[test]
fn user_message_serialization() {
    let msg = UserMessage::new("Fix bug".to_string());
    let json: serde_json::Value = serde_json::to_value(&msg).unwrap();
    assert_eq!(json["type"], "user");
    assert_eq!(json["content"], "Fix bug");
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
