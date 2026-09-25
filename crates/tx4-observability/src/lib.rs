//! TX4 observability boundary.
//!
//! Phase-1A: crate boundary only. No exporters, vendors, or business telemetry.
//! Logs/traces remain diagnostic and non-authoritative (frozen architecture).

#![forbid(unsafe_code)]

/// Marker that the observability crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_tx4_observability() {
        assert_eq!(crate_name(), "tx4-observability");
    }
}
