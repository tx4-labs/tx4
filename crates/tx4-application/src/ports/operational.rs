//! Phase-2C operational ports: idempotency, outbox, PaymentAttempt (IA §9 / §11 / §12).

use tx4_domain::{OperationId, TenantId, TransactionId};

use crate::ApplicationError;

// ---------------------------------------------------------------------------
// Idempotency (IA §9)
// ---------------------------------------------------------------------------

/// Durable idempotency reservation status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdempotencyStatus {
    InProgress,
    Completed,
    FailedClosed,
}

impl IdempotencyStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InProgress => "IN_PROGRESS",
            Self::Completed => "COMPLETED",
            Self::FailedClosed => "FAILED_CLOSED",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, ApplicationError> {
        match raw {
            "IN_PROGRESS" => Ok(Self::InProgress),
            "COMPLETED" => Ok(Self::Completed),
            "FAILED_CLOSED" => Ok(Self::FailedClosed),
            _ => Err(ApplicationError::permanent_internal(
                "corrupt idempotency status",
            )),
        }
    }
}

/// Durable idempotency reservation aggregate view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdempotencyReservation {
    pub tenant_id: TenantId,
    pub idempotency_key: String,
    pub operation_id: OperationId,
    pub request_fingerprint: String,
    pub transaction_id: Option<TransactionId>,
    pub status: IdempotencyStatus,
    pub lease_owner: Option<String>,
    pub lease_expires_at_unix_micros: Option<i64>,
    pub response_status: Option<String>,
    pub response_body: Option<String>,
}

/// Outcome of begin/reclaim against an idempotency key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdempotencyBeginOutcome {
    /// Caller holds the lease and may execute the command.
    Acquired(IdempotencyReservation),
    /// Active lease held by another executor; do not execute.
    InProgress(IdempotencyReservation),
    /// Finalized reservation; replay stored response (no new financial effect).
    Replay(IdempotencyReservation),
}

/// Inputs for beginning or reclaiming an idempotency reservation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdempotencyBeginRequest<'a> {
    pub tenant_id: &'a TenantId,
    pub idempotency_key: &'a str,
    pub operation_id: &'a OperationId,
    pub request_fingerprint: &'a str,
    pub transaction_id: Option<&'a TransactionId>,
    pub lease_owner: &'a str,
    pub lease_duration_secs: i64,
    pub now_unix_micros: i64,
}

/// Durable idempotency reservation store.
pub trait IdempotencyRepository: Send + Sync {
    /// Begin or recover a reservation under `UNIQUE (tenant_id, idempotency_key)`.
    ///
    /// Concurrent first writers: one insert wins; losers re-read under lock.
    fn begin_or_recover(
        &self,
        req: &IdempotencyBeginRequest<'_>,
    ) -> impl std::future::Future<Output = Result<IdempotencyBeginOutcome, ApplicationError>> + Send;

    /// Finalize success with stored replay payload (same DB txn as business finalize when required).
    fn complete(
        &self,
        tenant_id: &TenantId,
        idempotency_key: &str,
        lease_owner: &str,
        response_status: &str,
        response_body: &str,
        now_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<IdempotencyReservation, ApplicationError>> + Send;

    /// Finalize permanent business failure (`FAILED_CLOSED`). Not for provider timeout.
    fn fail_closed(
        &self,
        tenant_id: &TenantId,
        idempotency_key: &str,
        lease_owner: &str,
        response_status: &str,
        response_body: &str,
        now_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<IdempotencyReservation, ApplicationError>> + Send;

    fn find(
        &self,
        tenant_id: &TenantId,
        idempotency_key: &str,
    ) -> impl std::future::Future<Output = Result<Option<IdempotencyReservation>, ApplicationError>> + Send;
}

// ---------------------------------------------------------------------------
// Outbox (IA §11)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutboxStatus {
    Pending,
    Running,
    Succeeded,
    DeadLetter,
}

impl OutboxStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Running => "RUNNING",
            Self::Succeeded => "SUCCEEDED",
            Self::DeadLetter => "DEAD_LETTER",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, ApplicationError> {
        match raw {
            "PENDING" => Ok(Self::Pending),
            "RUNNING" => Ok(Self::Running),
            "SUCCEEDED" => Ok(Self::Succeeded),
            "DEAD_LETTER" => Ok(Self::DeadLetter),
            _ => Err(ApplicationError::permanent_internal(
                "corrupt outbox status",
            )),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutboxJob {
    pub id: String,
    pub tenant_id: TenantId,
    pub job_type: String,
    pub payload: String,
    pub status: OutboxStatus,
    pub locked_by: Option<String>,
    pub claim_epoch: u64,
    pub lease_expires_at_unix_micros: Option<i64>,
    pub attempt_count: i32,
    pub next_attempt_at_unix_micros: i64,
    pub last_error: Option<String>,
}

/// Claimed job with fencing identity held by this worker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimedOutboxJob {
    pub job: OutboxJob,
}

/// Durable transactional outbox store.
pub trait OutboxRepository: Send + Sync {
    fn enqueue(
        &self,
        job: &OutboxJob,
    ) -> impl std::future::Future<Output = Result<(), ApplicationError>> + Send;

    fn claim(
        &self,
        worker_id: &str,
        limit: i64,
        lease_duration_secs: i64,
        now_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<Vec<ClaimedOutboxJob>, ApplicationError>> + Send;

    /// Conditional success under fencing. Returns `Ok(false)` for stale worker (no-op).
    fn complete_succeeded(
        &self,
        job_id: &str,
        worker_id: &str,
        claim_epoch: u64,
        now_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<bool, ApplicationError>> + Send;

    /// Conditional retry → PENDING with backoff under fencing.
    fn complete_retry(
        &self,
        job_id: &str,
        worker_id: &str,
        claim_epoch: u64,
        last_error: &str,
        next_attempt_at_unix_micros: i64,
        now_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<bool, ApplicationError>> + Send;

    /// Conditional DEAD_LETTER under fencing.
    fn complete_dead_letter(
        &self,
        job_id: &str,
        worker_id: &str,
        claim_epoch: u64,
        last_error: &str,
        now_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<bool, ApplicationError>> + Send;

    fn find(
        &self,
        job_id: &str,
    ) -> impl std::future::Future<Output = Result<Option<OutboxJob>, ApplicationError>> + Send;
}

// ---------------------------------------------------------------------------
// PaymentAttempt (IA §12.4)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaymentAttemptStatus {
    Prepared,
    Submitted,
    Unknown,
    Succeeded,
    Failed,
}

impl PaymentAttemptStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "PREPARED",
            Self::Submitted => "SUBMITTED",
            Self::Unknown => "UNKNOWN",
            Self::Succeeded => "SUCCEEDED",
            Self::Failed => "FAILED",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, ApplicationError> {
        match raw {
            "PREPARED" => Ok(Self::Prepared),
            "SUBMITTED" => Ok(Self::Submitted),
            "UNKNOWN" => Ok(Self::Unknown),
            "SUCCEEDED" => Ok(Self::Succeeded),
            "FAILED" => Ok(Self::Failed),
            _ => Err(ApplicationError::permanent_internal(
                "corrupt payment attempt status",
            )),
        }
    }

    /// Whether reclaim must reconcile/query before any new provider I/O (IA §12.4.2).
    pub fn requires_reconcile_before_provider_io(self) -> bool {
        matches!(
            self,
            Self::Submitted | Self::Unknown | Self::Succeeded | Self::Failed
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentAttempt {
    pub attempt_id: String,
    pub tenant_id: TenantId,
    pub transaction_id: TransactionId,
    pub operation_id: OperationId,
    pub idempotency_key: Option<String>,
    pub provider_adapter: String,
    pub provider_ref: Option<String>,
    pub provider_idempotency_key: Option<String>,
    pub status: PaymentAttemptStatus,
    pub outcome_disposition: Option<String>,
    pub amount_atomic: Option<i64>,
    pub currency_id: Option<String>,
}

pub trait PaymentAttemptRepository: Send + Sync {
    fn insert_prepared(
        &self,
        attempt: &PaymentAttempt,
        now_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<(), ApplicationError>> + Send;

    /// Commit `PREPARED → SUBMITTED` with stable provider_idempotency_key (IA §12.4.1).
    /// Must complete **before** any provider I/O.
    fn mark_submitted(
        &self,
        tenant_id: &TenantId,
        attempt_id: &str,
        provider_idempotency_key: &str,
        now_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<PaymentAttempt, ApplicationError>> + Send;

    fn mark_unknown(
        &self,
        tenant_id: &TenantId,
        attempt_id: &str,
        disposition: &str,
        now_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<PaymentAttempt, ApplicationError>> + Send;

    fn mark_succeeded(
        &self,
        tenant_id: &TenantId,
        attempt_id: &str,
        provider_ref: &str,
        now_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<PaymentAttempt, ApplicationError>> + Send;

    fn mark_failed(
        &self,
        tenant_id: &TenantId,
        attempt_id: &str,
        disposition: &str,
        now_unix_micros: i64,
    ) -> impl std::future::Future<Output = Result<PaymentAttempt, ApplicationError>> + Send;

    fn find(
        &self,
        tenant_id: &TenantId,
        attempt_id: &str,
    ) -> impl std::future::Future<Output = Result<Option<PaymentAttempt>, ApplicationError>> + Send;

    fn find_by_operation(
        &self,
        tenant_id: &TenantId,
        operation_id: &OperationId,
    ) -> impl std::future::Future<Output = Result<Option<PaymentAttempt>, ApplicationError>> + Send;
}
