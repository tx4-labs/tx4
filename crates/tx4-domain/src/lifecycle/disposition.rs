//! Outcome disposition — orthogonal to primary lifecycle state (ADR-007 §7).

use core::fmt;
use serde::{Deserialize, Serialize};

use super::TransactionState;
use crate::DomainError;

/// Disposition of an external payment/operation outcome (not a ninth primary state).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutcomeDisposition {
    KnownSuccess,
    KnownFailure,
    Timeout,
    Unknown,
    ReconciliationRequired,
}

impl OutcomeDisposition {
    /// ADR-007 §7.2: timeout alone must never imply transaction `FAILED`.
    pub const fn implies_transaction_failed(self) -> bool {
        false
    }

    /// Whether this disposition may accompany remaining in `PENDING` (typical uncertainty path).
    pub const fn allows_remain_pending(self) -> bool {
        matches!(
            self,
            Self::KnownFailure
                | Self::Timeout
                | Self::Unknown
                | Self::ReconciliationRequired
                | Self::KnownSuccess
        )
    }
}

impl fmt::Display for OutcomeDisposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::KnownSuccess => "KNOWN_SUCCESS",
            Self::KnownFailure => "KNOWN_FAILURE",
            Self::Timeout => "TIMEOUT",
            Self::Unknown => "UNKNOWN",
            Self::ReconciliationRequired => "RECONCILIATION_REQUIRED",
        })
    }
}

/// Reject any attempt to treat timeout/unknown alone as a reason to move to `FAILED`.
pub fn reject_timeout_as_failed(
    disposition: OutcomeDisposition,
    target: TransactionState,
) -> Result<(), DomainError> {
    if target == TransactionState::Failed
        && matches!(
            disposition,
            OutcomeDisposition::Timeout | OutcomeDisposition::Unknown
        )
    {
        return Err(DomainError::TimeoutIsNotFailure);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_never_implies_failed() {
        assert!(!OutcomeDisposition::Timeout.implies_transaction_failed());
        assert!(!OutcomeDisposition::Unknown.implies_transaction_failed());
        assert!(!OutcomeDisposition::KnownFailure.implies_transaction_failed());
        assert_eq!(
            reject_timeout_as_failed(OutcomeDisposition::Timeout, TransactionState::Failed)
                .unwrap_err(),
            DomainError::TimeoutIsNotFailure
        );
        assert!(
            reject_timeout_as_failed(OutcomeDisposition::Timeout, TransactionState::Pending)
                .is_ok()
        );
    }
}
