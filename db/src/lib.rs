pub mod isins;
pub mod metrics;
pub mod shares;
pub mod utils;

use dotenv::dotenv;
use sqlx::{postgres::PgPoolOptions, Error, Pool, Postgres};
use std::{env, time::Duration};
use tracing::{error, info, warn};

pub async fn connect() -> Result<Pool<Postgres>, Error> {
    info!("Attempting to connecting to database");
    match dotenv() {
        Ok(_) => info!("Environment variables loaded successfully"),
        Err(e) => warn!("Failed to load .env file: {}", e),
    }

    let db_url = env::var("DATABASE_URL").map_err(|_| {
        error!("DATABASE_URL environment variable is not set");
        Error::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "DATABASE_URL environment variable is required",
        ))
    })?;

    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&db_url)
        .await
        .inspect_err(|e| error!("Database connection failed: {}", e))
}
