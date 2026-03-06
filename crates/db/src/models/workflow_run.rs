use composer_api_types::*;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct WorkflowRunRow {
    id: String,
    workflow_id: String,
    task_id: String,
    status: String,
    iteration_count: i32,
    activated_steps: String,
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
            iteration_count: row.iteration_count,
            activated_steps: serde_json::from_str(&row.activated_steps)?,
            created_at: row.created_at.parse()?,
            updated_at: row.updated_at.parse()?,
        })
    }
}

const RUN_COLUMNS: &str = "id, workflow_id, task_id, status, iteration_count, activated_steps, created_at, updated_at";

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

/// Find a workflow run where a given session_id is referenced in step outputs.
pub async fn find_by_step_session(pool: &SqlitePool, session_id: &str) -> anyhow::Result<Option<WorkflowRun>> {
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

pub async fn add_activated_step(pool: &SqlitePool, id: &str, step_id: &str) -> anyhow::Result<()> {
    let run = find_by_id(pool, id).await?
        .ok_or_else(|| anyhow::anyhow!("Workflow run not found"))?;
    let mut steps = run.activated_steps;
    if !steps.contains(&step_id.to_string()) {
        steps.push(step_id.to_string());
    }
    let steps_json = serde_json::to_string(&steps)?;
    sqlx::query(
        "UPDATE workflow_runs SET activated_steps = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?"
    )
    .bind(&steps_json)
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
