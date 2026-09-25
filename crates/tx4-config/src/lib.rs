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

pub use error::ConfigError;
pub use secret::SecretString;

/// Default HTTP bind when `TX4_HTTP_BIND` is unset.
pub const DEFAULT_HTTP_BIND: &str = "127.0.0.1:8080";

/// Validated immutable process configuration for TX4 runtime apps.
#[derive(Clone)]
pub struct Config {
    /// Listen address for the HTTP server (`TX4_HTTP_BIND`).
    pub http_bind: SocketAddr,
    /// PostgreSQL connection URL (`DATABASE_URL`) — stored only; not connected here.
    pub database_url: SecretString,
    /// Optional attempt budget default (`TX4_ATTEMPT_BUDGET_DEFAULT`).
    pub attempt_budget_default: Option<u32>,
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

        Ok(Self {
            http_bind,
            database_url: SecretString::new(database_url),
            attempt_budget_default,
        })
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("http_bind", &self.http_bind)
            .field("database_url", &self.database_url)
            .field("attempt_budget_default", &self.attempt_budget_default)
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
        assert_eq!(cfg.attempt_budget_default, None);
    }

    #[test]
    fn valid_with_overrides() {
        let mut m = base_vars();
        m.insert("TX4_HTTP_BIND".to_owned(), "0.0.0.0:9000".to_owned());
        m.insert("TX4_ATTEMPT_BUDGET_DEFAULT".to_owned(), "5".to_owned());
        let cfg = Config::from_map(&m).unwrap();
        assert_eq!(cfg.http_bind, "0.0.0.0:9000".parse().unwrap());
        assert_eq!(cfg.attempt_budget_default, Some(5));
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
    fn secret_redacted_in_debug() {
        let cfg = Config::from_map(&base_vars()).unwrap();
        let debug = format!("{cfg:?}");
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("secret"));
        assert!(!debug.contains("postgres://"));
    }
}
