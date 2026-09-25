//! Configuration errors.

use core::fmt;

/// Failures while loading or validating process configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    MissingRequired {
        key: &'static str,
    },
    InvalidValue {
        key: &'static str,
        reason: &'static str,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequired { key } => write!(f, "missing required configuration: {key}"),
            Self::InvalidValue { key, reason } => {
                write!(f, "invalid configuration for {key}: {reason}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}
