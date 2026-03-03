CREATE TABLE agents (
    id              TEXT PRIMARY KEY NOT NULL,
    name            TEXT NOT NULL,
    agent_type      TEXT NOT NULL DEFAULT 'claude_code',
    executable_path TEXT,
    status          TEXT NOT NULL DEFAULT 'idle',
    auth_status     TEXT NOT NULL DEFAULT 'unknown',
    last_heartbeat  TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE tasks (
    id              TEXT PRIMARY KEY NOT NULL,
    title           TEXT NOT NULL,
    description     TEXT,
    status          TEXT NOT NULL DEFAULT 'backlog',
    priority        INTEGER NOT NULL DEFAULT 0,
    assigned_agent_id TEXT REFERENCES agents(id) ON DELETE SET NULL,
    position        REAL NOT NULL DEFAULT 0.0,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_assigned_agent ON tasks(assigned_agent_id);

CREATE TABLE worktrees (
    id              TEXT PRIMARY KEY NOT NULL,
    agent_id        TEXT REFERENCES agents(id) ON DELETE SET NULL,
    session_id      TEXT,
    repo_path       TEXT NOT NULL,
    worktree_path   TEXT NOT NULL UNIQUE,
    branch_name     TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_worktrees_agent ON worktrees(agent_id);

CREATE TABLE sessions (
    id              TEXT PRIMARY KEY NOT NULL,
    agent_id        TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    task_id         TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    worktree_id     TEXT REFERENCES worktrees(id) ON DELETE SET NULL,
    status          TEXT NOT NULL DEFAULT 'created',
    resume_session_id TEXT,
    prompt          TEXT,
    result_summary  TEXT,
    started_at      TEXT,
    completed_at    TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_sessions_agent ON sessions(agent_id);
CREATE INDEX idx_sessions_task ON sessions(task_id);

CREATE TABLE session_logs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    log_type        TEXT NOT NULL,
    content         TEXT NOT NULL,
    timestamp       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_session_logs_session ON session_logs(session_id);
