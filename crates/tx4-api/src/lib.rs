//! TX4 HTTP API boundary.
//!
//! Phase-1A: crate boundary only. No routes, auth, or OpenAPI.

#![forbid(unsafe_code)]

pub use tx4_application;

/// Marker that the API crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_tx4_api() {
        assert_eq!(crate_name(), "tx4-api");
    }
}
