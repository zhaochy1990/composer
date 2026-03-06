use composer_api_types::*;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct WorkflowRow {
    id: String,
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
            name: row.name,
            definition: serde_json::from_str(&row.definition)?,
            created_at: row.created_at.parse()?,
            updated_at: row.updated_at.parse()?,
        })
    }
}

const COLUMNS: &str = "id, name, definition, created_at, updated_at";

pub async fn create(
    pool: &SqlitePool,
    name: &str,
    definition: &WorkflowDefinition,
) -> anyhow::Result<Workflow> {
    let id = Uuid::new_v4().to_string();
    let def_json = serde_json::to_string(definition)?;
    sqlx::query(
        "INSERT INTO workflows (id, name, definition) VALUES (?, ?, ?)"
    )
    .bind(&id)
    .bind(name)
    .bind(&def_json)
    .execute(pool)
    .await?;
    find_by_id(pool, &id).await?.ok_or_else(|| anyhow::anyhow!("Failed to create workflow"))
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Workflow>> {
    let row = sqlx::query_as::<_, WorkflowRow>(
        &format!("SELECT {COLUMNS} FROM workflows WHERE id = ?")
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(Workflow::try_from).transpose()
}

pub async fn find_by_name(pool: &SqlitePool, name: &str) -> anyhow::Result<Option<Workflow>> {
    let row = sqlx::query_as::<_, WorkflowRow>(
        &format!("SELECT {COLUMNS} FROM workflows WHERE name = ?")
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    row.map(Workflow::try_from).transpose()
}

pub async fn list_all(pool: &SqlitePool) -> anyhow::Result<Vec<Workflow>> {
    let rows = sqlx::query_as::<_, WorkflowRow>(
        &format!("SELECT {COLUMNS} FROM workflows ORDER BY name")
    )
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

/// List all workflow definitions as raw (id, definition_json) pairs.
/// Used for migrations that operate on raw JSON before deserialization.
pub async fn list_all_raw_definitions(pool: &SqlitePool) -> anyhow::Result<Vec<(String, String)>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, definition FROM workflows"
    ).fetch_all(pool).await?;
    Ok(rows)
}

/// Update a workflow's raw definition JSON string.
/// Used for migrations that operate on raw JSON before deserialization.
pub async fn update_raw_definition(pool: &SqlitePool, id: &str, definition_json: &str) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workflows SET definition = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?"
    )
    .bind(definition_json)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Check if any workflow definitions contain old step types that need migration.
pub async fn has_legacy_step_types(pool: &SqlitePool) -> anyhow::Result<bool> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflows WHERE \
         definition LIKE '%\"step_type\":\"plan\"%' \
         OR definition LIKE '%\"step_type\": \"plan\"%' \
         OR definition LIKE '%\"step_type\":\"implement\"%' \
         OR definition LIKE '%\"step_type\": \"implement\"%' \
         OR definition LIKE '%\"step_type\":\"pr_review\"%' \
         OR definition LIKE '%\"step_type\": \"pr_review\"%' \
         OR definition LIKE '%\"step_type\":\"human_review\"%' \
         OR definition LIKE '%\"step_type\": \"human_review\"%' \
         OR definition LIKE '%\"step_type\":\"complete_pr\"%' \
         OR definition LIKE '%\"step_type\": \"complete_pr\"%'"
    ).fetch_one(pool).await?;
    Ok(count.0 > 0)
}
