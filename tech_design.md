Excellent. Now I have a comprehensive understanding of both the product vision and the reference architecture. Let me now produce the complete technical design document.

---

# Composer -- Prototype Phase Technical Architecture

## Table of Contents

1. [Cargo Workspace Structure](#1-cargo-workspace-structure)
2. [SQLite Schema](#2-sqlite-schema)
3. [API Design](#3-api-design)
4. [Agent Executor Architecture](#4-agent-executor-architecture)
5. [Worktree Manager](#5-worktree-manager)
6. [Frontend Architecture](#6-frontend-architecture)
7. [Project Directory Structure](#7-project-directory-structure)
8. [Build System](#8-build-system)

---

## 1. Cargo Workspace Structure

The workspace follows the vibe-kanban pattern with a `crates/` directory, but is simplified for the prototype scope. Six crates, each with a clear single responsibility.

### Workspace Members

| Crate | Purpose | Key Dependencies |
|-------|---------|------------------|
| `composer-server` | Axum HTTP server, routes, WebSocket, frontend embedding | `axum`, `tower-http`, `rust-embed`, `tokio`, `composer-services`, `composer-db`, `composer-api-types` |
| `composer-db` | SQLx models, migrations, database pool | `sqlx` (SQLite + runtime-tokio), `uuid`, `chrono`, `composer-api-types` |
| `composer-services` | Business logic: agent management, worktree lifecycle, task orchestration | `tokio`, `git2`, `composer-db`, `composer-executors`, `composer-api-types` |
| `composer-executors` | Agent subprocess spawning, Claude Code protocol, process management | `tokio` (process, io, sync), `serde_json`, `uuid` |
| `composer-api-types` | Shared Rust structs/enums with `ts-rs` for TypeScript codegen | `serde`, `ts-rs`, `uuid`, `chrono` |
| `composer-git` | Git CLI wrapper for worktree operations | `tokio::process`, `thiserror` |

### Root `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
    "crates/server",
    "crates/db",
    "crates/services",
    "crates/executors",
    "crates/api-types",
    "crates/git",
]

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# Web framework
axum = { version = "0.8", features = ["ws", "macros"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }

# Database
sqlx = { version = "0.8", features = [
    "runtime-tokio", "sqlite", "uuid", "chrono", "migrate"
] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Type generation
ts-rs = { version = "10", features = ["uuid-impl", "chrono-impl", "serde-json-impl"] }

# Git
git2 = "0.20"

# Frontend embedding
rust-embed = { version = "8", features = ["mime-guess"] }

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
futures-util = "0.3"
tokio-stream = "0.1"

[profile.release]
strip = "symbols"
lto = "thin"
```

### Dependency Graph

```
composer-server
  ├── composer-services
  │     ├── composer-db
  │     │     └── composer-api-types
  │     ├── composer-executors
  │     │     └── composer-api-types
  │     ├── composer-git
  │     └── composer-api-types
  └── composer-api-types
```

---

## 2. SQLite Schema

### Migration: `001_initial_schema.sql`

```sql
-- ============================================================
-- AGENTS
-- ============================================================
CREATE TABLE agents (
    id              TEXT PRIMARY KEY NOT NULL,   -- UUID v4
    name            TEXT NOT NULL,               -- display name, e.g. "claude-1"
    agent_type      TEXT NOT NULL DEFAULT 'claude_code',  -- enum: claude_code (extensible later)
    executable_path TEXT,                        -- resolved path to npx/claude binary
    status          TEXT NOT NULL DEFAULT 'idle',-- idle | busy | error | offline
    auth_status     TEXT NOT NULL DEFAULT 'unknown', -- unknown | authenticated | unauthenticated
    last_heartbeat  TEXT,                        -- ISO 8601 timestamp
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- ============================================================
-- TASKS
-- ============================================================
CREATE TABLE tasks (
    id              TEXT PRIMARY KEY NOT NULL,   -- UUID v4
    title           TEXT NOT NULL,
    description     TEXT,
    status          TEXT NOT NULL DEFAULT 'backlog',  -- backlog | in_progress | waiting | done
    priority        INTEGER NOT NULL DEFAULT 0,       -- 0=none, 1=low, 2=medium, 3=high
    assigned_agent_id TEXT REFERENCES agents(id) ON DELETE SET NULL,
    position        REAL NOT NULL DEFAULT 0.0,        -- fractional ranking within column
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_assigned_agent ON tasks(assigned_agent_id);

-- ============================================================
-- SESSIONS
-- ============================================================
CREATE TABLE sessions (
    id              TEXT PRIMARY KEY NOT NULL,   -- UUID v4
    agent_id        TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    task_id         TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    worktree_id     TEXT REFERENCES worktrees(id) ON DELETE SET NULL,
    status          TEXT NOT NULL DEFAULT 'created',  -- created | running | paused | completed | failed
    resume_session_id TEXT,                    -- Claude Code session ID for resume
    prompt          TEXT,                       -- the prompt sent to the agent
    result_summary  TEXT,                       -- agent's final output summary
    started_at      TEXT,
    completed_at    TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_sessions_agent ON sessions(agent_id);
CREATE INDEX idx_sessions_task ON sessions(task_id);

-- ============================================================
-- WORKTREES
-- ============================================================
CREATE TABLE worktrees (
    id              TEXT PRIMARY KEY NOT NULL,   -- UUID v4
    agent_id        TEXT REFERENCES agents(id) ON DELETE SET NULL,
    session_id      TEXT REFERENCES sessions(id) ON DELETE SET NULL,
    repo_path       TEXT NOT NULL,               -- absolute path to main repo
    worktree_path   TEXT NOT NULL UNIQUE,         -- absolute path to worktree directory
    branch_name     TEXT NOT NULL,               -- branch created for this worktree
    status          TEXT NOT NULL DEFAULT 'active', -- active | stale | deleted
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_worktrees_agent ON worktrees(agent_id);

-- ============================================================
-- SESSION_LOGS  (streaming agent output)
-- ============================================================
CREATE TABLE session_logs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    log_type        TEXT NOT NULL,  -- stdout | stderr | control | status
    content         TEXT NOT NULL,  -- raw JSON line from agent
    timestamp       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_session_logs_session ON session_logs(session_id);
```

### Key Design Decisions

1. **All IDs are TEXT (UUID v4)** -- SQLite stores UUIDs as text. SQLx maps them via `sqlx::types::Uuid`.
2. **Timestamps as TEXT in ISO 8601** -- SQLite lacks a native datetime type. Using `strftime` with milliseconds for precision.
3. **Fractional positioning** -- `tasks.position` uses REAL to allow reordering without renumbering (insert between 1.0 and 2.0 = 1.5).
4. **Session logs are append-only** -- Every JSON line from the Claude Code subprocess gets persisted for replay/debugging.
5. **Worktree lifecycle** -- The `worktrees` table tracks the physical state. Cleanup operations mark as `deleted` rather than removing the row.

---

## 3. API Design

### REST Endpoints

All REST endpoints live under `/api/`. The server also serves the React SPA at `/` with fallback to `index.html`.

#### Tasks

| Method | Path | Description | Request Body | Response |
|--------|------|-------------|-------------|----------|
| `GET` | `/api/tasks` | List all tasks, optional `?status=backlog` filter | -- | `Task[]` |
| `POST` | `/api/tasks` | Create a new task | `CreateTaskRequest` | `Task` |
| `GET` | `/api/tasks/:id` | Get single task | -- | `Task` |
| `PUT` | `/api/tasks/:id` | Update task fields | `UpdateTaskRequest` | `Task` |
| `DELETE` | `/api/tasks/:id` | Delete a task | -- | `204 No Content` |
| `POST` | `/api/tasks/:id/assign` | Assign task to agent | `AssignTaskRequest` | `Task` |
| `POST` | `/api/tasks/:id/move` | Move task to column (status change) | `MoveTaskRequest` | `Task` |

#### Agents

| Method | Path | Description | Request Body | Response |
|--------|------|-------------|-------------|----------|
| `GET` | `/api/agents` | List all agents with status | -- | `Agent[]` |
| `POST` | `/api/agents` | Register a new agent | `CreateAgentRequest` | `Agent` |
| `GET` | `/api/agents/:id` | Get agent details | -- | `Agent` |
| `DELETE` | `/api/agents/:id` | Remove agent from pool | -- | `204 No Content` |
| `POST` | `/api/agents/discover` | Auto-discover installed agents | -- | `Agent[]` |
| `GET` | `/api/agents/:id/health` | Probe agent health/auth | -- | `AgentHealth` |

#### Sessions

| Method | Path | Description | Request Body | Response |
|--------|------|-------------|-------------|----------|
| `GET` | `/api/sessions` | List sessions, `?agent_id=` or `?task_id=` filter | -- | `Session[]` |
| `POST` | `/api/sessions` | Create and start a new session | `CreateSessionRequest` | `Session` |
| `GET` | `/api/sessions/:id` | Get session details | -- | `Session` |
| `POST` | `/api/sessions/:id/resume` | Resume a paused session | `ResumeSessionRequest` | `Session` |
| `POST` | `/api/sessions/:id/interrupt` | Interrupt a running session | -- | `Session` |
| `GET` | `/api/sessions/:id/logs` | Get historical logs | `?since=timestamp` | `SessionLog[]` |

#### Worktrees

| Method | Path | Description | Request Body | Response |
|--------|------|-------------|-------------|----------|
| `GET` | `/api/worktrees` | List worktrees | -- | `Worktree[]` |
| `POST` | `/api/worktrees/:id/cleanup` | Remove worktree and branch | -- | `204 No Content` |

### WebSocket Events

A single WebSocket endpoint at `/api/ws` carries all real-time events. The client connects once and receives a multiplexed stream.

#### Server-to-Client Events

```rust
#[derive(Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "payload")]
pub enum WsEvent {
    // Agent status changes
    AgentStatusChanged { agent_id: Uuid, status: AgentStatus },
    AgentHealthUpdated { agent_id: Uuid, health: AgentHealth },

    // Task lifecycle
    TaskCreated(Task),
    TaskUpdated(Task),
    TaskDeleted { task_id: Uuid },
    TaskMoved { task_id: Uuid, from_status: TaskStatus, to_status: TaskStatus },

    // Session lifecycle
    SessionStarted { session_id: Uuid, agent_id: Uuid, task_id: Option<Uuid> },
    SessionCompleted { session_id: Uuid, result_summary: Option<String> },
    SessionFailed { session_id: Uuid, error: String },
    SessionPaused { session_id: Uuid },

    // Real-time agent output streaming
    SessionOutput { session_id: Uuid, log_type: LogType, content: String },

    // Worktree events
    WorktreeCreated(Worktree),
    WorktreeDeleted { worktree_id: Uuid },
}
```

#### Client-to-Server Messages (over WebSocket)

```rust
#[derive(Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsCommand {
    // Subscribe to specific session's output stream
    SubscribeSession { session_id: Uuid },
    UnsubscribeSession { session_id: Uuid },
    // Ping for keepalive
    Ping,
}
```

### Shared API Types (`composer-api-types`)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

// ---- Enums ----

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

// ---- Models ----

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

// ---- Request types ----

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<i32>,
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
    pub agent_type: Option<AgentType>,  // defaults to ClaudeCode
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateSessionRequest {
    pub agent_id: Uuid,
    pub task_id: Option<Uuid>,
    pub prompt: String,
    pub repo_path: String,               // which repo to create worktree in
    pub auto_approve: Option<bool>,      // F8.3 auto-yes
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ResumeSessionRequest {
    pub prompt: Option<String>,          // optional new prompt for follow-up
}

// ---- Response types ----

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct AgentHealth {
    pub agent_id: Uuid,
    pub is_installed: bool,
    pub is_authenticated: bool,
    pub version: Option<String>,
}
```

---

## 4. Agent Executor Architecture

### Overview

The executor system is modeled after vibe-kanban's pattern but simplified for the prototype. It spawns Claude Code as a child process using `npx`, communicates via newline-delimited JSON over stdin/stdout, and manages the process lifecycle.

### Claude Code Protocol Types (in `composer-executors`)

```rust
// ---- Messages FROM Claude Code (read from stdout) ----

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CliMessage {
    #[serde(rename = "control_request")]
    ControlRequest {
        request_id: String,
        request: ControlRequestType,
    },
    #[serde(rename = "result")]
    Result(serde_json::Value),
    // Catch-all for message types we do not handle yet
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlRequestType {
    CanUseTool {
        tool_name: String,
        input: serde_json::Value,
    },
    HookCallback {
        callback_id: String,
        input: serde_json::Value,
    },
}

// ---- Messages TO Claude Code (written to stdin) ----

#[derive(Debug, Serialize)]
pub struct SdkControlRequest {
    pub message_type: String,   // always "control_request"
    pub request_id: String,     // UUID
    pub request: SdkControlRequestType,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SdkControlRequestType {
    Initialize {
        // Permission mode to start with
        permission_mode: PermissionMode,
        // Hooks configuration (empty for prototype)
    },
    Interrupt,
    SetPermissionMode {
        mode: PermissionMode,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    Default,
    AcceptEdits,
    Plan,
    BypassPermissions,   // This is the "auto-yes" mode (F8.3)
}

#[derive(Debug, Serialize)]
pub struct ControlResponse {
    pub message_type: String,   // "control_response"
    pub response: ControlResponseType,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlResponseType {
    Success {
        request_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<PermissionResult>,
    },
    Error {
        request_id: String,
        error: Option<String>,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum PermissionResult {
    Allow,
    Deny { message: String },
}

// User message to inject into conversation
#[derive(Debug, Serialize)]
pub struct UserMessage {
    pub message_type: String,   // "user_message"
    pub content: String,
    pub role: String,           // "user"
}
```

### Agent Process Manager

```rust
pub struct AgentProcessManager {
    /// Running processes indexed by session_id
    processes: Arc<DashMap<Uuid, AgentProcess>>,
    /// Channel for broadcasting events
    event_tx: broadcast::Sender<WsEvent>,
}

pub struct AgentProcess {
    pub session_id: Uuid,
    pub agent_id: Uuid,
    pub child: tokio::process::Child,
    pub stdin: Arc<Mutex<ChildStdin>>,
    pub cancel_token: CancellationToken,
    pub join_handle: JoinHandle<Result<(), AgentError>>,
}
```

### Spawn Sequence

The core spawn flow:

```
1. POST /api/sessions { agent_id, task_id, prompt, repo_path, auto_approve }
    |
2. SessionService::create_session()
    |-- Insert session row (status=created)
    |-- WorktreeService::create_worktree(repo_path, agent_id, session_id)
    |     |-- git worktree add <path> -b composer/<agent_name>/<session_id_short>
    |     |-- Insert worktree row
    |     |-- Return worktree
    |
3. AgentProcessManager::spawn(session, worktree, prompt, auto_approve)
    |
    |-- Build command:
    |     program: "npx"
    |     args: [
    |       "-y",
    |       "@anthropic-ai/claude-code@latest",
    |       "--verbose",
    |       "--output-format=stream-json",
    |       "--input-format=stream-json",
    |     ]
    |     cwd: worktree.worktree_path
    |     env: inherit + ANTHROPIC_API_KEY (if set)
    |
    |-- Spawn child process with piped stdin/stdout/stderr
    |
    |-- Send Initialize message via stdin:
    |     {
    |       "message_type": "control_request",
    |       "request_id": "<uuid>",
    |       "request": {
    |         "type": "initialize",
    |         "permission_mode": "bypass_permissions"  // if auto_approve=true
    |       }
    |     }
    |
    |-- Send user message (the task prompt):
    |     {
    |       "message_type": "user_message",
    |       "content": "<prompt built from task title + description>",
    |       "role": "user"
    |     }
    |
    |-- Spawn async read loop (tokio::spawn):
    |     loop {
    |       read line from stdout
    |       parse as CliMessage
    |       match {
    |         ControlRequest::CanUseTool => {
    |           if auto_approve {
    |             respond Success { result: Allow }
    |           } else {
    |             // Queue for human approval (future feature)
    |             // For prototype: always allow
    |             respond Success { result: Allow }
    |           }
    |         }
    |         Result(value) => {
    |           update session status = completed
    |           update session result_summary
    |           update task status = done (if bound)
    |           update agent status = idle
    |           broadcast WsEvent::SessionCompleted
    |           break
    |         }
    |         Other => {
    |           // Stream raw JSON to subscribers
    |           persist to session_logs
    |           broadcast WsEvent::SessionOutput
    |         }
    |       }
    |     }
    |
    |-- Spawn async stderr reader (tokio::spawn):
    |     forward lines as SessionOutput events with log_type=stderr
    |
    |-- Update session status = running
    |-- Update agent status = busy
    |-- Broadcast WsEvent::SessionStarted
```

### Auto-Yes Implementation (F8.3)

Auto-yes is implemented at the protocol level. When `auto_approve` is true:

1. During initialization, the permission mode is set to `BypassPermissions`
2. As a fallback, any `CanUseTool` control request is responded to with `Allow`
3. The `BypassPermissions` mode tells Claude Code to skip its own permission prompts entirely

```rust
impl AgentProcessManager {
    async fn handle_can_use_tool(
        &self,
        stdin: &Arc<Mutex<ChildStdin>>,
        request_id: &str,
        auto_approve: bool,
        _tool_name: &str,
        _input: &serde_json::Value,
    ) -> Result<()> {
        let response = if auto_approve {
            ControlResponse {
                message_type: "control_response".into(),
                response: ControlResponseType::Success {
                    request_id: request_id.to_string(),
                    response: Some(PermissionResult::Allow),
                },
            }
        } else {
            // Future: queue for human approval
            // Prototype: auto-allow
            ControlResponse {
                message_type: "control_response".into(),
                response: ControlResponseType::Success {
                    request_id: request_id.to_string(),
                    response: Some(PermissionResult::Allow),
                },
            }
        };

        let mut stdin = stdin.lock().await;
        let json = serde_json::to_string(&response)?;
        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }
}
```

### Session Resume (F2.5)

When resuming a paused session:

1. The previous session's `resume_session_id` (stored from the Claude Code `Result` message) is used
2. The same worktree is reused (it still has the branch and file state)
3. A new Claude Code process is spawned with `--resume <session_id>` flag
4. The follow-up prompt is sent as a new user message

```rust
pub async fn resume_session(
    &self,
    session_id: Uuid,
    new_prompt: Option<String>,
) -> Result<Session> {
    let session = self.db.sessions.find_by_id(session_id).await?;
    let worktree = self.db.worktrees.find_by_id(session.worktree_id.unwrap()).await?;

    // Spawn new process with --resume flag
    let mut cmd = Command::new("npx");
    cmd.args([
        "-y", "@anthropic-ai/claude-code@latest",
        "--verbose",
        "--output-format=stream-json",
        "--input-format=stream-json",
        "--resume", session.resume_session_id.as_deref().unwrap_or(""),
    ]);
    cmd.current_dir(&worktree.worktree_path);
    // ... rest of spawn logic
}
```

### Interrupt and Cleanup

```rust
pub async fn interrupt_session(&self, session_id: Uuid) -> Result<()> {
    if let Some(process) = self.processes.get(&session_id) {
        // 1. Send Interrupt control message
        let interrupt = SdkControlRequest {
            message_type: "control_request".into(),
            request_id: Uuid::new_v4().to_string(),
            request: SdkControlRequestType::Interrupt,
        };
        // Write to stdin
        let mut stdin = process.stdin.lock().await;
        let json = serde_json::to_string(&interrupt)?;
        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;

        // 2. Cancel the read loop
        process.cancel_token.cancel();

        // 3. Wait for graceful shutdown (timeout 5s, then kill)
        tokio::select! {
            _ = process.join_handle => {},
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                process.child.kill().await?;
            }
        }
    }
    Ok(())
}
```

---

## 5. Worktree Manager

### Design

The worktree manager uses `git` CLI commands (not `git2` libgit2) for worktree operations because `git2` has incomplete worktree support on Windows. It runs all commands via `tokio::process::Command` to stay non-blocking.

### `composer-git` Crate

```rust
// crates/git/src/lib.rs
pub mod worktree;

// crates/git/src/worktree.rs

use std::path::{Path, PathBuf};
use tokio::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitWorktreeError {
    #[error("git command failed: {0}")]
    CommandFailed(String),
    #[error("worktree already exists at {0}")]
    AlreadyExists(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct WorktreeInfo {
    pub worktree_path: PathBuf,
    pub branch_name: String,
}

/// Create a new git worktree with a new branch.
///
/// Equivalent to: git worktree add <worktree_path> -b <branch_name>
///
/// The worktree is created under: <repo_path>/.composer/worktrees/<name>/
pub async fn create_worktree(
    repo_path: &Path,
    name: &str,
    base_branch: Option<&str>,
) -> Result<WorktreeInfo, GitWorktreeError> {
    let worktree_dir = repo_path.join(".composer").join("worktrees").join(name);
    let branch_name = format!("composer/{}", name);

    if worktree_dir.exists() {
        return Err(GitWorktreeError::AlreadyExists(worktree_dir));
    }

    // Ensure parent directory exists
    tokio::fs::create_dir_all(worktree_dir.parent().unwrap()).await?;

    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path);
    cmd.args(["worktree", "add"]);

    // Path to new worktree
    cmd.arg(&worktree_dir);

    // Create new branch
    cmd.args(["-b", &branch_name]);

    // Base branch (default: HEAD)
    if let Some(base) = base_branch {
        cmd.arg(base);
    }

    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitWorktreeError::CommandFailed(stderr.to_string()));
    }

    Ok(WorktreeInfo {
        worktree_path: worktree_dir,
        branch_name,
    })
}

/// Remove a git worktree.
///
/// Equivalent to: git worktree remove --force <worktree_path>
/// Followed by: git branch -D <branch_name>
pub async fn remove_worktree(
    repo_path: &Path,
    worktree_path: &Path,
    branch_name: &str,
) -> Result<(), GitWorktreeError> {
    // Step 1: Remove worktree registration
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["worktree", "remove", "--force"])
        .arg(worktree_path)
        .output()
        .await?;

    if !output.status.success() {
        // If removal fails, force-clean the directory
        if worktree_path.exists() {
            tokio::fs::remove_dir_all(worktree_path).await?;
        }
    }

    // Step 2: Prune stale worktree entries
    let _ = Command::new("git")
        .current_dir(repo_path)
        .args(["worktree", "prune"])
        .output()
        .await;

    // Step 3: Delete the branch
    let _ = Command::new("git")
        .current_dir(repo_path)
        .args(["branch", "-D", branch_name])
        .output()
        .await;

    Ok(())
}

/// List all active worktrees.
///
/// Parses output of: git worktree list --porcelain
pub async fn list_worktrees(
    repo_path: &Path,
) -> Result<Vec<WorktreeInfo>, GitWorktreeError> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitWorktreeError::CommandFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(PathBuf::from(path));
        } else if let Some(branch) = line.strip_prefix("branch refs/heads/") {
            current_branch = Some(branch.to_string());
        } else if line.is_empty() {
            if let (Some(path), Some(branch)) = (current_path.take(), current_branch.take()) {
                worktrees.push(WorktreeInfo {
                    worktree_path: path,
                    branch_name: branch,
                });
            }
        }
    }

    Ok(worktrees)
}
```

### WorktreeService (in `composer-services`)

The service layer adds database tracking and per-path locking (following vibe-kanban's pattern):

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct WorktreeService {
    db: Arc<Database>,
    /// Per-path locks to prevent concurrent creation of the same worktree
    creation_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl WorktreeService {
    pub async fn create_for_session(
        &self,
        repo_path: &str,
        agent_name: &str,
        agent_id: Uuid,
        session_id: Uuid,
    ) -> Result<Worktree> {
        let worktree_name = format!("{}-{}", agent_name, &session_id.to_string()[..8]);

        // Acquire per-path lock
        let lock = {
            let mut locks = self.creation_locks.lock().await;
            locks.entry(worktree_name.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _guard = lock.lock().await;

        // Create git worktree
        let info = composer_git::worktree::create_worktree(
            Path::new(repo_path),
            &worktree_name,
            None,
        ).await?;

        // Persist to database
        let worktree = self.db.worktrees.create(
            agent_id,
            session_id,
            repo_path.to_string(),
            info.worktree_path.to_string_lossy().to_string(),
            info.branch_name,
        ).await?;

        Ok(worktree)
    }

    pub async fn cleanup(&self, worktree_id: Uuid) -> Result<()> {
        let wt = self.db.worktrees.find_by_id(worktree_id).await?;

        // Remove from filesystem
        composer_git::worktree::remove_worktree(
            Path::new(&wt.repo_path),
            Path::new(&wt.worktree_path),
            &wt.branch_name,
        ).await?;

        // Mark as deleted in DB
        self.db.worktrees.update_status(worktree_id, WorktreeStatus::Deleted).await?;

        Ok(())
    }
}
```

### Worktree Naming Convention

```
<repo_root>/.composer/worktrees/<agent_name>-<session_id_short>/
```

Example:
```
Q:\src\Substrate\.composer\worktrees\claude-1-a3f8b2c1\
```

Branch naming:
```
composer/claude-1-a3f8b2c1
```

---

## 6. Frontend Architecture

### Technology Stack

| Library | Version | Purpose |
|---------|---------|---------|
| React | 18 | UI framework |
| TypeScript | 5.x | Type safety |
| TanStack Router | latest | File-based routing |
| TanStack Query | latest | Server state management |
| Zustand | latest | Client state management (WebSocket, UI state) |
| Tailwind CSS | 4.x | Styling |
| shadcn/ui | latest | Component library (Radix-based) |
| Vite | 6.x | Build tool |

### Route Structure

```
/                    -> TaskBoard (default view)
/agents              -> AgentPool
/sessions            -> SessionList
/sessions/:id        -> SessionDetail (with live output)
```

### React Components

```
src/
├── app/
│   ├── App.tsx                   -- Root component with router + providers
│   ├── routes/
│   │   ├── __root.tsx            -- Root layout with sidebar navigation
│   │   ├── index.tsx             -- TaskBoard page (default)
│   │   ├── agents.tsx            -- AgentPool page
│   │   ├── sessions/
│   │   │   ├── index.tsx         -- SessionList page
│   │   │   └── $sessionId.tsx    -- SessionDetail page
│   │   └── routeTree.gen.ts      -- Auto-generated route tree
│   └── main.tsx                  -- Entry point, render App
│
├── components/
│   ├── layout/
│   │   ├── Sidebar.tsx           -- Navigation sidebar
│   │   ├── Header.tsx            -- Top bar with project info
│   │   └── MainLayout.tsx        -- Sidebar + content wrapper
│   │
│   ├── task-board/
│   │   ├── TaskBoard.tsx         -- Kanban board container
│   │   ├── TaskColumn.tsx        -- Single column (Backlog/InProgress/Waiting/Done)
│   │   ├── TaskCard.tsx          -- Individual task card
│   │   ├── TaskCreateDialog.tsx  -- Create task modal
│   │   ├── TaskEditDialog.tsx    -- Edit task modal
│   │   └── TaskAssignMenu.tsx    -- Agent assignment dropdown
│   │
│   ├── agents/
│   │   ├── AgentPool.tsx         -- Agent pool dashboard
│   │   ├── AgentCard.tsx         -- Agent status card
│   │   ├── AgentRegisterDialog.tsx -- Add new agent
│   │   └── AgentHealthBadge.tsx  -- Health/auth status indicator
│   │
│   ├── sessions/
│   │   ├── SessionList.tsx       -- All sessions overview
│   │   ├── SessionCard.tsx       -- Session summary card
│   │   ├── SessionDetail.tsx     -- Full session view with output
│   │   └── SessionOutput.tsx     -- Live-streaming agent output
│   │
│   └── ui/                       -- shadcn/ui components
│       ├── button.tsx
│       ├── card.tsx
│       ├── dialog.tsx
│       ├── badge.tsx
│       ├── select.tsx
│       ├── input.tsx
│       ├── textarea.tsx
│       ├── dropdown-menu.tsx
│       ├── scroll-area.tsx
│       └── separator.tsx
│
├── hooks/
│   ├── use-tasks.ts              -- TanStack Query hooks for tasks
│   ├── use-agents.ts             -- TanStack Query hooks for agents
│   ├── use-sessions.ts           -- TanStack Query hooks for sessions
│   └── use-websocket.ts          -- WebSocket connection + event dispatch
│
├── stores/
│   ├── ws-store.ts               -- Zustand store for WebSocket state
│   ├── ui-store.ts               -- Zustand store for UI state (selected task, etc.)
│   └── session-output-store.ts   -- Zustand store for live session output buffers
│
├── lib/
│   ├── api.ts                    -- Fetch wrapper for REST calls
│   ├── ws.ts                     -- WebSocket client (connect, reconnect, parse)
│   └── utils.ts                  -- Utility functions
│
├── types/
│   └── generated.ts              -- Auto-generated from ts-rs (DO NOT EDIT)
│
└── styles/
    └── globals.css               -- Tailwind base + custom styles
```

### Key Component Details

**TaskBoard.tsx** -- The main Kanban board:

```tsx
export function TaskBoard() {
    const { data: tasks } = useTasks();
    const columns: TaskStatus[] = ['backlog', 'in_progress', 'waiting', 'done'];

    return (
        <div className="flex gap-4 h-full overflow-x-auto p-4">
            {columns.map(status => (
                <TaskColumn
                    key={status}
                    status={status}
                    tasks={tasks?.filter(t => t.status === status) ?? []}
                />
            ))}
        </div>
    );
}
```

**SessionOutput.tsx** -- Live-streaming agent output:

```tsx
export function SessionOutput({ sessionId }: { sessionId: string }) {
    const output = useSessionOutputStore(state => state.outputs[sessionId] ?? []);
    const scrollRef = useRef<HTMLDivElement>(null);

    // Auto-scroll to bottom on new output
    useEffect(() => {
        scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
    }, [output.length]);

    return (
        <ScrollArea ref={scrollRef} className="h-[600px] font-mono text-sm">
            {output.map((line, i) => (
                <div key={i} className={cn(
                    "py-0.5 px-2",
                    line.log_type === 'stderr' && "text-red-400",
                    line.log_type === 'control' && "text-blue-400",
                )}>
                    {line.content}
                </div>
            ))}
        </ScrollArea>
    );
}
```

### State Management Architecture

```
                         ┌─────────────┐
  REST API ◄────────────►│ TanStack    │ Tasks, Agents, Sessions
  (fetch)                │ Query       │ (server state, cached)
                         └─────────────┘

                         ┌─────────────┐
  WebSocket ◄───────────►│ Zustand     │ Live output, connection state
  (streaming)            │ Stores      │ (client state, ephemeral)
                         └─────────────┘
```

**TanStack Query** handles all CRUD operations and caching. WebSocket events trigger query invalidation to keep data fresh:

```typescript
// hooks/use-websocket.ts
function handleWsEvent(event: WsEvent, queryClient: QueryClient) {
    switch (event.type) {
        case 'TaskCreated':
        case 'TaskUpdated':
        case 'TaskDeleted':
        case 'TaskMoved':
            queryClient.invalidateQueries({ queryKey: ['tasks'] });
            break;
        case 'AgentStatusChanged':
            queryClient.invalidateQueries({ queryKey: ['agents'] });
            break;
        case 'SessionOutput':
            // Append to Zustand store (not TanStack -- too high-frequency)
            useSessionOutputStore.getState().append(
                event.payload.session_id,
                event.payload
            );
            break;
        // ... etc
    }
}
```

---

## 7. Project Directory Structure

```
Q:\src\composer\
├── Cargo.toml                          # Workspace root
├── Cargo.lock
├── package.json                        # Root: pnpm workspace, scripts
├── pnpm-workspace.yaml                 # Defines packages/*
├── pnpm-lock.yaml
├── .gitignore
├── README.md
├── CLAUDE.md                           # Agent instructions for this project
│
├── crates/
│   ├── server/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs                 # Entry point: parse args, start server
│   │       ├── lib.rs                  # App state, router composition
│   │       ├── error.rs                # Axum error handling
│   │       └── routes/
│   │           ├── mod.rs              # Router tree assembly
│   │           ├── tasks.rs            # /api/tasks/* handlers
│   │           ├── agents.rs           # /api/agents/* handlers
│   │           ├── sessions.rs         # /api/sessions/* handlers
│   │           ├── worktrees.rs        # /api/worktrees/* handlers
│   │           ├── ws.rs               # WebSocket upgrade + event loop
│   │           ├── frontend.rs         # rust-embed SPA serving
│   │           └── health.rs           # /api/health
│   │
│   ├── db/
│   │   ├── Cargo.toml
│   │   ├── .sqlx/                      # SQLx offline query data
│   │   ├── migrations/
│   │   │   └── 001_initial_schema.sql
│   │   └── src/
│   │       ├── lib.rs                  # Database pool init, migration runner
│   │       └── models/
│   │           ├── mod.rs
│   │           ├── task.rs             # Task CRUD queries
│   │           ├── agent.rs            # Agent CRUD queries
│   │           ├── session.rs          # Session CRUD queries
│   │           ├── worktree.rs         # Worktree CRUD queries
│   │           └── session_log.rs      # Session log append/query
│   │
│   ├── services/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # ServiceContainer with all services
│   │       ├── task_service.rs         # Task business logic + status transitions
│   │       ├── agent_service.rs        # Agent pool + discovery
│   │       ├── session_service.rs      # Session orchestration (create/resume/interrupt)
│   │       ├── worktree_service.rs     # Worktree lifecycle with locking
│   │       └── event_bus.rs            # broadcast::Sender<WsEvent> wrapper
│   │
│   ├── executors/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Module declarations
│   │       ├── process_manager.rs      # AgentProcessManager: spawn, interrupt, cleanup
│   │       ├── protocol.rs             # ProtocolPeer: stdin/stdout JSON message loop
│   │       ├── types.rs                # Claude Code protocol message types
│   │       ├── discovery.rs            # Auto-discover installed agents (npx, claude)
│   │       └── error.rs               # Executor error types
│   │
│   ├── api-types/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # All shared types (models, enums, requests)
│   │       └── events.rs              # WsEvent, WsCommand enums
│   │
│   └── git/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           └── worktree.rs             # Git worktree create/remove/list
│
├── packages/
│   └── web/
│       ├── package.json                # React app dependencies
│       ├── tsconfig.json
│       ├── vite.config.ts
│       ├── tailwind.config.ts
│       ├── postcss.config.js
│       ├── components.json             # shadcn/ui config
│       ├── index.html
│       └── src/
│           ├── main.tsx
│           ├── app/
│           │   ├── App.tsx
│           │   └── routes/
│           │       ├── __root.tsx
│           │       ├── index.tsx        # TaskBoard
│           │       ├── agents.tsx       # AgentPool
│           │       └── sessions/
│           │           ├── index.tsx
│           │           └── $sessionId.tsx
│           ├── components/
│           │   ├── layout/
│           │   ├── task-board/
│           │   ├── agents/
│           │   ├── sessions/
│           │   └── ui/
│           ├── hooks/
│           ├── stores/
│           ├── lib/
│           ├── types/
│           │   └── generated.ts         # ts-rs output
│           └── styles/
│               └── globals.css
│
├── shared/
│   └── types.ts                         # ts-rs generated types (symlink or copy target)
│
└── scripts/
    ├── generate-types.sh                # Run ts-rs export, copy to shared/
    └── build.sh                         # Full build: pnpm build + cargo build --release
```

---

## 8. Build System

### How pnpm + cargo work together

The build produces a **single binary** by:

1. **pnpm builds the React app** into `packages/web/dist/`
2. **rust-embed** compiles `packages/web/dist/` into the Rust binary at compile time
3. **The Axum server** serves these embedded assets at `/` and falls back to `index.html` for SPA routing

### Root `package.json`

```json
{
    "name": "composer",
    "version": "0.1.0",
    "private": true,
    "packageManager": "pnpm@9.15.0",
    "engines": { "node": ">=20" },
    "scripts": {
        "dev": "concurrently \"pnpm run dev:web\" \"pnpm run dev:server\"",
        "dev:web": "cd packages/web && pnpm run dev",
        "dev:server": "cargo watch -w crates -x 'run --bin composer-server'",
        "build:web": "cd packages/web && pnpm run build",
        "build:server": "cargo build --release --bin composer-server",
        "build": "pnpm run build:web && pnpm run build:server",
        "generate-types": "cargo test --workspace -- export_bindings && node scripts/copy-types.js",
        "lint": "cd packages/web && pnpm run lint",
        "format": "cargo fmt --all && cd packages/web && pnpm run format"
    },
    "devDependencies": {
        "concurrently": "^9.0.0"
    }
}
```

### `pnpm-workspace.yaml`

```yaml
packages:
    - "packages/*"
```

### `packages/web/vite.config.ts`

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { TanStackRouterVite } from '@tanstack/router-plugin/vite';
import path from 'path';

export default defineConfig({
    plugins: [
        TanStackRouterVite(),
        react(),
    ],
    resolve: {
        alias: {
            '@': path.resolve(__dirname, './src'),
        },
    },
    server: {
        port: 5173,
        proxy: {
            '/api': {
                target: 'http://127.0.0.1:3000',
                ws: true,
            },
        },
    },
    build: {
        outDir: 'dist',
        emptyOutDir: true,
    },
});
```

### Frontend Embedding in Rust

```rust
// crates/server/src/routes/frontend.rs
use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../packages/web/dist"]
struct FrontendAssets;

pub async fn serve_frontend(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    serve_file(path).await
}

pub async fn serve_frontend_root() -> impl IntoResponse {
    serve_file("index.html").await
}

async fn serve_file(path: &str) -> Response {
    match FrontendAssets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(file.data.to_vec()))
                .unwrap()
        }
        None => {
            // SPA fallback: serve index.html for client-side routing
            match FrontendAssets::get("index.html") {
                Some(index) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html")
                    .body(Body::from(index.data.to_vec()))
                    .unwrap(),
                None => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap(),
            }
        }
    }
}
```

### Server Entry Point

```rust
// crates/server/src/main.rs
use std::net::SocketAddr;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("composer=debug".parse()?))
        .init();

    // Database setup
    let db = composer_db::Database::connect("sqlite:composer.db?mode=rwc").await?;
    db.run_migrations().await?;

    // Services
    let event_bus = composer_services::EventBus::new();
    let services = composer_services::ServiceContainer::new(db, event_bus.clone());

    // Router
    let app = Router::new()
        .merge(routes::api_router(services.clone(), event_bus))
        .merge(routes::frontend_router())
        .layer(CorsLayer::new().allow_origin(Any))
        .layer(TraceLayer::new_for_http());

    // Bind and serve
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Composer listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

### Development Workflow

**First-time setup:**
```bash
# Install pnpm if needed
npm install -g pnpm

# Install frontend dependencies
pnpm install

# Install cargo-watch for hot reload
cargo install cargo-watch

# Install sqlx-cli for migrations
cargo install sqlx-cli --features sqlite

# Run initial migration
cd crates/db && sqlx database create --database-url sqlite:../../composer.db
cd crates/db && sqlx migrate run --database-url sqlite:../../composer.db
```

**Daily development:**
```bash
# Start both frontend and backend with hot reload
pnpm run dev

# This runs concurrently:
#   - Vite dev server on :5173 (proxies /api to :3000)
#   - cargo watch rebuilds on file changes
```

**Production build:**
```bash
# Build frontend, then embed into Rust binary
pnpm run build

# Output: target/release/composer-server (or composer-server.exe on Windows)
# This single binary contains the entire app
```

### TypeScript Type Generation

```bash
# In Cargo.toml of api-types:
[dev-dependencies]
ts-rs = { workspace = true }

# Types are generated via test:
# Each struct with #[derive(TS)] and #[ts(export)] produces a .ts file
cargo test --package composer-api-types -- export_bindings

# Then copy to frontend:
cp crates/api-types/bindings/*.ts packages/web/src/types/generated.ts
```

### Windows-Specific Notes

1. **Path handling** -- All paths stored in SQLite use forward slashes. The Rust code normalizes paths at the boundary.
2. **npx on Windows** -- `npx.cmd` is the actual executable. Use `which::which("npx")` to resolve cross-platform.
3. **Git worktrees on Windows** -- Long paths can be an issue. Keep worktree paths short (`.composer/worktrees/<short-name>`).
4. **Process spawning** -- `tokio::process::Command` handles Windows process creation correctly. Claude Code spawned via npx inherits the console allocation.
5. **SQLite file location** -- `composer.db` lives in the working directory. For production, use an explicit path like `%LOCALAPPDATA%\composer\composer.db`.

---

## Implementation Sequence

The recommended build order, accounting for dependencies:

| Phase | Crate/Package | Features Covered | Estimated Effort |
|-------|--------------|-----------------|-----------------|
| 1 | `composer-api-types` | Shared types, ts-rs setup | 0.5 day |
| 2 | `composer-db` | SQLite schema, migrations, model CRUD | 1 day |
| 3 | `composer-git` | Worktree create/remove/list | 0.5 day |
| 4 | `composer-executors` | Claude Code protocol, process spawn | 2 days |
| 5 | `composer-services` | Business logic glue, event bus | 1.5 days |
| 6 | `composer-server` | Axum routes, WebSocket, frontend embed | 1.5 days |
| 7 | `packages/web` scaffolding | Vite + React + Tailwind + shadcn setup | 0.5 day |
| 8 | `packages/web` Task Board | F5.1 + F5.5: Kanban board UI with CRUD | 1.5 days |
| 9 | `packages/web` Agent Pool | F2.1: Agent registration + health display | 1 day |
| 10 | `packages/web` Sessions | F2.2 + F2.5: Live dashboard + session output | 1.5 days |
| 11 | Integration | F5.2 + F8.3: Task-agent binding, auto-yes end-to-end | 1 day |
| 12 | Polish | Error handling, edge cases, Windows testing | 1 day |

**Total: ~12 days**

---

## Key Architectural Decisions and Rationale

1. **SQLite instead of PostgreSQL** -- The prototype runs on a single DevBox. SQLite is zero-configuration, embedded, and sufficient for the expected scale (tens of tasks, handful of agents). Migration to PostgreSQL later is straightforward since SQLx supports both.

2. **Git CLI instead of libgit2** -- `git2` (libgit2 Rust bindings) has incomplete worktree support and inconsistent behavior on Windows. Shelling out to `git` is more reliable and matches how vibe-kanban handles worktrees in production.

3. **WebSocket for real-time instead of SSE** -- While vibe-kanban uses SSE, WebSocket is chosen here because we need bidirectional communication (client subscribes/unsubscribes to specific sessions). SSE is server-to-client only.

4. **Zustand for streaming output, TanStack Query for CRUD** -- Agent output arrives at high frequency (multiple lines per second). Putting this into TanStack Query would cause excessive re-renders and cache invalidation. Zustand provides a lightweight, subscription-based store that components can read directly.

5. **Single binary via rust-embed** -- Following the vibe-kanban pattern exactly. The compiled React app is embedded as static bytes in the Rust binary. No need for a separate web server, Docker container, or file deployment.

6. **`BypassPermissions` mode for auto-yes** -- Claude Code's protocol supports a `BypassPermissions` permission mode that disables all tool-use prompts. This is the cleanest way to implement F8.3 without having to parse and respond to every individual permission request.

---

Sources:
- [BloopAI/vibe-kanban GitHub Repository](https://github.com/BloopAI/vibe-kanban)
- [vibe-kanban AGENTS.md](https://github.com/BloopAI/vibe-kanban/blob/main/AGENTS.md)

### Critical Files for Implementation
- `Q:/src/composer/crates/executors/src/process_manager.rs` - Core of agent lifecycle: spawn Claude Code subprocess, manage stdin/stdout protocol, handle auto-yes. This is the most complex and highest-risk component.
- `Q:/src/composer/crates/executors/src/types.rs` - Claude Code protocol message types. Must exactly match the Claude Code CLI's JSON schema or communication will fail silently.
- `Q:/src/composer/crates/db/migrations/001_initial_schema.sql` - Foundation schema for all four entities (agents, tasks, sessions, worktrees). Every service and route depends on this being correct.
- `Q:/src/composer/crates/server/src/routes/ws.rs` - WebSocket hub that multiplexes all real-time events to connected frontends. Critical for the live dashboard (F2.2) and session streaming.
- `Q:/src/composer/packages/web/src/components/task-board/TaskBoard.tsx` - Primary UI surface. The Kanban board is what users interact with most and drives task-agent binding (F5.2).
agentId: af6ba35c896e581f3 (for resuming to continue this agent's work if needed)
<usage>total_tokens: 91353
tool_uses: 47
duration_ms: 500447</usage>
