use futures::{stream::FuturesUnordered, StreamExt};
use sqlx::{query, query_as, Pool, Postgres};
use tracing::{error, info};

use scraper::isins::types::ShareIsin;

use crate::metrics::InsertionMetrics;

pub async fn insert_all_isins(isins: Vec<ShareIsin>, pool: &Pool<Postgres>) -> InsertionMetrics {
    let isin_num = isins.len() as i32;

    let mut tasks = FuturesUnordered::new();

    info!("Total ISINs found: {}", isins.len());
    for isin in isins.clone() {
        tasks.push(insert_isin(isin, pool));
    }

    let mut curr_idx = 0;
    let mut successful_inserts = 0;
    while let Some(res) = tasks.next().await {
        curr_idx += 1;

        if let Err(e) = res {
            error!(
                "Unable to insert ISIN {}/{}, ({}) {}",
                curr_idx,
                isin_num,
                isins[curr_idx - 1].isin,
                e
            );
        } else {
            info!(
                "Inserted ISIN {}/{}, ({})",
                curr_idx,
                isin_num,
                isins[curr_idx - 1].isin
            );
            successful_inserts += 1;
        }
    }

    InsertionMetrics {
        total: isin_num,
        successful: successful_inserts,
    }
}

pub async fn insert_isin(
    isin: ShareIsin,
    pool: &Pool<Postgres>,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    query!(
        "INSERT INTO share_isins (isin, share_name, updated_at) VALUES ($1, $2, $3)",
        isin.isin.to_string(),
        isin.share_name,
        isin.updated_at,
    )
    .execute(pool)
    .await
}

pub async fn query_all_isins(pool: &Pool<Postgres>) -> Result<Vec<ShareIsin>, sqlx::Error> {
    info!("Querying all isins from db");
    let share_isins: Vec<ShareIsin> = query_as("SELECT * FROM share_isins")
        .fetch_all(pool)
        .await?;
    info!("Got a total of {} from db", share_isins.len());

    Ok(share_isins)
}
