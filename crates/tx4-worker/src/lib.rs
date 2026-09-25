//! TX4 worker library boundary.
//!
//! Phase-1A: crate boundary only. No outbox, lease, fencing, or job loops.

#![forbid(unsafe_code)]

pub use tx4_application;

/// Marker that the worker library crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_tx4_worker() {
        assert_eq!(crate_name(), "tx4-worker");
    }
}
