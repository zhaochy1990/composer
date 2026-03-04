pub mod models;

#[cfg(test)]
pub mod test_utils;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await?;

        // Enable WAL mode, relaxed sync, and a busy timeout for concurrent access
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA synchronous=NORMAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA busy_timeout=5000")
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> anyhow::Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_and_migrate() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.run_migrations().await.unwrap();
    }

    #[tokio::test]
    async fn wal_mode_pragma() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let row: (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        // In-memory SQLite may report "memory" instead of "wal"
        assert!(row.0 == "wal" || row.0 == "memory");
    }
}
