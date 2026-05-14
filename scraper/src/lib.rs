mod errors;
pub mod exponential_backoff;
pub mod isins;
pub mod metrics;
pub mod shares;

use app_config::ScraperConfig;
use chrono::{NaiveTime, Utc};
use errors::{ScraperResult, ScrapingError};
use reqwest::Client;
use tracing::{debug, debug_span, error, Instrument};

use crate::exponential_backoff::{exponential_backoff, BackoffMessage};

const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64; rv:123.0) Gecko/20100101 Firefox/123.0",
];

#[derive(Clone)]
pub struct ScraperRuntime {
    client: Client,
}

impl ScraperRuntime {
    pub fn new(config: &ScraperConfig) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(config.http_pool_max_idle_per_host)
            .tcp_nodelay(true)
            .pool_idle_timeout(config.http_idle_timeout)
            .tcp_keepalive(config.http_keepalive)
            .timeout(config.http_request_timeout)
            .connect_timeout(config.http_connect_timeout)
            .build()?;

        Ok(Self { client })
    }

    pub async fn get_page_text(&self, url: String) -> ScraperResult<String> {
        let url = url.as_str();
        let page_response = exponential_backoff(|| async {
            let ua = USER_AGENTS[rand::random_range(0..USER_AGENTS.len())];
            match self
                .client
                .get(url)
                .header("User-Agent", ua)
                .header(
                    "Accept",
                    "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
                )
                .header("Accept-Language", "en-US,en;q=0.5")
                .header(
                    "Referer",
                    "https://www.borsaitaliana.it/borsa/azioni/listino-a-z.html",
                )
                .header("Connection", "keep-alive")
                .header("Cache-Control", "no-cache")
                .send()
                .await
            {
                Ok(res) => match res.status() {
                    reqwest::StatusCode::OK => {
                        debug!("Returning text for url {url}");
                        BackoffMessage::Return(res)
                    }
                    reqwest::StatusCode::TOO_MANY_REQUESTS
                    // The target site uses these statuses when request pressure is too high.
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
}

pub fn get_elapsed_time(time: NaiveTime) -> i64 {
    (Utc::now().time() - time).num_milliseconds()
}
