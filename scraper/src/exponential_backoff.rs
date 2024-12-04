use std::{
    future,
    time::{Duration, SystemTime},
};

use tokio::time::sleep;
use tracing::debug;

use crate::utils::get_elapsed_time;

const MAX_RETRIES: u32 = 5;

pub enum BackoffMessage<T> {
    Retry,
    Exit,
    Return(T),
}

pub async fn exponential_backoff<T, F, Fut>(action: F) -> Option<T>
where
    F: Fn() -> Fut,
    Fut: future::Future<Output = BackoffMessage<T>>,
{
    let start_time = SystemTime::now();
    let mut try_count = 0;

    while try_count <= MAX_RETRIES {
        match action().await {
            BackoffMessage::Return(res) => {
                debug!(
                    "Successfully completed after {try_count} retries. Time elapsed {}",
                    get_elapsed_time(start_time)
                );
                return Some(res);
            }
            BackoffMessage::Retry => {
                try_count += 1;

                if try_count > MAX_RETRIES {
                    debug!("Reached max retries. Exiting.");
                    break;
                }

                let wait_time = (2u64.pow(try_count.min(10)) * 100).min(30_000);

                debug!(
                    "Retry {}/{} - Waiting for {} ms",
                    try_count, MAX_RETRIES, wait_time
                );
                sleep(Duration::from_millis(wait_time)).await;
            }

            BackoffMessage::Exit => {
                debug!(
                    "Exiting after {try_count} retries. Time elapsed {}",
                    get_elapsed_time(start_time)
                );

                return None;
            }
        }
    }

    debug!(
        "Max retries exhausted. Time elapsed {}",
        get_elapsed_time(start_time)
    );
    None
}
