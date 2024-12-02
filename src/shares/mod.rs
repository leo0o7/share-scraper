use std::time::Duration;

use futures::{stream::FuturesUnordered, StreamExt};
use models::ScrapableStruct;
use models::Share;
use scraper::Html;
use tokio::time::timeout;

use crate::{isins::types::ShareIsin, utils::get_page_text};

pub mod models;
pub mod parsers;
pub mod selectors;

pub async fn scrape_all_shares(share_isins: Vec<ShareIsin>) -> Vec<Share> {
    let mut tasks = FuturesUnordered::new();

    for share_isin in share_isins {
        tasks.push(scrape_share_with_max_duration(share_isin, 240));
    }

    let mut res: Vec<Share> = Vec::new();

    let mut curr_num = 0;
    let share_num = tasks.len();

    while let Some(result) = tasks.next().await {
        // match serde_json::to_string_pretty(&result) {
        //     Ok(formatted_share) => {
        //         println!("Scraped Share Information:\n{}", formatted_share);
        //     }
        //     Err(e) => {
        //         eprintln!("Error serializing share: {}", e);
        //         println!("Fallback debug print:\n{:#?}", result);
        //     }
        // }
        curr_num += 1;
        println!("Finished scraping share {}/{}", curr_num, share_num);
        // print!("\rScraping share {}/{}", curr_num, share_num);
        // let _ = std::io::stdout().flush();
        res.push(result);
    }
    println!("Scraped {} shares.", res.len());
    res
}

pub async fn scrape_share_with_max_duration(share_isin: ShareIsin, max_duration: u64) -> Share {
    match timeout(Duration::from_secs(max_duration), scrape_share(&share_isin)).await {
        Ok(res) => res,
        Err(_) => {
            eprintln!("Operation timed out");
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

    // println!("Scraping share at {url}");

    let res_txt = get_page_text(&url).await.unwrap_or_default();

    parse_page(res_txt, &share_isin).unwrap_or_else(|| Share::with_isin(share_isin))
}

fn parse_page(res_txt: String, share_isin: &ShareIsin) -> Option<Share> {
    let doc = Html::parse_document(&res_txt);

    let share_wrapper_selector =
        scraper::Selector::parse("article.l-grid__cell div.l-box.-pb.-pt.h-bg--white").unwrap();

    doc.select(&share_wrapper_selector)
        .next()
        .map(|table| Share::from_element(share_isin, table))
}
