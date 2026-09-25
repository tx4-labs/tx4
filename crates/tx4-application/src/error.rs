//! Application-layer error vocabulary (IA §22).
//!
//! HTTP status mapping is intentionally deferred to the API phase.
//! Infrastructure-specific error types must not appear here.

use core::fmt;

/// Application error classes for future command/query results.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationError {
    Validation {
        message: String,
    },
    BusinessRejection {
        message: String,
    },
    Conflict {
        message: String,
    },
    IdempotencyConflict {
        message: String,
    },
    /// Provider/outcome uncertainty — not a terminal business failure.
    Uncertainty {
        message: String,
    },
    Retryable {
        message: String,
    },
    PermanentInternal {
        message: String,
    },
    /// Explicit timeout disposition. Must never be treated as transaction `FAILED`.
    Timeout {
        message: String,
    },
}

impl ApplicationError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn business_rejection(message: impl Into<String>) -> Self {
        Self::BusinessRejection {
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    pub fn idempotency_conflict(message: impl Into<String>) -> Self {
        Self::IdempotencyConflict {
            message: message.into(),
        }
    }

    pub fn uncertainty(message: impl Into<String>) -> Self {
        Self::Uncertainty {
            message: message.into(),
        }
    }

    pub fn retryable(message: impl Into<String>) -> Self {
        Self::Retryable {
            message: message.into(),
        }
    }

    pub fn permanent_internal(message: impl Into<String>) -> Self {
        Self::PermanentInternal {
            message: message.into(),
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Timeout {
            message: message.into(),
        }
    }

    /// Terminal business failures that may finalize a business outcome.
    ///
    /// Timeout and uncertainty are excluded (IA: `TIMEOUT ≠ FAILED`).
    pub fn is_terminal_business_failure(&self) -> bool {
        matches!(
            self,
            Self::BusinessRejection { .. }
                | Self::Conflict { .. }
                | Self::IdempotencyConflict { .. }
                | Self::PermanentInternal { .. }
        )
    }

    /// Whether this error alone may move a transaction to `FAILED`.
    ///
    /// Always false for timeout/uncertainty/retryable/validation.
    pub fn implies_transaction_failed(&self) -> bool {
        false
    }

    pub fn is_uncertainty_or_timeout(&self) -> bool {
        matches!(self, Self::Uncertainty { .. } | Self::Timeout { .. })
    }

    pub fn is_retryable_infrastructure(&self) -> bool {
        matches!(self, Self::Retryable { .. })
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Validation { message }
            | Self::BusinessRejection { message }
            | Self::Conflict { message }
            | Self::IdempotencyConflict { message }
            | Self::Uncertainty { message }
            | Self::Retryable { message }
            | Self::PermanentInternal { message }
            | Self::Timeout { message } => message,
        }
    }
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation { message } => write!(f, "validation error: {message}"),
            Self::BusinessRejection { message } => write!(f, "business rejection: {message}"),
            Self::Conflict { message } => write!(f, "conflict: {message}"),
            Self::IdempotencyConflict { message } => {
                write!(f, "idempotency conflict: {message}")
            }
            Self::Uncertainty { message } => write!(f, "uncertainty: {message}"),
            Self::Retryable { message } => write!(f, "retryable error: {message}"),
            Self::PermanentInternal { message } => {
                write!(f, "permanent internal error: {message}")
            }
            Self::Timeout { message } => write!(f, "timeout: {message}"),
        }
    }
}

impl std::error::Error for ApplicationError {}
