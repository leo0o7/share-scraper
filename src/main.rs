use std::future::Future;
use std::io::IsTerminal;

use app_config::{load_config, AppConfig};
use scraper_utils::{
    progress::ProgressSender, run_scrape_and_insert, run_scrape_and_insert_isins,
    run_share_refresh, ScrapeAndInsertInfo,
};
use tokio::sync::mpsc;
use tracing::info;

mod cli;
mod logging;
mod operation;
mod progress_state;
mod progress_ui;

use crate::cli::{decide_runtime_output, parse_cli};
use crate::logging::init_logging;
use crate::operation::ScraperOperation;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config(env!("CARGO_MANIFEST_DIR"))?;
    let options = parse_cli(std::env::args().skip(1))?;
    let runtime_output = decide_runtime_output(
        options.output_mode,
        std::io::stdout().is_terminal(),
        progress_renderer_can_initialize(),
    );

    if runtime_output.logging.fallback_notice {
        println!("progress output unavailable; falling back to logs");
    }

    init_logging(&config.logging, runtime_output.logging)?;

    let operation_succeeded =
        run_operation(options.operation, &config, runtime_output.render_progress).await;

    if !operation_succeeded {
        std::process::exit(1);
    }

    Ok(())
}

fn progress_renderer_can_initialize() -> bool {
    true
}

async fn run_operation(
    operation: ScraperOperation,
    config: &AppConfig,
    render_progress: bool,
) -> bool {
    info!(
        operation = operation.metadata().cli_name,
        "Starting scraper operation"
    );

    let result = run_operation_observing(operation, render_progress, |progress| async move {
        match operation {
            ScraperOperation::ScrapeAndInsertShares => {
                run_scrape_and_insert(config, progress).await
            }
            ScraperOperation::ScrapeAndInsertIsins => {
                run_scrape_and_insert_isins(config, progress).await
            }
            ScraperOperation::RefreshShares => run_share_refresh(config, progress).await,
        }
    })
    .await;

    info!(?result, "Finished scraper operation");
    operation_succeeded(&result.metrics)
}

async fn run_operation_observing<F, Fut>(
    operation: ScraperOperation,
    render_progress: bool,
    run: F,
) -> ScrapeAndInsertInfo
where
    F: FnOnce(Option<ProgressSender>) -> Fut,
    Fut: Future<Output = ScrapeAndInsertInfo>,
{
    if render_progress {
        let (sender, receiver) = mpsc::unbounded_channel();
        let renderer = tokio::spawn(progress_ui::render(operation, receiver));
        let result = run(Some(ProgressSender::new(sender))).await;
        let _ = renderer.await;
        result
    } else {
        run(None).await
    }
}

fn operation_succeeded(metrics: &scraper_utils::ScrapeAndInsertMetrics) -> bool {
    metrics.scrape.total == metrics.scrape.successful && metrics.insert.failed == 0
}

#[cfg(test)]
mod tests {
    use super::operation_succeeded;
    use db::metrics::InsertionMetrics;
    use scraper::metrics::{ScrapingErrorMetrics, ScrapingMetrics};
    use scraper_utils::ScrapeAndInsertMetrics;

    #[test]
    fn operation_status_succeeds_only_when_scrape_and_insert_are_fully_successful() {
        assert!(operation_succeeded(&ScrapeAndInsertMetrics {
            scrape: ScrapingMetrics {
                total: 2,
                successful: 2,
                errors: ScrapingErrorMetrics::empty(),
            },
            insert: InsertionMetrics {
                total: 2,
                successful: 2,
                failed: 0,
            },
        }));
        assert!(!operation_succeeded(&ScrapeAndInsertMetrics {
            scrape: ScrapingMetrics {
                total: 2,
                successful: 1,
                errors: ScrapingErrorMetrics {
                    network_error: 1,
                    invalid_page: 0,
                    timeout: 0,
                    max_retries: 0,
                    parsing_error: 0,
                },
            },
            insert: InsertionMetrics {
                total: 1,
                successful: 1,
                failed: 0,
            },
        }));
        assert!(!operation_succeeded(&ScrapeAndInsertMetrics {
            scrape: ScrapingMetrics {
                total: 2,
                successful: 2,
                errors: ScrapingErrorMetrics::empty(),
            },
            insert: InsertionMetrics {
                total: 2,
                successful: 1,
                failed: 1,
            },
        }));
    }
}
