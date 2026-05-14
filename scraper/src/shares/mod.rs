mod models;
pub mod parsers;
mod property_selector;
pub use models::{share::Share, ScrapableStruct};

use futures::future::join_all;
use html_scraper::Html;
use once_cell::sync::Lazy;
use std::{sync::Arc, time::Duration};
use tokio::{sync::Semaphore, time::timeout};
use tracing::{error, info, info_span, warn, Instrument};

use crate::{
    errors::{ScraperResult, ScrapingError},
    isins::types::ShareIsin,
    metrics::{ScrapingMetrics, WithMetrics},
    ScraperRuntime,
};
use property_selector::PropertySelector;

static PARSE_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .build()
        .unwrap()
});

pub async fn scrape_all_shares(
    runtime: &ScraperRuntime,
    share_isins: Vec<ShareIsin>,
) -> WithMetrics<Vec<Share>> {
    let mut metrics = ScrapingMetrics::empty();
    let total_shares = share_isins.len();
    metrics.total = total_shares as i32;

    let mut res: Vec<Share> = Vec::new();
    let permits = Arc::new(Semaphore::new(200));
    let tasks: Vec<_> = share_isins
        .into_iter()
        .enumerate()
        .map(|(i, share_isin)| {
            let permits = Arc::clone(&permits);
            let runtime = runtime.clone();
            tokio::spawn(async move {
                let isin_str = &share_isin.isin.to_string();
                let _permit = permits.acquire().await.unwrap();
                scrape_share_with_max_duration(&runtime, share_isin, 5 * 60)
                    .instrument(info_span!(
                        "scraping_share",
                        isin = isin_str,
                        curr = i,
                        total = total_shares,
                    ))
                    .await
            })
        })
        .collect();

    for result in join_all(tasks).await {
        match result {
            Ok(Ok(result)) => {
                metrics.successful += 1;
                res.push(result);
            }
            Ok(Err(e)) => metrics.errors.update(e),
            Err(e) => error!("task failed {e}"),
        }
    }
    info!("Scraped a total of {} shares.", res.len());

    WithMetrics::new(res, metrics)
}

pub async fn scrape_share_with_max_duration(
    runtime: &ScraperRuntime,
    share_isin: ShareIsin,
    max_duration: u64,
) -> ScraperResult<Share> {
    match timeout(
        Duration::from_secs(max_duration),
        scrape_share(runtime, &share_isin),
    )
    .await
    {
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

    let share = parse_page(res_txt, share_isin).await;
    Ok(share)
}

async fn parse_page(res_txt: String, share_isin: &ShareIsin) -> Share {
    let share_isin = share_isin.clone();
    let (sender, receiver) = tokio::sync::oneshot::channel();

    PARSE_POOL.spawn(move || {
        let doc = Html::parse_document(&res_txt);
        let selector = PropertySelector::new(&doc);
        let share = Share::from_selector(&share_isin, &selector);
        let _ = sender.send(share);
    });

    receiver.await.unwrap()
}
// fn parse_page(res_txt: String, share_isin: &ShareIsin) -> Share {
//     let doc = Html::parse_document(&res_txt);
//     let selector = PropertySelector::new(&doc);
//
//     Share::from_selector(share_isin, &selector)
// }
