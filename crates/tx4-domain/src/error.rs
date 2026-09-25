//! Domain-oriented errors for Phase-1B primitives.
//!
//! No HTTP status codes and no infrastructure error types.

use core::fmt;

/// Typed domain failure for identity, money, currency, and version primitives.
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
        }
    }
}

impl std::error::Error for DomainError {}
