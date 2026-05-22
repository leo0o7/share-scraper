use app_config::{AppConfig, DatabaseConfig, ScraperConfig};
use chrono::{NaiveTime, Utc};
use db::{
    isins::{insert_all_isins_with_progress, query_all_isins},
    metrics::InsertionMetrics,
    shares::{get_shares_to_refresh, insert_all_shares_with_progress},
};
use scraper::{get_elapsed_time, metrics::ScrapingMetrics, ScraperRuntime};
use tracing::{info, info_span, instrument, Instrument};

pub mod progress;

use progress::{
    IsinInsertProgress, IsinPageScrapeProgress, ProgressPhase, ProgressSender, ScrapeErrorCategory,
    ShareInsertProgress, ShareScrapeProgress,
};

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

    fn share_scraped(&self, completion: scraper::shares::ShareScrapeCompletion) {
        if let Some(progress) = &self.progress {
            progress.share_scraped(share_scrape_progress(completion));
        }
    }

    fn share_inserted(&self, completion: db::shares::ShareInsertCompletion) {
        if let Some(progress) = &self.progress {
            progress.share_inserted(share_insert_progress(completion));
        }
    }

    fn isin_page_scraped(&self, completion: scraper::isins::IsinScrapeCompletion) {
        if let Some(progress) = &self.progress {
            progress.isin_page_scraped(isin_page_scrape_progress(completion));
        }
    }

    fn isin_letter_completed(&self, letter: char) {
        if let Some(progress) = &self.progress {
            progress.isin_letter_completed(letter);
        }
    }

    fn isin_inserted(&self, completion: db::isins::IsinInsertCompletion) {
        if let Some(progress) = &self.progress {
            progress.isin_inserted(isin_insert_progress(completion));
        }
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
    completion: scraper::shares::ShareScrapeCompletion,
) -> ShareScrapeProgress {
    ShareScrapeProgress {
        isin: completion.isin,
        result: completion.result.map_err(scrape_error_category),
    }
}

fn isin_page_scrape_progress(
    completion: scraper::isins::IsinScrapeCompletion,
) -> IsinPageScrapeProgress {
    IsinPageScrapeProgress {
        letter: completion.letter,
        page: completion.page,
        isins_found: completion.isins_found,
        result: completion.result.map_err(scrape_error_category),
        parsing_errors: completion.parsing_errors,
    }
}

fn share_insert_progress(completion: db::shares::ShareInsertCompletion) -> ShareInsertProgress {
    ShareInsertProgress {
        isin: completion.isin,
        successful: completion.successful,
    }
}

fn isin_insert_progress(completion: db::isins::IsinInsertCompletion) -> IsinInsertProgress {
    IsinInsertProgress {
        isin: completion.isin,
        successful: completion.successful,
    }
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
            scraper::shares::scrape_all_shares_with_progress(runtime, share_isins, |event| {
                workflow_progress.share_scraped(event);
            }),
        )
        .await;
    let scraped_shares = shares.unmetric();
    let insertion_metrics = workflow_progress
        .run_phase(
            ProgressPhase::InsertShares,
            Some(scraped_shares.len() as u64),
            insert_all_shares_with_progress(scraped_shares, &pool, |event| {
                workflow_progress.share_inserted(event);
            }),
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
            scraper::shares::scrape_all_shares_with_progress(runtime, share_isins, |event| {
                workflow_progress.share_scraped(event);
            }),
        )
        .await;
    let scraped_shares = shares.unmetric();
    let insertion_metrics = workflow_progress
        .run_phase(
            ProgressPhase::InsertShares,
            Some(scraped_shares.len() as u64),
            insert_all_shares_with_progress(scraped_shares, &pool, |event| {
                workflow_progress.share_inserted(event);
            }),
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
            scraper::isins::scrape_all_isins_with_progress(
                runtime,
                |event| {
                    workflow_progress.isin_page_scraped(event);
                },
                |letter| {
                    workflow_progress.isin_letter_completed(letter);
                },
            ),
        )
        .await;
    let scraped_isins = isins.unmetric().into_iter().collect::<Vec<_>>();
    let pool = db::connect(database_config).await.unwrap();
    let insertion_metrics = workflow_progress
        .run_phase(
            ProgressPhase::InsertIsins,
            Some(scraped_isins.len() as u64),
            insert_all_isins_with_progress(scraped_isins, &pool, |event| {
                workflow_progress.isin_inserted(event);
            })
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
}
