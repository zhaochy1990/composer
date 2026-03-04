use composer_api_types::*;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct AgentRow {
    id: String,
    name: String,
    agent_type: String,
    executable_path: Option<String>,
    status: String,
    auth_status: String,
    last_heartbeat: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<AgentRow> for Agent {
    type Error = anyhow::Error;

    fn try_from(row: AgentRow) -> Result<Self, Self::Error> {
        Ok(Agent {
            id: row.id.parse()?,
            name: row.name,
            agent_type: serde_json::from_value(serde_json::Value::String(row.agent_type))?,
            executable_path: row.executable_path,
            status: serde_json::from_value(serde_json::Value::String(row.status))?,
            auth_status: serde_json::from_value(serde_json::Value::String(row.auth_status))?,
            last_heartbeat: row.last_heartbeat.map(|s| s.parse()).transpose()?,
            created_at: row.created_at.parse()?,
            updated_at: row.updated_at.parse()?,
        })
    }
}

pub async fn create(
    pool: &SqlitePool,
    name: &str,
    agent_type: &AgentType,
    executable_path: Option<&str>,
) -> anyhow::Result<Agent> {
    let id = Uuid::new_v4().to_string();
    let agent_type_str = serde_json::to_value(agent_type)?
        .as_str().unwrap_or("claude_code").to_string();

    sqlx::query(
        "INSERT INTO agents (id, name, agent_type, executable_path) VALUES (?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(name)
    .bind(&agent_type_str)
    .bind(executable_path)
    .execute(pool)
    .await?;

    find_by_id(pool, &id).await?.ok_or_else(|| anyhow::anyhow!("Failed to create agent"))
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Agent>> {
    let row = sqlx::query_as::<_, AgentRow>("SELECT * FROM agents WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(Agent::try_from).transpose()
}

pub async fn find_by_agent_type(pool: &SqlitePool, agent_type: &AgentType) -> anyhow::Result<Option<Agent>> {
    let type_str = serde_json::to_value(agent_type)?
        .as_str().unwrap_or("claude_code").to_string();
    let row = sqlx::query_as::<_, AgentRow>("SELECT * FROM agents WHERE agent_type = ? LIMIT 1")
        .bind(&type_str)
        .fetch_optional(pool)
        .await?;
    row.map(Agent::try_from).transpose()
}

pub async fn update_executable_path(pool: &SqlitePool, id: &str, executable_path: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE agents SET executable_path = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(executable_path)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_all(pool: &SqlitePool) -> anyhow::Result<Vec<Agent>> {
    let rows = sqlx::query_as::<_, AgentRow>("SELECT * FROM agents ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    rows.into_iter().map(Agent::try_from).collect()
}

pub async fn update_status(pool: &SqlitePool, id: &str, status: &AgentStatus) -> anyhow::Result<()> {
    let status_str = serde_json::to_value(status)?
        .as_str().unwrap_or("idle").to_string();
    sqlx::query("UPDATE agents SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(&status_str)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_auth_status(pool: &SqlitePool, id: &str, auth_status: &AuthStatus) -> anyhow::Result<()> {
    let auth_str = serde_json::to_value(auth_status)?
        .as_str().unwrap_or("unknown").to_string();
    sqlx::query("UPDATE agents SET auth_status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(&auth_str)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Reset all "busy" agents to "idle" (used on server startup to recover from unclean shutdown).
pub async fn reset_all_busy_to_idle(pool: &SqlitePool) -> anyhow::Result<u64> {
    let result = sqlx::query(
        "UPDATE agents SET status = 'idle', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE status = 'busy'"
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn delete(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM agents WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
