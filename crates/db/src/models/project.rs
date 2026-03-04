use composer_api_types::*;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: String,
    name: String,
    description: Option<String>,
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
            created_at: row.created_at.parse()?,
            updated_at: row.updated_at.parse()?,
        })
    }
}

pub async fn create(
    pool: &SqlitePool,
    name: &str,
    description: Option<&str>,
) -> anyhow::Result<Project> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO projects (id, name, description) VALUES (?, ?, ?)"
    )
    .bind(&id)
    .bind(name)
    .bind(description)
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
    sqlx::query(
        "UPDATE projects SET \
         name = COALESCE(?, name), \
         description = COALESCE(?, description), \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?"
    )
    .bind(name)
    .bind(description)
    .bind(id)
    .execute(pool)
    .await?;

    find_by_id(pool, id).await?.ok_or_else(|| anyhow::anyhow!("Project not found"))
}

pub async fn delete(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(id).execute(pool).await?;
    Ok(())
}
