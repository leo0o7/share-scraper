use std::fmt::{Display, Formatter};
use std::io::IsTerminal;
use std::sync::Mutex;

use app_config::{load_config, AppConfig, LoggingConfig};
use scraper_utils::{run_scrape_and_insert, run_scrape_and_insert_isins, run_share_refresh};
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScraperOperation {
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
    let logging_mode = decide_logging_mode(options.output_mode, std::io::stdout().is_terminal());

    if logging_mode.fallback_notice {
        println!("progress output requested but stdout is not interactive; falling back to logs");
    }

    init_logging(&config.logging, logging_mode)?;

    run_operation(options.operation, &config).await;

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

fn decide_logging_mode(output_mode: OutputMode, stdout_is_terminal: bool) -> LoggingMode {
    match (output_mode, stdout_is_terminal) {
        (OutputMode::Progress, true) => LoggingMode {
            stdout: false,
            fallback_notice: false,
        },
        (OutputMode::Progress, false) => LoggingMode {
            stdout: true,
            fallback_notice: true,
        },
        (OutputMode::Logs, _) => LoggingMode {
            stdout: true,
            fallback_notice: false,
        },
    }
}

async fn run_operation(operation: ScraperOperation, config: &AppConfig) {
    info!(?operation, "Starting scraper operation");

    match operation {
        ScraperOperation::ScrapeAndInsertShares => {
            let result = run_scrape_and_insert(config).await;
            info!(?result, "Finished scraper operation");
        }
        ScraperOperation::ScrapeAndInsertIsins => {
            let result = run_scrape_and_insert_isins(config).await;
            info!(?result, "Finished scraper operation");
        }
        ScraperOperation::RefreshShares => {
            let result = run_share_refresh(config).await;
            info!(?result, "Finished scraper operation");
        }
    }
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
        decide_logging_mode, parse_cli, CliOptions, LoggingMode, OutputMode, ScraperOperation,
    };

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
    fn logging_mode_decisions_follow_output_contract() {
        assert_eq!(
            decide_logging_mode(OutputMode::Logs, true),
            LoggingMode {
                stdout: true,
                fallback_notice: false,
            }
        );
        assert_eq!(
            decide_logging_mode(OutputMode::Progress, true),
            LoggingMode {
                stdout: false,
                fallback_notice: false,
            }
        );
        assert_eq!(
            decide_logging_mode(OutputMode::Progress, false),
            LoggingMode {
                stdout: true,
                fallback_notice: true,
            }
        );
    }
}
