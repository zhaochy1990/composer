use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AgentHealth, AgentStatus, LogType, Project, ProjectRepository, Task, TaskStatus, Worktree,
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
    SessionCompleted { session_id: Uuid, result_summary: Option<String>, claude_session_id: Option<String> },
    SessionFailed { session_id: Uuid, error: String, claude_session_id: Option<String> },
    SessionPaused { session_id: Uuid },
    SessionOutput { session_id: Uuid, log_type: LogType, content: String },
    SessionResumeIdCaptured { session_id: Uuid, claude_session_id: String },
    WorktreeCreated(Worktree),
    WorktreeDeleted { worktree_id: Uuid },
    ProjectCreated(Project),
    ProjectUpdated(Project),
    ProjectDeleted { project_id: Uuid },
    ProjectRepositoryAdded { project_id: Uuid, repository: ProjectRepository },
    ProjectRepositoryRemoved { project_id: Uuid, repository_id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", content = "payload")]
pub enum WsCommand {
    SubscribeSession { session_id: Uuid },
    UnsubscribeSession { session_id: Uuid },
    SendInput { session_id: Uuid, message: String },
    Ping,
}
