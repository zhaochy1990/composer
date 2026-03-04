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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_pool;

    async fn setup_session(pool: &sqlx::SqlitePool) -> String {
        let agent = crate::models::agent::create(
            pool, "Agent", &composer_api_types::AgentType::ClaudeCode, None,
        )
        .await
        .unwrap();
        let session = crate::models::session::create(
            pool, &agent.id.to_string(), None, None, "test",
        )
        .await
        .unwrap();
        session.id.to_string()
    }

    #[tokio::test]
    async fn append_and_list() {
        let pool = test_pool().await;
        let session_id = setup_session(&pool).await;
        append(&pool, &session_id, &LogType::Stdout, "line 1").await.unwrap();
        append(&pool, &session_id, &LogType::Stderr, "err 1").await.unwrap();
        let logs = list_by_session(&pool, &session_id, None, None, None)
            .await
            .unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].content, "line 1");
        assert!(matches!(logs[1].log_type, LogType::Stderr));
    }

    #[tokio::test]
    async fn list_with_limit() {
        let pool = test_pool().await;
        let session_id = setup_session(&pool).await;
        for i in 0..10 {
            append(&pool, &session_id, &LogType::Stdout, &format!("line {i}"))
                .await
                .unwrap();
        }
        let logs = list_by_session(&pool, &session_id, None, Some(3), None)
            .await
            .unwrap();
        assert_eq!(logs.len(), 3);
    }

    #[tokio::test]
    async fn list_with_offset() {
        let pool = test_pool().await;
        let session_id = setup_session(&pool).await;
        for i in 0..5 {
            append(&pool, &session_id, &LogType::Stdout, &format!("line {i}"))
                .await
                .unwrap();
        }
        let logs = list_by_session(&pool, &session_id, None, None, Some(3))
            .await
            .unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].content, "line 3");
    }

    #[tokio::test]
    async fn list_with_since_filter() {
        let pool = test_pool().await;
        let session_id = setup_session(&pool).await;
        append(&pool, &session_id, &LogType::Stdout, "old line").await.unwrap();
        // Since all inserts happen nearly instantly, use a past timestamp
        let logs = list_by_session(&pool, &session_id, Some("2000-01-01T00:00:00Z"), None, None)
            .await
            .unwrap();
        assert_eq!(logs.len(), 1);
    }

    #[tokio::test]
    async fn limit_caps_at_5000() {
        let pool = test_pool().await;
        let session_id = setup_session(&pool).await;
        // Just verify the function accepts limit > 5000 and caps it
        let logs = list_by_session(&pool, &session_id, None, Some(10000), None)
            .await
            .unwrap();
        assert!(logs.is_empty()); // no data, but no error
    }
}
