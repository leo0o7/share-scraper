use std::sync::Mutex;

use app_config::LoggingConfig;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::cli::LoggingMode;

pub(crate) fn init_logging(
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
