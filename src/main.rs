use std::time::SystemTime;

use isins::scrape_all_isins;
use utils::{get_elapsed_time, scrape_and_insert_all_isins};

mod db;
mod exponential_backoff;
mod isins;
mod utils;

#[tokio::main]
async fn main() {
    let start_time = SystemTime::now();

    println!("Time elapsed {}", get_elapsed_time(start_time));
}
