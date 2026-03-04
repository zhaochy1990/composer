use serde::{Deserialize, Serialize};

// Messages FROM Claude Code (read from stdout)

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CliMessage {
    #[serde(rename = "system")]
    System(serde_json::Value),
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    #[serde(rename = "result")]
    Result(ResultMessage),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub struct AssistantMessage {
    #[serde(default)]
    pub message: serde_json::Value,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResultMessage {
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub is_error: Option<bool>,
}

// Messages TO Claude Code (written to stdin)

#[derive(Debug, Serialize)]
pub struct UserMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub content: String,
}

impl UserMessage {
    pub fn new(content: String) -> Self {
        Self {
            message_type: "user".to_string(),
            content,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
