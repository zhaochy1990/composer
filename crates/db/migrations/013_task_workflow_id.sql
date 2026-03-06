-- Add workflow_id column to tasks to store the intended workflow template
-- (separate from workflow_run_id which tracks a running workflow instance)
ALTER TABLE tasks ADD COLUMN workflow_id TEXT REFERENCES workflows(id) ON DELETE SET NULL;
