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
                "missing database URL: set DATABASE_URL or SHARE_SERVICE__DATABASE__URL"
            ),
            ConfigError::InvalidDatabasePoolMaxConnections => {
                write!(f, "database.pool_max_connections must be greater than zero")
            }
            ConfigError::InvalidDatabaseAcquireTimeoutSeconds => write!(
                f,
                "database.acquire_timeout_seconds must be greater than zero"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub database: DatabaseConfig,
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

#[derive(Debug, Deserialize)]
struct RawAppConfig {
    server: RawServerConfig,
    logging: RawLoggingConfig,
    database: RawDatabaseConfig,
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

pub fn load_config(manifest_dir: impl AsRef<Path>) -> ConfigResult<AppConfig> {
    let manifest_dir = manifest_dir.as_ref();
    let config_path = root_config_path(manifest_dir)?;

    if let Some(root_dir) = config_path.parent() {
        let _ = dotenvy::from_path(root_dir.join(".env"));
    }

    let mut raw = Config::builder()
        .add_source(File::from(config_path))
        .add_source(
            Environment::with_prefix("SHARE_SERVICE")
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
    })
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
            temp_env::with_var("SHARE_SERVICE__SERVER__BIND_ADDRESS", None::<&str>, || {
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
                "SHARE_SERVICE__SERVER__BIND_ADDRESS",
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
        temp_env::with_var("SHARE_SERVICE__SERVER__BIND_ADDRESS", None::<&str>, || {
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
            temp_env::with_var("SHARE_SERVICE__SERVER__BIND_ADDRESS", None::<&str>, || {
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
            temp_env::with_var("SHARE_SERVICE__SERVER__BIND_ADDRESS", None::<&str>, || {
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
            temp_env::with_var(
                "DATABASE_URL",
                Some("postgres://localhost/share_service"),
                || {
                    let root = tempdir().unwrap();
                    write_config_without_database_url(root.path(), "127.0.0.1:3000", "info", true);

                    let config = load_config(root.path()).unwrap();

                    assert_eq!(config.database.url, "postgres://localhost/share_service");
                    assert_eq!(config.database.pool_max_connections, 5);
                    assert_eq!(config.database.acquire_timeout.as_secs(), 10);
                },
            )
        });
    }

    #[test]
    #[serial]
    fn rejects_missing_database_url() {
        temp_env::with_var("RUST_LOG", None::<&str>, || {
            temp_env::with_var("DATABASE_URL", None::<&str>, || {
                temp_env::with_var("SHARE_SERVICE__DATABASE__URL", None::<&str>, || {
                    let root = tempdir().unwrap();
                    write_config_without_database_url(root.path(), "127.0.0.1:3000", "info", true);

                    let err = load_config(root.path()).unwrap_err();

                    assert!(matches!(err, ConfigError::MissingDatabaseUrl));
                })
            })
        });
    }

    fn write_config(root: &std::path::Path, bind_address: &str, level: &str, stdout: bool) {
        write_config_with_database_url(
            root,
            bind_address,
            level,
            stdout,
            "postgres://localhost/share_service",
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
"#
            ),
        )
        .unwrap();
    }
}
