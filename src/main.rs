use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::IsTerminal;
use std::sync::Mutex;
use std::time::Duration;

use app_config::{load_config, AppConfig, LoggingConfig};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use scraper_utils::{
    progress::{ProgressEvent, ProgressPhase, ProgressSender, ScrapeErrorCategory},
    run_scrape_and_insert, run_scrape_and_insert_isins, run_scrape_and_insert_with_progress,
    run_share_refresh,
};
use tokio::sync::mpsc;
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

    let operation_succeeded = run_operation(
        options.operation,
        &config,
        options.output_mode == OutputMode::Progress && !logging_mode.stdout,
    )
    .await;

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

async fn run_operation(
    operation: ScraperOperation,
    config: &AppConfig,
    render_progress: bool,
) -> bool {
    info!(?operation, "Starting scraper operation");

    match operation {
        ScraperOperation::ScrapeAndInsertShares => {
            let result = if render_progress {
                let (sender, receiver) = mpsc::channel(256);
                let renderer = tokio::spawn(render_progress_events(receiver));
                let result =
                    run_scrape_and_insert_with_progress(config, Some(ProgressSender::new(sender)))
                        .await;
                let _ = renderer.await;
                result
            } else {
                run_scrape_and_insert(config).await
            };
            info!(?result, "Finished scraper operation");
            insertion_succeeded(&result.metrics.insert)
        }
        ScraperOperation::ScrapeAndInsertIsins => {
            let result = run_scrape_and_insert_isins(config).await;
            info!(?result, "Finished scraper operation");
            insertion_succeeded(&result.metrics.insert)
        }
        ScraperOperation::RefreshShares => {
            let result = run_share_refresh(config).await;
            info!(?result, "Finished scraper operation");
            insertion_succeeded(&result.metrics.insert)
        }
    }
}

fn insertion_succeeded(metrics: &db::metrics::InsertionMetrics) -> bool {
    metrics.failed == 0
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PhaseProgress {
    total: Option<u64>,
    completed: u64,
    successful: u64,
    errors: u64,
    network_errors: u64,
    invalid_pages: u64,
    timeouts: u64,
    max_retries: u64,
    parsing_errors: u64,
    last: Option<String>,
    finished: bool,
}

#[derive(Debug, Default)]
struct ProgressState {
    phases: HashMap<ProgressPhase, PhaseProgress>,
}

impl ProgressState {
    fn apply(&mut self, event: &ProgressEvent) {
        match event {
            ProgressEvent::PhaseStarted { phase, total } => {
                self.phases.insert(
                    *phase,
                    PhaseProgress {
                        total: *total,
                        ..PhaseProgress::default()
                    },
                );
            }
            ProgressEvent::PhaseFinished { phase } => {
                self.phases.entry(*phase).or_default().finished = true;
            }
            ProgressEvent::ShareScraped { isin, result } => {
                let phase = self.phases.entry(ProgressPhase::ScrapeShares).or_default();
                phase.completed += 1;
                phase.last = Some(isin.clone());
                match result {
                    Ok(()) => phase.successful += 1,
                    Err(category) => {
                        phase.errors += 1;
                        match category {
                            ScrapeErrorCategory::NetworkError => phase.network_errors += 1,
                            ScrapeErrorCategory::InvalidPage => phase.invalid_pages += 1,
                            ScrapeErrorCategory::Timeout => phase.timeouts += 1,
                            ScrapeErrorCategory::MaxRetries => phase.max_retries += 1,
                            ScrapeErrorCategory::ParsingError => phase.parsing_errors += 1,
                        }
                    }
                }
            }
            ProgressEvent::ShareInserted { isin, successful } => {
                let phase = self.phases.entry(ProgressPhase::InsertShares).or_default();
                phase.completed += 1;
                phase.last = Some(isin.clone());
                if *successful {
                    phase.successful += 1;
                } else {
                    phase.errors += 1;
                }
            }
        }
    }

    fn phase(&self, phase: ProgressPhase) -> Option<&PhaseProgress> {
        self.phases.get(&phase)
    }
}

struct ProgressRenderer {
    multi: MultiProgress,
    bars: HashMap<ProgressPhase, ProgressBar>,
}

impl ProgressRenderer {
    fn new() -> Self {
        Self {
            multi: MultiProgress::new(),
            bars: HashMap::new(),
        }
    }

    fn apply(&mut self, state: &ProgressState, event: &ProgressEvent) {
        match event {
            ProgressEvent::PhaseStarted { phase, total } => self.start_phase(*phase, *total),
            ProgressEvent::PhaseFinished { phase } => self.finish_phase(state, *phase),
            ProgressEvent::ShareScraped { .. } => {
                self.update_phase(state, ProgressPhase::ScrapeShares)
            }
            ProgressEvent::ShareInserted { .. } => {
                self.update_phase(state, ProgressPhase::InsertShares)
            }
        }
    }

    fn start_phase(&mut self, phase: ProgressPhase, total: Option<u64>) {
        let bar = match total {
            Some(total) => {
                let bar = self.multi.add(ProgressBar::new(total));
                bar.set_style(progress_bar_style());
                bar
            }
            None => {
                let bar = self.multi.add(ProgressBar::new_spinner());
                bar.set_style(spinner_style());
                bar.enable_steady_tick(Duration::from_millis(100));
                bar
            }
        };
        bar.set_message(phase_label(phase).to_string());
        self.bars.insert(phase, bar);
    }

    fn update_phase(&mut self, state: &ProgressState, phase: ProgressPhase) {
        let Some(bar) = self.bars.get(&phase) else {
            return;
        };
        let Some(phase_state) = state.phase(phase) else {
            return;
        };
        bar.set_position(phase_state.completed);
        bar.set_message(phase_message(phase, phase_state));
    }

    fn finish_phase(&mut self, state: &ProgressState, phase: ProgressPhase) {
        self.update_phase(state, phase);
        if let Some(bar) = self.bars.get(&phase) {
            bar.finish_with_message(finished_phase_message(phase, state.phase(phase)));
        }
    }
}

async fn render_progress_events(mut receiver: mpsc::Receiver<ProgressEvent>) {
    let mut state = ProgressState::default();
    let mut renderer = ProgressRenderer::new();

    while let Some(event) = receiver.recv().await {
        state.apply(&event);
        renderer.apply(&state, &event);
    }
}

fn progress_bar_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{spinner:.green} {msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len}",
    )
    .unwrap()
    .progress_chars("=> ")
}

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.green} {msg} [{elapsed_precise}]").unwrap()
}

fn phase_label(phase: ProgressPhase) -> &'static str {
    match phase {
        ProgressPhase::LoadShareIsins => "loading share ISINs",
        ProgressPhase::ScrapeShares => "scraping shares",
        ProgressPhase::InsertShares => "inserting shares",
    }
}

fn phase_message(phase: ProgressPhase, state: &PhaseProgress) -> String {
    match phase {
        ProgressPhase::LoadShareIsins => phase_label(phase).to_string(),
        ProgressPhase::ScrapeShares => format!(
            "scraping shares ok={} errors={} [{}] last={}",
            state.successful,
            state.errors,
            scrape_error_summary(state),
            state.last.as_deref().unwrap_or("-")
        ),
        ProgressPhase::InsertShares => format!(
            "inserting shares ok={} errors={} last={}",
            state.successful,
            state.errors,
            state.last.as_deref().unwrap_or("-")
        ),
    }
}

fn finished_phase_message(phase: ProgressPhase, state: Option<&PhaseProgress>) -> String {
    match (phase, state) {
        (ProgressPhase::LoadShareIsins, _) => "loaded share ISINs".to_string(),
        (ProgressPhase::ScrapeShares, Some(state)) => format!(
            "scraped shares completed={} ok={} errors={} [{}] last={}",
            state.completed,
            state.successful,
            state.errors,
            scrape_error_summary(state),
            state.last.as_deref().unwrap_or("-")
        ),
        (ProgressPhase::ScrapeShares, None) => "scraped shares".to_string(),
        (ProgressPhase::InsertShares, Some(state)) => format!(
            "inserted shares completed={} ok={} errors={} last={}",
            state.completed,
            state.successful,
            state.errors,
            state.last.as_deref().unwrap_or("-")
        ),
        (ProgressPhase::InsertShares, None) => "inserted shares".to_string(),
    }
}

fn scrape_error_summary(state: &PhaseProgress) -> String {
    format!(
        "network={} invalid={} timeout={} max_retries={} parsing={}",
        state.network_errors,
        state.invalid_pages,
        state.timeouts,
        state.max_retries,
        state.parsing_errors
    )
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
        decide_logging_mode, parse_cli, CliOptions, LoggingMode, OutputMode, PhaseProgress,
        ProgressEvent, ProgressPhase, ProgressState, ScrapeErrorCategory, ScraperOperation,
    };
    use db::metrics::InsertionMetrics;

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

    #[test]
    fn share_scrape_progress_counts_successes_failures_and_last_completed_share() {
        let mut state = ProgressState::default();

        state.apply(&ProgressEvent::PhaseStarted {
            phase: ProgressPhase::ScrapeShares,
            total: Some(3),
        });
        state.apply(&ProgressEvent::ShareScraped {
            isin: "IT0000000001".to_string(),
            result: Ok(()),
        });
        state.apply(&ProgressEvent::ShareScraped {
            isin: "IT0000000002".to_string(),
            result: Err(ScrapeErrorCategory::Timeout),
        });
        state.apply(&ProgressEvent::ShareScraped {
            isin: "IT0000000003".to_string(),
            result: Err(ScrapeErrorCategory::ParsingError),
        });
        state.apply(&ProgressEvent::PhaseFinished {
            phase: ProgressPhase::ScrapeShares,
        });

        assert_eq!(
            state.phase(ProgressPhase::ScrapeShares),
            Some(&PhaseProgress {
                total: Some(3),
                completed: 3,
                successful: 1,
                errors: 2,
                timeouts: 1,
                parsing_errors: 1,
                last: Some("IT0000000003".to_string()),
                finished: true,
                ..PhaseProgress::default()
            })
        );
    }

    #[test]
    fn loading_isins_progress_supports_unknown_total_spinner_state() {
        let mut state = ProgressState::default();

        state.apply(&ProgressEvent::PhaseStarted {
            phase: ProgressPhase::LoadShareIsins,
            total: None,
        });
        state.apply(&ProgressEvent::PhaseFinished {
            phase: ProgressPhase::LoadShareIsins,
        });

        assert_eq!(
            state.phase(ProgressPhase::LoadShareIsins),
            Some(&PhaseProgress {
                total: None,
                finished: true,
                ..PhaseProgress::default()
            })
        );
    }

    #[test]
    fn share_insert_progress_counts_successes_failures_and_last_completed_share() {
        let mut state = ProgressState::default();

        state.apply(&ProgressEvent::PhaseStarted {
            phase: ProgressPhase::InsertShares,
            total: Some(3),
        });
        state.apply(&ProgressEvent::ShareInserted {
            isin: "IT0000000001".to_string(),
            successful: true,
        });
        state.apply(&ProgressEvent::ShareInserted {
            isin: "IT0000000002".to_string(),
            successful: false,
        });
        state.apply(&ProgressEvent::ShareInserted {
            isin: "IT0000000003".to_string(),
            successful: true,
        });
        state.apply(&ProgressEvent::PhaseFinished {
            phase: ProgressPhase::InsertShares,
        });

        assert_eq!(
            state.phase(ProgressPhase::InsertShares),
            Some(&PhaseProgress {
                total: Some(3),
                completed: 3,
                successful: 2,
                errors: 1,
                last: Some("IT0000000003".to_string()),
                finished: true,
                ..PhaseProgress::default()
            })
        );
    }

    #[test]
    fn insertion_status_fails_when_any_insert_fails() {
        assert!(super::insertion_succeeded(&InsertionMetrics {
            total: 2,
            successful: 2,
            failed: 0,
        }));
        assert!(!super::insertion_succeeded(&InsertionMetrics {
            total: 2,
            successful: 1,
            failed: 1,
        }));
    }
}
