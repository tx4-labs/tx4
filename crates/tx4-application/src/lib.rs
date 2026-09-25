//! TX4 application / use-case boundary.
//!
//! Ports for infrastructure adapters are defined here (frozen architecture).
//! Phase-1A: structural boundary only — no complete workflows.

#![forbid(unsafe_code)]

pub use tx4_domain;

/// Marker that the application crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

/// Placeholder module for future application ports (repositories, providers, etc.).
pub mod ports {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_tx4_application() {
        assert_eq!(crate_name(), "tx4-application");
    }

    #[test]
    fn depends_on_domain() {
        assert_eq!(tx4_domain::crate_name(), "tx4-domain");
    }
}
