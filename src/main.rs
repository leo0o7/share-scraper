use std::fmt::{Display, Formatter};
use std::io::IsTerminal;
use std::sync::Mutex;

use app_config::{load_config, AppConfig, LoggingConfig};
use scraper_utils::{
    progress::ProgressSender, run_scrape_and_insert, run_scrape_and_insert_isins,
    run_scrape_and_insert_with_progress, run_share_refresh, run_share_refresh_with_progress,
};
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod progress_ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScraperOperation {
    ScrapeAndInsertShares,
    ScrapeAndInsertIsins,
    RefreshShares,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Progress,
    Logs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CliOptions {
    operation: ScraperOperation,
    output_mode: OutputMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LoggingMode {
    stdout: bool,
    fallback_notice: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeOutputMode {
    logging: LoggingMode,
    render_progress: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliError(String);

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CliError {}

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

fn init_logging(
    config: &LoggingConfig,
    logging_mode: LoggingMode,
) -> Result<(), Box<dyn std::error::Error>> {
    let log_file = std::fs::File::create(&config.scraper_file_path)?;

    let file_logger = fmt::layer()
        .with_writer(Mutex::new(log_file))
        .with_ansi(false);
    let env_filter = EnvFilter::try_new(&config.level)?;

    if logging_mode.stdout {
        let stdout_logger = fmt::layer().with_ansi(true);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_logger)
            .with(stdout_logger)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_logger)
            .init();
    }

    Ok(())
}

fn progress_renderer_can_initialize() -> bool {
    true
}

fn decide_runtime_output(
    output_mode: OutputMode,
    stdout_is_terminal: bool,
    progress_renderer_available: bool,
) -> RuntimeOutputMode {
    match (output_mode, stdout_is_terminal, progress_renderer_available) {
        (OutputMode::Progress, true, true) => RuntimeOutputMode {
            logging: LoggingMode {
                stdout: false,
                fallback_notice: false,
            },
            render_progress: true,
        },
        (OutputMode::Progress, _, _) => RuntimeOutputMode {
            logging: LoggingMode {
                stdout: true,
                fallback_notice: true,
            },
            render_progress: false,
        },
        (OutputMode::Logs, _, _) => RuntimeOutputMode {
            logging: LoggingMode {
                stdout: true,
                fallback_notice: false,
            },
            render_progress: false,
        },
    }
}

async fn run_operation(
    operation: ScraperOperation,
    config: &AppConfig,
    render_progress: bool,
) -> bool {
    info!(?operation, "Starting scraper operation");

    match operation {
        ScraperOperation::ScrapeAndInsertShares => {
            let result = if render_progress {
                let (sender, receiver) = mpsc::unbounded_channel();
                let renderer = tokio::spawn(progress_ui::render(
                    ScraperOperation::ScrapeAndInsertShares,
                    receiver,
                ));
                let result =
                    run_scrape_and_insert_with_progress(config, Some(ProgressSender::new(sender)))
                        .await;
                let _ = renderer.await;
                result
            } else {
                run_scrape_and_insert(config).await
            };
            info!(?result, "Finished scraper operation");
            operation_succeeded(&result.metrics)
        }
        ScraperOperation::ScrapeAndInsertIsins => {
            let result = if render_progress {
                let (sender, receiver) = mpsc::unbounded_channel();
                let renderer = tokio::spawn(progress_ui::render(
                    ScraperOperation::ScrapeAndInsertIsins,
                    receiver,
                ));
                let result = scraper_utils::run_scrape_and_insert_isins_with_progress(
                    config,
                    Some(ProgressSender::new(sender)),
                )
                .await;
                let _ = renderer.await;
                result
            } else {
                run_scrape_and_insert_isins(config).await
            };
            info!(?result, "Finished scraper operation");
            operation_succeeded(&result.metrics)
        }
        ScraperOperation::RefreshShares => {
            let result = if render_progress {
                let (sender, receiver) = mpsc::unbounded_channel();
                let renderer = tokio::spawn(progress_ui::render(
                    ScraperOperation::RefreshShares,
                    receiver,
                ));
                let result =
                    run_share_refresh_with_progress(config, Some(ProgressSender::new(sender)))
                        .await;
                let _ = renderer.await;
                result
            } else {
                run_share_refresh(config).await
            };
            info!(?result, "Finished scraper operation");
            operation_succeeded(&result.metrics)
        }
    }
}

fn operation_succeeded(metrics: &scraper_utils::ScrapeAndInsertMetrics) -> bool {
    metrics.scrape.total == metrics.scrape.successful && metrics.insert.failed == 0
}

fn parse_cli(args: impl IntoIterator<Item = String>) -> Result<CliOptions, CliError> {
    let mut output_mode = OutputMode::Progress;
    let mut operation = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => {
                let Some(value) = args.next() else {
                    return Err(CliError(format!(
                        "missing value for --output\n\n{}",
                        operation_help()
                    )));
                };
                output_mode = parse_output_mode(&value)?;
            }
            "-h" | "--help" => return Err(CliError(operation_help().to_string())),
            "scrape-shares" | "scrape-isins" | "refresh-shares" => {
                if operation.replace(parse_operation_name(&arg)).is_some() {
                    return Err(CliError(format!(
                        "too many arguments\n\n{}",
                        operation_help()
                    )));
                }
            }
            other if other.starts_with('-') => {
                return Err(CliError(format!(
                    "unsupported option: {other}\n\n{}",
                    operation_help()
                )));
            }
            other => {
                if operation.is_some() {
                    return Err(CliError(format!(
                        "too many arguments\n\n{}",
                        operation_help()
                    )));
                }
                return Err(CliError(format!(
                    "unsupported scraper operation: {other}\n\n{}",
                    operation_help()
                )));
            }
        }
    }

    Ok(CliOptions {
        operation: operation.unwrap_or(ScraperOperation::ScrapeAndInsertShares),
        output_mode,
    })
}

fn parse_operation_name(operation: &str) -> ScraperOperation {
    match operation {
        "scrape-shares" => ScraperOperation::ScrapeAndInsertShares,
        "scrape-isins" => ScraperOperation::ScrapeAndInsertIsins,
        "refresh-shares" => ScraperOperation::RefreshShares,
        _ => unreachable!("operation names are filtered by parse_cli"),
    }
}

fn parse_output_mode(value: &str) -> Result<OutputMode, CliError> {
    match value {
        "progress" => Ok(OutputMode::Progress),
        "logs" => Ok(OutputMode::Logs),
        other => Err(CliError(format!(
            "unsupported output mode: {other}\n\n{}",
            operation_help()
        ))),
    }
}

fn operation_help() -> &'static str {
    "Usage: share_service [--output progress|logs] [operation]\n\nOperations:\n  scrape-shares    Scrape and insert all shares (default)\n  scrape-isins     Scrape and insert all ISINs\n  refresh-shares   Refresh shares older than the configured threshold\n\nOptions:\n  --output progress|logs    Choose terminal progress UI or stdout tracing logs"
}

#[cfg(test)]
mod tests {
    use super::{
        decide_runtime_output, operation_succeeded, parse_cli, CliOptions, LoggingMode, OutputMode,
        RuntimeOutputMode, ScraperOperation,
    };
    use db::metrics::InsertionMetrics;
    use db::{isins::IsinInsertCompletion, shares::ShareInsertCompletion};
    use scraper::metrics::{ScrapingErrorMetrics, ScrapingMetrics};
    use scraper::{
        errors::ScrapingError, isins::IsinScrapeCompletion, shares::ShareScrapeCompletion,
    };
    use scraper_utils::{progress::ProgressPhase, ScrapeAndInsertMetrics};
    use tokio::sync::mpsc;

    #[test]
    fn missing_operation_uses_default_share_scrape() {
        let options = parse_cli([]).unwrap();

        assert_eq!(
            options,
            CliOptions {
                operation: ScraperOperation::ScrapeAndInsertShares,
                output_mode: OutputMode::Progress,
            }
        );
    }

    #[test]
    fn parses_supported_operations() {
        assert_eq!(
            parse_cli(["scrape-shares".to_string()]).unwrap().operation,
            ScraperOperation::ScrapeAndInsertShares
        );
        assert_eq!(
            parse_cli(["scrape-isins".to_string()]).unwrap().operation,
            ScraperOperation::ScrapeAndInsertIsins
        );
        assert_eq!(
            parse_cli(["refresh-shares".to_string()]).unwrap().operation,
            ScraperOperation::RefreshShares
        );
    }

    #[test]
    fn output_mode_can_appear_before_or_after_operation() {
        assert_eq!(
            parse_cli([
                "--output".to_string(),
                "logs".to_string(),
                "scrape-isins".to_string()
            ])
            .unwrap(),
            CliOptions {
                operation: ScraperOperation::ScrapeAndInsertIsins,
                output_mode: OutputMode::Logs,
            }
        );
        assert_eq!(
            parse_cli([
                "refresh-shares".to_string(),
                "--output".to_string(),
                "progress".to_string()
            ])
            .unwrap(),
            CliOptions {
                operation: ScraperOperation::RefreshShares,
                output_mode: OutputMode::Progress,
            }
        );
    }

    #[test]
    fn invalid_operation_returns_help_text() {
        let err = parse_cli(["unknown".to_string()]).unwrap_err();

        assert!(err.0.contains("unsupported scraper operation: unknown"));
        assert!(err.0.contains("Usage: share_service"));
        assert!(err.0.contains("scrape-shares"));
    }

    #[test]
    fn invalid_output_mode_returns_help_text() {
        let err = parse_cli(["--output".to_string(), "json".to_string()]).unwrap_err();

        assert!(err.0.contains("unsupported output mode: json"));
        assert!(err.0.contains("--output progress|logs"));
    }

    #[test]
    fn too_many_positional_arguments_return_help_text() {
        let err =
            parse_cli(["scrape-shares".to_string(), "refresh-shares".to_string()]).unwrap_err();

        assert!(err.0.contains("too many arguments"));
        assert!(err.0.contains("Usage: share_service"));
    }

    #[test]
    fn runtime_output_decisions_follow_output_contract() {
        assert_eq!(
            decide_runtime_output(OutputMode::Logs, true, true),
            RuntimeOutputMode {
                logging: LoggingMode {
                    stdout: true,
                    fallback_notice: false,
                },
                render_progress: false,
            }
        );
        assert_eq!(
            decide_runtime_output(OutputMode::Progress, true, true),
            RuntimeOutputMode {
                logging: LoggingMode {
                    stdout: false,
                    fallback_notice: false,
                },
                render_progress: true,
            }
        );
        assert_eq!(
            decide_runtime_output(OutputMode::Progress, false, true),
            RuntimeOutputMode {
                logging: LoggingMode {
                    stdout: true,
                    fallback_notice: true,
                },
                render_progress: false,
            }
        );
        assert_eq!(
            decide_runtime_output(OutputMode::Progress, true, false),
            RuntimeOutputMode {
                logging: LoggingMode {
                    stdout: true,
                    fallback_notice: true,
                },
                render_progress: false,
            }
        );
    }

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

    #[tokio::test]
    async fn progress_sender_tolerates_closed_channels_for_lifecycle_and_updates() {
        let (sender, receiver) = mpsc::unbounded_channel();
        drop(receiver);
        let progress = super::ProgressSender::new(sender);

        progress
            .phase_started(ProgressPhase::ScrapeShares, Some(1))
            .await;
        progress.phase_finished(ProgressPhase::ScrapeShares).await;
        progress.share_scraped(ShareScrapeCompletion {
            isin: "IT0000000001".to_string(),
            result: Err(ScrapingError::Timeout),
        });
        progress.share_inserted(ShareInsertCompletion {
            isin: "IT0000000001".to_string(),
            successful: false,
        });
        progress.isin_page_scraped(IsinScrapeCompletion {
            letter: 'A',
            page: 1,
            isins_found: 0,
            result: Ok(()),
            parsing_errors: 0,
        });
        progress.isin_letter_completed('A');
        progress.isin_inserted(IsinInsertCompletion {
            isin: "IT0000000001".to_string(),
            successful: true,
        });
    }
}
