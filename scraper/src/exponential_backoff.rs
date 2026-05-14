use std::{future, time::Duration};

use chrono::Utc;
use futures::future::select;
use tokio::time::{sleep, timeout};
use tracing::debug;

use app_config::BackoffConfig;

use crate::get_elapsed_time;

pub enum BackoffMessage<T> {
    Retry,
    Exit,
    Return(T),
}

pub enum BackoffError {
    MaxRetries,
    Exit,
    Timeout,
}

pub async fn exponential_backoff<T, F, Fut>(
    config: &BackoffConfig,
    action: F,
) -> Result<T, BackoffError>
where
    F: Fn() -> Fut,
    Fut: future::Future<Output = BackoffMessage<T>>,
{
    let start_time = Utc::now().time();
    let mut try_count = 0;
    let max_total_duration = config.total_timeout;

    match timeout(max_total_duration, async {
        while try_count <= config.retry_count {
            match action().await {
                BackoffMessage::Return(res) => {
                    debug!(
                        "Successfully completed after {try_count} retries. Time elapsed {}",
                        get_elapsed_time(start_time)
                    );
                    return Ok(res);
                }
                BackoffMessage::Retry => {
                    try_count += 1;
                    if try_count > config.retry_count {
                        debug!("Reached max retries. Exiting.");
                        break;
                    }
                    let jitter_max =
                        u64::try_from(config.jitter_max.as_millis()).unwrap_or(u64::MAX);
                    let jitter = rand::random_range(0..jitter_max);
                    let wait_time = config
                        .base_delay
                        .saturating_mul(2u32.saturating_pow(try_count))
                        .saturating_add(Duration::from_millis(jitter));

                    select(
                        Box::pin(sleep(wait_time)),
                        Box::pin(futures::future::pending::<()>()),
                    )
                    .await;
                }
                BackoffMessage::Exit => {
                    debug!(
                        "Exiting after {try_count} retries. Time elapsed {}",
                        get_elapsed_time(start_time)
                    );
                    return Err(BackoffError::Exit);
                }
            }
        }
        Err(BackoffError::MaxRetries)
    })
    .await
    {
        Ok(result) => result,
        Err(_) => Err(BackoffError::Timeout),
    }
}
