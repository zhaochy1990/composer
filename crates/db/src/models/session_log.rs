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
        .as_str()
        .unwrap_or("stdout")
        .to_string();
    sqlx::query("INSERT INTO session_logs (session_id, log_type, content) VALUES (?, ?, ?)")
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
    let limit = limit.unwrap_or(5000).min(5000);
    let offset = offset.unwrap_or(0);

    // Use a subquery to get the most recent `limit` rows, then re-sort ASC for display order.
    // Without this, ORDER BY id ASC LIMIT N returns the *oldest* N rows, missing recent messages.
    let rows = if let Some(since) = since {
        sqlx::query_as::<_, SessionLogRow>(
            "SELECT * FROM (SELECT * FROM session_logs WHERE session_id = ? AND timestamp > ? ORDER BY id DESC LIMIT ? OFFSET ?) sub ORDER BY id ASC"
        )
        .bind(session_id)
        .bind(since)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, SessionLogRow>(
            "SELECT * FROM (SELECT * FROM session_logs WHERE session_id = ? ORDER BY id DESC LIMIT ? OFFSET ?) sub ORDER BY id ASC",
        )
        .bind(session_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };
    rows.into_iter().map(SessionLog::try_from).collect()
}

/// Count total logs for a session.
pub async fn count_by_session(pool: &SqlitePool, session_id: &str) -> anyhow::Result<i64> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM session_logs WHERE session_id = ?")
            .bind(session_id)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

/// Cursor-based pagination: fetch `limit` rows ordered by id DESC (newest first),
/// optionally before a given id. Returns results re-sorted in ASC order for display.
pub async fn list_by_session_cursor(
    pool: &SqlitePool,
    session_id: &str,
    before: Option<i64>,
    limit: i64,
) -> anyhow::Result<Vec<SessionLog>> {
    let rows = if let Some(before_id) = before {
        sqlx::query_as::<_, SessionLogRow>(
            "SELECT * FROM (SELECT * FROM session_logs WHERE session_id = ? AND id < ? ORDER BY id DESC LIMIT ?) sub ORDER BY id ASC",
        )
        .bind(session_id)
        .bind(before_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, SessionLogRow>(
            "SELECT * FROM (SELECT * FROM session_logs WHERE session_id = ? ORDER BY id DESC LIMIT ?) sub ORDER BY id ASC",
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };
    rows.into_iter().map(SessionLog::try_from).collect()
}

/// Fetch all logs for a session in chronological order.
/// Safety limit of 50 000 rows prevents OOM on pathologically long sessions.
pub async fn list_all_by_session(
    pool: &SqlitePool,
    session_id: &str,
) -> anyhow::Result<Vec<SessionLog>> {
    let rows = sqlx::query_as::<_, SessionLogRow>(
        "SELECT * FROM session_logs WHERE session_id = ? ORDER BY id ASC LIMIT 50000",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(SessionLog::try_from).collect()
}

/// Check if there are logs with id < before_id for a session.
pub async fn has_logs_before(
    pool: &SqlitePool,
    session_id: &str,
    before_id: i64,
) -> anyhow::Result<bool> {
    let row: (i64,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM session_logs WHERE session_id = ? AND id < ?)",
    )
    .bind(session_id)
    .bind(before_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0 == 1)
}
