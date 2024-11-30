use scraper::Html;
use types::Share;

use crate::{
    isins::types::{Isin, ShareIsin},
    utils::get_page_text,
};

pub mod parsers;
pub mod selectors;
pub mod types;

pub async fn scrape_share(share_isin: ShareIsin) -> Share {
    let isin = &share_isin.isin;
    let url = format!(
        "https://www.borsaitaliana.it/borsa/azioni/dati-completi.html?isin={}&lang=it",
        isin.get_str()
    );

    println!("scraping shrae at {url}");
    let res_txt = get_page_text(&url).await.unwrap_or_default();

    parse_page(res_txt, &share_isin).unwrap_or_else(|| {
        let mut share = Share::default();
        share.share_details.isin = isin.get_str().to_owned();
        share
    })
}

fn parse_page(res_txt: String, share_isin: &ShareIsin) -> Option<Share> {
    let doc = Html::parse_document(&res_txt);

    let share_wrapper_selector =
        scraper::Selector::parse("article.l-grid__cell div.l-box.-pb.-pt.h-bg--white").unwrap();

    doc.select(&share_wrapper_selector)
        .next()
        .map(|table| Share::from_element(&share_isin, table))
}
