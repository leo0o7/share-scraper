use std::time::Duration;

use futures::{stream::FuturesUnordered, StreamExt};
use models::share::Share;
use models::ScrapableStruct;
use property_selector::PropertySelector;
use scraper::Html;
use tokio::time::timeout;
use tracing::error;
use tracing::info;
use tracing::info_span;
use tracing::Instrument;

use crate::{isins::types::ShareIsin, utils::get_page_text};

pub mod models;
mod parsers;
mod property_selector;

pub async fn scrape_all_shares(share_isins: Vec<ShareIsin>) -> Vec<Share> {
    let mut tasks = FuturesUnordered::new();

    for share_isin in share_isins {
        let isin_str = &share_isin.isin.get_str();
        tasks.push(
            scrape_share_with_max_duration(share_isin, 5 * 60)
                .instrument(info_span!("scraping_share", isin = isin_str)),
        );
    }

    let mut res: Vec<Share> = Vec::new();

    let mut curr_share = 0;
    let total_shares = tasks.len();
    while let Some(result) = tasks.next().await {
        // match serde_json::to_string_pretty(&result) {
        //     Ok(formatted_share) => {
        //         info!("Scraped Share Information:\n{}", formatted_share);
        //     }
        //     Err(e) => {
        //         eprintln!("Error serializing share: {}", e);
        //         println!("Fallback debug print:\n{:#?}", result);
        //     }
        // }
        curr_share += 1;
        info!("Scraping share {}/{}", curr_share, total_shares);
        res.push(result);
    }
    info!("Scraped a total of {} shares.", res.len());
    res
}

pub async fn scrape_share_with_max_duration(share_isin: ShareIsin, max_duration: u64) -> Share {
    match timeout(Duration::from_secs(max_duration), scrape_share(&share_isin)).await {
        Ok(res) => {
            info!("Finished scraping share");
            res
        }
        Err(_) => {
            error!("Operation timed");
            Share::with_isin(&share_isin)
        }
    }
}

pub async fn scrape_share(share_isin: &ShareIsin) -> Share {
    let isin = &share_isin.isin;
    let url = format!(
        "https://www.borsaitaliana.it/borsa/azioni/dati-completi.html?isin={}&lang=it",
        isin.get_str()
    );

    let res_txt = get_page_text(&url)
        .instrument(info_span!("fetching_page"))
        .await
        .unwrap_or_default();

    parse_page(res_txt, share_isin).unwrap_or_else(|| Share::with_isin(share_isin))
}

fn parse_page(res_txt: String, share_isin: &ShareIsin) -> Option<Share> {
    let doc = Html::parse_document(&res_txt);

    let selector = PropertySelector::new(&doc);

    Some(Share::from_selector(share_isin, &selector))
}
