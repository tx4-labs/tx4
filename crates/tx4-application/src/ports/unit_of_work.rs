//! Application unit-of-work / atomic persistence ports (IA §9.2).

use tx4_domain::{OperationId, TenantId};

use crate::{
    ApplicationError, IdempotencyBeginOutcome, IdempotencyBeginRequest, IdempotencyReservation,
    PaymentAttempt,
};

/// Finalization payload for idempotency stored response (IA §9.2(8)).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdempotencyFinalize {
    Completed {
        response_status: String,
        response_body: String,
    },
    FailedClosed {
        response_status: String,
        response_body: String,
    },
}

/// Factory for a composable PostgreSQL unit of work (no independent nested commits).
pub trait UnitOfWorkFactory: Send + Sync {
    type Uow<'a>: UnitOfWork
    where
        Self: 'a;

    fn begin(
        &self,
    ) -> impl std::future::Future<Output = Result<Self::Uow<'_>, ApplicationError>> + Send;
}

/// Shared transaction context for atomic Phase-2C workflows.
///
/// Drop without [`UnitOfWork::commit`] rolls back. Provider I/O must not run while this is open.
pub trait UnitOfWork: Send {
    fn begin_or_recover(
        &mut self,
        req: &IdempotencyBeginRequest<'_>,
    ) -> impl std::future::Future<Output = Result<IdempotencyBeginOutcome, ApplicationError>> + Send;

    fn insert_prepared_attempt(
        &mut self,
        attempt: &PaymentAttempt,
        now_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<(), ApplicationError>> + Send;

    fn complete_idempotency(
        &mut self,
        tenant_id: &TenantId,
        idempotency_key: &str,
        lease_owner: &str,
        finalize: &IdempotencyFinalize,
        now_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<IdempotencyReservation, ApplicationError>> + Send;

    fn find_attempt_by_operation(
        &mut self,
        tenant_id: &TenantId,
        operation_id: &OperationId,
    ) -> impl std::future::Future<Output = Result<Option<PaymentAttempt>, ApplicationError>> + Send;

    fn find_idempotency(
        &mut self,
        tenant_id: &TenantId,
        idempotency_key: &str,
    ) -> impl std::future::Future<Output = Result<Option<IdempotencyReservation>, ApplicationError>> + Send;

    fn commit(self) -> impl std::future::Future<Output = Result<(), ApplicationError>> + Send;
}
