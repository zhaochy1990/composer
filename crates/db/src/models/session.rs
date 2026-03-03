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
    let status_str = serde_json::to_value(status)?
        .as_str().unwrap_or("created").to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let started_update = match status {
        SessionStatus::Running => format!(", started_at = '{}'", now),
        _ => String::new(),
    };
    let completed_update = match status {
        SessionStatus::Completed | SessionStatus::Failed => format!(", completed_at = '{}'", now),
        _ => String::new(),
    };

    let sql = format!(
        "UPDATE sessions SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'){}{} WHERE id = ?",
        started_update, completed_update
    );
    sqlx::query(&sql)
        .bind(&status_str)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
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
