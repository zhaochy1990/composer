-- Task links: symmetrical "Related" relationship between tasks.
-- Each link is stored once with task_id_a < task_id_b (lexicographic).
CREATE TABLE task_links (
    task_id_a TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    task_id_b TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (task_id_a, task_id_b),
    CHECK (task_id_a < task_id_b)
);

CREATE INDEX idx_task_links_b ON task_links(task_id_b);
