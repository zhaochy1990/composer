-- Add completed_at timestamp to tasks for tracking when a task moves to done
ALTER TABLE tasks ADD COLUMN completed_at TEXT;

-- Backfill existing done tasks with their updated_at as best approximation
UPDATE tasks SET completed_at = updated_at WHERE status = 'done';
