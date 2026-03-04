use serde::{Deserialize, Serialize};

// Messages FROM Claude Code (read from stdout)

/// Top-level message types from Claude Code's stream-json stdout.
/// Covers the full control protocol plus regular conversation messages.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CliMessage {
    #[serde(rename = "system")]
    System(serde_json::Value),
    #[serde(rename = "user")]
    User(UserOutputMessage),
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    #[serde(rename = "result")]
    Result(ResultMessage),

    // Control protocol messages
    #[serde(rename = "control_request")]
    ControlRequest {
        request_id: String,
        request: ControlRequestPayload,
    },
    #[serde(rename = "control_response")]
    ControlResponse {
        response: serde_json::Value,
    },
    #[serde(rename = "control_cancel_request")]
    ControlCancelRequest {
        request_id: String,
    },

    #[serde(other)]
    Other,
}

/// User message echoed back from Claude Code (when --replay-user-messages is used).
#[derive(Debug, Deserialize)]
pub struct UserOutputMessage {
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub message: serde_json::Value,
    #[serde(default)]
    pub session_id: Option<String>,
    /// True when this is a historical message replayed during --resume.
    /// These should be filtered out to avoid duplicate output.
    #[serde(default, rename = "isReplay")]
    pub is_replay: bool,
}

#[derive(Debug, Deserialize)]
pub struct AssistantMessage {
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub message: serde_json::Value,
    #[serde(default)]
    pub session_id: Option<String>,
    /// True when this is a historical message replayed during --resume.
    #[serde(default, rename = "isReplay")]
    pub is_replay: bool,
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

/// Control request payload from Claude Code (tool permission requests, hook callbacks).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum ControlRequestPayload {
    CanUseTool {
        tool_name: String,
        input: serde_json::Value,
        #[serde(default)]
        permission_suggestions: Option<Vec<serde_json::Value>>,
        #[serde(default)]
        tool_use_id: Option<String>,
    },
    HookCallback {
        callback_id: String,
        input: serde_json::Value,
        #[serde(default)]
        tool_use_id: Option<String>,
    },
    #[serde(other)]
    Unknown,
}

// Messages TO Claude Code (written to stdin)

/// User message sent to Claude Code via stream-json stdin protocol.
/// Format: `{"type":"user","message":{"role":"user","content":"..."}}`
#[derive(Debug, Serialize)]
pub struct UserMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub message: UserMessageContent,
}

#[derive(Debug, Serialize)]
pub struct UserMessageContent {
    pub role: String,
    pub content: String,
}

impl UserMessage {
    pub fn new(content: String) -> Self {
        Self {
            message_type: "user".to_string(),
            message: UserMessageContent {
                role: "user".to_string(),
                content,
            },
        }
    }
}

/// SDK control request sent to Claude Code to control execution.
/// Format: `{"type":"control_request","request_id":"...","request":{...}}`
#[derive(Debug, Serialize)]
pub struct SDKControlRequest {
    #[serde(rename = "type")]
    pub message_type: String,
    pub request_id: String,
    pub request: SDKControlRequestType,
}

impl SDKControlRequest {
    pub fn new(request: SDKControlRequestType) -> Self {
        Self {
            message_type: "control_request".to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            request,
        }
    }
}

/// SDK control response sent to Claude Code (e.g. tool permission results).
/// Format: `{"type":"control_response","response":{...}}`
#[derive(Debug, Serialize)]
pub struct SDKControlResponse {
    #[serde(rename = "type")]
    pub message_type: String,
    pub response: ControlResponsePayload,
}

impl SDKControlResponse {
    pub fn new(response: ControlResponsePayload) -> Self {
        Self {
            message_type: "control_response".to_string(),
            response,
        }
    }
}

/// Control request subtypes.
#[derive(Debug, Serialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum SDKControlRequestType {
    Interrupt {},
}

/// Control response subtypes.
#[derive(Debug, Serialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum ControlResponsePayload {
    Success {
        request_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<serde_json::Value>,
    },
    Error {
        request_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
}
