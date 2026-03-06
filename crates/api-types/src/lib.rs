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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, sqlx::Type)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, sqlx::Type)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum RepositoryRole {
    Primary,
    Dependency,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, sqlx::Type)]
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
    UserInput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum WorkflowRunStatus {
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum WorkflowStepType {
    Agentic,
    HumanGate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    New,
    Resume,
    Separate,
}

impl Default for SessionMode {
    fn default() -> Self {
        SessionMode::Resume
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum WorkflowStepStatus {
    Pending,
    Running,
    WaitingForHuman,
    Completed,
    Rejected,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowResumeAction {
    ContinueLoop,
    SkipToNext,
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
    pub project_id: Option<Uuid>,
    pub auto_approve: bool,
    pub position: f64,
    pub task_number: i32,
    pub simple_id: String,
    pub pr_urls: Vec<String>,
    pub workflow_run_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Session {
    pub id: Uuid,
    pub name: Option<String>,
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
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub task_prefix: String,
    pub task_counter: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectRepository {
    pub id: Uuid,
    pub project_id: Uuid,
    pub local_path: String,
    pub remote_url: Option<String>,
    pub role: RepositoryRole,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectInstruction {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub content: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Workflow {
    pub id: Uuid,
    pub name: String,
    pub is_template: bool,
    pub definition: WorkflowDefinition,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowDefinition {
    pub steps: Vec<WorkflowStepDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowStepDefinition {
    pub id: String,
    pub step_type: WorkflowStepType,
    pub name: String,
    pub prompt_template: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub on_approve: Option<String>,
    pub on_reject: Option<String>,
    pub max_retries: Option<i32>,
    #[serde(default)]
    pub loop_back_to: Option<String>,
    #[serde(default)]
    pub session_mode: Option<SessionMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowRun {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub task_id: Uuid,
    pub status: WorkflowRunStatus,
    pub iteration_count: i32,
    pub activated_steps: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowStepOutput {
    pub id: Uuid,
    pub workflow_run_id: Uuid,
    pub step_id: String,
    pub step_type: WorkflowStepType,
    pub output: Option<String>,
    pub attempt: i32,
    pub status: WorkflowStepStatus,
    pub session_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
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
    pub project_id: Option<Uuid>,
    pub assigned_agent_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<i32>,
    pub status: Option<TaskStatus>,
    pub position: Option<f64>,
    pub project_id: Option<Uuid>,
    pub assigned_agent_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
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
    pub name: Option<String>,
    pub auto_approve: Option<bool>,
    #[serde(default)]
    pub exit_on_result: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ResumeSessionRequest {
    pub prompt: Option<String>,
    #[serde(default)]
    pub exit_on_result: bool,
    /// When true, resume the session for interactive multi-turn conversation
    /// (keeps the process alive after each result). Used for post-completion chat.
    #[serde(default)]
    pub continue_chat: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct SendSessionInputRequest {
    pub message: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct AddProjectRepositoryRequest {
    pub local_path: String,
    pub remote_url: Option<String>,
    pub role: Option<RepositoryRole>,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateProjectRepositoryRequest {
    pub local_path: Option<String>,
    pub remote_url: Option<String>,
    pub role: Option<RepositoryRole>,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct AddProjectInstructionRequest {
    pub title: String,
    pub content: String,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateProjectInstructionRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub definition: WorkflowDefinition,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub definition: Option<WorkflowDefinition>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct StartWorkflowRequest {
    pub workflow_id: Uuid,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowDecisionRequest {
    pub step_id: String,
    pub approved: bool,
    pub comments: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowResumeRequest {
    pub step_id: Option<String>,
    pub action: Option<WorkflowResumeAction>,
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StartTaskResponse {
    pub task: Task,
    pub session: Session,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BrowseResponse {
    pub current_path: String,
    pub parent: Option<String>,
    pub entries: Vec<DirEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}
