-- Make workflows global by removing the project_id requirement.
-- SQLite doesn't support ALTER COLUMN, so we recreate the table.
--
-- Safety: temporarily disable FK enforcement to avoid CASCADE deletes
-- on workflow_runs/workflow_step_outputs when dropping the old table.
-- The workflow IDs are preserved (copied with same id), so all FK
-- references remain valid after the swap.

PRAGMA foreign_keys = OFF;

CREATE TABLE IF NOT EXISTS workflows_new (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    definition TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO workflows_new (id, name, definition, created_at, updated_at)
    SELECT id, name, definition, created_at, updated_at FROM workflows;

DROP TABLE workflows;
ALTER TABLE workflows_new RENAME TO workflows;

PRAGMA foreign_keys = ON;
