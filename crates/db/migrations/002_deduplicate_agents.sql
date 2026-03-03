-- Remove duplicate agents, keeping only the oldest one per agent_type.
-- First, update any foreign keys pointing to duplicates to point to the keeper.
UPDATE sessions SET agent_id = (
    SELECT a2.id FROM agents a2
    WHERE a2.agent_type = (SELECT agent_type FROM agents WHERE id = sessions.agent_id)
    ORDER BY a2.created_at ASC
    LIMIT 1
)
WHERE agent_id IN (
    SELECT id FROM agents WHERE id NOT IN (
        SELECT id FROM (
            SELECT id, ROW_NUMBER() OVER (PARTITION BY agent_type ORDER BY created_at ASC) AS rn
            FROM agents
        ) WHERE rn = 1
    )
);

UPDATE worktrees SET agent_id = (
    SELECT a2.id FROM agents a2
    WHERE a2.agent_type = (SELECT agent_type FROM agents WHERE id = worktrees.agent_id)
    ORDER BY a2.created_at ASC
    LIMIT 1
)
WHERE agent_id IS NOT NULL AND agent_id IN (
    SELECT id FROM agents WHERE id NOT IN (
        SELECT id FROM (
            SELECT id, ROW_NUMBER() OVER (PARTITION BY agent_type ORDER BY created_at ASC) AS rn
            FROM agents
        ) WHERE rn = 1
    )
);

-- Delete the duplicate agents
DELETE FROM agents WHERE id NOT IN (
    SELECT id FROM (
        SELECT id, ROW_NUMBER() OVER (PARTITION BY agent_type ORDER BY created_at ASC) AS rn
        FROM agents
    ) WHERE rn = 1
);

-- Add unique index on agent_type to prevent future duplicates
CREATE UNIQUE INDEX IF NOT EXISTS idx_agents_agent_type ON agents(agent_type);
