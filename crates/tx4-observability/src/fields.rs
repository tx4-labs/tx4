//! Canonical correlation / identity field names for structured telemetry.

/// HTTP / ingress request identifier.
pub const REQUEST_ID: &str = "request_id";
/// Cross-service correlation identifier.
pub const CORRELATION_ID: &str = "correlation_id";
/// Logical operation identity (ADR-007), when present.
pub const OPERATION_ID: &str = "operation_id";
/// Tenant identity, when present.
pub const TENANT_ID: &str = "tenant_id";
/// Transaction identity, when present.
pub const TRANSACTION_ID: &str = "transaction_id";
/// Payment provider adapter identity, when present.
pub const PROVIDER_ID: &str = "provider_id";
