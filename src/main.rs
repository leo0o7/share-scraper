use std::time::SystemTime;

use utils::{get_elapsed_time, scrape_and_insert_all_shares};

mod db;
mod exponential_backoff;
mod isins;
mod shares;
mod utils;

// TEST SCRAPING SHARE SHARE IN DB
// let share_isin = ShareIsin::new("Beghelli", "IT0001223277").unwrap();
// let share = scrape_share(share_isin).await;

// TEST GETTING SHARE IN DB
// let pool = match connect().await {
//     Ok(pool) => pool,
//     Err(e) => panic!("Couldn't connect to DB cause of {e}"),
// };
// let result = Share::from(
//     get_share_by_isin("IT0001223277", &pool)
//         .await
//         .unwrap()
//         .unwrap(),
// );

#[tokio::main]
async fn main() {
    let start_time = SystemTime::now();

    scrape_and_insert_all_shares().await;

    println!("Total Time elapsed {}s", get_elapsed_time(start_time));
}
