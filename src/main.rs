use std::time::SystemTime;

use db::{
    connect,
    utils::{insert_all_shares, query_all_isins},
};
use futures::{stream::FuturesUnordered, StreamExt};
use shares::{models::Share, scrape_share};
use utils::get_elapsed_time;

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

    let pool = match connect().await {
        Ok(pool) => pool,
        Err(e) => panic!("Couldn't connect to DB cause of {e}"),
    };

    let share_isins = query_all_isins(&pool).await.unwrap();

    let mut tasks = FuturesUnordered::new();

    for share_isin in share_isins {
        tasks.push(scrape_share(share_isin));
    }

    let mut res: Vec<Share> = Vec::new();

    while let Some(result) = tasks.next().await {
        match serde_json::to_string_pretty(&result) {
            Ok(formatted_share) => {
                println!("Scraped Share Information:\n{}", formatted_share);
            }
            Err(e) => {
                eprintln!("Error serializing share: {}", e);
                println!("Fallback debug print:\n{:#?}", result);
            }
        }
        res.push(result);
    }
    println!("Scraped {} shares.", res.len());

    insert_all_shares(res, &pool).await;

    println!("Time elapsed {}", get_elapsed_time(start_time));
}
