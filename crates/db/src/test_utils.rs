use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

/// Create a fresh in-memory SQLite pool with migrations applied.
/// Each call produces an isolated database for test independence.
pub async fn test_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("failed to create in-memory test pool");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    pool
}
