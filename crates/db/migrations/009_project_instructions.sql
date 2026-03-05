CREATE TABLE IF NOT EXISTS project_instructions (
    id           TEXT PRIMARY KEY NOT NULL,
    project_id   TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title        TEXT NOT NULL,
    content      TEXT NOT NULL,
    sort_order   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_project_instructions_project ON project_instructions(project_id);
