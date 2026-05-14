use app_config::{AppConfig, DatabaseConfig, ScraperConfig};
use chrono::{NaiveTime, Utc};
use db::{
    isins::{insert_all_isins, query_all_isins},
    metrics::InsertionMetrics,
    shares::{get_shares_to_refresh, insert_all_shares},
};
use scraper::{
    get_elapsed_time, isins::scrape_all_isins, metrics::ScrapingMetrics, shares::scrape_all_shares,
};
use tracing::{info, info_span, instrument, Instrument};

#[derive(Debug)]
pub struct ScrapeAndInsertInfo {
    pub metrics: ScrapeAndInsertMetrics,
    pub start_time: NaiveTime,
    pub duration_millis: i64,
}
#[derive(Debug)]
pub struct ScrapeAndInsertMetrics {
    pub scrape: ScrapingMetrics,
    pub insert: InsertionMetrics,
}

async fn run_timed<F, Fut>(operation: F) -> ScrapeAndInsertInfo
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ScrapeAndInsertMetrics>,
{
    let start_time = Utc::now().time();
    let metrics = operation().await;
    let duration = get_elapsed_time(start_time);

    info!("Total Time elapsed {}ms", duration);

    ScrapeAndInsertInfo {
        metrics,
        start_time,
        duration_millis: duration,
    }
}

pub async fn run_scrape_and_insert(config: &AppConfig) -> ScrapeAndInsertInfo {
    run_timed(|| scrape_and_insert_all_shares(&config.database)).await
}

pub async fn run_share_refresh(config: &AppConfig) -> ScrapeAndInsertInfo {
    run_timed(|| async move {
        refresh_shares(&config.database, chrono_share_refresh_age(&config.scraper)).await
    })
    .await
}

pub async fn run_scrape_and_insert_isins(config: &AppConfig) -> ScrapeAndInsertInfo {
    run_timed(|| scrape_and_insert_all_isins(&config.database)).await
}

fn chrono_share_refresh_age(config: &ScraperConfig) -> chrono::Duration {
    chrono::Duration::from_std(config.share_refresh_age)
        .expect("validated scraper refresh age should fit chrono duration")
}

#[instrument]
pub async fn refresh_shares(
    database_config: &DatabaseConfig,
    before: chrono::Duration,
) -> ScrapeAndInsertMetrics {
    info!("Refreshing all shares not updated in {:?}", before);

    let pool = db::connect(database_config).await.unwrap();
    let share_isins = get_shares_to_refresh(&pool, before)
        .await
        .expect("Failed to query shares to scrape");

    let mut shares = scrape_all_shares(share_isins).await;
    let insertion_metrics = insert_all_shares(shares.unmetric(), &pool).await;

    ScrapeAndInsertMetrics {
        scrape: shares.metrics,
        insert: insertion_metrics,
    }
}

#[instrument]
pub async fn scrape_and_insert_all_shares(
    database_config: &DatabaseConfig,
) -> ScrapeAndInsertMetrics {
    info!("Started scraping and inserting all shares");

    let pool = db::connect(database_config).await.unwrap();
    let share_isins = query_all_isins(&pool)
        .await
        .expect("Failed to query all ISINs");

    let mut shares = scrape_all_shares(share_isins).await;
    let insertion_metrics = insert_all_shares(shares.unmetric(), &pool).await;

    ScrapeAndInsertMetrics {
        scrape: shares.metrics,
        insert: insertion_metrics,
    }
}

#[instrument]
pub async fn scrape_and_insert_all_isins(
    database_config: &DatabaseConfig,
) -> ScrapeAndInsertMetrics {
    info!("Started scraping and inserting all isins");

    let mut isins = scrape_all_isins().await;
    let pool = db::connect(database_config).await.unwrap();
    let insertion_metrics = insert_all_isins(isins.unmetric().into_iter().collect(), &pool)
        .instrument(info_span!("insert_all_isins"))
        .await;

    ScrapeAndInsertMetrics {
        scrape: isins.metrics,
        insert: insertion_metrics,
    }
}
