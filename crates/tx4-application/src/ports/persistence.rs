//! Persistence ports (IA §5.2) — Phase-2B transaction repository.

use tx4_domain::{
    LifecycleCommand, LifecycleContext, TenantId, Transaction, TransactionId, Version,
};

use crate::ApplicationError;

/// Durable transaction aggregate repository (no SQLx types in the port).
pub trait TransactionRepository: Send + Sync {
    /// Insert a new aggregate. Fails if `(tenant_id, id)` already exists.
    fn insert(
        &self,
        tx: &Transaction,
    ) -> impl std::future::Future<Output = Result<(), ApplicationError>> + Send;

    /// Load by tenant ownership + identity. Returns `Ok(None)` when absent.
    fn find(
        &self,
        tenant_id: &TenantId,
        id: &TransactionId,
    ) -> impl std::future::Future<Output = Result<Option<Transaction>, ApplicationError>> + Send;

    /// Mutate under `SELECT … FOR UPDATE` with expected-version check and domain lifecycle evaluation.
    ///
    /// `updated_at_unix_micros` is an explicit clock input (no internal wall clock).
    fn apply_transition(
        &self,
        tenant_id: &TenantId,
        id: &TransactionId,
        expected_version: Version,
        command: &LifecycleCommand,
        ctx: &LifecycleContext,
        updated_at_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<Transaction, ApplicationError>> + Send;
}
