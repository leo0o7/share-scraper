use std::fmt::{Display, Formatter};

use crate::operation::ScraperOperation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputMode {
    Progress,
    Logs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliOptions {
    pub(crate) operation: ScraperOperation,
    pub(crate) output_mode: OutputMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LoggingMode {
    pub(crate) stdout: bool,
    pub(crate) fallback_notice: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeOutputMode {
    pub(crate) logging: LoggingMode,
    pub(crate) render_progress: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliError(String);

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CliError {}

pub(crate) fn parse_cli(args: impl IntoIterator<Item = String>) -> Result<CliOptions, CliError> {
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

pub(crate) fn decide_runtime_output(
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

fn parse_operation_name(operation: &str) -> ScraperOperation {
    ScraperOperation::from_cli_name(operation).expect("operation names are filtered by parse_cli")
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
    use super::*;

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
    fn help_option_returns_help_text() {
        let err = parse_cli(["--help".to_string()]).unwrap_err();

        assert!(err.0.starts_with("Usage: share_service"));
        assert!(err.0.contains("refresh-shares"));
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
}
