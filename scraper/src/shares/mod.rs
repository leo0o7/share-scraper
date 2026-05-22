mod models;
pub mod parsers;
pub(crate) mod property_selector;
pub use models::{share::Share, ScrapableStruct};

use futures::stream::{FuturesUnordered, StreamExt};
use std::{future::Future, sync::Arc};
use tokio::{sync::Semaphore, time::timeout};
use tracing::{error, info, info_span, warn, Instrument};

use crate::{
    errors::{ScraperResult, ScrapingError},
    isins::types::ShareIsin,
    metrics::{ScrapingMetrics, WithMetrics},
    ScraperRuntime,
};

pub async fn scrape_all_shares(
    runtime: &ScraperRuntime,
    share_isins: Vec<ShareIsin>,
) -> WithMetrics<Vec<Share>> {
    scrape_all_shares_with_progress(runtime, share_isins, NoopShareScrapeProgress).await
}

struct ShareScraper<F> {
    share_isins: Vec<ShareIsin>,
    concurrency: usize,
    scrape_share: F,
}

impl<F> ShareScraper<F> {
    fn new(share_isins: Vec<ShareIsin>, concurrency: usize, scrape_share: F) -> Self {
        Self {
            share_isins,
            concurrency,
            scrape_share,
        }
    }
}

impl<F, Fut> ShareScraper<F>
where
    F: Fn(ShareIsin) -> Fut + Clone,
    Fut: Future<Output = ScraperResult<Share>>,
{
    async fn scrape_observing<P>(self, progress: P) -> WithMetrics<Vec<Share>>
    where
        P: ShareScrapeProgress,
    {
        let mut metrics = ScrapingMetrics::empty();
        let total_shares = self.share_isins.len();
        metrics.total = total_shares as i32;

        let mut res: Vec<Share> = Vec::new();
        let permits = Arc::new(Semaphore::new(self.concurrency));
        let mut tasks: FuturesUnordered<_> = self
            .share_isins
            .into_iter()
            .enumerate()
            .map(|(i, share_isin)| {
                let permits = Arc::clone(&permits);
                let scrape_share = self.scrape_share.clone();
                async move {
                    let isin_str = share_isin.isin.to_string();
                    let _permit = permits.acquire().await.unwrap();
                    let result = scrape_share(share_isin)
                        .instrument(info_span!(
                            "scraping_share",
                            isin = isin_str,
                            curr = i,
                            total = total_shares,
                        ))
                        .await;
                    (isin_str, result)
                }
            })
            .collect();

        while let Some((isin, result)) = tasks.next().await {
            match result {
                Ok(result) => {
                    metrics.successful += 1;
                    progress.share_scraped(isin, Ok(()));
                    res.push(result);
                }
                Err(e) => {
                    metrics.errors.update(e.clone());
                    progress.share_scraped(isin, Err(e));
                }
            }
        }
        info!("Scraped a total of {} shares.", res.len());

        WithMetrics::new(res, metrics)
    }
}

pub trait ShareScrapeProgress: Clone {
    fn share_scraped(&self, isin: String, result: ScraperResult<()>);
}

#[derive(Clone)]
pub struct NoopShareScrapeProgress;

impl ShareScrapeProgress for NoopShareScrapeProgress {
    fn share_scraped(&self, _isin: String, _result: ScraperResult<()>) {}
}

pub async fn scrape_all_shares_with_progress<P>(
    runtime: &ScraperRuntime,
    share_isins: Vec<ShareIsin>,
    progress: P,
) -> WithMetrics<Vec<Share>>
where
    P: ShareScrapeProgress,
{
    ShareScraper::new(
        share_isins,
        runtime.share_concurrency(),
        |share_isin| async move { scrape_share_with_max_duration(runtime, share_isin).await },
    )
    .scrape_observing(progress)
    .await
}

pub async fn scrape_share_with_max_duration(
    runtime: &ScraperRuntime,
    share_isin: ShareIsin,
) -> ScraperResult<Share> {
    match timeout(runtime.share_timeout(), scrape_share(runtime, &share_isin)).await {
        Ok(res) => {
            if let Err(e) = &res {
                warn!("Error scraping share {:?}", e);
            } else {
                info!("Finished scraping share");
            }

            res
        }
        Err(_) => {
            error!("Operation timed out");
            Err(ScrapingError::Timeout)
        }
    }
}

pub async fn scrape_share(
    runtime: &ScraperRuntime,
    share_isin: &ShareIsin,
) -> ScraperResult<Share> {
    let isin = &share_isin.isin;
    let url = format!(
        "https://www.borsaitaliana.it/borsa/azioni/dati-completi.html?isin={}&lang=it",
        isin
    );

    let res_txt = runtime
        .get_page_text(url)
        .instrument(info_span!("fetching_page"))
        .await?;

    let share = runtime.parse_share_page(res_txt, share_isin).await;
    Ok(share)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::shares::ScrapableStruct;

    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct ObservedShareScrape {
        isin: String,
        result: ScraperResult<()>,
    }

    #[derive(Clone)]
    struct RecordingShareScrapeProgress {
        events: Arc<Mutex<Vec<ObservedShareScrape>>>,
    }

    impl ShareScrapeProgress for RecordingShareScrapeProgress {
        fn share_scraped(&self, isin: String, result: ScraperResult<()>) {
            self.events
                .lock()
                .unwrap()
                .push(ObservedShareScrape { isin, result });
        }
    }

    fn share_isin(isin: &str) -> ShareIsin {
        ShareIsin::new(format!("Share {isin}"), isin.to_string()).unwrap()
    }

    #[tokio::test]
    async fn share_scraper_records_metrics_and_progress_from_injected_scraper() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let progress = RecordingShareScrapeProgress {
            events: Arc::clone(&events),
        };
        let isins = vec![share_isin("IT0000000001"), share_isin("IT0000000002")];

        let mut shares = ShareScraper::new(isins, 2, |share_isin: ShareIsin| async move {
            if share_isin.isin.to_string().ends_with('1') {
                Ok(Share::with_isin(&share_isin))
            } else {
                Err(ScrapingError::Timeout)
            }
        })
        .scrape_observing(progress)
        .await;

        assert_eq!(shares.metrics.total, 2);
        assert_eq!(shares.metrics.successful, 1);
        assert_eq!(shares.metrics.errors.timeout, 1);
        assert_eq!(shares.unmetric().len(), 1);

        let mut events = events.lock().unwrap().clone();
        events.sort_by(|left, right| left.isin.cmp(&right.isin));
        assert_eq!(
            events,
            vec![
                ObservedShareScrape {
                    isin: "IT0000000001".to_string(),
                    result: Ok(()),
                },
                ObservedShareScrape {
                    isin: "IT0000000002".to_string(),
                    result: Err(ScrapingError::Timeout),
                },
            ]
        );
    }
}
