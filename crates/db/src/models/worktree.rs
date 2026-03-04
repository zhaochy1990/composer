use composer_api_types::*;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct WorktreeRow {
    id: String,
    agent_id: Option<String>,
    session_id: Option<String>,
    repo_path: String,
    worktree_path: String,
    branch_name: String,
    status: String,
    created_at: String,
    updated_at: String,
}

impl TryFrom<WorktreeRow> for Worktree {
    type Error = anyhow::Error;

    fn try_from(row: WorktreeRow) -> Result<Self, Self::Error> {
        Ok(Worktree {
            id: row.id.parse()?,
            agent_id: row.agent_id.map(|s| s.parse()).transpose()?,
            session_id: row.session_id.map(|s| s.parse()).transpose()?,
            repo_path: row.repo_path,
            worktree_path: row.worktree_path,
            branch_name: row.branch_name,
            status: serde_json::from_value(serde_json::Value::String(row.status))?,
            created_at: row.created_at.parse()?,
            updated_at: row.updated_at.parse()?,
        })
    }
}

pub async fn create(
    pool: &SqlitePool,
    agent_id: &str,
    session_id: &str,
    repo_path: &str,
    worktree_path: &str,
    branch_name: &str,
) -> anyhow::Result<Worktree> {
    let id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO worktrees (id, agent_id, session_id, repo_path, worktree_path, branch_name) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(agent_id)
    .bind(session_id)
    .bind(repo_path)
    .bind(worktree_path)
    .bind(branch_name)
    .execute(pool)
    .await?;

    find_by_id(pool, &id).await?.ok_or_else(|| anyhow::anyhow!("Failed to create worktree"))
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Worktree>> {
    let row = sqlx::query_as::<_, WorktreeRow>("SELECT * FROM worktrees WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(Worktree::try_from).transpose()
}

pub async fn find_by_session(pool: &SqlitePool, session_id: &str) -> anyhow::Result<Option<Worktree>> {
    let row = sqlx::query_as::<_, WorktreeRow>("SELECT * FROM worktrees WHERE session_id = ?")
        .bind(session_id)
        .fetch_optional(pool)
        .await?;
    row.map(Worktree::try_from).transpose()
}

pub async fn list_all(pool: &SqlitePool) -> anyhow::Result<Vec<Worktree>> {
    let rows = sqlx::query_as::<_, WorktreeRow>("SELECT * FROM worktrees ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    rows.into_iter().map(Worktree::try_from).collect()
}

pub async fn update_status(pool: &SqlitePool, id: &str, status: &WorktreeStatus) -> anyhow::Result<()> {
    let status_str = serde_json::to_value(status)?
        .as_str().unwrap_or("active").to_string();
    sqlx::query("UPDATE worktrees SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(&status_str)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
