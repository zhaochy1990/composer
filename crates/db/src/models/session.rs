use composer_api_types::*;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: String,
    agent_id: String,
    task_id: Option<String>,
    worktree_id: Option<String>,
    status: String,
    resume_session_id: Option<String>,
    prompt: Option<String>,
    result_summary: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<SessionRow> for Session {
    type Error = anyhow::Error;

    fn try_from(row: SessionRow) -> Result<Self, Self::Error> {
        Ok(Session {
            id: row.id.parse()?,
            agent_id: row.agent_id.parse()?,
            task_id: row.task_id.map(|s| s.parse()).transpose()?,
            worktree_id: row.worktree_id.map(|s| s.parse()).transpose()?,
            status: serde_json::from_value(serde_json::Value::String(row.status))?,
            resume_session_id: row.resume_session_id,
            prompt: row.prompt,
            result_summary: row.result_summary,
            started_at: row.started_at.map(|s| s.parse()).transpose()?,
            completed_at: row.completed_at.map(|s| s.parse()).transpose()?,
            created_at: row.created_at.parse()?,
            updated_at: row.updated_at.parse()?,
        })
    }
}

pub async fn create(
    pool: &SqlitePool,
    agent_id: &str,
    task_id: Option<&str>,
    worktree_id: Option<&str>,
    prompt: &str,
) -> anyhow::Result<Session> {
    let id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO sessions (id, agent_id, task_id, worktree_id, prompt) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(agent_id)
    .bind(task_id)
    .bind(worktree_id)
    .bind(prompt)
    .execute(pool)
    .await?;

    find_by_id(pool, &id).await?.ok_or_else(|| anyhow::anyhow!("Failed to create session"))
}

pub async fn create_with_status(
    pool: &SqlitePool,
    id: &str,
    agent_id: &str,
    task_id: Option<&str>,
    worktree_id: Option<&str>,
    prompt: &str,
    status: &SessionStatus,
) -> anyhow::Result<Session> {
    let status_str = match status {
        SessionStatus::Created => "created",
        SessionStatus::Running => "running",
        SessionStatus::Paused => "paused",
        SessionStatus::Completed => "completed",
        SessionStatus::Failed => "failed",
    };

    sqlx::query(
        "INSERT INTO sessions (id, agent_id, task_id, worktree_id, prompt, status, \
         started_at) VALUES (?, ?, ?, ?, ?, ?, \
         CASE WHEN ? = 'running' THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now') ELSE NULL END)"
    )
    .bind(id)
    .bind(agent_id)
    .bind(task_id)
    .bind(worktree_id)
    .bind(prompt)
    .bind(status_str)
    .bind(status_str)
    .execute(pool)
    .await?;

    find_by_id(pool, id).await?.ok_or_else(|| anyhow::anyhow!("Failed to create session"))
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Session>> {
    let row = sqlx::query_as::<_, SessionRow>("SELECT * FROM sessions WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(Session::try_from).transpose()
}

pub async fn list_all(pool: &SqlitePool) -> anyhow::Result<Vec<Session>> {
    let rows = sqlx::query_as::<_, SessionRow>("SELECT * FROM sessions ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    rows.into_iter().map(Session::try_from).collect()
}

pub async fn list_by_agent(pool: &SqlitePool, agent_id: &str) -> anyhow::Result<Vec<Session>> {
    let rows = sqlx::query_as::<_, SessionRow>(
        "SELECT * FROM sessions WHERE agent_id = ? ORDER BY created_at DESC"
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(Session::try_from).collect()
}

pub async fn list_by_task(pool: &SqlitePool, task_id: &str) -> anyhow::Result<Vec<Session>> {
    let rows = sqlx::query_as::<_, SessionRow>(
        "SELECT * FROM sessions WHERE task_id = ? ORDER BY created_at DESC"
    )
    .bind(task_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(Session::try_from).collect()
}

pub async fn update_status(pool: &SqlitePool, id: &str, status: &SessionStatus) -> anyhow::Result<()> {
    let status_str = match status {
        SessionStatus::Created => "created",
        SessionStatus::Running => "running",
        SessionStatus::Paused => "paused",
        SessionStatus::Completed => "completed",
        SessionStatus::Failed => "failed",
    };

    // Use a single parameterized query with CASE expressions to conditionally set timestamps
    sqlx::query(
        "UPDATE sessions SET \
         status = ?, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), \
         started_at = CASE WHEN ? = 'running' THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now') ELSE started_at END, \
         completed_at = CASE WHEN ? IN ('completed', 'failed') THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now') ELSE completed_at END \
         WHERE id = ?"
    )
    .bind(status_str)
    .bind(status_str)
    .bind(status_str)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Mark all sessions in "running" status as "failed" (used on server startup to recover orphaned sessions).
pub async fn fail_orphaned_running(pool: &SqlitePool) -> anyhow::Result<u64> {
    let result = sqlx::query(
        "UPDATE sessions SET \
         status = 'failed', \
         result_summary = COALESCE(result_summary, 'Server restarted while session was running'), \
         completed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE status = 'running'"
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn update_result(pool: &SqlitePool, id: &str, result_summary: Option<&str>, resume_session_id: Option<&str>) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE sessions SET result_summary = ?, resume_session_id = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?"
    )
    .bind(result_summary)
    .bind(resume_session_id)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
