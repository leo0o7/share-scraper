use futures::{stream::FuturesUnordered, StreamExt};
use scraper::Html;
use types::ShareIsin;

use crate::exponential_backoff::{exponential_backoff, BackoffMessage};

pub mod types;
pub async fn scrape_all_isins() -> Vec<ShareIsin> {
    let mut tasks = FuturesUnordered::new();

    for letter in b'A'..=b'Z' {
        for page in 1..=5 {
            tasks.push(scrape_isins(letter as char, page));
        }
    }

    let mut res: Vec<ShareIsin> = Vec::new();

    while let Some(result) = tasks.next().await {
        if let Some(mut isins) = result {
            res.append(&mut isins);
        }
    }

    res
}

async fn scrape_isins(letter: char, page: u8) -> Option<Vec<ShareIsin>> {
    println!("scraping isins at {letter} page {page}");

    let isin_page_response = fetch_isin_page(letter, page)
        .await
        .ok_or("Failed to fetch page");

    let res_txt = match isin_page_response.ok()?.text().await {
        Ok(txt) => txt,
        Err(e) => {
            println!("Failed to read page {}", e);
            return None;
        }
    };

    if res_txt.is_empty() {
        eprintln!(
            "Page {} for letter '{}' is empty or could not be loaded properly.",
            page, letter
        );
        return None;
    }

    let isins = parse_page(res_txt);

    return Some(isins);
}

fn parse_page(res_txt: String) -> Vec<ShareIsin> {
    let doc = Html::parse_document(&res_txt);
    let isin_element_selector = scraper::Selector::parse("a.u-hidden.-xs").unwrap();

    return doc
        .select(&isin_element_selector)
        .filter_map(ShareIsin::from_element)
        .collect();
}

async fn fetch_isin_page(letter: char, page: u8) -> Option<reqwest::Response> {
    let url = format!("https://www.borsaitaliana.it/borsa/azioni/listino-a-z.html?initial={letter}&page={page}&lang=it");

    exponential_backoff(
        || async {
            match reqwest::get(&url).await {
                Ok(res) => match res.status() {
                    reqwest::StatusCode::OK => BackoffMessage::Return(res),
                    reqwest::StatusCode::TOO_MANY_REQUESTS => BackoffMessage::Retry,
                    _ => BackoffMessage::Exit,
                },
                Err(e) => {
                    eprintln!(
                        "Failed to fetch page {} for letter '{}': {}",
                        page, letter, e
                    );
                    BackoffMessage::Exit
                }
            }
        },
        false,
    )
    .await
}
