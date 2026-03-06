use composer_api_types::*;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct StepOutputRow {
    id: String,
    workflow_run_id: String,
    step_id: String,
    step_type: String,
    output: Option<String>,
    attempt: i32,
    status: String,
    session_id: Option<String>,
    created_at: String,
}

impl TryFrom<StepOutputRow> for WorkflowStepOutput {
    type Error = anyhow::Error;

    fn try_from(row: StepOutputRow) -> Result<Self, Self::Error> {
        Ok(WorkflowStepOutput {
            id: row.id.parse()?,
            workflow_run_id: row.workflow_run_id.parse()?,
            step_id: row.step_id,
            step_type: serde_json::from_value(serde_json::Value::String(row.step_type))?,
            output: row.output,
            attempt: row.attempt,
            status: serde_json::from_value(serde_json::Value::String(row.status))?,
            session_id: row.session_id.map(|s| s.parse()).transpose()?,
            created_at: row.created_at.parse()?,
        })
    }
}

const STEP_COLUMNS: &str = "id, workflow_run_id, step_id, step_type, output, attempt, status, session_id, created_at";

pub async fn create(
    pool: &SqlitePool,
    workflow_run_id: &str,
    step_id: &str,
    step_type: &WorkflowStepType,
    status: &WorkflowStepStatus,
    session_id: Option<&str>,
) -> anyhow::Result<WorkflowStepOutput> {
    let id = Uuid::new_v4().to_string();
    let type_str = serde_json::to_value(step_type)?
        .as_str().ok_or_else(|| anyhow::anyhow!("Failed to serialize step type"))?.to_string();
    let status_str = serde_json::to_value(status)?
        .as_str().ok_or_else(|| anyhow::anyhow!("Failed to serialize step status"))?.to_string();

    // Get current max attempt for this step
    let max_attempt: Option<(i32,)> = sqlx::query_as(
        "SELECT COALESCE(MAX(attempt), 0) FROM workflow_step_outputs WHERE workflow_run_id = ? AND step_id = ?"
    )
    .bind(workflow_run_id)
    .bind(step_id)
    .fetch_optional(pool)
    .await?;
    let attempt = max_attempt.map(|r| r.0).unwrap_or(0) + 1;

    sqlx::query(
        &format!("INSERT INTO workflow_step_outputs ({STEP_COLUMNS}) VALUES (?, ?, ?, ?, NULL, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))")
    )
    .bind(&id)
    .bind(workflow_run_id)
    .bind(step_id)
    .bind(&type_str)
    .bind(attempt)
    .bind(&status_str)
    .bind(session_id)
    .execute(pool)
    .await?;
    find_by_id(pool, &id).await?.ok_or_else(|| anyhow::anyhow!("Failed to create step output"))
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<WorkflowStepOutput>> {
    let row = sqlx::query_as::<_, StepOutputRow>(
        &format!("SELECT {STEP_COLUMNS} FROM workflow_step_outputs WHERE id = ?")
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(WorkflowStepOutput::try_from).transpose()
}

pub async fn list_by_run(pool: &SqlitePool, workflow_run_id: &str) -> anyhow::Result<Vec<WorkflowStepOutput>> {
    let rows = sqlx::query_as::<_, StepOutputRow>(
        &format!("SELECT {STEP_COLUMNS} FROM workflow_step_outputs WHERE workflow_run_id = ? ORDER BY step_id, attempt")
    )
    .bind(workflow_run_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(WorkflowStepOutput::try_from).collect()
}

/// Get the latest step output for a given step_id in a run.
pub async fn latest_for_step(
    pool: &SqlitePool,
    workflow_run_id: &str,
    step_id: &str,
) -> anyhow::Result<Option<WorkflowStepOutput>> {
    let row = sqlx::query_as::<_, StepOutputRow>(
        &format!("SELECT {STEP_COLUMNS} FROM workflow_step_outputs WHERE workflow_run_id = ? AND step_id = ? ORDER BY attempt DESC LIMIT 1")
    )
    .bind(workflow_run_id)
    .bind(step_id)
    .fetch_optional(pool)
    .await?;
    row.map(WorkflowStepOutput::try_from).transpose()
}

/// Find all step outputs with status 'running' for a given run.
pub async fn find_running_steps(pool: &SqlitePool, workflow_run_id: &str) -> anyhow::Result<Vec<WorkflowStepOutput>> {
    let rows = sqlx::query_as::<_, StepOutputRow>(
        &format!("SELECT {STEP_COLUMNS} FROM workflow_step_outputs WHERE workflow_run_id = ? AND status = 'running'")
    )
    .bind(workflow_run_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(WorkflowStepOutput::try_from).collect()
}

/// Find all step IDs that have a completed output (latest attempt).
pub async fn find_completed_step_ids(pool: &SqlitePool, workflow_run_id: &str) -> anyhow::Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT step_id FROM workflow_step_outputs WHERE workflow_run_id = ? AND status = 'completed'"
    )
    .bind(workflow_run_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

pub async fn update_status(pool: &SqlitePool, id: &str, status: &WorkflowStepStatus) -> anyhow::Result<()> {
    let status_str = serde_json::to_value(status)?
        .as_str().ok_or_else(|| anyhow::anyhow!("Failed to serialize step status"))?.to_string();
    sqlx::query("UPDATE workflow_step_outputs SET status = ? WHERE id = ?")
        .bind(&status_str)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_output(pool: &SqlitePool, id: &str, output: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE workflow_step_outputs SET output = ? WHERE id = ?")
        .bind(output)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_status_and_output(
    pool: &SqlitePool,
    id: &str,
    status: &WorkflowStepStatus,
    output: Option<&str>,
) -> anyhow::Result<()> {
    let status_str = serde_json::to_value(status)?
        .as_str().ok_or_else(|| anyhow::anyhow!("Failed to serialize step status"))?.to_string();
    sqlx::query("UPDATE workflow_step_outputs SET status = ?, output = COALESCE(?, output) WHERE id = ?")
        .bind(&status_str)
        .bind(output)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Find step output by session_id (for review sessions).
pub async fn find_by_session(pool: &SqlitePool, session_id: &str) -> anyhow::Result<Option<WorkflowStepOutput>> {
    let row = sqlx::query_as::<_, StepOutputRow>(
        &format!("SELECT {STEP_COLUMNS} FROM workflow_step_outputs WHERE session_id = ? ORDER BY created_at DESC LIMIT 1")
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?;
    row.map(WorkflowStepOutput::try_from).transpose()
}
