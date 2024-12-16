mod models;
pub mod parsers;
mod property_selector;
pub use models::{share::Share, ScrapableStruct};

use futures::{stream::FuturesUnordered, StreamExt};
use scraper::Html;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info, info_span, warn, Instrument};

use crate::{
    errors::{ScraperResult, ScrapingError},
    get_page_text,
    isins::types::ShareIsin,
    metrics::{ScrapingMetrics, WithMetrics},
};
use property_selector::PropertySelector;

pub async fn scrape_all_shares(share_isins: Vec<ShareIsin>) -> WithMetrics<Vec<Share>> {
    let mut metrics = ScrapingMetrics::empty();
    let total_shares = share_isins.len();
    metrics.total = total_shares as i32;

    let mut tasks = FuturesUnordered::new();

    for (i, share_isin) in share_isins.into_iter().enumerate() {
        let isin_str = &share_isin.isin.to_string();
        tasks.push(
            scrape_share_with_max_duration(share_isin, 5 * 60).instrument(info_span!(
                "scraping_share",
                isin = isin_str,
                curr = i,
                total = total_shares,
            )),
        );
    }

    let mut res: Vec<Share> = Vec::new();

    while let Some(result) = tasks.next().await {
        match result {
            Ok(result) => {
                metrics.successful += 1;
                res.push(result);
            }
            Err(e) => metrics.errors.update(e),
        };
    }
    info!("Scraped a total of {} shares.", res.len());

    WithMetrics::new(res, metrics)
}

pub async fn scrape_share_with_max_duration(
    share_isin: ShareIsin,
    max_duration: u64,
) -> ScraperResult<Share> {
    match timeout(Duration::from_secs(max_duration), scrape_share(&share_isin)).await {
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

pub async fn scrape_share(share_isin: &ShareIsin) -> ScraperResult<Share> {
    let isin = &share_isin.isin;
    let url = format!(
        "https://www.borsaitaliana.it/borsa/azioni/dati-completi.html?isin={}&lang=it",
        isin
    );

    let res_txt = get_page_text(&url)
        .instrument(info_span!("fetching_page"))
        .await?;

    let share = parse_page(res_txt, share_isin);
    Ok(share)
}

fn parse_page(res_txt: String, share_isin: &ShareIsin) -> Share {
    let doc = Html::parse_document(&res_txt);
    let selector = PropertySelector::new(&doc);

    Share::from_selector(share_isin, &selector)
}
