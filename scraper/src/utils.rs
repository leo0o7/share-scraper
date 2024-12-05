use std::time::SystemTime;

use tracing::{debug, debug_span, error, warn, Instrument};

use crate::exponential_backoff::{exponential_backoff, BackoffMessage};

pub async fn get_page_text(url: &str) -> Option<String> {
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
            return None;
        }
    };

    if res_txt.is_empty() {
        warn!("Text at url {} couldn't be fetched or isn't present", url);
        return None;
    }

    Some(res_txt)
}

pub fn get_elapsed_time(time: SystemTime) -> u128 {
    match time.elapsed() {
        Ok(duration) => duration.as_nanos(),
        // pretty bad if we can't do that
        Err(_) => panic!("Error calculating time elapsed"),
    }
}
