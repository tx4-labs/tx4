//! Domain-oriented errors for primitives and lifecycle (ADR-007).
//!
//! No HTTP status codes and no infrastructure error types.

use core::fmt;

/// Typed domain failure for identity, money, currency, version, and lifecycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainError {
    InvalidIdentity(&'static str),
    InvalidCurrency,
    CurrencyMismatch,
    MoneyOverflow,
    InvalidMoneyAmount,
    InvalidAllocation,
    InvalidVersion,
    InvalidRounding,
    /// Attempt budget must be absent or ≥ 1 (ADR-007 §7.3).
    InvalidAttemptBudget,
    /// Matrix cell is forbidden (ADR-007 §6).
    ForbiddenTransition,
    /// Conditional predicate (C1/C2/C3) not satisfied.
    ConditionalPredicateUnsatisfied,
    /// Post-acceptance exit without compensating-obligation intent (ADR-007 §6.1).
    CompensatingIntentRequired,
    /// `PENDING` → `PAID` without known-success evidence path.
    MissingKnownSuccessEvidence,
    /// Command expected a different source state (stale / already advanced).
    StaleTransition,
    /// Duplicate replay disagrees with current state, or conflicting apply.
    ConflictingTransition,
    /// Terminal primary state forbids further primary progression (ADR-007 §11).
    TerminalStateImmutable,
    /// Attempt to treat timeout/unknown alone as transaction `FAILED` (ADR-007 §7.2).
    TimeoutIsNotFailure,
    /// Conflict set contained no matrix-eligible candidate.
    NoEligibleConflictCandidate,
    /// Same operation identity with distinct targets (should not occur).
    AmbiguousConflictTie,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentity(kind) => write!(f, "invalid {kind}"),
            Self::InvalidCurrency => write!(f, "invalid currency identity"),
            Self::CurrencyMismatch => write!(f, "currency mismatch"),
            Self::MoneyOverflow => write!(f, "money arithmetic overflow"),
            Self::InvalidMoneyAmount => write!(f, "invalid money amount"),
            Self::InvalidAllocation => write!(f, "invalid money allocation"),
            Self::InvalidVersion => write!(f, "invalid version"),
            Self::InvalidRounding => write!(f, "invalid rounding inputs"),
            Self::InvalidAttemptBudget => write!(f, "invalid attempt budget"),
            Self::ForbiddenTransition => write!(f, "forbidden lifecycle transition"),
            Self::ConditionalPredicateUnsatisfied => {
                write!(f, "lifecycle conditional predicate unsatisfied")
            }
            Self::CompensatingIntentRequired => {
                write!(f, "compensating-obligation intent required")
            }
            Self::MissingKnownSuccessEvidence => {
                write!(f, "known-success evidence required for PAID")
            }
            Self::StaleTransition => write!(f, "stale lifecycle transition"),
            Self::ConflictingTransition => write!(f, "conflicting lifecycle transition"),
            Self::TerminalStateImmutable => write!(f, "terminal lifecycle state is immutable"),
            Self::TimeoutIsNotFailure => {
                write!(
                    f,
                    "timeout or unknown disposition is not transaction FAILED"
                )
            }
            Self::NoEligibleConflictCandidate => {
                write!(f, "no eligible lifecycle conflict candidate")
            }
            Self::AmbiguousConflictTie => write!(f, "ambiguous lifecycle conflict tie"),
        }
    }
}

impl std::error::Error for DomainError {}
