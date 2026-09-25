//! TX4 domain primitives.
//!
//! Phase-1B: identity, money/currency (ADR-006), version, and domain errors.
//! No transaction lifecycle, ledger, settlement, or payment execution.

#![forbid(unsafe_code)]

mod allocation;
mod currency;
mod error;
mod identity;
mod money;
mod rounding;
mod version;

pub use allocation::allocate;
pub use currency::CurrencyId;
pub use error::DomainError;
pub use identity::{OperationId, TenantId, TransactionId};
pub use money::Money;
pub use rounding::round_half_even_rational;
pub use version::Version;

/// Marker that the domain crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_tx4_domain() {
        assert_eq!(crate_name(), "tx4-domain");
    }
}
