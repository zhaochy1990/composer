use composer_api_types::*;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct ProjectRepositoryRow {
    id: String,
    project_id: String,
    local_path: String,
    remote_url: Option<String>,
    role: String,
    display_name: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<ProjectRepositoryRow> for ProjectRepository {
    type Error = anyhow::Error;

    fn try_from(row: ProjectRepositoryRow) -> Result<Self, Self::Error> {
        Ok(ProjectRepository {
            id: row.id.parse()?,
            project_id: row.project_id.parse()?,
            local_path: row.local_path,
            remote_url: row.remote_url,
            role: serde_json::from_value(serde_json::Value::String(row.role))?,
            display_name: row.display_name,
            created_at: row.created_at.parse()?,
            updated_at: row.updated_at.parse()?,
        })
    }
}

pub async fn create(
    pool: &SqlitePool,
    project_id: &str,
    local_path: &str,
    remote_url: Option<&str>,
    role: Option<&RepositoryRole>,
    display_name: Option<&str>,
) -> anyhow::Result<ProjectRepository> {
    let id = Uuid::new_v4().to_string();
    let role_str = role
        .map(|r| serde_json::to_value(r).ok()
            .and_then(|v| v.as_str().map(|s| s.to_string())))
        .flatten()
        .unwrap_or_else(|| "primary".to_string());

    sqlx::query(
        "INSERT INTO project_repositories (id, project_id, local_path, remote_url, role, display_name) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(project_id)
    .bind(local_path)
    .bind(remote_url)
    .bind(&role_str)
    .bind(display_name)
    .execute(pool)
    .await?;

    find_by_id(pool, &id).await?.ok_or_else(|| anyhow::anyhow!("Failed to create project repository"))
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<ProjectRepository>> {
    let row = sqlx::query_as::<_, ProjectRepositoryRow>("SELECT * FROM project_repositories WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(ProjectRepository::try_from).transpose()
}

pub async fn list_by_project(pool: &SqlitePool, project_id: &str) -> anyhow::Result<Vec<ProjectRepository>> {
    let rows = sqlx::query_as::<_, ProjectRepositoryRow>(
        "SELECT * FROM project_repositories WHERE project_id = ? ORDER BY created_at ASC"
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(ProjectRepository::try_from).collect()
}

pub async fn update(
    pool: &SqlitePool,
    project_id: &str,
    id: &str,
    local_path: Option<&str>,
    remote_url: Option<&str>,
    role: Option<&RepositoryRole>,
    display_name: Option<&str>,
) -> anyhow::Result<ProjectRepository> {
    let role_str: Option<String> = role
        .map(|r| serde_json::to_value(r).ok()
            .and_then(|v| v.as_str().map(|s| s.to_string())))
        .flatten();

    let rows = sqlx::query(
        "UPDATE project_repositories SET \
         local_path = COALESCE(?, local_path), \
         remote_url = COALESCE(?, remote_url), \
         role = COALESCE(?, role), \
         display_name = COALESCE(?, display_name), \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ? AND project_id = ?"
    )
    .bind(local_path)
    .bind(remote_url)
    .bind(role_str.as_deref())
    .bind(display_name)
    .bind(id)
    .bind(project_id)
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        anyhow::bail!("Repository not found or does not belong to this project");
    }

    find_by_id(pool, id).await?.ok_or_else(|| anyhow::anyhow!("Project repository not found"))
}

pub async fn delete(pool: &SqlitePool, project_id: &str, id: &str) -> anyhow::Result<()> {
    let rows = sqlx::query("DELETE FROM project_repositories WHERE id = ? AND project_id = ?")
        .bind(id).bind(project_id).execute(pool).await?;
    if rows.rows_affected() == 0 {
        anyhow::bail!("Repository not found or does not belong to this project");
    }
    Ok(())
}
