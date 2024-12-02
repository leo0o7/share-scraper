use futures::{stream::FuturesUnordered, StreamExt};
use scraper::Html;
use types::ShareIsin;

use crate::utils::get_page_text;

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
    let url = format!("https://www.borsaitaliana.it/borsa/azioni/listino-a-z.html?initial={letter}&page={page}&lang=it");

    println!("scraping isins at {letter} page {page}");

    let res_txt = get_page_text(&url).await.unwrap();

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
