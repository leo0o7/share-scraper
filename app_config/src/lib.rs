use std::env;
use std::fmt::{Display, Formatter};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use config::{Config, Environment, File};
use serde::Deserialize;
use tracing::Level;
use tracing_subscriber::EnvFilter;

pub type ConfigResult<T> = Result<T, ConfigError>;

#[derive(Debug, Clone)]
pub enum ConfigError {
    ConfigFileNotFound { manifest_dir: PathBuf },
    LoadConfig(String),
    InvalidServerBindAddress(String),
    InvalidLoggingLevel(String),
    InvalidLoggingFilePath(&'static str),
    MissingDatabaseUrl,
    InvalidDatabasePoolMaxConnections,
    InvalidDatabaseAcquireTimeoutSeconds,
    InvalidScraperShareRefreshAgeMinutes,
    InvalidScraperShareConcurrency,
    InvalidScraperShareTimeoutSeconds,
    InvalidScraperIsinMaxPagesPerLetter,
    InvalidScraperParseThreads,
    InvalidScraperHttpTimeout(&'static str),
    InvalidScraperRetryDuration(&'static str),
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ConfigFileNotFound { manifest_dir } => write!(
                f,
                "config.toml was not found from manifest directory {}",
                manifest_dir.display()
            ),
            ConfigError::LoadConfig(err) => write!(f, "failed to load configuration: {err}"),
            ConfigError::InvalidServerBindAddress(addr) => {
                write!(f, "invalid server bind address: {addr}")
            }
            ConfigError::InvalidLoggingLevel(level) => {
                write!(f, "invalid logging level/filter: {level}")
            }
            ConfigError::InvalidLoggingFilePath(field) => {
                write!(f, "invalid empty logging file path: {field}")
            }
            ConfigError::MissingDatabaseUrl => write!(
                f,
                "missing database URL: set DATABASE_URL or PIAZZA__DATABASE__URL"
            ),
            ConfigError::InvalidDatabasePoolMaxConnections => {
                write!(f, "database.pool_max_connections must be greater than zero")
            }
            ConfigError::InvalidDatabaseAcquireTimeoutSeconds => write!(
                f,
                "database.acquire_timeout_seconds must be greater than zero"
            ),
            ConfigError::InvalidScraperShareRefreshAgeMinutes => write!(
                f,
                "scraper.share_refresh_age_minutes must be greater than zero"
            ),
            ConfigError::InvalidScraperShareConcurrency => {
                write!(f, "scraper.share_concurrency must be greater than zero")
            }
            ConfigError::InvalidScraperShareTimeoutSeconds => {
                write!(f, "scraper.share_timeout_seconds must be greater than zero")
            }
            ConfigError::InvalidScraperIsinMaxPagesPerLetter => {
                write!(
                    f,
                    "scraper.isin_max_pages_per_letter must be greater than zero"
                )
            }
            ConfigError::InvalidScraperParseThreads => {
                write!(
                    f,
                    "scraper.parse_threads must be greater than zero when set"
                )
            }
            ConfigError::InvalidScraperHttpTimeout(field) => {
                write!(f, "scraper.{field} must be greater than zero")
            }
            ConfigError::InvalidScraperRetryDuration(field) => {
                write!(f, "scraper.{field} must be greater than zero")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub database: DatabaseConfig,
    pub scraper: ScraperConfig,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_address: SocketAddr,
}

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub stdout: bool,
    pub server_file_path: PathBuf,
    pub scraper_file_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub pool_max_connections: u32,
    pub acquire_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct ScraperConfig {
    pub share_refresh_age: Duration,
    pub share_concurrency: usize,
    pub share_timeout: Duration,
    pub isin_max_pages_per_letter: u8,
    pub parse_threads: Option<usize>,
    pub http_pool_max_idle_per_host: usize,
    pub http_request_timeout: Duration,
    pub http_connect_timeout: Duration,
    pub http_idle_timeout: Duration,
    pub http_keepalive: Duration,
    pub backoff: BackoffConfig,
}

#[derive(Debug, Clone)]
pub struct BackoffConfig {
    pub retry_count: u32,
    pub total_timeout: Duration,
    pub base_delay: Duration,
    pub jitter_max: Duration,
}

#[derive(Debug, Deserialize)]
struct RawAppConfig {
    server: RawServerConfig,
    logging: RawLoggingConfig,
    database: RawDatabaseConfig,
    scraper: RawScraperConfig,
}

#[derive(Debug, Deserialize)]
struct RawServerConfig {
    bind_address: String,
}

#[derive(Debug, Deserialize)]
struct RawLoggingConfig {
    level: String,
    stdout: bool,
    server_file_path: String,
    scraper_file_path: String,
}

#[derive(Debug, Deserialize)]
struct RawDatabaseConfig {
    url: Option<String>,
    pool_max_connections: u32,
    acquire_timeout_seconds: u64,
}

#[derive(Debug, Deserialize)]
struct RawScraperConfig {
    share_refresh_age_minutes: u64,
    share_concurrency: usize,
    share_timeout_seconds: u64,
    isin_max_pages_per_letter: u8,
    parse_threads: Option<usize>,
    http_pool_max_idle_per_host: usize,
    http_request_timeout_seconds: u64,
    http_connect_timeout_seconds: u64,
    http_idle_timeout_seconds: u64,
    http_keepalive_seconds: u64,
    retry_count: u32,
    retry_total_timeout_seconds: u64,
    retry_base_delay_milliseconds: u64,
    retry_jitter_max_milliseconds: u64,
}

pub fn load_config(manifest_dir: impl AsRef<Path>) -> ConfigResult<AppConfig> {
    let manifest_dir = manifest_dir.as_ref();
    let config_path = root_config_path(manifest_dir)?;

    if let Some(root_dir) = config_path.parent() {
        let _ = dotenvy::from_path(root_dir.join(".env"));
    }

    let mut raw = Config::builder()
        .add_source(File::from(config_path))
        .add_source(
            Environment::with_prefix("PIAZZA")
                .separator("__")
                .try_parsing(true),
        )
        .build()
        .map_err(|err| ConfigError::LoadConfig(err.to_string()))?
        .try_deserialize::<RawAppConfig>()
        .map_err(|err| ConfigError::LoadConfig(err.to_string()))?;

    if raw.database.url.is_none() {
        raw.database.url = env::var("DATABASE_URL").ok();
    }

    validate_config(raw)
}

fn root_config_path(manifest_dir: &Path) -> ConfigResult<PathBuf> {
    let local_config_path = manifest_dir.join("config.toml");
    if local_config_path.exists() {
        return Ok(local_config_path);
    }

    let parent_config_path = manifest_dir.join("..").join("config.toml");
    if parent_config_path.exists() {
        return Ok(parent_config_path);
    }

    Err(ConfigError::ConfigFileNotFound {
        manifest_dir: manifest_dir.to_path_buf(),
    })
}

fn validate_config(raw: RawAppConfig) -> ConfigResult<AppConfig> {
    let bind_address = raw
        .server
        .bind_address
        .parse::<SocketAddr>()
        .map_err(|_| ConfigError::InvalidServerBindAddress(raw.server.bind_address.clone()))?;

    let level = match env::var("RUST_LOG") {
        Ok(rust_log) => {
            EnvFilter::try_new(&rust_log)
                .map_err(|_| ConfigError::InvalidLoggingLevel(rust_log.clone()))?;
            rust_log
        }
        Err(_) => {
            Level::from_str(&raw.logging.level)
                .map_err(|_| ConfigError::InvalidLoggingLevel(raw.logging.level.clone()))?;
            raw.logging.level
        }
    };

    let server_file_path = validate_path("logging.server_file_path", raw.logging.server_file_path)?;
    let scraper_file_path =
        validate_path("logging.scraper_file_path", raw.logging.scraper_file_path)?;
    let database_url = raw
        .database
        .url
        .filter(|url| !url.trim().is_empty())
        .ok_or(ConfigError::MissingDatabaseUrl)?;
    if raw.database.pool_max_connections == 0 {
        return Err(ConfigError::InvalidDatabasePoolMaxConnections);
    }
    if raw.database.acquire_timeout_seconds == 0 {
        return Err(ConfigError::InvalidDatabaseAcquireTimeoutSeconds);
    }
    let share_refresh_age_seconds = raw
        .scraper
        .share_refresh_age_minutes
        .checked_mul(60)
        .ok_or(ConfigError::InvalidScraperShareRefreshAgeMinutes)?;
    if share_refresh_age_seconds == 0 || share_refresh_age_seconds > i64::MAX as u64 {
        return Err(ConfigError::InvalidScraperShareRefreshAgeMinutes);
    }
    if raw.scraper.share_concurrency == 0 {
        return Err(ConfigError::InvalidScraperShareConcurrency);
    }
    if raw.scraper.share_timeout_seconds == 0 {
        return Err(ConfigError::InvalidScraperShareTimeoutSeconds);
    }
    if raw.scraper.isin_max_pages_per_letter == 0 {
        return Err(ConfigError::InvalidScraperIsinMaxPagesPerLetter);
    }
    if raw.scraper.parse_threads == Some(0) {
        return Err(ConfigError::InvalidScraperParseThreads);
    }
    validate_nonzero_scraper_duration(
        "http_request_timeout_seconds",
        raw.scraper.http_request_timeout_seconds,
    )?;
    validate_nonzero_scraper_duration(
        "http_connect_timeout_seconds",
        raw.scraper.http_connect_timeout_seconds,
    )?;
    validate_nonzero_scraper_duration(
        "http_idle_timeout_seconds",
        raw.scraper.http_idle_timeout_seconds,
    )?;
    validate_nonzero_scraper_duration(
        "http_keepalive_seconds",
        raw.scraper.http_keepalive_seconds,
    )?;
    validate_nonzero_scraper_retry_duration(
        "retry_total_timeout_seconds",
        raw.scraper.retry_total_timeout_seconds,
    )?;
    validate_nonzero_scraper_retry_duration(
        "retry_base_delay_milliseconds",
        raw.scraper.retry_base_delay_milliseconds,
    )?;
    validate_nonzero_scraper_retry_duration(
        "retry_jitter_max_milliseconds",
        raw.scraper.retry_jitter_max_milliseconds,
    )?;

    Ok(AppConfig {
        server: ServerConfig { bind_address },
        logging: LoggingConfig {
            level,
            stdout: raw.logging.stdout,
            server_file_path,
            scraper_file_path,
        },
        database: DatabaseConfig {
            url: database_url,
            pool_max_connections: raw.database.pool_max_connections,
            acquire_timeout: Duration::from_secs(raw.database.acquire_timeout_seconds),
        },
        scraper: ScraperConfig {
            share_refresh_age: Duration::from_secs(share_refresh_age_seconds),
            share_concurrency: raw.scraper.share_concurrency,
            share_timeout: Duration::from_secs(raw.scraper.share_timeout_seconds),
            isin_max_pages_per_letter: raw.scraper.isin_max_pages_per_letter,
            parse_threads: raw.scraper.parse_threads,
            http_pool_max_idle_per_host: raw.scraper.http_pool_max_idle_per_host,
            http_request_timeout: Duration::from_secs(raw.scraper.http_request_timeout_seconds),
            http_connect_timeout: Duration::from_secs(raw.scraper.http_connect_timeout_seconds),
            http_idle_timeout: Duration::from_secs(raw.scraper.http_idle_timeout_seconds),
            http_keepalive: Duration::from_secs(raw.scraper.http_keepalive_seconds),
            backoff: BackoffConfig {
                retry_count: raw.scraper.retry_count,
                total_timeout: Duration::from_secs(raw.scraper.retry_total_timeout_seconds),
                base_delay: Duration::from_millis(raw.scraper.retry_base_delay_milliseconds),
                jitter_max: Duration::from_millis(raw.scraper.retry_jitter_max_milliseconds),
            },
        },
    })
}

fn validate_nonzero_scraper_duration(field: &'static str, value: u64) -> ConfigResult<()> {
    if value == 0 {
        return Err(ConfigError::InvalidScraperHttpTimeout(field));
    }

    Ok(())
}

fn validate_nonzero_scraper_retry_duration(field: &'static str, value: u64) -> ConfigResult<()> {
    if value == 0 {
        return Err(ConfigError::InvalidScraperRetryDuration(field));
    }

    Ok(())
}

fn validate_path(field: &'static str, path: String) -> ConfigResult<PathBuf> {
    if path.trim().is_empty() {
        return Err(ConfigError::InvalidLoggingFilePath(field));
    }

    Ok(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serial_test::serial;
    use tempfile::tempdir;

    use crate::{load_config, ConfigError};

    #[test]
    #[serial]
    fn loads_root_config_from_binary_manifest_dir() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var("PIAZZA__SERVER__BIND_ADDRESS", None::<&str>, || {
                let root = tempdir().unwrap();
                let server_dir = root.path().join("server");
                fs::create_dir(&server_dir).unwrap();
                write_config(root.path(), "127.0.0.1:3000", "info", true);

                let config = load_config(&server_dir).unwrap();

                assert_eq!(config.server.bind_address.to_string(), "127.0.0.1:3000");
                assert_eq!(config.logging.level, "info");
                assert!(config.logging.stdout);
            })
        });
    }

    #[test]
    #[serial]
    fn supports_namespaced_environment_overrides() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var(
                "PIAZZA__SERVER__BIND_ADDRESS",
                Some("127.0.0.1:4000"),
                || {
                    let root = tempdir().unwrap();
                    write_config(root.path(), "127.0.0.1:3000", "info", true);

                    let config = load_config(root.path()).unwrap();

                    assert_eq!(config.server.bind_address.to_string(), "127.0.0.1:4000");
                },
            )
        });
    }

    #[test]
    #[serial]
    fn rust_log_overrides_configured_logging_level() {
        temp_env::with_var("PIAZZA__SERVER__BIND_ADDRESS", None::<&str>, || {
            temp_env::with_var("RUST_LOG", Some("debug"), || {
                let root = tempdir().unwrap();
                write_config(root.path(), "127.0.0.1:3000", "info", true);

                let config = load_config(root.path()).unwrap();

                assert_eq!(config.logging.level, "debug");
            })
        });
    }

    #[test]
    #[serial]
    fn rejects_invalid_socket_addresses() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var("PIAZZA__SERVER__BIND_ADDRESS", None::<&str>, || {
                let root = tempdir().unwrap();
                write_config(root.path(), "not-a-socket", "info", true);

                let err = load_config(root.path()).unwrap_err();

                assert!(matches!(err, ConfigError::InvalidServerBindAddress(_)));
            })
        });
    }

    #[test]
    #[serial]
    fn rejects_invalid_logging_filters() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var("PIAZZA__SERVER__BIND_ADDRESS", None::<&str>, || {
                let root = tempdir().unwrap();
                write_config(root.path(), "127.0.0.1:3000", "not a filter", true);

                let err = load_config(root.path()).unwrap_err();

                assert!(matches!(err, ConfigError::InvalidLoggingLevel(_)));
            })
        });
    }

    #[test]
    #[serial]
    fn legacy_database_url_populates_database_config() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var("DATABASE_URL", Some("postgres://localhost/piazza"), || {
                let root = tempdir().unwrap();
                write_config_without_database_url(root.path(), "127.0.0.1:3000", "info", true);

                let config = load_config(root.path()).unwrap();

                assert_eq!(config.database.url, "postgres://localhost/piazza");
                assert_eq!(config.database.pool_max_connections, 5);
                assert_eq!(config.database.acquire_timeout.as_secs(), 10);
            })
        });
    }

    #[test]
    #[serial]
    fn loads_configured_share_refresh_age() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var(
                "PIAZZA__SCRAPER__SHARE_REFRESH_AGE_MINUTES",
                Some("30"),
                || {
                    let root = tempdir().unwrap();
                    write_config(root.path(), "127.0.0.1:3000", "info", true);

                    let config = load_config(root.path()).unwrap();

                    assert_eq!(config.scraper.share_refresh_age.as_secs(), 30 * 60);
                },
            )
        });
    }

    #[test]
    #[serial]
    fn loads_configured_scraper_retry_settings() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var("PIAZZA__SCRAPER__RETRY_COUNT", Some("4"), || {
                let root = tempdir().unwrap();
                write_config(root.path(), "127.0.0.1:3000", "info", true);

                let config = load_config(root.path()).unwrap();

                assert_eq!(config.scraper.backoff.retry_count, 4);
                assert_eq!(config.scraper.backoff.total_timeout.as_secs(), 128);
                assert_eq!(config.scraper.backoff.base_delay.as_millis(), 500);
                assert_eq!(config.scraper.backoff.jitter_max.as_millis(), 1000);
            })
        });
    }

    #[test]
    #[serial]
    fn loads_configured_share_execution_settings() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_vars(
                [
                    ("PIAZZA__SCRAPER__SHARE_CONCURRENCY", Some("25")),
                    ("PIAZZA__SCRAPER__PARSE_THREADS", Some("2")),
                    ("PIAZZA__SCRAPER__ISIN_MAX_PAGES_PER_LETTER", Some("12")),
                ],
                || {
                    let root = tempdir().unwrap();
                    write_config(root.path(), "127.0.0.1:3000", "info", true);

                    let config = load_config(root.path()).unwrap();

                    assert_eq!(config.scraper.share_concurrency, 25);
                    assert_eq!(config.scraper.share_timeout.as_secs(), 5 * 60);
                    assert_eq!(config.scraper.parse_threads, Some(2));
                    assert_eq!(config.scraper.isin_max_pages_per_letter, 12);
                },
            )
        });
    }

    #[test]
    #[serial]
    fn rejects_zero_isin_page_cap() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var(
                "PIAZZA__SCRAPER__ISIN_MAX_PAGES_PER_LETTER",
                Some("0"),
                || {
                    let root = tempdir().unwrap();
                    write_config(root.path(), "127.0.0.1:3000", "info", true);

                    let err = load_config(root.path()).unwrap_err();

                    assert!(matches!(
                        err,
                        ConfigError::InvalidScraperIsinMaxPagesPerLetter
                    ));
                },
            )
        });
    }

    #[test]
    #[serial]
    fn rejects_zero_share_refresh_age() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var(
                "PIAZZA__SCRAPER__SHARE_REFRESH_AGE_MINUTES",
                Some("0"),
                || {
                    let root = tempdir().unwrap();
                    write_config(root.path(), "127.0.0.1:3000", "info", true);

                    let err = load_config(root.path()).unwrap_err();

                    assert!(matches!(
                        err,
                        ConfigError::InvalidScraperShareRefreshAgeMinutes
                    ));
                },
            )
        });
    }

    #[test]
    #[serial]
    fn rejects_zero_share_concurrency() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var("PIAZZA__SCRAPER__SHARE_CONCURRENCY", Some("0"), || {
                let root = tempdir().unwrap();
                write_config(root.path(), "127.0.0.1:3000", "info", true);

                let err = load_config(root.path()).unwrap_err();

                assert!(matches!(err, ConfigError::InvalidScraperShareConcurrency));
            })
        });
    }

    #[test]
    #[serial]
    fn rejects_zero_share_timeout() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var("PIAZZA__SCRAPER__SHARE_TIMEOUT_SECONDS", Some("0"), || {
                let root = tempdir().unwrap();
                write_config(root.path(), "127.0.0.1:3000", "info", true);

                let err = load_config(root.path()).unwrap_err();

                assert!(matches!(
                    err,
                    ConfigError::InvalidScraperShareTimeoutSeconds
                ));
            })
        });
    }

    #[test]
    #[serial]
    fn rejects_zero_scraper_http_timeout() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var(
                "PIAZZA__SCRAPER__HTTP_REQUEST_TIMEOUT_SECONDS",
                Some("0"),
                || {
                    let root = tempdir().unwrap();
                    write_config(root.path(), "127.0.0.1:3000", "info", true);

                    let err = load_config(root.path()).unwrap_err();

                    assert!(matches!(
                        err,
                        ConfigError::InvalidScraperHttpTimeout("http_request_timeout_seconds")
                    ));
                },
            )
        });
    }

    #[test]
    #[serial]
    fn rejects_zero_scraper_retry_delay() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var(
                "PIAZZA__SCRAPER__RETRY_BASE_DELAY_MILLISECONDS",
                Some("0"),
                || {
                    let root = tempdir().unwrap();
                    write_config(root.path(), "127.0.0.1:3000", "info", true);

                    let err = load_config(root.path()).unwrap_err();

                    assert!(matches!(
                        err,
                        ConfigError::InvalidScraperRetryDuration("retry_base_delay_milliseconds")
                    ));
                },
            )
        });
    }

    #[test]
    #[serial]
    fn rejects_missing_database_url() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var("DATABASE_URL", None::<&str>, || {
                temp_env::with_var("PIAZZA__DATABASE__URL", None::<&str>, || {
                    let root = tempdir().unwrap();
                    write_config_without_database_url(root.path(), "127.0.0.1:3000", "info", true);

                    let err = load_config(root.path()).unwrap_err();

                    assert!(matches!(err, ConfigError::MissingDatabaseUrl));
                })
            })
        });
    }

    #[test]
    fn committed_config_is_safe_and_uses_explicit_duration_units() {
        let config = include_str!("../../config.toml");

        assert!(!config.contains("url ="));
        assert!(config.contains("acquire_timeout_seconds"));
        assert!(config.contains("share_refresh_age_minutes"));
        assert!(config.contains("isin_max_pages_per_letter = 20"));
        assert!(config.contains("share_timeout_seconds"));
        assert!(config.contains("http_request_timeout_seconds"));
        assert!(config.contains("http_connect_timeout_seconds"));
        assert!(config.contains("http_idle_timeout_seconds"));
        assert!(config.contains("http_keepalive_seconds"));
        assert!(config.contains("retry_total_timeout_seconds"));
        assert!(config.contains("retry_base_delay_milliseconds"));
        assert!(config.contains("retry_jitter_max_milliseconds"));
    }

    fn write_config(root: &std::path::Path, bind_address: &str, level: &str, stdout: bool) {
        write_config_with_database_url(
            root,
            bind_address,
            level,
            stdout,
            "postgres://localhost/piazza",
        );
    }

    fn write_config_with_database_url(
        root: &std::path::Path,
        bind_address: &str,
        level: &str,
        stdout: bool,
        database_url: &str,
    ) {
        fs::write(
            root.join("config.toml"),
            format!(
                r#"
[server]
bind_address = "{bind_address}"

[logging]
level = "{level}"
stdout = {stdout}
server_file_path = "../server.log"
scraper_file_path = "share_scraper.log"

[database]
url = "{database_url}"
pool_max_connections = 5
acquire_timeout_seconds = 10

[scraper]
share_refresh_age_minutes = 15
isin_max_pages_per_letter = 20
share_concurrency = 200
share_timeout_seconds = 300
http_pool_max_idle_per_host = 200
http_request_timeout_seconds = 30
http_connect_timeout_seconds = 10
http_idle_timeout_seconds = 15
http_keepalive_seconds = 30
retry_count = 8
retry_total_timeout_seconds = 128
retry_base_delay_milliseconds = 500
retry_jitter_max_milliseconds = 1000
"#
            ),
        )
        .unwrap();
    }

    fn write_config_without_database_url(
        root: &std::path::Path,
        bind_address: &str,
        level: &str,
        stdout: bool,
    ) {
        fs::write(
            root.join("config.toml"),
            format!(
                r#"
[server]
bind_address = "{bind_address}"

[logging]
level = "{level}"
stdout = {stdout}
server_file_path = "../server.log"
scraper_file_path = "share_scraper.log"

[database]
pool_max_connections = 5
acquire_timeout_seconds = 10

[scraper]
share_refresh_age_minutes = 15
isin_max_pages_per_letter = 20
share_concurrency = 200
share_timeout_seconds = 300
http_pool_max_idle_per_host = 200
http_request_timeout_seconds = 30
http_connect_timeout_seconds = 10
http_idle_timeout_seconds = 15
http_keepalive_seconds = 30
retry_count = 8
retry_total_timeout_seconds = 128
retry_base_delay_milliseconds = 500
retry_jitter_max_milliseconds = 1000
"#
            ),
        )
        .unwrap();
    }
}
