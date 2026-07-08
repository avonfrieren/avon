use sqlx::postgres::{PgPool, PgPoolOptions};

/// Creates a connection pool. Panics on failure since the app is useless without a DB.
pub async fn init_pool(database_url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .expect("Failed to connect to Postgres")
}
