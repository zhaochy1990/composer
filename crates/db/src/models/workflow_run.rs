use composer_api_types::*;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct WorkflowRunRow {
    id: String,
    workflow_id: String,
    task_id: String,
    status: String,
    current_step_index: i32,
    iteration_count: i32,
    main_session_id: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<WorkflowRunRow> for WorkflowRun {
    type Error = anyhow::Error;

    fn try_from(row: WorkflowRunRow) -> Result<Self, Self::Error> {
        Ok(WorkflowRun {
            id: row.id.parse()?,
            workflow_id: row.workflow_id.parse()?,
            task_id: row.task_id.parse()?,
            status: serde_json::from_value(serde_json::Value::String(row.status))?,
            current_step_index: row.current_step_index,
            iteration_count: row.iteration_count,
            main_session_id: row.main_session_id.map(|s| s.parse()).transpose()?,
            created_at: row.created_at.parse()?,
            updated_at: row.updated_at.parse()?,
        })
    }
}

const RUN_COLUMNS: &str = "id, workflow_id, task_id, status, current_step_index, iteration_count, main_session_id, created_at, updated_at";

pub async fn create(
    pool: &SqlitePool,
    workflow_id: &str,
    task_id: &str,
) -> anyhow::Result<WorkflowRun> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO workflow_runs (id, workflow_id, task_id) VALUES (?, ?, ?)"
    )
    .bind(&id)
    .bind(workflow_id)
    .bind(task_id)
    .execute(pool)
    .await?;
    find_by_id(pool, &id).await?.ok_or_else(|| anyhow::anyhow!("Failed to create workflow run"))
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<WorkflowRun>> {
    let row = sqlx::query_as::<_, WorkflowRunRow>(
        &format!("SELECT {RUN_COLUMNS} FROM workflow_runs WHERE id = ?")
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(WorkflowRun::try_from).transpose()
}

pub async fn find_by_task(pool: &SqlitePool, task_id: &str) -> anyhow::Result<Option<WorkflowRun>> {
    let row = sqlx::query_as::<_, WorkflowRunRow>(
        &format!("SELECT {RUN_COLUMNS} FROM workflow_runs WHERE task_id = ? ORDER BY created_at DESC LIMIT 1")
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await?;
    row.map(WorkflowRun::try_from).transpose()
}

pub async fn find_by_session(pool: &SqlitePool, session_id: &str) -> anyhow::Result<Option<WorkflowRun>> {
    let row = sqlx::query_as::<_, WorkflowRunRow>(
        &format!("SELECT {RUN_COLUMNS} FROM workflow_runs WHERE main_session_id = ?")
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?;
    row.map(WorkflowRun::try_from).transpose()
}

/// Find a workflow run where a given session_id is referenced in step outputs (e.g., review sessions).
pub async fn find_by_step_session(pool: &SqlitePool, session_id: &str) -> anyhow::Result<Option<WorkflowRun>> {
    // First find the workflow_run_id from the step output
    let run_id: Option<(String,)> = sqlx::query_as(
        "SELECT workflow_run_id FROM workflow_step_outputs WHERE session_id = ? LIMIT 1"
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?;

    match run_id {
        Some((id,)) => find_by_id(pool, &id).await,
        None => Ok(None),
    }
}

pub async fn update_status(pool: &SqlitePool, id: &str, status: &WorkflowRunStatus) -> anyhow::Result<()> {
    let status_str = serde_json::to_value(status)?
        .as_str().unwrap_or("running").to_string();
    sqlx::query(
        "UPDATE workflow_runs SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?"
    )
    .bind(&status_str)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_step_index(pool: &SqlitePool, id: &str, step_index: i32) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workflow_runs SET current_step_index = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?"
    )
    .bind(step_index)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_main_session(pool: &SqlitePool, id: &str, session_id: &str) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workflow_runs SET main_session_id = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?"
    )
    .bind(session_id)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn increment_iteration(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workflow_runs SET iteration_count = iteration_count + 1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?"
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Find all workflow runs that were in "running" status (orphaned by server crash).
pub async fn find_running(pool: &SqlitePool) -> anyhow::Result<Vec<WorkflowRun>> {
    let rows = sqlx::query_as::<_, WorkflowRunRow>(
        &format!("SELECT {RUN_COLUMNS} FROM workflow_runs WHERE status = 'running'")
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(WorkflowRun::try_from).collect()
}
