
use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn establish_connection(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(50) // Supports high-throughput asynchronous polling
        .connect(database_url)
        .await?;
        
    Ok(pool)
}