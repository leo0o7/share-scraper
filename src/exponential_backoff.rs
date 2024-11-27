use crate::utils::get_elapsed_time;
use std::{
    future,
    time::{Duration, SystemTime},
};

use tokio::time::sleep;

const MAX_RETRIES: u32 = 5;

pub enum BackoffMessage<T> {
    Retry,
    Exit,
    Return(T),
}

pub async fn exponential_backoff<T, F, Fut>(action: F, debug: bool) -> Option<T>
where
    F: Fn() -> Fut,
    Fut: future::Future<Output = BackoffMessage<T>>,
{
    let start_time = SystemTime::now();
    let mut try_count = 0;

    while try_count <= MAX_RETRIES {
        match action().await {
            BackoffMessage::Return(res) => {
                if debug {
                    println!("Successfully completed after {try_count} retries.");
                    println!("Time elapsed {}", get_elapsed_time(start_time));
                }
                return Some(res);
            }
            BackoffMessage::Retry => {
                try_count += 1;
                if try_count > MAX_RETRIES {
                    if debug {
                        println!("Reached max retries. Exiting.");
                    }
                    break;
                }

                let wait_time = 2u64.pow(try_count) * 100;
                if debug {
                    println!(
                        "Retry {try_count}/{MAX_RETRIES}. Waiting for {} ms before retrying...",
                        wait_time
                    );
                }
                sleep(Duration::from_millis(wait_time)).await;
            }
            BackoffMessage::Exit => {
                println!("Exiting after {try_count} retries.");
                println!("Time elapsed {}", get_elapsed_time(start_time));
                return None;
            }
        }
    }

    if debug {
        println!("Max retries exhausted.");
        println!("Time elapsed {}", get_elapsed_time(start_time));
    }
    None
}
