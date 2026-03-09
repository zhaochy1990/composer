use sqlx::SqlitePool;
use std::collections::HashMap;

/// Insert a link between two tasks. Enforces canonical ordering (a < b).
pub async fn create(pool: &SqlitePool, task_id_1: &str, task_id_2: &str) -> anyhow::Result<()> {
    let (a, b) = if task_id_1 < task_id_2 {
        (task_id_1, task_id_2)
    } else {
        (task_id_2, task_id_1)
    };
    sqlx::query("INSERT OR IGNORE INTO task_links (task_id_a, task_id_b) VALUES (?, ?)")
        .bind(a)
        .bind(b)
        .execute(pool)
        .await?;
    Ok(())
}

/// Insert a link within an existing transaction.
pub async fn create_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    task_id_1: &str,
    task_id_2: &str,
) -> anyhow::Result<()> {
    let (a, b) = if task_id_1 < task_id_2 {
        (task_id_1, task_id_2)
    } else {
        (task_id_2, task_id_1)
    };
    sqlx::query("INSERT OR IGNORE INTO task_links (task_id_a, task_id_b) VALUES (?, ?)")
        .bind(a)
        .bind(b)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Get all task IDs linked to a given task.
pub async fn list_linked_task_ids(pool: &SqlitePool, task_id: &str) -> anyhow::Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT task_id_b AS linked_id FROM task_links WHERE task_id_a = ? \
         UNION \
         SELECT task_id_a AS linked_id FROM task_links WHERE task_id_b = ?",
    )
    .bind(task_id)
    .bind(task_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Fetch all task links and return a map: task_id -> Vec<linked_task_id>.
/// Used for batch-populating related_task_ids on task lists.
pub async fn list_all_links_map(pool: &SqlitePool) -> anyhow::Result<HashMap<String, Vec<String>>> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT task_id_a, task_id_b FROM task_links")
            .fetch_all(pool)
            .await?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (a, b) in rows {
        map.entry(a.clone()).or_default().push(b.clone());
        map.entry(b).or_default().push(a);
    }
    Ok(map)
}
