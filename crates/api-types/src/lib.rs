pub mod events;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

pub use events::*;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, TS, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum AgentType {
    ClaudeCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Busy,
    Error,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum AuthStatus {
    Unknown,
    Authenticated,
    Unauthenticated,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum TaskStatus {
    Backlog,
    InProgress,
    Waiting,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum SessionStatus {
    Created,
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum WorktreeStatus {
    Active,
    Stale,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum LogType {
    Stdout,
    Stderr,
    Control,
    Status,
}

// ---------------------------------------------------------------------------
// Model structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub agent_type: AgentType,
    pub executable_path: Option<String>,
    pub status: AgentStatus,
    pub auth_status: AuthStatus,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: i32,
    pub assigned_agent_id: Option<Uuid>,
    pub position: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Session {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub task_id: Option<Uuid>,
    pub worktree_id: Option<Uuid>,
    pub status: SessionStatus,
    pub resume_session_id: Option<String>,
    pub prompt: Option<String>,
    pub result_summary: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Worktree {
    pub id: Uuid,
    pub agent_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub repo_path: String,
    pub worktree_path: String,
    pub branch_name: String,
    pub status: WorktreeStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SessionLog {
    pub id: i64,
    pub session_id: Uuid,
    pub log_type: LogType,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<i32>,
    pub status: Option<TaskStatus>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<i32>,
    pub status: Option<TaskStatus>,
    pub position: Option<f64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct AssignTaskRequest {
    pub agent_id: Uuid,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct MoveTaskRequest {
    pub status: TaskStatus,
    pub position: Option<f64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateAgentRequest {
    pub name: String,
    pub agent_type: Option<AgentType>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateSessionRequest {
    pub agent_id: Uuid,
    pub task_id: Uuid,
    pub prompt: String,
    pub repo_path: String,
    pub auto_approve: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ResumeSessionRequest {
    pub prompt: Option<String>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentHealth {
    pub agent_id: Uuid,
    pub is_installed: bool,
    pub is_authenticated: bool,
    pub version: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Enum serde roundtrips ---

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
            agent_id: Uuid::nil(),
            is_installed: true,
            is_authenticated: false,
            version: None,
        };
        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("\"version\":null"));
        let parsed: AgentHealth = serde_json::from_str(&json).unwrap();
        assert!(parsed.version.is_none());
    }
}
