//! TX4 domain primitives.
//!
//! Phase-1A: structural crate boundary only. No transaction lifecycle,
//! ledger, settlement, or payment execution.

#![forbid(unsafe_code)]

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
