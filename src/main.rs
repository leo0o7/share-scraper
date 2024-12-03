use std::{sync::Mutex, time::SystemTime};
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
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
    let log_file = std::fs::File::create("share_scraper.log").expect("Can't create log file");

    let file_logger = fmt::layer()
        .with_writer(Mutex::new(log_file))
        .with_ansi(true);
    let stdout_logger = fmt::layer().with_ansi(true);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(file_logger)
        .with(stdout_logger)
        .init();

    let start_time = SystemTime::now();

    scrape_and_insert_all_shares().await;

    info!("Total Time elapsed {}s", get_elapsed_time(start_time));
}
