//! TX4 persistence adapter boundary.
//!
//! Phase-1A: crate boundary only. No PostgreSQL, migrations, or repositories.

#![forbid(unsafe_code)]

pub use tx4_application;
pub use tx4_domain;

/// Marker that the persistence crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_tx4_persistence() {
        assert_eq!(crate_name(), "tx4-persistence");
    }
}
