//! TX4 application / use-case boundary.
//!
//! Ports for infrastructure adapters are defined here (frozen architecture).
//! Phase-1D: error vocabulary, correlation context, and structural command/query/port
//! namespaces — no business use cases.

#![forbid(unsafe_code)]

mod context;
mod error;

pub use context::RequestContext;
pub use error::ApplicationError;
pub use tx4_domain;

/// Marker that the application crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

/// Future application commands (unauthorized business commands must not be added here yet).
pub mod commands {}

/// Future application queries (no business queries in Phase-1D).
pub mod queries {}

/// Infrastructure ports defined at the application boundary (IA §5.2).
///
/// Concrete port traits and adapters are authorized in later phases.
/// Payment provider ports must not perform I/O here.
pub mod ports {
    /// Future payment-provider port namespace (no Xendit/DOKU/Midtrans in Phase-1D).
    pub mod payment {}

    /// Future persistence port namespace (no business repositories in Phase-1D).
    pub mod persistence {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use tx4_domain::{OperationId, TenantId, TransactionId};

    #[test]
    fn crate_name_is_tx4_application() {
        assert_eq!(crate_name(), "tx4-application");
    }

    #[test]
    fn depends_on_domain() {
        assert_eq!(tx4_domain::crate_name(), "tx4-domain");
    }

    #[test]
    fn error_variants_cover_foundation_vocabulary() {
        let cases = [
            ApplicationError::validation("bad"),
            ApplicationError::business_rejection("rule"),
            ApplicationError::conflict("stale"),
            ApplicationError::idempotency_conflict("dup"),
            ApplicationError::uncertainty("provider timeout disposition"),
            ApplicationError::retryable("db blip"),
            ApplicationError::permanent_internal("bug"),
            ApplicationError::timeout("upstream timed out"),
        ];
        assert_eq!(cases.len(), 8);
    }

    #[test]
    fn timeout_is_not_terminal_business_failure() {
        let timeout = ApplicationError::timeout("provider call timed out");
        assert!(!timeout.is_terminal_business_failure());
        assert!(!timeout.implies_transaction_failed());
        assert!(timeout.is_uncertainty_or_timeout());

        let uncertainty = ApplicationError::uncertainty("unknown provider disposition");
        assert!(!uncertainty.is_terminal_business_failure());
        assert!(!uncertainty.implies_transaction_failed());

        let rejected = ApplicationError::business_rejection("forbidden transition");
        assert!(rejected.is_terminal_business_failure());
    }

    #[test]
    fn request_context_carries_optional_correlation_fields() {
        let ctx = RequestContext::new()
            .with_request_id("req-1")
            .with_correlation_id("corr-1")
            .with_tenant_id(TenantId::new("t-1").unwrap())
            .with_operation_id(OperationId::new("op-1").unwrap())
            .with_transaction_id(TransactionId::new("tx-1").unwrap())
            .with_provider_id("mock");
        assert_eq!(ctx.request_id.as_deref(), Some("req-1"));
        assert_eq!(ctx.provider_id.as_deref(), Some("mock"));
        assert_eq!(ctx.tenant_id.as_ref().map(TenantId::as_str), Some("t-1"));
    }
}
