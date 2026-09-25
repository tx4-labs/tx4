//! Primary transaction lifecycle states (ADR-007 §5).

use core::fmt;
use serde::{Deserialize, Serialize};

/// Canonical TX4 primary lifecycle state (exactly one per transaction).
///
/// Refunds are **not** primary states (ADR-007 §5.2 Option C).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionState {
    Created,
    Pending,
    Paid,
    Processing,
    Completed,
    Cancelled,
    Expired,
    Failed,
}

impl TransactionState {
    /// All eight primary states in ADR-007 declaration order.
    pub const ALL: [Self; 8] = [
        Self::Created,
        Self::Pending,
        Self::Paid,
        Self::Processing,
        Self::Completed,
        Self::Cancelled,
        Self::Expired,
        Self::Failed,
    ];

    /// Terminal primary states (ADR-007 §11): no further primary progression.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Cancelled | Self::Expired | Self::Failed
        )
    }

    /// ADR-007 §9.3 precedence class by **target** state (higher wins).
    pub const fn precedence_class(self) -> u8 {
        match self {
            Self::Paid => 60,
            Self::Completed => 50,
            Self::Processing => 40,
            Self::Failed => 30,
            Self::Cancelled => 20,
            Self::Expired => 10,
            Self::Pending => 0,
            // CREATED is never a transition *target* in the matrix (only initial).
            Self::Created => 0,
        }
    }
}

impl fmt::Display for TransactionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Created => "CREATED",
            Self::Pending => "PENDING",
            Self::Paid => "PAID",
            Self::Processing => "PROCESSING",
            Self::Completed => "COMPLETED",
            Self::Cancelled => "CANCELLED",
            Self::Expired => "EXPIRED",
            Self::Failed => "FAILED",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_states_and_terminals() {
        assert_eq!(TransactionState::ALL.len(), 8);
        assert!(!TransactionState::Created.is_terminal());
        assert!(!TransactionState::Pending.is_terminal());
        assert!(!TransactionState::Paid.is_terminal());
        assert!(!TransactionState::Processing.is_terminal());
        assert!(TransactionState::Completed.is_terminal());
        assert!(TransactionState::Cancelled.is_terminal());
        assert!(TransactionState::Expired.is_terminal());
        assert!(TransactionState::Failed.is_terminal());
    }

    #[test]
    fn precedence_matches_adr_007_9_3() {
        assert_eq!(TransactionState::Paid.precedence_class(), 60);
        assert_eq!(TransactionState::Completed.precedence_class(), 50);
        assert_eq!(TransactionState::Processing.precedence_class(), 40);
        assert_eq!(TransactionState::Failed.precedence_class(), 30);
        assert_eq!(TransactionState::Cancelled.precedence_class(), 20);
        assert_eq!(TransactionState::Expired.precedence_class(), 10);
        assert_eq!(TransactionState::Pending.precedence_class(), 0);
    }
}
