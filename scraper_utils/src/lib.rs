use std::time::SystemTime;

use db::{
    connect,
    utils::{insert_all_isins, insert_all_shares, query_all_isins},
};
use scraper::{isins::scrape_all_isins, shares::scrape_all_shares};
use tracing::{error, info, instrument};

pub async fn run_scrape_and_insert() {
    let start_time = SystemTime::now();

    scrape_and_insert_all_shares().await;

    info!("Total Time elapsed {}s", get_elapsed_time(start_time));
}

#[instrument]
pub async fn scrape_and_insert_all_shares() {
    info!("Started scraping and inserting all shares");

    let pool = match connect().await {
        Ok(pool) => pool,
        Err(e) => {
            error!("Couldn't connect to DB cause of {e}");
            panic!()
        }
    };

    let share_isins = query_all_isins(&pool).await.unwrap();

    let shares = scrape_all_shares(share_isins).await;

    insert_all_shares(shares, &pool).await;
}

#[instrument]
pub async fn scrape_and_insert_all_isins() {
    info!("Started scraping and inserting all isins");

    let isins = scrape_all_isins().await;

    if let Ok(pool) = connect().await {
        insert_all_isins(isins, &pool).await;
    }
}

pub fn get_elapsed_time(time: SystemTime) -> u64 {
    match time.elapsed() {
        Ok(duration) => duration.as_secs(),
        // pretty bad if we can't do that
        Err(_) => panic!("Error calculating time elapsed"),
    }
}
