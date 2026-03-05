use composer_api_types::*;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct WorkflowRow {
    id: String,
    project_id: String,
    name: String,
    definition: String,
    created_at: String,
    updated_at: String,
}

impl TryFrom<WorkflowRow> for Workflow {
    type Error = anyhow::Error;

    fn try_from(row: WorkflowRow) -> Result<Self, Self::Error> {
        Ok(Workflow {
            id: row.id.parse()?,
            project_id: row.project_id.parse()?,
            name: row.name,
            definition: serde_json::from_str(&row.definition)?,
            created_at: row.created_at.parse()?,
            updated_at: row.updated_at.parse()?,
        })
    }
}

pub async fn create(
    pool: &SqlitePool,
    project_id: &str,
    name: &str,
    definition: &WorkflowDefinition,
) -> anyhow::Result<Workflow> {
    let id = Uuid::new_v4().to_string();
    let def_json = serde_json::to_string(definition)?;
    sqlx::query(
        "INSERT INTO workflows (id, project_id, name, definition) VALUES (?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(project_id)
    .bind(name)
    .bind(&def_json)
    .execute(pool)
    .await?;
    find_by_id(pool, &id).await?.ok_or_else(|| anyhow::anyhow!("Failed to create workflow"))
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Workflow>> {
    let row = sqlx::query_as::<_, WorkflowRow>(
        "SELECT id, project_id, name, definition, created_at, updated_at FROM workflows WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(Workflow::try_from).transpose()
}

pub async fn list_by_project(pool: &SqlitePool, project_id: &str) -> anyhow::Result<Vec<Workflow>> {
    let rows = sqlx::query_as::<_, WorkflowRow>(
        "SELECT id, project_id, name, definition, created_at, updated_at FROM workflows WHERE project_id = ? ORDER BY name"
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(Workflow::try_from).collect()
}

pub async fn update(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    definition: Option<&WorkflowDefinition>,
) -> anyhow::Result<Workflow> {
    let def_json = definition.map(|d| serde_json::to_string(d)).transpose()?;
    sqlx::query(
        "UPDATE workflows SET \
         name = COALESCE(?, name), \
         definition = COALESCE(?, definition), \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?"
    )
    .bind(name)
    .bind(def_json.as_deref())
    .bind(id)
    .execute(pool)
    .await?;
    find_by_id(pool, id).await?.ok_or_else(|| anyhow::anyhow!("Workflow not found"))
}

pub async fn delete(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM workflows WHERE id = ?")
        .bind(id).execute(pool).await?;
    Ok(())
}
