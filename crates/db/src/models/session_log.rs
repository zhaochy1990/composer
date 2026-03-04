use composer_api_types::*;
use sqlx::SqlitePool;

#[derive(sqlx::FromRow)]
struct SessionLogRow {
    id: i64,
    session_id: String,
    log_type: String,
    content: String,
    timestamp: String,
}

impl TryFrom<SessionLogRow> for SessionLog {
    type Error = anyhow::Error;

    fn try_from(row: SessionLogRow) -> Result<Self, Self::Error> {
        Ok(SessionLog {
            id: row.id,
            session_id: row.session_id.parse()?,
            log_type: serde_json::from_value(serde_json::Value::String(row.log_type))?,
            content: row.content,
            timestamp: row.timestamp.parse()?,
        })
    }
}

pub async fn append(
    pool: &SqlitePool,
    session_id: &str,
    log_type: &LogType,
    content: &str,
) -> anyhow::Result<()> {
    let log_type_str = serde_json::to_value(log_type)?
        .as_str().unwrap_or("stdout").to_string();
    sqlx::query(
        "INSERT INTO session_logs (session_id, log_type, content) VALUES (?, ?, ?)"
    )
    .bind(session_id)
    .bind(&log_type_str)
    .bind(content)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_by_session(
    pool: &SqlitePool,
    session_id: &str,
    since: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> anyhow::Result<Vec<SessionLog>> {
    let limit = limit.unwrap_or(500).min(5000);
    let offset = offset.unwrap_or(0);

    let rows = if let Some(since) = since {
        sqlx::query_as::<_, SessionLogRow>(
            "SELECT * FROM session_logs WHERE session_id = ? AND timestamp > ? ORDER BY id ASC LIMIT ? OFFSET ?"
        )
        .bind(session_id)
        .bind(since)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, SessionLogRow>(
            "SELECT * FROM session_logs WHERE session_id = ? ORDER BY id ASC LIMIT ? OFFSET ?"
        )
        .bind(session_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };
    rows.into_iter().map(SessionLog::try_from).collect()
}
