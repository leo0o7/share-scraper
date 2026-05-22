use futures::{stream::FuturesUnordered, StreamExt};
use sqlx::{query, query_as, Pool, Postgres};
use tracing::{error, info};

use scraper::isins::types::ShareIsin;

use crate::metrics::InsertionMetrics;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsinInsertCompletion {
    pub isin: String,
    pub successful: bool,
}

pub async fn insert_all_isins(isins: Vec<ShareIsin>, pool: &Pool<Postgres>) -> InsertionMetrics {
    insert_all_isins_with_progress(isins, pool, |_| {}).await
}

pub async fn insert_all_isins_with_progress(
    isins: Vec<ShareIsin>,
    pool: &Pool<Postgres>,
    on_completion: impl Fn(IsinInsertCompletion),
) -> InsertionMetrics {
    let isin_num = isins.len() as i32;

    let mut tasks = FuturesUnordered::new();

    info!("Total ISINs found: {}", isins.len());
    for isin in isins {
        let isin_str = isin.isin.to_string();
        tasks.push(async move { (isin_str, insert_isin(isin, pool).await) });
    }

    let mut curr_idx = 0;
    let mut successful_inserts = 0;
    let mut failed_inserts = 0;
    while let Some((isin, res)) = tasks.next().await {
        curr_idx += 1;

        let successful = if let Err(e) = res {
            error!(
                "Unable to insert ISIN {}/{}, ({}) {}",
                curr_idx, isin_num, isin, e
            );
            failed_inserts += 1;
            false
        } else {
            info!("Inserted ISIN {}/{}, ({})", curr_idx, isin_num, isin);
            successful_inserts += 1;
            true
        };
        on_completion(IsinInsertCompletion { isin, successful });
    }

    InsertionMetrics {
        total: isin_num,
        successful: successful_inserts,
        failed: failed_inserts,
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
