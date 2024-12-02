use crate::db::utils::insert_all_isins;
use crate::db::{
    connect,
    utils::{insert_all_shares, query_all_isins},
};
use crate::exponential_backoff::{exponential_backoff, BackoffMessage};
use crate::isins::scrape_all_isins;
use crate::shares::scrape_all_shares;
use std::time::SystemTime;

pub async fn scrape_and_insert_all_shares() {
    let pool = match connect().await {
        Ok(pool) => pool,
        Err(e) => panic!("Couldn't connect to DB cause of {e}"),
    };

    let share_isins = query_all_isins(&pool).await.unwrap();

    let shares = scrape_all_shares(share_isins).await;

    insert_all_shares(shares, &pool).await;
}

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
                    _ => {
                        println!("Exiting, status code {}", res.status());
                        BackoffMessage::Exit
                    }
                },
                Err(e) => {
                    eprintln!("Network error fetching page at url {}: {}", url, e);
                    BackoffMessage::Exit
                }
            }
        },
        false,
    )
    .await
    .ok_or("Failed to fetch page at {url}");

    let res_txt = match page_response.ok()?.text().await {
        Ok(txt) => txt,
        Err(e) => {
            eprintln!("Failed to read page text {}", e);
            return None;
        }
    };

    if res_txt.is_empty() {
        eprintln!("Text at url {} couldn't be fetched or isn't present", url);
        return None;
    }

    Some(res_txt)
}
