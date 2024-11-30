use crate::db::connect;
use crate::db::utils::insert_all_isins;
use crate::exponential_backoff::{exponential_backoff, BackoffMessage};
use crate::isins::scrape_all_isins;
use std::time::SystemTime;

pub async fn scrape_and_insert_all_isins() {
    let isins = scrape_all_isins().await;

    if let Ok(pool) = connect().await {
        insert_all_isins(isins, &pool).await;
    }
}

pub fn get_elapsed_time(time: SystemTime) -> u64 {
    match time.elapsed() {
        Ok(duration) => duration.as_secs(),
        // pretty bad if we can't do that
        Err(_) => panic!("Error calculating time elapsed"),
    }
}

pub async fn get_page_text(url: &str) -> Option<String> {
    let page_response = exponential_backoff(
        || async {
            match reqwest::get(url).await {
                Ok(res) => match res.status() {
                    reqwest::StatusCode::OK => BackoffMessage::Return(res),
                    reqwest::StatusCode::TOO_MANY_REQUESTS => BackoffMessage::Retry,
                    _ => BackoffMessage::Exit,
                },
                Err(e) => {
                    eprintln!("Failed to fetch page at url {} cause {}", url, e);
                    BackoffMessage::Exit
                }
            }
        },
        false,
    )
    .await
    .ok_or("Failed to isin fetch page");

    let res_txt = match page_response.ok()?.text().await {
        Ok(txt) => txt,
        Err(e) => {
            println!("Failed to read page {}", e);
            return None;
        }
    };

    if res_txt.is_empty() {
        eprintln!("Text at url {} couldn't be fetched", url);
        return None;
    }

    Some(res_txt)
}
