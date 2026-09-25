//! Optional correlation / request context for future commands and telemetry.

use tx4_domain::{OperationId, TenantId, TransactionId};

/// Non-authoritative request/operation context carried across layers.
///
/// Presence of identifiers does not create durable business records.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RequestContext {
    pub request_id: Option<String>,
    pub correlation_id: Option<String>,
    pub operation_id: Option<OperationId>,
    pub tenant_id: Option<TenantId>,
    pub transaction_id: Option<TransactionId>,
    pub provider_id: Option<String>,
}

impl RequestContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn with_operation_id(mut self, id: OperationId) -> Self {
        self.operation_id = Some(id);
        self
    }

    pub fn with_tenant_id(mut self, id: TenantId) -> Self {
        self.tenant_id = Some(id);
        self
    }

    pub fn with_transaction_id(mut self, id: TransactionId) -> Self {
        self.transaction_id = Some(id);
        self
    }

    pub fn with_provider_id(mut self, id: impl Into<String>) -> Self {
        self.provider_id = Some(id.into());
        self
    }
}
