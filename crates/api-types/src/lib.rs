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
