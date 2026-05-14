pub mod isins;
pub mod metrics;
pub mod shares;
pub mod utils;

use app_config::DatabaseConfig;
use sqlx::{postgres::PgPoolOptions, Error, Pool, Postgres};
use tracing::{error, info};

pub async fn connect(config: &DatabaseConfig) -> Result<Pool<Postgres>, Error> {
    info!("Attempting to connecting to database");

    PgPoolOptions::new()
        .max_connections(config.pool_max_connections)
        .acquire_timeout(config.acquire_timeout)
        .connect(&config.url)
        .await
        .inspect_err(|e| error!("Database connection failed: {}", e))
}
