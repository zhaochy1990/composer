use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AgentHealth, AgentStatus, LogType, Task, TaskStatus, Worktree,
};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", content = "payload")]
pub enum WsEvent {
    AgentStatusChanged { agent_id: Uuid, status: AgentStatus },
    AgentHealthUpdated { agent_id: Uuid, health: AgentHealth },
    TaskCreated(Task),
    TaskUpdated(Task),
    TaskDeleted { task_id: Uuid },
    TaskMoved { task_id: Uuid, from_status: TaskStatus, to_status: TaskStatus },
    SessionStarted { session_id: Uuid, agent_id: Uuid, task_id: Option<Uuid> },
    SessionCompleted { session_id: Uuid, result_summary: Option<String> },
    SessionFailed { session_id: Uuid, error: String },
    SessionPaused { session_id: Uuid },
    SessionOutput { session_id: Uuid, log_type: LogType, content: String },
    WorktreeCreated(Worktree),
    WorktreeDeleted { worktree_id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", content = "payload")]
pub enum WsCommand {
    SubscribeSession { session_id: Uuid },
    UnsubscribeSession { session_id: Uuid },
    Ping,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_event_tagged_serialization_task_created() {
        let task = crate::Task {
            id: Uuid::nil(),
            title: "Test".to_string(),
            description: None,
            status: crate::TaskStatus::Backlog,
            priority: 0,
            assigned_agent_id: None,
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
            agent_id: Uuid::nil(),
            status: crate::AgentStatus::Busy,
        };
        let json: serde_json::Value = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "AgentStatusChanged");
        assert_eq!(json["payload"]["status"], "busy");
    }

    #[test]
    fn ws_event_session_output_shape() {
        let event = WsEvent::SessionOutput {
            session_id: Uuid::nil(),
            log_type: crate::LogType::Stdout,
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
            session_id: Uuid::nil(),
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
            task_id: Uuid::nil(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: WsEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            WsEvent::TaskDeleted { task_id } => assert_eq!(task_id, Uuid::nil()),
            _ => panic!("Wrong variant"),
        }
    }
}
