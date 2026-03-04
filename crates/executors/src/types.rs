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
