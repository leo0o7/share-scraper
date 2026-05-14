use std::fmt::{Display, Formatter};
use std::sync::Mutex;

use app_config::{load_config, DatabaseConfig, LoggingConfig};
use scraper_utils::{run_scrape_and_insert, run_scrape_and_insert_isins, run_share_refresh};
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScraperOperation {
    ScrapeAndInsertShares,
    ScrapeAndInsertIsins,
    RefreshShares,
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
    init_logging(&config.logging)?;

    let operation = parse_operation(std::env::args().skip(1))?;
    run_operation(operation, &config.database).await;

    Ok(())
}

fn init_logging(config: &LoggingConfig) -> Result<(), Box<dyn std::error::Error>> {
    let log_file = std::fs::File::create(&config.scraper_file_path)?;

    let file_logger = fmt::layer()
        .with_writer(Mutex::new(log_file))
        .with_ansi(false);
    let env_filter = EnvFilter::try_new(&config.level)?;

    if config.stdout {
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

async fn run_operation(operation: ScraperOperation, database_config: &DatabaseConfig) {
    info!(?operation, "Starting scraper operation");

    match operation {
        ScraperOperation::ScrapeAndInsertShares => {
            dbg!(run_scrape_and_insert(database_config).await);
        }
        ScraperOperation::ScrapeAndInsertIsins => {
            dbg!(run_scrape_and_insert_isins(database_config).await);
        }
        ScraperOperation::RefreshShares => {
            dbg!(run_share_refresh(database_config).await);
        }
    }
}

fn parse_operation(args: impl IntoIterator<Item = String>) -> Result<ScraperOperation, CliError> {
    let mut args = args.into_iter();
    let Some(operation) = args.next() else {
        return Ok(ScraperOperation::ScrapeAndInsertShares);
    };

    if args.next().is_some() {
        return Err(CliError(format!(
            "too many arguments\n\n{}",
            operation_help()
        )));
    }

    match operation.as_str() {
        "scrape-shares" => Ok(ScraperOperation::ScrapeAndInsertShares),
        "scrape-isins" => Ok(ScraperOperation::ScrapeAndInsertIsins),
        "refresh-shares" => Ok(ScraperOperation::RefreshShares),
        "-h" | "--help" => Err(CliError(operation_help().to_string())),
        other => Err(CliError(format!(
            "unsupported scraper operation: {other}\n\n{}",
            operation_help()
        ))),
    }
}

fn operation_help() -> &'static str {
    "Usage: share_service [operation]\n\nOperations:\n  scrape-shares    Scrape and insert all shares (default)\n  scrape-isins     Scrape and insert all ISINs\n  refresh-shares   Refresh shares older than the configured threshold"
}

#[cfg(test)]
mod tests {
    use super::{parse_operation, ScraperOperation};

    #[test]
    fn missing_operation_uses_default_share_scrape() {
        let operation = parse_operation([]).unwrap();

        assert_eq!(operation, ScraperOperation::ScrapeAndInsertShares);
    }

    #[test]
    fn parses_supported_operations() {
        assert_eq!(
            parse_operation(["scrape-shares".to_string()]).unwrap(),
            ScraperOperation::ScrapeAndInsertShares
        );
        assert_eq!(
            parse_operation(["scrape-isins".to_string()]).unwrap(),
            ScraperOperation::ScrapeAndInsertIsins
        );
        assert_eq!(
            parse_operation(["refresh-shares".to_string()]).unwrap(),
            ScraperOperation::RefreshShares
        );
    }

    #[test]
    fn invalid_operation_returns_help_text() {
        let err = parse_operation(["unknown".to_string()]).unwrap_err();

        assert!(err.0.contains("unsupported scraper operation: unknown"));
        assert!(err.0.contains("Usage: share_service [operation]"));
        assert!(err.0.contains("scrape-shares"));
    }
}
