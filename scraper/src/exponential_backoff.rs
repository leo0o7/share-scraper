use std::{future, time::Duration};

use chrono::Utc;
use futures::future::select;
use tokio::time::{sleep, timeout};
use tracing::debug;

use crate::get_elapsed_time;

const MAX_RETRIES: u32 = 7;

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

pub async fn exponential_backoff<T, F, Fut>(action: F) -> Result<T, BackoffError>
where
    F: Fn() -> Fut,
    Fut: future::Future<Output = BackoffMessage<T>>,
{
    let start_time = Utc::now().time();
    let mut try_count = 0;
    let max_total_duration = Duration::from_secs(256);

    match timeout(max_total_duration, async {
        while try_count <= MAX_RETRIES {
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
                    if try_count > MAX_RETRIES {
                        debug!("Reached max retries. Exiting.");
                        break;
                    }

                    let wait_time = Duration::from_secs(2u64.pow(try_count));

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
