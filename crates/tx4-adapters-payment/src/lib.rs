//! TX4 payment provider adapter boundary.
//!
//! Phase-1A: crate boundary only. No Xendit/DOKU/Midtrans, Mock runtime, or I/O.

#![forbid(unsafe_code)]

pub use tx4_application;
pub use tx4_domain;

/// Marker that the payment adapters crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_tx4_adapters_payment() {
        assert_eq!(crate_name(), "tx4-adapters-payment");
    }
}
