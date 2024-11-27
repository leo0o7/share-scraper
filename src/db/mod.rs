pub mod utils;

use dotenv::dotenv;
use sqlx::{postgres::PgPoolOptions, Error, Pool, Postgres};
use std::env;

pub async fn connect() -> Result<Pool<Postgres>, Error> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL variable required");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
}
