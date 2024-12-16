mod errors;
pub mod exponential_backoff;
pub mod isins;
pub mod metrics;
pub mod shares;

use chrono::{NaiveTime, Utc};
use errors::{ScraperResult, ScrapingError};
use tracing::{debug, debug_span, error, warn, Instrument};

use crate::exponential_backoff::{exponential_backoff, BackoffMessage};

pub async fn get_page_text(url: &str) -> ScraperResult<String> {
    let page_response = exponential_backoff(|| async {
        match reqwest::get(url).await {
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

    let res_txt = match page_response.text().await {
        Ok(txt) => txt,
        Err(e) => {
            warn!("Failed to read page text at url {}: {}", url, e);
            return Err(ScrapingError::InvalidPage);
        }
    };

    if res_txt.is_empty() {
        warn!("Text at url {} couldn't be fetched or isn't present", url);
        return Err(ScrapingError::InvalidPage);
    }

    Ok(res_txt)
}

pub fn get_elapsed_time(time: NaiveTime) -> i64 {
    (Utc::now().time() - time).num_milliseconds()
}
