use futures::{stream::FuturesUnordered, StreamExt};
use sqlx::{query, Pool, Postgres};

use crate::isins::types::ShareIsin;

pub async fn insert_all_isins(isins: Vec<ShareIsin>, pool: &Pool<Postgres>) {
    println!("Total ISINs found: {}", isins.len());

    let mut tasks = FuturesUnordered::new();

    for isin in isins {
        tasks.push(insert_isin(isin, pool));
    }

    while let Some(res) = tasks.next().await {
        if let Err(e) = res {
            eprint!("Unable to insert ISIN, {e}");
        }
    }
}

pub async fn insert_isin(isin: ShareIsin, pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    let _ = query!(
        "INSERT INTO share_isins (isin, share_name) VALUES ($1, $2)",
        isin.isin.get_str(),
        isin.name
    )
    .execute(pool)
    .await?;
    Ok(())
}
