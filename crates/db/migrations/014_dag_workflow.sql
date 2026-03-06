-- DAG-based workflow engine migration.
-- This is a breaking change: all in-progress workflow runs are failed,
-- existing workflow definitions are deleted and re-seeded on startup.

-- Fail all in-progress workflow runs
UPDATE workflow_runs SET status = 'failed', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE status IN ('running', 'paused');

-- Add is_template to workflows
ALTER TABLE workflows ADD COLUMN is_template INTEGER NOT NULL DEFAULT 0;

-- Delete all existing workflow definitions (will be re-seeded with DAG format)
DELETE FROM workflows;

-- Recreate workflow_runs table: drop current_step_index and main_session_id, add activated_steps
PRAGMA foreign_keys = OFF;

CREATE TABLE workflow_runs_new (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'running',
    iteration_count INTEGER NOT NULL DEFAULT 0,
    activated_steps TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Don't migrate old runs (they were failed above and definitions deleted)
DROP TABLE workflow_runs;
ALTER TABLE workflow_runs_new RENAME TO workflow_runs;

-- Recreate workflow_step_outputs table: step_index -> step_id
CREATE TABLE workflow_step_outputs_new (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_run_id TEXT NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
    step_id TEXT NOT NULL,
    step_type TEXT NOT NULL,
    output TEXT,
    attempt INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'pending',
    session_id TEXT REFERENCES sessions(id),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

DROP TABLE workflow_step_outputs;
ALTER TABLE workflow_step_outputs_new RENAME TO workflow_step_outputs;

PRAGMA foreign_keys = ON;
