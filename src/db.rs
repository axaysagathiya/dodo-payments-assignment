use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing::info;

/// Initializes the database connection pool.
///
/// It reads the `DATABASE_URL` environment variable to connect to the PostgreSQL database.
pub async fn init_pool() -> Result<PgPool, String> {
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL environment variable must be set".to_string())?;

    info!("Connecting to database...");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(3))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(&database_url)
        .await
        .map_err(|e| format!("Failed to connect to the database: {}", e))?;

    info!("Successfully connected to the database.");

    Ok(pool)
}
