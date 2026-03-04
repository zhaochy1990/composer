-- Add agent configuration fields to tasks table
ALTER TABLE tasks ADD COLUMN repo_path TEXT;
ALTER TABLE tasks ADD COLUMN auto_approve INTEGER NOT NULL DEFAULT 1;
