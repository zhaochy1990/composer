pub mod models;

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
        // Acquire a single connection and disable FK checks on it so that
        // migrations can drop/recreate tables without FK constraint errors.
        // PRAGMA foreign_keys is per-connection and is a no-op inside
        // transactions, so we must set it on the connection before sqlx
        // wraps each migration in a transaction.
        let mut conn = self.pool.acquire().await?;
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&mut *conn)
            .await?;

        sqlx::migrate!("./migrations")
            .run(&mut *conn)
            .await?;

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}
