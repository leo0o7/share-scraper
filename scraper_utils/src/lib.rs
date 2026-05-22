use app_config::{AppConfig, DatabaseConfig, ScraperConfig};
use chrono::{NaiveTime, Utc};
use db::{isins::query_all_isins, metrics::InsertionMetrics, shares::get_shares_to_refresh};
use futures::{stream::FuturesUnordered, StreamExt};
use scraper::{get_elapsed_time, metrics::ScrapingMetrics, ScraperRuntime};
use sqlx::{Pool, Postgres};
use tracing::{info, info_span, instrument, Instrument};

pub mod progress;

use progress::{
    IsinInsertProgress, ProgressPhase, ProgressSender, ScrapeErrorCategory, ShareInsertProgress,
    ShareScrapeProgress,
};

#[derive(Clone)]
struct WorkflowProgress {
    progress: Option<ProgressSender>,
}

impl WorkflowProgress {
    fn new(progress: Option<ProgressSender>) -> Self {
        Self { progress }
    }

    async fn run_phase<F, T>(&self, phase: ProgressPhase, total: Option<u64>, operation: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        if let Some(progress) = &self.progress {
            progress.phase_started(phase, total).await;
        }

        let result = operation.await;

        if let Some(progress) = &self.progress {
            progress.phase_finished(phase).await;
        }

        result
    }

    fn share_scraped(&self, isin: String, result: scraper::errors::ScraperResult<()>) {
        if let Some(progress) = &self.progress {
            progress.share_scraped(share_scrape_progress(isin, result));
        }
    }

    fn share_inserted(&self, isin: String, successful: bool) {
        if let Some(progress) = &self.progress {
            progress.share_inserted(ShareInsertProgress { isin, successful });
        }
    }

    fn isin_page_scraped(
        &self,
        letter: char,
        page: u8,
        isins_found: u64,
        result: scraper::errors::ScraperResult<()>,
        parsing_errors: u64,
    ) {
        if let Some(progress) = &self.progress {
            progress.isin_page_scraped(progress::IsinPageScrapeProgress {
                letter,
                page,
                isins_found,
                result: result.map_err(scrape_error_category),
                parsing_errors,
            });
        }
    }

    fn isin_letter_completed(&self, letter: char) {
        if let Some(progress) = &self.progress {
            progress.isin_letter_completed(letter);
        }
    }

    fn isin_inserted(&self, isin: String, successful: bool) {
        if let Some(progress) = &self.progress {
            progress.isin_inserted(IsinInsertProgress { isin, successful });
        }
    }
}

impl scraper::isins::IsinCrawlProgress for WorkflowProgress {
    fn page_scraped(
        &self,
        letter: char,
        page: u8,
        isins_found: u64,
        result: scraper::errors::ScraperResult<()>,
        parsing_errors: u64,
    ) {
        self.isin_page_scraped(letter, page, isins_found, result, parsing_errors);
    }

    fn letter_completed(&self, letter: char) {
        self.isin_letter_completed(letter);
    }
}

impl scraper::shares::ShareScrapeProgress for WorkflowProgress {
    fn share_scraped(&self, isin: String, result: scraper::errors::ScraperResult<()>) {
        self.share_scraped(isin, result);
    }
}

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
    run_scrape_and_insert_with_progress(config, None).await
}

pub async fn run_scrape_and_insert_with_progress(
    config: &AppConfig,
    progress: Option<ProgressSender>,
) -> ScrapeAndInsertInfo {
    let runtime = ScraperRuntime::new(&config.scraper)
        .expect("validated scraper configuration should build HTTP client");
    run_timed(|| scrape_and_insert_all_shares(&config.database, &runtime, progress)).await
}

pub async fn run_share_refresh(config: &AppConfig) -> ScrapeAndInsertInfo {
    run_share_refresh_with_progress(config, None).await
}

pub async fn run_share_refresh_with_progress(
    config: &AppConfig,
    progress: Option<ProgressSender>,
) -> ScrapeAndInsertInfo {
    let runtime = ScraperRuntime::new(&config.scraper)
        .expect("validated scraper configuration should build HTTP client");
    run_timed(|| async move {
        refresh_shares(
            &config.database,
            &runtime,
            chrono_share_refresh_age(&config.scraper),
            progress,
        )
        .await
    })
    .await
}

pub async fn run_scrape_and_insert_isins(config: &AppConfig) -> ScrapeAndInsertInfo {
    run_scrape_and_insert_isins_with_progress(config, None).await
}

pub async fn run_scrape_and_insert_isins_with_progress(
    config: &AppConfig,
    progress: Option<ProgressSender>,
) -> ScrapeAndInsertInfo {
    let runtime = ScraperRuntime::new(&config.scraper)
        .expect("validated scraper configuration should build HTTP client");
    run_timed(|| scrape_and_insert_all_isins(&config.database, &runtime, progress)).await
}

fn chrono_share_refresh_age(config: &ScraperConfig) -> chrono::Duration {
    chrono::Duration::from_std(config.share_refresh_age)
        .expect("validated scraper refresh age should fit chrono duration")
}

fn scrape_error_category(error: scraper::errors::ScrapingError) -> ScrapeErrorCategory {
    match error {
        scraper::errors::ScrapingError::NetworkError => ScrapeErrorCategory::NetworkError,
        scraper::errors::ScrapingError::InvalidPage => ScrapeErrorCategory::InvalidPage,
        scraper::errors::ScrapingError::Timeout => ScrapeErrorCategory::Timeout,
        scraper::errors::ScrapingError::MaxRetries => ScrapeErrorCategory::MaxRetries,
        scraper::errors::ScrapingError::ParsingErr => ScrapeErrorCategory::ParsingError,
    }
}

fn share_scrape_progress(
    isin: String,
    result: scraper::errors::ScraperResult<()>,
) -> ShareScrapeProgress {
    ShareScrapeProgress {
        isin,
        result: result.map_err(scrape_error_category),
    }
}

async fn insert_items_with_progress<T, F, Fut, E, Label, Report>(
    items: Vec<T>,
    item_label: Label,
    insert: F,
    report: Report,
    item_kind: &str,
) -> InsertionMetrics
where
    F: Fn(T) -> Fut,
    Fut: std::future::Future<Output = Result<(), E>>,
    E: std::fmt::Display,
    Label: Fn(&T) -> String,
    Report: Fn(String, bool),
{
    let total = items.len() as i32;
    let mut tasks = FuturesUnordered::new();

    info!("Inserting a total of {} {}s", total, item_kind);

    for item in items {
        let label = item_label(&item);
        let future = insert(item);
        tasks.push(async move { (label, future.await) });
    }

    let mut curr_idx = 0;
    let mut successful_inserts = 0;
    let mut failed_inserts = 0;

    while let Some((label, result)) = tasks.next().await {
        curr_idx += 1;

        let successful = if let Err(error) = result {
            tracing::error!(
                "Unable to insert {} {}/{}, ({}) {}",
                item_kind,
                curr_idx,
                total,
                label,
                error
            );
            failed_inserts += 1;
            false
        } else {
            info!("Inserted {} {}/{}, ({})", item_kind, curr_idx, total, label);
            successful_inserts += 1;
            true
        };

        report(label, successful);
    }

    InsertionMetrics {
        total,
        successful: successful_inserts,
        failed: failed_inserts,
    }
}

async fn insert_shares_with_progress(
    shares: Vec<scraper::shares::Share>,
    pool: &Pool<Postgres>,
    progress: WorkflowProgress,
) -> InsertionMetrics {
    insert_items_with_progress(
        shares,
        |share| share.share_id.isin.to_string(),
        |share| async { db::shares::insert_share(share, pool).await },
        |isin, successful| progress.share_inserted(isin, successful),
        "Share",
    )
    .await
}

async fn insert_isins_with_progress(
    isins: Vec<scraper::isins::types::ShareIsin>,
    pool: &Pool<Postgres>,
    progress: WorkflowProgress,
) -> InsertionMetrics {
    insert_items_with_progress(
        isins,
        |isin| isin.isin.to_string(),
        |isin| async { db::isins::insert_isin(isin, pool).await.map(|_| ()) },
        |isin, successful| progress.isin_inserted(isin, successful),
        "ISIN",
    )
    .await
}

#[instrument(skip_all)]
pub async fn refresh_shares(
    database_config: &DatabaseConfig,
    runtime: &ScraperRuntime,
    before: chrono::Duration,
    progress: Option<ProgressSender>,
) -> ScrapeAndInsertMetrics {
    info!("Refreshing all shares not updated in {:?}", before);
    let workflow_progress = WorkflowProgress::new(progress);

    let pool = db::connect(database_config).await.unwrap();
    let share_isins = workflow_progress
        .run_phase(ProgressPhase::LoadStaleShares, None, async {
            get_shares_to_refresh(&pool, before)
                .await
                .expect("Failed to query shares to scrape")
        })
        .await;

    let mut shares = workflow_progress
        .run_phase(
            ProgressPhase::ScrapeShares,
            Some(share_isins.len() as u64),
            scraper::shares::scrape_all_shares_with_progress(
                runtime,
                share_isins,
                workflow_progress.clone(),
            ),
        )
        .await;
    let scraped_shares = shares.unmetric();
    let insertion_metrics = workflow_progress
        .run_phase(
            ProgressPhase::InsertShares,
            Some(scraped_shares.len() as u64),
            insert_shares_with_progress(scraped_shares, &pool, workflow_progress.clone()),
        )
        .await;

    ScrapeAndInsertMetrics {
        scrape: shares.metrics,
        insert: insertion_metrics,
    }
}

#[instrument(skip_all)]
pub async fn scrape_and_insert_all_shares(
    database_config: &DatabaseConfig,
    runtime: &ScraperRuntime,
    progress: Option<ProgressSender>,
) -> ScrapeAndInsertMetrics {
    info!("Started scraping and inserting all shares");
    let workflow_progress = WorkflowProgress::new(progress);

    let pool = db::connect(database_config).await.unwrap();
    let share_isins = workflow_progress
        .run_phase(ProgressPhase::LoadShareIsins, None, async {
            query_all_isins(&pool)
                .await
                .expect("Failed to query all ISINs")
        })
        .await;

    let mut shares = workflow_progress
        .run_phase(
            ProgressPhase::ScrapeShares,
            Some(share_isins.len() as u64),
            scraper::shares::scrape_all_shares_with_progress(
                runtime,
                share_isins,
                workflow_progress.clone(),
            ),
        )
        .await;
    let scraped_shares = shares.unmetric();
    let insertion_metrics = workflow_progress
        .run_phase(
            ProgressPhase::InsertShares,
            Some(scraped_shares.len() as u64),
            insert_shares_with_progress(scraped_shares, &pool, workflow_progress.clone()),
        )
        .await;

    ScrapeAndInsertMetrics {
        scrape: shares.metrics,
        insert: insertion_metrics,
    }
}

#[instrument(skip_all)]
pub async fn scrape_and_insert_all_isins(
    database_config: &DatabaseConfig,
    runtime: &ScraperRuntime,
    progress: Option<ProgressSender>,
) -> ScrapeAndInsertMetrics {
    info!("Started scraping and inserting all isins");
    let workflow_progress = WorkflowProgress::new(progress);

    let mut isins = workflow_progress
        .run_phase(
            ProgressPhase::ScrapeIsins,
            None,
            scraper::isins::scrape_all_isins_with_progress(runtime, workflow_progress.clone()),
        )
        .await;
    let scraped_isins = isins.unmetric().into_iter().collect::<Vec<_>>();
    let pool = db::connect(database_config).await.unwrap();
    let insertion_metrics = workflow_progress
        .run_phase(
            ProgressPhase::InsertIsins,
            Some(scraped_isins.len() as u64),
            insert_isins_with_progress(scraped_isins, &pool, workflow_progress.clone())
                .instrument(info_span!("insert_all_isins")),
        )
        .await;

    ScrapeAndInsertMetrics {
        scrape: isins.metrics,
        insert: insertion_metrics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use progress::ProgressEvent;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn workflow_progress_emits_phase_lifecycle_around_work() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let progress = WorkflowProgress::new(Some(ProgressSender::new(sender)));

        let result = progress
            .run_phase(ProgressPhase::ScrapeShares, Some(2), async { 7 })
            .await;
        drop(progress);

        assert_eq!(result, 7);
        assert_eq!(
            receiver.recv().await,
            Some(ProgressEvent::PhaseStarted {
                phase: ProgressPhase::ScrapeShares,
                total: Some(2),
            })
        );
        assert_eq!(
            receiver.recv().await,
            Some(ProgressEvent::PhaseFinished {
                phase: ProgressPhase::ScrapeShares,
            })
        );
        assert_eq!(receiver.recv().await, None);
    }

    #[tokio::test]
    async fn workflow_progress_runs_without_a_progress_sender() {
        let progress = WorkflowProgress::new(None);

        let result = progress
            .run_phase(ProgressPhase::InsertShares, Some(1), async { "saved" })
            .await;

        assert_eq!(result, "saved");
    }

    #[tokio::test]
    async fn workflow_progress_forwards_isin_crawl_updates_as_normalized_events() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let progress = WorkflowProgress::new(Some(ProgressSender::new(sender)));

        progress.isin_page_scraped('A', 2, 3, Ok(()), 1);
        progress.isin_letter_completed('A');
        drop(progress);

        assert_eq!(
            receiver.recv().await,
            Some(ProgressEvent::IsinPageScraped {
                letter: 'A',
                page: 2,
                isins_found: 3,
                result: Ok(()),
                parsing_errors: 1,
            })
        );
        assert_eq!(
            receiver.recv().await,
            Some(ProgressEvent::IsinLetterCompleted { letter: 'A' })
        );
        assert_eq!(receiver.recv().await, None);
    }

    #[tokio::test]
    async fn workflow_progress_forwards_share_scrape_updates_as_normalized_events() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let progress = WorkflowProgress::new(Some(ProgressSender::new(sender)));

        progress.share_scraped(
            "IT0000000001".to_string(),
            Err(scraper::errors::ScrapingError::Timeout),
        );
        drop(progress);

        assert_eq!(
            receiver.recv().await,
            Some(ProgressEvent::ShareScraped {
                isin: "IT0000000001".to_string(),
                result: Err(ScrapeErrorCategory::Timeout),
            })
        );
        assert_eq!(receiver.recv().await, None);
    }

    #[tokio::test]
    async fn workflow_progress_forwards_insert_updates_as_normalized_events() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let progress = WorkflowProgress::new(Some(ProgressSender::new(sender)));

        progress.share_inserted("IT0000000001".to_string(), true);
        progress.isin_inserted("IT0000000002".to_string(), false);
        drop(progress);

        assert_eq!(
            receiver.recv().await,
            Some(ProgressEvent::ShareInserted {
                isin: "IT0000000001".to_string(),
                successful: true,
            })
        );
        assert_eq!(
            receiver.recv().await,
            Some(ProgressEvent::IsinInserted {
                isin: "IT0000000002".to_string(),
                successful: false,
            })
        );
        assert_eq!(receiver.recv().await, None);
    }

    #[tokio::test]
    async fn insert_boundary_reports_metrics_and_progress_without_database_payloads() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let progress = WorkflowProgress::new(Some(ProgressSender::new(sender)));
        let items = vec![
            ("IT0000000001".to_string(), true),
            ("IT0000000002".to_string(), false),
        ];

        let metrics = insert_items_with_progress(
            items,
            |(isin, _)| isin.clone(),
            |(_, succeeds)| async move { succeeds.then_some(()).ok_or("insert failed") },
            |isin, successful| progress.isin_inserted(isin, successful),
            "ISIN",
        )
        .await;
        drop(progress);

        assert_eq!(metrics.total, 2);
        assert_eq!(metrics.successful, 1);
        assert_eq!(metrics.failed, 1);

        let mut events = Vec::new();
        while let Some(event) = receiver.recv().await {
            events.push(event);
        }
        events.sort_by_key(|event| match event {
            ProgressEvent::IsinInserted { isin, .. } => isin.clone(),
            _ => String::new(),
        });

        assert_eq!(
            events,
            vec![
                ProgressEvent::IsinInserted {
                    isin: "IT0000000001".to_string(),
                    successful: true,
                },
                ProgressEvent::IsinInserted {
                    isin: "IT0000000002".to_string(),
                    successful: false,
                },
            ]
        );
    }
}
