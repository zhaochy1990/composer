use composer_api_types::*;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: String,
    title: String,
    description: Option<String>,
    status: String,
    priority: i32,
    assigned_agent_id: Option<String>,
    project_id: Option<String>,
    auto_approve: bool,
    position: f64,
    task_number: i32,
    simple_id: String,
    pr_urls: String,
    created_at: String,
    updated_at: String,
}

impl TryFrom<TaskRow> for Task {
    type Error = anyhow::Error;

    fn try_from(row: TaskRow) -> Result<Self, Self::Error> {
        let pr_urls: Vec<String> = serde_json::from_str(&row.pr_urls).unwrap_or_default();
        Ok(Task {
            id: row.id.parse()?,
            title: row.title,
            description: row.description,
            status: serde_json::from_value(serde_json::Value::String(row.status))?,
            priority: row.priority,
            assigned_agent_id: row.assigned_agent_id.map(|s| s.parse()).transpose()?,
            project_id: row.project_id.map(|s| s.parse()).transpose()?,
            auto_approve: row.auto_approve,
            position: row.position,
            task_number: row.task_number,
            simple_id: row.simple_id,
            pr_urls,
            created_at: row.created_at.parse()?,
            updated_at: row.updated_at.parse()?,
        })
    }
}

pub async fn create(
    pool: &SqlitePool,
    title: &str,
    description: Option<&str>,
    priority: Option<i32>,
    status: Option<&TaskStatus>,
    project_id: Option<&str>,
    assigned_agent_id: Option<&str>,
) -> anyhow::Result<Task> {
    let id = Uuid::new_v4().to_string();
    let priority = priority.unwrap_or(0);
    let status_str = status
        .map(|s| serde_json::to_value(s).ok()
            .and_then(|v| v.as_str().map(|s| s.to_string())))
        .flatten()
        .unwrap_or_else(|| "backlog".to_string());

    let mut tx = pool.begin().await?;

    let max_pos: Option<(f64,)> = sqlx::query_as(
        "SELECT COALESCE(MAX(position), 0.0) FROM tasks WHERE status = ?"
    )
    .bind(&status_str)
    .fetch_optional(&mut *tx)
    .await?;
    let position = max_pos.map(|r| r.0).unwrap_or(0.0) + 1.0;

    // If project_id is provided, atomically increment the project's task_counter
    // and compute the simple_id from prefix + counter
    let (task_number, simple_id) = if let Some(pid) = project_id {
        let row: Option<(i32, String)> = sqlx::query_as(
            "UPDATE projects SET task_counter = task_counter + 1, \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
             WHERE id = ? \
             RETURNING task_counter, task_prefix"
        )
        .bind(pid)
        .fetch_optional(&mut *tx)
        .await?;

        match row {
            Some((counter, prefix)) => (counter, format!("{}-{}", prefix, counter)),
            None => return Err(anyhow::anyhow!("Project not found: {}", pid)),
        }
    } else {
        (0, String::new())
    };

    sqlx::query(
        "INSERT INTO tasks (id, title, description, status, priority, position, project_id, assigned_agent_id, task_number, simple_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(title)
    .bind(description)
    .bind(&status_str)
    .bind(priority)
    .bind(position)
    .bind(project_id)
    .bind(assigned_agent_id)
    .bind(task_number)
    .bind(&simple_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    find_by_id(pool, &id).await?.ok_or_else(|| anyhow::anyhow!("Failed to create task"))
}

const TASK_COLUMNS: &str = "id, title, description, status, priority, assigned_agent_id, project_id, auto_approve, position, task_number, simple_id, pr_urls, created_at, updated_at";

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Task>> {
    let row = sqlx::query_as::<_, TaskRow>(&format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(Task::try_from).transpose()
}

pub async fn list_all(pool: &SqlitePool) -> anyhow::Result<Vec<Task>> {
    let rows = sqlx::query_as::<_, TaskRow>(&format!("SELECT {TASK_COLUMNS} FROM tasks ORDER BY position ASC"))
        .fetch_all(pool)
        .await?;
    rows.into_iter().map(Task::try_from).collect()
}

pub async fn list_by_status(pool: &SqlitePool, status: &TaskStatus) -> anyhow::Result<Vec<Task>> {
    let status_str = serde_json::to_value(status)?
        .as_str().unwrap_or("backlog").to_string();
    let rows = sqlx::query_as::<_, TaskRow>(
        &format!("SELECT {TASK_COLUMNS} FROM tasks WHERE status = ? ORDER BY position ASC")
    )
    .bind(&status_str)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(Task::try_from).collect()
}

/// Fix #13: Single UPDATE statement using COALESCE pattern
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    priority: Option<i32>,
    status: Option<&TaskStatus>,
    position: Option<f64>,
    project_id: Option<&str>,
    assigned_agent_id: Option<&str>,
) -> anyhow::Result<Task> {
    let status_str: Option<String> = status
        .map(|s| serde_json::to_value(s).ok()
            .and_then(|v| v.as_str().map(|s| s.to_string())))
        .flatten();

    sqlx::query(
        "UPDATE tasks SET \
         title = COALESCE(?, title), \
         description = COALESCE(?, description), \
         priority = COALESCE(?, priority), \
         status = COALESCE(?, status), \
         position = COALESCE(?, position), \
         project_id = COALESCE(?, project_id), \
         assigned_agent_id = COALESCE(?, assigned_agent_id), \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?"
    )
    .bind(title)
    .bind(description)
    .bind(priority)
    .bind(status_str.as_deref())
    .bind(position)
    .bind(project_id)
    .bind(assigned_agent_id)
    .bind(id)
    .execute(pool)
    .await?;

    find_by_id(pool, id).await?.ok_or_else(|| anyhow::anyhow!("Task not found"))
}

pub async fn update_status(pool: &SqlitePool, id: &str, status: &TaskStatus) -> anyhow::Result<()> {
    let status_str = serde_json::to_value(status)?
        .as_str().unwrap_or("backlog").to_string();
    sqlx::query("UPDATE tasks SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(&status_str).bind(id).execute(pool).await?;
    Ok(())
}

pub async fn update_assigned_agent(pool: &SqlitePool, id: &str, agent_id: Option<&str>) -> anyhow::Result<()> {
    sqlx::query("UPDATE tasks SET assigned_agent_id = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(agent_id).bind(id).execute(pool).await?;
    Ok(())
}

pub async fn list_by_project(pool: &SqlitePool, project_id: &str) -> anyhow::Result<Vec<Task>> {
    let rows = sqlx::query_as::<_, TaskRow>(
        &format!("SELECT {TASK_COLUMNS} FROM tasks WHERE project_id = ? ORDER BY position ASC")
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(Task::try_from).collect()
}

pub async fn delete(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(id).execute(pool).await?;
    Ok(())
}

/// Append PR URLs to a task, deduplicating against existing entries.
/// Returns true if any new URLs were actually added.
pub async fn append_pr_urls(pool: &SqlitePool, id: &str, new_urls: &[String]) -> anyhow::Result<bool> {
    let row: Option<(String,)> = sqlx::query_as("SELECT pr_urls FROM tasks WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    let existing_json = row.map(|r| r.0).unwrap_or_else(|| "[]".to_string());
    let mut urls: Vec<String> = serde_json::from_str(&existing_json).unwrap_or_default();
    let before_len = urls.len();
    for url in new_urls {
        if !urls.contains(url) {
            urls.push(url.clone());
        }
    }
    if urls.len() == before_len {
        return Ok(false);
    }
    let json = serde_json::to_string(&urls)?;
    sqlx::query("UPDATE tasks SET pr_urls = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(&json)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(true)
}
