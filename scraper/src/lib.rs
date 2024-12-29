mod errors;
pub mod exponential_backoff;
pub mod isins;
pub mod metrics;
pub mod shares;

use std::time::Duration;

use chrono::{NaiveTime, Utc};
use errors::{ScraperResult, ScrapingError};
use once_cell::sync::Lazy;
use reqwest::Client;
use tracing::{debug, debug_span, error, Instrument};

use crate::exponential_backoff::{exponential_backoff, BackoffMessage};

static CLIENT: Lazy<Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_max_idle_per_host(100) // Keep more connections alive
        .tcp_nodelay(true)
        .pool_idle_timeout(Duration::from_secs(15))
        .tcp_keepalive(Duration::from_secs(30))
        .build()
        .unwrap()
});

pub async fn get_page_text(url: String) -> ScraperResult<String> {
    let url = url.as_str();
    let page_response = exponential_backoff(|| async {
        match CLIENT.get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.5")
            .send().await {
            Ok(res) => match res.status() {
                reqwest::StatusCode::OK => {
                    debug!("Returning text for url {url}");
                    BackoffMessage::Return(res)
                }
                reqwest::StatusCode::TOO_MANY_REQUESTS
                // the following status codes are sent when too many request are sent to 'www.borsaitaliana.it' 
                | reqwest::StatusCode::BAD_GATEWAY
                | reqwest::StatusCode::SERVICE_UNAVAILABLE
                | reqwest::StatusCode::GATEWAY_TIMEOUT
                | reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                    debug!("Retrying for url {url}");
                    BackoffMessage::Retry
                }
                _ => {
                    error!("Exiting, status code {}", res.status());
                    BackoffMessage::Exit
                }
            },
            Err(e) => {
                error!("Network error fetching page at url {}: {}", url, e);
                BackoffMessage::Exit
            }
        }
    })
    .instrument(debug_span!("exponential_backoff"))
    .await?;

    match page_response.text().await {
        Ok(txt) if !txt.is_empty() => Ok(txt),
        _ => Err(ScrapingError::InvalidPage),
    }
}

pub fn get_elapsed_time(time: NaiveTime) -> i64 {
    (Utc::now().time() - time).num_milliseconds()
}
