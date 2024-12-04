use std::sync::Mutex;

use scraper_utils::run_scrape_and_insert;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    let log_file = std::fs::File::create("share_scraper.log").expect("Can't create log file");

    let file_logger = fmt::layer()
        .with_writer(Mutex::new(log_file))
        .with_ansi(false);
    let stdout_logger = fmt::layer().with_ansi(true);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(file_logger)
        .with(stdout_logger)
        .init();

    run_scrape_and_insert().await;
}
