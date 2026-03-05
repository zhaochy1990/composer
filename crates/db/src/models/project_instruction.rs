use composer_api_types::*;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct ProjectInstructionRow {
    id: String,
    project_id: String,
    title: String,
    content: String,
    sort_order: i32,
    created_at: String,
    updated_at: String,
}

impl TryFrom<ProjectInstructionRow> for ProjectInstruction {
    type Error = anyhow::Error;

    fn try_from(row: ProjectInstructionRow) -> Result<Self, Self::Error> {
        Ok(ProjectInstruction {
            id: row.id.parse()?,
            project_id: row.project_id.parse()?,
            title: row.title,
            content: row.content,
            sort_order: row.sort_order,
            created_at: row.created_at.parse()?,
            updated_at: row.updated_at.parse()?,
        })
    }
}

pub async fn create(
    pool: &SqlitePool,
    project_id: &str,
    title: &str,
    content: &str,
    sort_order: Option<i32>,
) -> anyhow::Result<ProjectInstruction> {
    let id = Uuid::new_v4().to_string();
    let order = sort_order.unwrap_or(0);

    sqlx::query(
        "INSERT INTO project_instructions (id, project_id, title, content, sort_order) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(project_id)
    .bind(title)
    .bind(content)
    .bind(order)
    .execute(pool)
    .await?;

    find_by_id(pool, &id).await?.ok_or_else(|| anyhow::anyhow!("Failed to create project instruction"))
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<ProjectInstruction>> {
    let row = sqlx::query_as::<_, ProjectInstructionRow>("SELECT * FROM project_instructions WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(ProjectInstruction::try_from).transpose()
}

pub async fn list_by_project(pool: &SqlitePool, project_id: &str) -> anyhow::Result<Vec<ProjectInstruction>> {
    let rows = sqlx::query_as::<_, ProjectInstructionRow>(
        "SELECT * FROM project_instructions WHERE project_id = ? ORDER BY sort_order ASC, created_at ASC"
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(ProjectInstruction::try_from).collect()
}

pub async fn update(
    pool: &SqlitePool,
    project_id: &str,
    id: &str,
    title: Option<&str>,
    content: Option<&str>,
    sort_order: Option<i32>,
) -> anyhow::Result<ProjectInstruction> {
    let rows = sqlx::query(
        "UPDATE project_instructions SET \
         title = COALESCE(?, title), \
         content = COALESCE(?, content), \
         sort_order = COALESCE(?, sort_order), \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ? AND project_id = ?"
    )
    .bind(title)
    .bind(content)
    .bind(sort_order)
    .bind(id)
    .bind(project_id)
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        anyhow::bail!("Instruction not found or does not belong to this project");
    }

    find_by_id(pool, id).await?.ok_or_else(|| anyhow::anyhow!("Project instruction not found"))
}

/// Format a list of instructions into a structured prompt block.
/// Returns `None` if the list is empty.
pub fn format_instructions_block(instructions: &[ProjectInstruction]) -> Option<String> {
    if instructions.is_empty() {
        return None;
    }
    let entries: Vec<String> = instructions.iter()
        .map(|i| format!("<instruction title=\"{}\">\n{}\n</instruction>", i.title, i.content))
        .collect();
    Some(format!("Project Instructions:\n{}", entries.join("\n")))
}

pub async fn delete(pool: &SqlitePool, project_id: &str, id: &str) -> anyhow::Result<()> {
    let rows = sqlx::query("DELETE FROM project_instructions WHERE id = ? AND project_id = ?")
        .bind(id).bind(project_id).execute(pool).await?;
    if rows.rows_affected() == 0 {
        anyhow::bail!("Instruction not found or does not belong to this project");
    }
    Ok(())
}
