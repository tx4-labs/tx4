//! TX4 configuration boundary.
//!
//! Phase-1A: crate boundary only. No env loading, secrets, or production config.

#![forbid(unsafe_code)]

/// Marker that the config crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_tx4_config() {
        assert_eq!(crate_name(), "tx4-config");
    }
}
