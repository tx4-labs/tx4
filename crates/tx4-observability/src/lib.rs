//! TX4 observability foundation.
//!
//! Structured logging and spans via `tracing`, with an OpenTelemetry-compatible
//! export interface. Hosted vendor remains OPEN (IA §2.7).
//!
//! **Authority boundary:** logs/traces/metrics are diagnostic only. They are never
//! authoritative financial, lifecycle, or audit records.

#![forbid(unsafe_code)]

mod fields;
mod init;
mod otel;
mod redact;

pub use fields::{
    CORRELATION_ID, OPERATION_ID, PROVIDER_ID, REQUEST_ID, TENANT_ID, TRANSACTION_ID,
};
pub use init::{init, ObservabilityGuard};
pub use otel::{OtelExportInterface, OtelExportPlan};
pub use redact::{is_forbidden_log_key, redact_value, FORBIDDEN_LOG_KEYS};

/// Marker that the observability crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tx4_config::Config;

    fn test_config() -> Config {
        let mut m = BTreeMap::new();
        m.insert(
            "DATABASE_URL".to_owned(),
            "postgres://tx4:secret@localhost:5432/tx4".to_owned(),
        );
        m.insert("TX4_SERVICE_NAME".to_owned(), "tx4-test".to_owned());
        m.insert("TX4_LOG_LEVEL".to_owned(), "info".to_owned());
        Config::from_map(&m).expect("valid config")
    }

    #[test]
    fn crate_name_is_tx4_observability() {
        assert_eq!(crate_name(), "tx4-observability");
    }

    #[test]
    fn init_succeeds_and_is_idempotent() {
        let cfg = test_config();
        let _g1 = init(&cfg).expect("first init");
        // Second init returns a guard without installing a second global subscriber.
        let _g2 = init(&cfg).expect("second init");
    }

    #[test]
    fn structured_fields_can_be_emitted() {
        let cfg = test_config();
        let _guard = init(&cfg).expect("init");
        tracing::info!(
            request_id = "req-1",
            correlation_id = "corr-1",
            tenant_id = "ten-1",
            "phase-1d foundation event"
        );
    }

    #[test]
    fn sensitive_keys_are_forbidden() {
        assert!(is_forbidden_log_key("password"));
        assert!(is_forbidden_log_key("Authorization"));
        assert!(is_forbidden_log_key("DATABASE_URL"));
        assert!(is_forbidden_log_key("api_key"));
        assert!(!is_forbidden_log_key("request_id"));
        assert_eq!(redact_value("super-secret"), "REDACTED");
    }

    #[test]
    fn otel_plan_is_compatible_without_vendor_lock() {
        let cfg = test_config();
        let plan = OtelExportPlan::from_config(&cfg);
        assert_eq!(plan.service_name, "tx4-test");
        assert!(plan.endpoint.is_none());
        assert_eq!(plan.interface(), OtelExportInterface::TracingBridge);
    }
}
