//! TX4 configuration boundary.
//!
//! Deterministic, validated, environment-driven process configuration.
//! No business logic. No database connections. No provider SDKs.

#![forbid(unsafe_code)]

mod error;
mod secret;

use std::collections::BTreeMap;
use std::env;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

pub use error::ConfigError;
pub use secret::SecretString;

/// Default HTTP bind when `TX4_HTTP_BIND` is unset.
pub const DEFAULT_HTTP_BIND: &str = "127.0.0.1:8080";

/// Default SQLx pool max connections when `TX4_DATABASE_MAX_CONNECTIONS` is unset.
pub const DEFAULT_DATABASE_MAX_CONNECTIONS: u32 = 10;

/// Default SQLx pool min connections when `TX4_DATABASE_MIN_CONNECTIONS` is unset.
pub const DEFAULT_DATABASE_MIN_CONNECTIONS: u32 = 0;

/// Default pool acquire timeout seconds when `TX4_DATABASE_ACQUIRE_TIMEOUT_SECS` is unset.
pub const DEFAULT_DATABASE_ACQUIRE_TIMEOUT_SECS: u64 = 30;

/// Default service name when `TX4_SERVICE_NAME` is unset.
pub const DEFAULT_SERVICE_NAME: &str = "tx4";

/// Default deployment environment label when `TX4_ENVIRONMENT` is unset.
pub const DEFAULT_ENVIRONMENT: &str = "development";

/// Default tracing filter directive when `TX4_LOG_LEVEL` is unset.
pub const DEFAULT_LOG_LEVEL: &str = "info";

/// Validated immutable process configuration for TX4 runtime apps.
#[derive(Clone)]
pub struct Config {
    /// Listen address for the HTTP server (`TX4_HTTP_BIND`).
    pub http_bind: SocketAddr,
    /// PostgreSQL connection URL (`DATABASE_URL`) — stored only; not connected here.
    pub database_url: SecretString,
    /// Maximum connections in the PostgreSQL pool.
    pub database_max_connections: u32,
    /// Minimum idle connections retained by the pool.
    pub database_min_connections: u32,
    /// Maximum time to wait when acquiring a pool connection.
    pub database_acquire_timeout: Duration,
    /// Optional attempt budget default (`TX4_ATTEMPT_BUDGET_DEFAULT`).
    pub attempt_budget_default: Option<u32>,
    /// Service name for observability resource attributes (`TX4_SERVICE_NAME`).
    pub service_name: String,
    /// Environment label for observability (`TX4_ENVIRONMENT`).
    pub environment: String,
    /// Tracing filter / log level directive (`TX4_LOG_LEVEL`).
    pub log_level: String,
    /// Optional OTLP endpoint for OpenTelemetry-compatible export (`TX4_OTLP_ENDPOINT`).
    /// Hosted vendor remains OPEN; this stores the endpoint only.
    pub otlp_endpoint: Option<String>,
}

impl Config {
    /// Load and validate configuration from process environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut map = BTreeMap::new();
        for (k, v) in env::vars() {
            map.insert(k, v);
        }
        Self::from_map(&map)
    }

    /// Load and validate configuration from an explicit key/value map (tests / tooling).
    pub fn from_map(vars: &BTreeMap<String, String>) -> Result<Self, ConfigError> {
        let database_url = required(vars, "DATABASE_URL")?;
        if database_url.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "DATABASE_URL",
                reason: "must not be empty",
            });
        }

        let http_bind_raw = optional(vars, "TX4_HTTP_BIND")
            .map(str::to_owned)
            .unwrap_or_else(|| DEFAULT_HTTP_BIND.to_owned());
        let http_bind =
            SocketAddr::from_str(&http_bind_raw).map_err(|_| ConfigError::InvalidValue {
                key: "TX4_HTTP_BIND",
                reason: "must be a valid socket address (host:port)",
            })?;

        let database_max_connections = parse_u32_or_default(
            vars,
            "TX4_DATABASE_MAX_CONNECTIONS",
            DEFAULT_DATABASE_MAX_CONNECTIONS,
        )?;
        if database_max_connections == 0 {
            return Err(ConfigError::InvalidValue {
                key: "TX4_DATABASE_MAX_CONNECTIONS",
                reason: "must be >= 1",
            });
        }

        let database_min_connections = parse_u32_or_default(
            vars,
            "TX4_DATABASE_MIN_CONNECTIONS",
            DEFAULT_DATABASE_MIN_CONNECTIONS,
        )?;
        if database_min_connections > database_max_connections {
            return Err(ConfigError::InvalidValue {
                key: "TX4_DATABASE_MIN_CONNECTIONS",
                reason: "must be <= TX4_DATABASE_MAX_CONNECTIONS",
            });
        }

        let acquire_secs = parse_u64_or_default(
            vars,
            "TX4_DATABASE_ACQUIRE_TIMEOUT_SECS",
            DEFAULT_DATABASE_ACQUIRE_TIMEOUT_SECS,
        )?;
        if acquire_secs == 0 {
            return Err(ConfigError::InvalidValue {
                key: "TX4_DATABASE_ACQUIRE_TIMEOUT_SECS",
                reason: "must be >= 1",
            });
        }

        let attempt_budget_default = match optional(vars, "TX4_ATTEMPT_BUDGET_DEFAULT") {
            None => None,
            Some(raw) => {
                let parsed = raw.parse::<u32>().map_err(|_| ConfigError::InvalidValue {
                    key: "TX4_ATTEMPT_BUDGET_DEFAULT",
                    reason: "must be a non-negative integer fitting u32",
                })?;
                if parsed == 0 {
                    return Err(ConfigError::InvalidValue {
                        key: "TX4_ATTEMPT_BUDGET_DEFAULT",
                        reason: "must be >= 1 when set",
                    });
                }
                Some(parsed)
            }
        };

        let service_name = optional(vars, "TX4_SERVICE_NAME")
            .map(str::to_owned)
            .unwrap_or_else(|| DEFAULT_SERVICE_NAME.to_owned());
        if service_name.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "TX4_SERVICE_NAME",
                reason: "must not be empty when set",
            });
        }

        let environment = optional(vars, "TX4_ENVIRONMENT")
            .map(str::to_owned)
            .unwrap_or_else(|| DEFAULT_ENVIRONMENT.to_owned());
        if environment.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "TX4_ENVIRONMENT",
                reason: "must not be empty when set",
            });
        }

        let log_level = optional(vars, "TX4_LOG_LEVEL")
            .map(str::to_owned)
            .unwrap_or_else(|| DEFAULT_LOG_LEVEL.to_owned());
        if log_level.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "TX4_LOG_LEVEL",
                reason: "must not be empty when set",
            });
        }

        let otlp_endpoint = match optional(vars, "TX4_OTLP_ENDPOINT") {
            None => None,
            Some(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return Err(ConfigError::InvalidValue {
                        key: "TX4_OTLP_ENDPOINT",
                        reason: "must not be empty when set",
                    });
                }
                // Reject credential-bearing URLs in OTLP config to avoid secret leakage.
                if trimmed.contains('@') {
                    return Err(ConfigError::InvalidValue {
                        key: "TX4_OTLP_ENDPOINT",
                        reason: "must not embed credentials",
                    });
                }
                Some(trimmed.to_owned())
            }
        };

        Ok(Self {
            http_bind,
            database_url: SecretString::new(database_url),
            database_max_connections,
            database_min_connections,
            database_acquire_timeout: Duration::from_secs(acquire_secs),
            attempt_budget_default,
            service_name,
            environment,
            log_level,
            otlp_endpoint,
        })
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("http_bind", &self.http_bind)
            .field("database_url", &self.database_url)
            .field("database_max_connections", &self.database_max_connections)
            .field("database_min_connections", &self.database_min_connections)
            .field("database_acquire_timeout", &self.database_acquire_timeout)
            .field("attempt_budget_default", &self.attempt_budget_default)
            .field("service_name", &self.service_name)
            .field("environment", &self.environment)
            .field("log_level", &self.log_level)
            .field("otlp_endpoint", &self.otlp_endpoint)
            .finish()
    }
}

fn required(vars: &BTreeMap<String, String>, key: &'static str) -> Result<String, ConfigError> {
    vars.get(key)
        .cloned()
        .ok_or(ConfigError::MissingRequired { key })
}

fn optional<'a>(vars: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    vars.get(key).map(String::as_str)
}

fn parse_u32_or_default(
    vars: &BTreeMap<String, String>,
    key: &'static str,
    default: u32,
) -> Result<u32, ConfigError> {
    match optional(vars, key) {
        None => Ok(default),
        Some(raw) => raw.parse::<u32>().map_err(|_| ConfigError::InvalidValue {
            key,
            reason: "must be a non-negative integer fitting u32",
        }),
    }
}

fn parse_u64_or_default(
    vars: &BTreeMap<String, String>,
    key: &'static str,
    default: u64,
) -> Result<u64, ConfigError> {
    match optional(vars, key) {
        None => Ok(default),
        Some(raw) => raw.parse::<u64>().map_err(|_| ConfigError::InvalidValue {
            key,
            reason: "must be a non-negative integer fitting u64",
        }),
    }
}

/// Marker that the config crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_vars() -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert(
            "DATABASE_URL".to_owned(),
            "postgres://tx4:secret@localhost:5432/tx4".to_owned(),
        );
        m
    }

    #[test]
    fn crate_name_is_tx4_config() {
        assert_eq!(crate_name(), "tx4-config");
    }

    #[test]
    fn valid_with_defaults() {
        let cfg = Config::from_map(&base_vars()).unwrap();
        assert_eq!(cfg.http_bind, "127.0.0.1:8080".parse().unwrap());
        assert_eq!(
            cfg.database_url.expose(),
            "postgres://tx4:secret@localhost:5432/tx4"
        );
        assert_eq!(
            cfg.database_max_connections,
            DEFAULT_DATABASE_MAX_CONNECTIONS
        );
        assert_eq!(
            cfg.database_min_connections,
            DEFAULT_DATABASE_MIN_CONNECTIONS
        );
        assert_eq!(
            cfg.database_acquire_timeout,
            Duration::from_secs(DEFAULT_DATABASE_ACQUIRE_TIMEOUT_SECS)
        );
        assert_eq!(cfg.attempt_budget_default, None);
        assert_eq!(cfg.service_name, DEFAULT_SERVICE_NAME);
        assert_eq!(cfg.environment, DEFAULT_ENVIRONMENT);
        assert_eq!(cfg.log_level, DEFAULT_LOG_LEVEL);
        assert_eq!(cfg.otlp_endpoint, None);
    }

    #[test]
    fn valid_with_observability_overrides() {
        let mut m = base_vars();
        m.insert("TX4_SERVICE_NAME".to_owned(), "tx4-server".to_owned());
        m.insert("TX4_ENVIRONMENT".to_owned(), "staging".to_owned());
        m.insert("TX4_LOG_LEVEL".to_owned(), "debug".to_owned());
        m.insert(
            "TX4_OTLP_ENDPOINT".to_owned(),
            "http://127.0.0.1:4317".to_owned(),
        );
        let cfg = Config::from_map(&m).unwrap();
        assert_eq!(cfg.service_name, "tx4-server");
        assert_eq!(cfg.environment, "staging");
        assert_eq!(cfg.log_level, "debug");
        assert_eq!(cfg.otlp_endpoint.as_deref(), Some("http://127.0.0.1:4317"));
    }

    #[test]
    fn rejects_otlp_endpoint_with_embedded_credentials() {
        let mut m = base_vars();
        m.insert(
            "TX4_OTLP_ENDPOINT".to_owned(),
            "http://user:pass@collector:4317".to_owned(),
        );
        assert!(matches!(
            Config::from_map(&m).unwrap_err(),
            ConfigError::InvalidValue {
                key: "TX4_OTLP_ENDPOINT",
                ..
            }
        ));
    }

    #[test]
    fn valid_with_pool_overrides() {
        let mut m = base_vars();
        m.insert("TX4_HTTP_BIND".to_owned(), "0.0.0.0:9000".to_owned());
        m.insert("TX4_ATTEMPT_BUDGET_DEFAULT".to_owned(), "5".to_owned());
        m.insert("TX4_DATABASE_MAX_CONNECTIONS".to_owned(), "4".to_owned());
        m.insert("TX4_DATABASE_MIN_CONNECTIONS".to_owned(), "1".to_owned());
        m.insert(
            "TX4_DATABASE_ACQUIRE_TIMEOUT_SECS".to_owned(),
            "5".to_owned(),
        );
        let cfg = Config::from_map(&m).unwrap();
        assert_eq!(cfg.http_bind, "0.0.0.0:9000".parse().unwrap());
        assert_eq!(cfg.attempt_budget_default, Some(5));
        assert_eq!(cfg.database_max_connections, 4);
        assert_eq!(cfg.database_min_connections, 1);
        assert_eq!(cfg.database_acquire_timeout, Duration::from_secs(5));
    }

    #[test]
    fn missing_required_database_url() {
        let m = BTreeMap::new();
        assert_eq!(
            Config::from_map(&m).unwrap_err(),
            ConfigError::MissingRequired {
                key: "DATABASE_URL"
            }
        );
    }

    #[test]
    fn empty_database_url_invalid() {
        let mut m = BTreeMap::new();
        m.insert("DATABASE_URL".to_owned(), "   ".to_owned());
        assert!(matches!(
            Config::from_map(&m).unwrap_err(),
            ConfigError::InvalidValue {
                key: "DATABASE_URL",
                ..
            }
        ));
    }

    #[test]
    fn invalid_http_bind() {
        let mut m = base_vars();
        m.insert("TX4_HTTP_BIND".to_owned(), "not-a-socket".to_owned());
        assert!(matches!(
            Config::from_map(&m).unwrap_err(),
            ConfigError::InvalidValue {
                key: "TX4_HTTP_BIND",
                ..
            }
        ));
    }

    #[test]
    fn invalid_attempt_budget() {
        let mut m = base_vars();
        m.insert("TX4_ATTEMPT_BUDGET_DEFAULT".to_owned(), "0".to_owned());
        assert!(matches!(
            Config::from_map(&m).unwrap_err(),
            ConfigError::InvalidValue {
                key: "TX4_ATTEMPT_BUDGET_DEFAULT",
                ..
            }
        ));
    }

    #[test]
    fn invalid_pool_bounds() {
        let mut m = base_vars();
        m.insert("TX4_DATABASE_MAX_CONNECTIONS".to_owned(), "0".to_owned());
        assert!(matches!(
            Config::from_map(&m).unwrap_err(),
            ConfigError::InvalidValue {
                key: "TX4_DATABASE_MAX_CONNECTIONS",
                ..
            }
        ));

        let mut m = base_vars();
        m.insert("TX4_DATABASE_MAX_CONNECTIONS".to_owned(), "2".to_owned());
        m.insert("TX4_DATABASE_MIN_CONNECTIONS".to_owned(), "3".to_owned());
        assert!(matches!(
            Config::from_map(&m).unwrap_err(),
            ConfigError::InvalidValue {
                key: "TX4_DATABASE_MIN_CONNECTIONS",
                ..
            }
        ));
    }

    #[test]
    fn secret_redacted_in_debug() {
        let cfg = Config::from_map(&base_vars()).unwrap();
        let debug = format!("{cfg:?}");
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("secret"));
        assert!(!debug.contains("postgres://"));
    }
}
