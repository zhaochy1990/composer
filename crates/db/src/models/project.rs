use composer_api_types::*;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: String,
    name: String,
    description: Option<String>,
    task_prefix: String,
    task_counter: i32,
    created_at: String,
    updated_at: String,
}

impl TryFrom<ProjectRow> for Project {
    type Error = anyhow::Error;

    fn try_from(row: ProjectRow) -> Result<Self, Self::Error> {
        Ok(Project {
            id: row.id.parse()?,
            name: row.name,
            description: row.description,
            task_prefix: row.task_prefix,
            task_counter: row.task_counter,
            created_at: row.created_at.parse()?,
            updated_at: row.updated_at.parse()?,
        })
    }
}

/// Derive a task prefix from a project name: first 3 alpha chars, uppercased.
/// Falls back to "TSK" if fewer than 3 alpha chars are found.
pub fn derive_task_prefix(name: &str) -> String {
    let alpha: String = name.chars().filter(|c| c.is_ascii_alphabetic()).take(3).collect();
    if alpha.len() < 3 {
        "TSK".to_string()
    } else {
        alpha.to_uppercase()
    }
}

pub async fn create(
    pool: &SqlitePool,
    name: &str,
    description: Option<&str>,
) -> anyhow::Result<Project> {
    tracing::debug!(name = %name, "DB: creating project");
    let id = Uuid::new_v4().to_string();
    let prefix = derive_task_prefix(name);
    sqlx::query(
        "INSERT INTO projects (id, name, description, task_prefix) VALUES (?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(name)
    .bind(description)
    .bind(&prefix)
    .execute(pool)
    .await?;

    find_by_id(pool, &id).await?.ok_or_else(|| anyhow::anyhow!("Failed to create project"))
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Project>> {
    let row = sqlx::query_as::<_, ProjectRow>("SELECT * FROM projects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(Project::try_from).transpose()
}

pub async fn list_all(pool: &SqlitePool) -> anyhow::Result<Vec<Project>> {
    let rows = sqlx::query_as::<_, ProjectRow>("SELECT * FROM projects ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    rows.into_iter().map(Project::try_from).collect()
}

pub async fn update(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> anyhow::Result<Project> {
    // If name is being changed, re-derive the prefix
    let new_prefix = name.map(derive_task_prefix);

    sqlx::query(
        "UPDATE projects SET \
         name = COALESCE(?, name), \
         description = COALESCE(?, description), \
         task_prefix = COALESCE(?, task_prefix), \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?"
    )
    .bind(name)
    .bind(description)
    .bind(new_prefix.as_deref())
    .bind(id)
    .execute(pool)
    .await?;

    find_by_id(pool, id).await?.ok_or_else(|| anyhow::anyhow!("Project not found"))
}

pub async fn delete(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    tracing::debug!(project_id = %id, "DB: deleting project");
    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(id).execute(pool).await?;
    Ok(())
}
