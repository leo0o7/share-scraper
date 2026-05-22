mod models;
pub mod parsers;
pub(crate) mod property_selector;
pub use models::{share::Share, ScrapableStruct};

use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;
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
    scrape_all_shares_with_progress(runtime, share_isins, |_| {}).await
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareScrapeCompletion {
    pub isin: String,
    pub result: Result<(), ScrapingError>,
}

pub async fn scrape_all_shares_with_progress<F>(
    runtime: &ScraperRuntime,
    share_isins: Vec<ShareIsin>,
    mut on_completion: F,
) -> WithMetrics<Vec<Share>>
where
    F: FnMut(ShareScrapeCompletion),
{
    let mut metrics = ScrapingMetrics::empty();
    let total_shares = share_isins.len();
    metrics.total = total_shares as i32;

    let mut res: Vec<Share> = Vec::new();
    let permits = Arc::new(Semaphore::new(runtime.share_concurrency()));
    let mut tasks: FuturesUnordered<_> = share_isins
        .into_iter()
        .enumerate()
        .map(|(i, share_isin)| {
            let permits = Arc::clone(&permits);
            let runtime = runtime.clone();
            tokio::spawn(async move {
                let isin_str = share_isin.isin.to_string();
                let _permit = permits.acquire().await.unwrap();
                let result = scrape_share_with_max_duration(&runtime, share_isin)
                    .instrument(info_span!(
                        "scraping_share",
                        isin = isin_str,
                        curr = i,
                        total = total_shares,
                    ))
                    .await;
                (isin_str, result)
            })
        })
        .collect();

    while let Some(result) = tasks.next().await {
        match result {
            Ok((isin, Ok(result))) => {
                metrics.successful += 1;
                on_completion(ShareScrapeCompletion {
                    isin,
                    result: Ok(()),
                });
                res.push(result);
            }
            Ok((isin, Err(e))) => {
                metrics.errors.update(e.clone());
                on_completion(ShareScrapeCompletion {
                    isin,
                    result: Err(e),
                });
            }
            Err(e) => error!("task failed {e}"),
        }
    }
    info!("Scraped a total of {} shares.", res.len());

    WithMetrics::new(res, metrics)
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
