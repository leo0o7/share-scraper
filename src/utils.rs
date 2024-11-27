use crate::db::connect;
use crate::db::utils::insert_all_isins;
use crate::isins::scrape_all_isins;
use std::time::SystemTime;

pub async fn scrape_and_insert_all_isins() {
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
