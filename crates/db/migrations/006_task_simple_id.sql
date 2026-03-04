-- Add prefix + counter to projects
ALTER TABLE projects ADD COLUMN task_prefix TEXT NOT NULL DEFAULT 'TSK';
ALTER TABLE projects ADD COLUMN task_counter INTEGER NOT NULL DEFAULT 0;

-- Backfill task_prefix from project name (first 3 alpha chars uppercased, fallback TSK)
UPDATE projects SET task_prefix = COALESCE(
    NULLIF(
        UPPER(
            SUBSTR(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(
                name, '0',''), '1',''), '2',''), '3',''), '4',''), '5',''), '6',''), '7',''), '8',''), '9',''),
            1, 3)
        ),
        ''
    ),
    'TSK'
);
-- If fewer than 3 alpha chars remain, fall back to TSK
UPDATE projects SET task_prefix = 'TSK' WHERE LENGTH(task_prefix) < 3;

-- Add task_number + simple_id to tasks (default 0 / '' for new columns)
ALTER TABLE tasks ADD COLUMN task_number INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tasks ADD COLUMN simple_id TEXT NOT NULL DEFAULT '';

-- Backfill: assign sequential task_number per project, ordered by created_at
-- Uses a correlated subquery to compute a row number per project
UPDATE tasks SET task_number = (
    SELECT COUNT(*)
    FROM tasks AS t2
    WHERE t2.project_id = tasks.project_id
      AND t2.project_id IS NOT NULL
      AND (t2.created_at < tasks.created_at
           OR (t2.created_at = tasks.created_at AND t2.id <= tasks.id))
)
WHERE project_id IS NOT NULL;

-- Backfill simple_id from project prefix + task_number
UPDATE tasks SET simple_id = (
    SELECT p.task_prefix || '-' || tasks.task_number
    FROM projects p
    WHERE p.id = tasks.project_id
)
WHERE project_id IS NOT NULL AND task_number > 0;

-- Sync project task_counter to the max task_number assigned
UPDATE projects SET task_counter = COALESCE(
    (SELECT MAX(t.task_number) FROM tasks t WHERE t.project_id = projects.id),
    0
);

-- Index for quick lookup
CREATE INDEX idx_tasks_simple_id ON tasks(simple_id);
CREATE UNIQUE INDEX idx_tasks_project_task_number ON tasks(project_id, task_number);
