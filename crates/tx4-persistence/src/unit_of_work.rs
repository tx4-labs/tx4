//! Composable PostgreSQL unit of work (IA §9.2 atomicity).

use sqlx::{Postgres, Transaction};
use tx4_application::{
    ApplicationError, IdempotencyBeginOutcome, IdempotencyBeginRequest, IdempotencyFinalize,
    IdempotencyReservation, IdempotencyStatus, PaymentAttempt, PaymentAttemptStatus, UnitOfWork,
    UnitOfWorkFactory,
};
use tx4_domain::{OperationId, TenantId, TransactionId};

use crate::sql_support::{map_sqlx_err, ts_unix_micros_expr};

/// Factory over a shared [`sqlx::PgPool`].
#[derive(Clone, Debug)]
pub struct PgUnitOfWorkFactory {
    pool: sqlx::PgPool,
}

impl PgUnitOfWorkFactory {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

impl UnitOfWorkFactory for PgUnitOfWorkFactory {
    type Uow<'a>
        = PgUnitOfWork<'a>
    where
        Self: 'a;

    async fn begin(&self) -> Result<PgUnitOfWork<'_>, ApplicationError> {
        let tx = self.pool.begin().await.map_err(map_sqlx_err)?;
        Ok(PgUnitOfWork { tx })
    }
}

/// Open PostgreSQL transaction. Drop rolls back unless [`UnitOfWork::commit`] succeeds.
pub struct PgUnitOfWork<'c> {
    tx: Transaction<'c, Postgres>,
}

impl UnitOfWork for PgUnitOfWork<'_> {
    async fn begin_or_recover(
        &mut self,
        req: &IdempotencyBeginRequest<'_>,
    ) -> Result<IdempotencyBeginOutcome, ApplicationError> {
        begin_or_recover_tx(&mut self.tx, req).await
    }

    async fn insert_prepared_attempt(
        &mut self,
        attempt: &PaymentAttempt,
        now_unix_micros: i64,
    ) -> Result<(), ApplicationError> {
        insert_prepared_on(&mut self.tx, attempt, now_unix_micros).await
    }

    async fn complete_idempotency(
        &mut self,
        tenant_id: &TenantId,
        idempotency_key: &str,
        lease_owner: &str,
        finalize: &IdempotencyFinalize,
        now_unix_micros: i64,
    ) -> Result<IdempotencyReservation, ApplicationError> {
        let (status, response_status, response_body) = match finalize {
            IdempotencyFinalize::Completed {
                response_status,
                response_body,
            } => (
                IdempotencyStatus::Completed,
                response_status.as_str(),
                response_body.as_str(),
            ),
            IdempotencyFinalize::FailedClosed {
                response_status,
                response_body,
            } => (
                IdempotencyStatus::FailedClosed,
                response_status.as_str(),
                response_body.as_str(),
            ),
        };
        finalize_idempotency_on(
            &mut self.tx,
            tenant_id,
            idempotency_key,
            lease_owner,
            status,
            response_status,
            response_body,
            now_unix_micros,
        )
        .await
    }

    async fn find_attempt_by_operation(
        &mut self,
        tenant_id: &TenantId,
        operation_id: &OperationId,
    ) -> Result<Option<PaymentAttempt>, ApplicationError> {
        find_attempt_by_operation_on(&mut self.tx, tenant_id, operation_id).await
    }

    async fn find_idempotency(
        &mut self,
        tenant_id: &TenantId,
        idempotency_key: &str,
    ) -> Result<Option<IdempotencyReservation>, ApplicationError> {
        find_idempotency_on(&mut self.tx, tenant_id, idempotency_key).await
    }

    async fn commit(self) -> Result<(), ApplicationError> {
        self.tx.commit().await.map_err(map_sqlx_err)
    }
}

// ---------------------------------------------------------------------------
// Shared SQL used by UoW and standalone repositories
// ---------------------------------------------------------------------------

fn idemp_select_cols() -> String {
    format!(
        "tenant_id, idempotency_key, operation_id, request_fingerprint,
         transaction_id, status, lease_owner,
         {lease} AS lease_expires_at_unix_micros,
         response_status, response_body",
        lease = ts_unix_micros_expr("lease_expires_at")
    )
}

fn attempt_select_cols() -> &'static str {
    "attempt_id, tenant_id, transaction_id, operation_id, idempotency_key,
     provider_adapter, provider_ref, provider_idempotency_key, status, outcome_disposition,
     amount_atomic, currency_id"
}

#[derive(Debug, sqlx::FromRow)]
struct IdempotencyRow {
    tenant_id: String,
    idempotency_key: String,
    operation_id: String,
    request_fingerprint: String,
    transaction_id: Option<String>,
    status: String,
    lease_owner: Option<String>,
    lease_expires_at_unix_micros: Option<i64>,
    response_status: Option<String>,
    response_body: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct AttemptRow {
    attempt_id: String,
    tenant_id: String,
    transaction_id: String,
    operation_id: String,
    idempotency_key: Option<String>,
    provider_adapter: String,
    provider_ref: Option<String>,
    provider_idempotency_key: Option<String>,
    status: String,
    outcome_disposition: Option<String>,
    amount_atomic: Option<i64>,
    currency_id: Option<String>,
}

fn idemp_row_to_domain(row: IdempotencyRow) -> Result<IdempotencyReservation, ApplicationError> {
    Ok(IdempotencyReservation {
        tenant_id: TenantId::new(row.tenant_id)
            .map_err(|e| ApplicationError::validation(e.to_string()))?,
        idempotency_key: row.idempotency_key,
        operation_id: OperationId::new(row.operation_id)
            .map_err(|e| ApplicationError::validation(e.to_string()))?,
        request_fingerprint: row.request_fingerprint,
        transaction_id: match row.transaction_id {
            None => None,
            Some(id) => Some(
                TransactionId::new(id).map_err(|e| ApplicationError::validation(e.to_string()))?,
            ),
        },
        status: IdempotencyStatus::parse(&row.status)?,
        lease_owner: row.lease_owner,
        lease_expires_at_unix_micros: row.lease_expires_at_unix_micros,
        response_status: row.response_status,
        response_body: row.response_body,
    })
}

fn attempt_row_to_domain(row: AttemptRow) -> Result<PaymentAttempt, ApplicationError> {
    Ok(PaymentAttempt {
        attempt_id: row.attempt_id,
        tenant_id: TenantId::new(row.tenant_id)
            .map_err(|e| ApplicationError::validation(e.to_string()))?,
        transaction_id: TransactionId::new(row.transaction_id)
            .map_err(|e| ApplicationError::validation(e.to_string()))?,
        operation_id: OperationId::new(row.operation_id)
            .map_err(|e| ApplicationError::validation(e.to_string()))?,
        idempotency_key: row.idempotency_key,
        provider_adapter: row.provider_adapter,
        provider_ref: row.provider_ref,
        provider_idempotency_key: row.provider_idempotency_key,
        status: PaymentAttemptStatus::parse(&row.status)?,
        outcome_disposition: row.outcome_disposition,
        amount_atomic: row.amount_atomic,
        currency_id: row.currency_id,
    })
}

pub(crate) async fn begin_or_recover_tx(
    tx: &mut Transaction<'_, Postgres>,
    req: &IdempotencyBeginRequest<'_>,
) -> Result<IdempotencyBeginOutcome, ApplicationError> {
    let cols = idemp_select_cols();
    let tenant_id = req.tenant_id;
    let idempotency_key = req.idempotency_key;
    let lease_duration_secs = req.lease_duration_secs;
    let now_unix_micros = req.now_unix_micros;
    if lease_duration_secs < 1 {
        return Err(ApplicationError::validation(
            "lease_duration_secs must be >= 1",
        ));
    }
    let lease_expires = now_unix_micros
        .checked_add(lease_duration_secs.saturating_mul(1_000_000))
        .ok_or_else(|| ApplicationError::permanent_internal("lease expiry overflow"))?;

    let inserted = sqlx::query_as::<_, IdempotencyRow>(&format!(
        "INSERT INTO tx4_infra.idempotency_records (
            tenant_id, idempotency_key, operation_id, request_fingerprint,
            transaction_id, status, lease_owner, lease_expires_at, created_at, updated_at
         ) VALUES (
            $1, $2, $3, $4, $5, 'IN_PROGRESS', $6,
            TIMESTAMPTZ 'epoch' + (($7::bigint) * INTERVAL '1 microsecond'),
            TIMESTAMPTZ 'epoch' + (($8::bigint) * INTERVAL '1 microsecond'),
            TIMESTAMPTZ 'epoch' + (($8::bigint) * INTERVAL '1 microsecond')
         )
         ON CONFLICT (tenant_id, idempotency_key) DO NOTHING
         RETURNING {cols}"
    ))
    .bind(tenant_id.as_str())
    .bind(idempotency_key)
    .bind(req.operation_id.as_str())
    .bind(req.request_fingerprint)
    .bind(req.transaction_id.map(TransactionId::as_str))
    .bind(req.lease_owner)
    .bind(lease_expires)
    .bind(now_unix_micros)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_err)?;

    if let Some(row) = inserted {
        return Ok(IdempotencyBeginOutcome::Acquired(idemp_row_to_domain(row)?));
    }

    let row = sqlx::query_as::<_, IdempotencyRow>(&format!(
        "SELECT {cols}
         FROM tx4_infra.idempotency_records
         WHERE tenant_id = $1 AND idempotency_key = $2
         FOR UPDATE"
    ))
    .bind(tenant_id.as_str())
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_err)?
    .ok_or_else(|| ApplicationError::not_found("idempotency reservation not found"))?;

    let current = idemp_row_to_domain(row)?;
    if current.request_fingerprint != req.request_fingerprint {
        return Err(ApplicationError::idempotency_conflict(
            "idempotency key reused with different request fingerprint",
        ));
    }

    match current.status {
        IdempotencyStatus::Completed | IdempotencyStatus::FailedClosed => {
            Ok(IdempotencyBeginOutcome::Replay(current))
        }
        IdempotencyStatus::InProgress => {
            let expired = current
                .lease_expires_at_unix_micros
                .map(|exp| exp < now_unix_micros)
                .unwrap_or(true);
            if !expired {
                return Ok(IdempotencyBeginOutcome::InProgress(current));
            }

            let updated = sqlx::query_as::<_, IdempotencyRow>(&format!(
                "UPDATE tx4_infra.idempotency_records SET
                    lease_owner = $3,
                    lease_expires_at = TIMESTAMPTZ 'epoch' + (($4::bigint) * INTERVAL '1 microsecond'),
                    updated_at = TIMESTAMPTZ 'epoch' + (($5::bigint) * INTERVAL '1 microsecond')
                 WHERE tenant_id = $1 AND idempotency_key = $2 AND status = 'IN_PROGRESS'
                 RETURNING {cols}"
            ))
            .bind(tenant_id.as_str())
            .bind(idempotency_key)
            .bind(req.lease_owner)
            .bind(lease_expires)
            .bind(now_unix_micros)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_sqlx_err)?;
            let reservation = idemp_row_to_domain(updated)?;

            // IA §9.2(7)/§12.4.2: inspect bound PaymentAttempt before authorizing provider I/O.
            if let Some(attempt) =
                find_attempt_by_operation_on(tx, tenant_id, &reservation.operation_id).await?
            {
                if attempt.status.requires_reconcile_before_provider_io() {
                    return Ok(IdempotencyBeginOutcome::ReclaimedRequiresReconcile {
                        reservation,
                        attempt: Box::new(attempt),
                    });
                }
            }
            Ok(IdempotencyBeginOutcome::Acquired(reservation))
        }
    }
}

pub(crate) async fn insert_prepared_on(
    tx: &mut Transaction<'_, Postgres>,
    attempt: &PaymentAttempt,
    now_unix_micros: i64,
) -> Result<(), ApplicationError> {
    if attempt.status != PaymentAttemptStatus::Prepared {
        return Err(ApplicationError::validation(
            "insert_prepared requires PREPARED status",
        ));
    }
    let result = sqlx::query(
        "INSERT INTO tx4_infra.payment_attempts (
            attempt_id, tenant_id, transaction_id, operation_id, idempotency_key,
            provider_adapter, provider_ref, provider_idempotency_key, status,
            outcome_disposition, amount_atomic, currency_id, created_at, updated_at
         ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, 'PREPARED', $9, $10, $11,
            TIMESTAMPTZ 'epoch' + (($12::bigint) * INTERVAL '1 microsecond'),
            TIMESTAMPTZ 'epoch' + (($12::bigint) * INTERVAL '1 microsecond')
         )",
    )
    .bind(&attempt.attempt_id)
    .bind(attempt.tenant_id.as_str())
    .bind(attempt.transaction_id.as_str())
    .bind(attempt.operation_id.as_str())
    .bind(attempt.idempotency_key.as_deref())
    .bind(&attempt.provider_adapter)
    .bind(attempt.provider_ref.as_deref())
    .bind(attempt.provider_idempotency_key.as_deref())
    .bind(attempt.outcome_disposition.as_deref())
    .bind(attempt.amount_atomic)
    .bind(attempt.currency_id.as_deref())
    .bind(now_unix_micros)
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_err)?;
    if result.rows_affected() != 1 {
        return Err(ApplicationError::permanent_internal(
            "payment attempt insert affected unexpected row count",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn finalize_idempotency_on(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &TenantId,
    idempotency_key: &str,
    lease_owner: &str,
    status: IdempotencyStatus,
    response_status: &str,
    response_body: &str,
    now_unix_micros: i64,
) -> Result<IdempotencyReservation, ApplicationError> {
    let cols = idemp_select_cols();
    let current = find_idempotency_for_update(tx, tenant_id, idempotency_key).await?;
    if current.status != IdempotencyStatus::InProgress {
        return Err(ApplicationError::conflict(
            "idempotency reservation is not IN_PROGRESS",
        ));
    }
    if current.lease_owner.as_deref() != Some(lease_owner) {
        return Err(ApplicationError::conflict(
            "idempotency lease owner mismatch",
        ));
    }
    let row = sqlx::query_as::<_, IdempotencyRow>(&format!(
        "UPDATE tx4_infra.idempotency_records SET
            status = $3,
            response_status = $4,
            response_body = $5,
            lease_owner = NULL,
            lease_expires_at = NULL,
            updated_at = TIMESTAMPTZ 'epoch' + (($6::bigint) * INTERVAL '1 microsecond')
         WHERE tenant_id = $1 AND idempotency_key = $2 AND status = 'IN_PROGRESS'
         RETURNING {cols}"
    ))
    .bind(tenant_id.as_str())
    .bind(idempotency_key)
    .bind(status.as_str())
    .bind(response_status)
    .bind(response_body)
    .bind(now_unix_micros)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_err)?
    .ok_or_else(|| ApplicationError::conflict("idempotency finalize lost race"))?;
    idemp_row_to_domain(row)
}

async fn find_idempotency_for_update(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &TenantId,
    idempotency_key: &str,
) -> Result<IdempotencyReservation, ApplicationError> {
    let cols = idemp_select_cols();
    let row = sqlx::query_as::<_, IdempotencyRow>(&format!(
        "SELECT {cols}
         FROM tx4_infra.idempotency_records
         WHERE tenant_id = $1 AND idempotency_key = $2
         FOR UPDATE"
    ))
    .bind(tenant_id.as_str())
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_err)?
    .ok_or_else(|| ApplicationError::not_found("idempotency reservation not found"))?;
    idemp_row_to_domain(row)
}

pub(crate) async fn find_idempotency_on(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &TenantId,
    idempotency_key: &str,
) -> Result<Option<IdempotencyReservation>, ApplicationError> {
    let cols = idemp_select_cols();
    let row = sqlx::query_as::<_, IdempotencyRow>(&format!(
        "SELECT {cols}
         FROM tx4_infra.idempotency_records
         WHERE tenant_id = $1 AND idempotency_key = $2"
    ))
    .bind(tenant_id.as_str())
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_err)?;
    match row {
        None => Ok(None),
        Some(r) => Ok(Some(idemp_row_to_domain(r)?)),
    }
}

pub(crate) async fn find_attempt_by_operation_on(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &TenantId,
    operation_id: &OperationId,
) -> Result<Option<PaymentAttempt>, ApplicationError> {
    let row = sqlx::query_as::<_, AttemptRow>(&format!(
        "SELECT {cols} FROM tx4_infra.payment_attempts
         WHERE tenant_id = $1 AND operation_id = $2",
        cols = attempt_select_cols()
    ))
    .bind(tenant_id.as_str())
    .bind(operation_id.as_str())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_err)?;
    match row {
        None => Ok(None),
        Some(r) => Ok(Some(attempt_row_to_domain(r)?)),
    }
}
