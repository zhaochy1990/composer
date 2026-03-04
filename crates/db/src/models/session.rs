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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_pool;

    /// Helper: create an agent so FK constraints are satisfied
    async fn setup_agent(pool: &sqlx::SqlitePool) -> String {
        let agent = crate::models::agent::create(pool, "Test Agent", &AgentType::ClaudeCode, None)
            .await
            .unwrap();
        agent.id.to_string()
    }

    #[tokio::test]
    async fn create_session_defaults() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let session = create(&pool, &agent_id, None, None, "do stuff").await.unwrap();
        assert!(matches!(session.status, SessionStatus::Created));
        assert_eq!(session.prompt.as_deref(), Some("do stuff"));
        assert!(session.task_id.is_none());
        assert!(session.worktree_id.is_none());
        assert!(session.started_at.is_none());
        assert!(session.completed_at.is_none());
    }

    #[tokio::test]
    async fn create_with_status_running() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let id = uuid::Uuid::new_v4().to_string();
        let session = create_with_status(
            &pool, &id, &agent_id, None, None, "run it", &SessionStatus::Running,
        )
        .await
        .unwrap();
        assert!(matches!(session.status, SessionStatus::Running));
        assert!(session.started_at.is_some());
    }

    #[tokio::test]
    async fn create_with_status_created() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let id = uuid::Uuid::new_v4().to_string();
        let session = create_with_status(
            &pool, &id, &agent_id, None, None, "pending", &SessionStatus::Created,
        )
        .await
        .unwrap();
        assert!(matches!(session.status, SessionStatus::Created));
        assert!(session.started_at.is_none());
    }

    #[tokio::test]
    async fn find_by_id_hit_and_miss() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let session = create(&pool, &agent_id, None, None, "prompt").await.unwrap();
        let found = find_by_id(&pool, &session.id.to_string()).await.unwrap();
        assert!(found.is_some());
        let miss = find_by_id(&pool, "00000000-0000-0000-0000-000000000000")
            .await
            .unwrap();
        assert!(miss.is_none());
    }

    #[tokio::test]
    async fn list_by_agent_filters() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        create(&pool, &agent_id, None, None, "s1").await.unwrap();
        create(&pool, &agent_id, None, None, "s2").await.unwrap();
        let sessions = list_by_agent(&pool, &agent_id).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn list_by_task_filters() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let task = crate::models::task::create(&pool, "Task", None, None, None)
            .await
            .unwrap();
        let task_id = task.id.to_string();
        create(&pool, &agent_id, Some(&task_id), None, "s1")
            .await
            .unwrap();
        let sessions = list_by_task(&pool, &task_id).await.unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[tokio::test]
    async fn update_status_completed_sets_timestamp() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let id = uuid::Uuid::new_v4().to_string();
        create_with_status(
            &pool, &id, &agent_id, None, None, "run", &SessionStatus::Running,
        )
        .await
        .unwrap();
        update_status(&pool, &id, &SessionStatus::Completed).await.unwrap();
        let found = find_by_id(&pool, &id).await.unwrap().unwrap();
        assert!(matches!(found.status, SessionStatus::Completed));
        assert!(found.completed_at.is_some());
    }

    #[tokio::test]
    async fn update_status_failed_sets_timestamp() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let id = uuid::Uuid::new_v4().to_string();
        create_with_status(
            &pool, &id, &agent_id, None, None, "run", &SessionStatus::Running,
        )
        .await
        .unwrap();
        update_status(&pool, &id, &SessionStatus::Failed).await.unwrap();
        let found = find_by_id(&pool, &id).await.unwrap().unwrap();
        assert!(matches!(found.status, SessionStatus::Failed));
        assert!(found.completed_at.is_some());
    }

    #[tokio::test]
    async fn update_result_sets_summary() {
        let pool = test_pool().await;
        let agent_id = setup_agent(&pool).await;
        let session = create(&pool, &agent_id, None, None, "prompt").await.unwrap();
        let id = session.id.to_string();
        update_result(&pool, &id, Some("All done"), Some("resume-123"))
            .await
            .unwrap();
        let found = find_by_id(&pool, &id).await.unwrap().unwrap();
        assert_eq!(found.result_summary.as_deref(), Some("All done"));
        assert_eq!(found.resume_session_id.as_deref(), Some("resume-123"));
    }
}
