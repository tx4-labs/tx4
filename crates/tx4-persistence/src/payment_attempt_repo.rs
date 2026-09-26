//! PostgreSQL PaymentAttempt repository (IA §12.4).

use sqlx::PgPool;
use tx4_application::{
    ApplicationError, PaymentAttempt, PaymentAttemptRepository, PaymentAttemptStatus,
};
use tx4_domain::{OperationId, TenantId, TransactionId};

use crate::sql_support::{map_sqlx_err, sleep_backoff, MAX_DB_RETRIES};

/// SQLx-backed durable PaymentAttempt store.
#[derive(Clone, Debug)]
pub struct PgPaymentAttemptRepository {
    pool: PgPool,
}

impl PgPaymentAttemptRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl PaymentAttemptRepository for PgPaymentAttemptRepository {
    async fn insert_prepared(
        &self,
        attempt: &PaymentAttempt,
        now_unix_micros: i64,
    ) -> Result<(), ApplicationError> {
        with_retry(|| insert_once(&self.pool, attempt, now_unix_micros)).await
    }

    async fn mark_submitted(
        &self,
        tenant_id: &TenantId,
        attempt_id: &str,
        provider_idempotency_key: &str,
        now_unix_micros: i64,
    ) -> Result<PaymentAttempt, ApplicationError> {
        with_retry(|| {
            transition_once(
                &self.pool,
                tenant_id,
                attempt_id,
                PaymentAttemptStatus::Prepared,
                PaymentAttemptStatus::Submitted,
                Some(provider_idempotency_key),
                None,
                None,
                now_unix_micros,
            )
        })
        .await
    }

    async fn mark_unknown(
        &self,
        tenant_id: &TenantId,
        attempt_id: &str,
        disposition: &str,
        now_unix_micros: i64,
    ) -> Result<PaymentAttempt, ApplicationError> {
        with_retry(|| {
            set_outcome_once(
                &self.pool,
                tenant_id,
                attempt_id,
                PaymentAttemptStatus::Unknown,
                disposition,
                None,
                now_unix_micros,
            )
        })
        .await
    }

    async fn mark_succeeded(
        &self,
        tenant_id: &TenantId,
        attempt_id: &str,
        provider_ref: &str,
        now_unix_micros: i64,
    ) -> Result<PaymentAttempt, ApplicationError> {
        with_retry(|| {
            set_outcome_once(
                &self.pool,
                tenant_id,
                attempt_id,
                PaymentAttemptStatus::Succeeded,
                "SUCCEEDED",
                Some(provider_ref),
                now_unix_micros,
            )
        })
        .await
    }

    async fn mark_failed(
        &self,
        tenant_id: &TenantId,
        attempt_id: &str,
        disposition: &str,
        now_unix_micros: i64,
    ) -> Result<PaymentAttempt, ApplicationError> {
        with_retry(|| {
            set_outcome_once(
                &self.pool,
                tenant_id,
                attempt_id,
                PaymentAttemptStatus::Failed,
                disposition,
                None,
                now_unix_micros,
            )
        })
        .await
    }

    async fn find(
        &self,
        tenant_id: &TenantId,
        attempt_id: &str,
    ) -> Result<Option<PaymentAttempt>, ApplicationError> {
        with_retry(|| find_once(&self.pool, tenant_id, Some(attempt_id), None)).await
    }

    async fn find_by_operation(
        &self,
        tenant_id: &TenantId,
        operation_id: &OperationId,
    ) -> Result<Option<PaymentAttempt>, ApplicationError> {
        with_retry(|| find_once(&self.pool, tenant_id, None, Some(operation_id))).await
    }
}

async fn with_retry<T, F, Fut>(mut f: F) -> Result<T, ApplicationError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, ApplicationError>>,
{
    let mut attempt = 0u32;
    loop {
        match f().await {
            Ok(v) => return Ok(v),
            Err(err) if err.is_retryable_infrastructure() && attempt + 1 < MAX_DB_RETRIES => {
                attempt += 1;
                sleep_backoff(attempt).await;
            }
            Err(err) => return Err(err),
        }
    }
}

const SELECT_COLS: &str = "attempt_id, tenant_id, transaction_id, operation_id, idempotency_key,
    provider_adapter, provider_ref, provider_idempotency_key, status, outcome_disposition,
    amount_atomic, currency_id";

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

fn row_to_domain(row: AttemptRow) -> Result<PaymentAttempt, ApplicationError> {
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

async fn insert_once(
    pool: &PgPool,
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
    .execute(pool)
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
async fn transition_once(
    pool: &PgPool,
    tenant_id: &TenantId,
    attempt_id: &str,
    from: PaymentAttemptStatus,
    to: PaymentAttemptStatus,
    provider_idempotency_key: Option<&str>,
    disposition: Option<&str>,
    provider_ref: Option<&str>,
    now_unix_micros: i64,
) -> Result<PaymentAttempt, ApplicationError> {
    let key = provider_idempotency_key.ok_or_else(|| {
        ApplicationError::validation("provider_idempotency_key required for SUBMITTED")
    })?;
    let row = sqlx::query_as::<_, AttemptRow>(&format!(
        "UPDATE tx4_infra.payment_attempts SET
            status = $4,
            provider_idempotency_key = $5,
            outcome_disposition = COALESCE($6, outcome_disposition),
            provider_ref = COALESCE($7, provider_ref),
            updated_at = TIMESTAMPTZ 'epoch' + (($8::bigint) * INTERVAL '1 microsecond')
         WHERE tenant_id = $1 AND attempt_id = $2 AND status = $3
         RETURNING {SELECT_COLS}"
    ))
    .bind(tenant_id.as_str())
    .bind(attempt_id)
    .bind(from.as_str())
    .bind(to.as_str())
    .bind(key)
    .bind(disposition)
    .bind(provider_ref)
    .bind(now_unix_micros)
    .fetch_optional(pool)
    .await
    .map_err(map_sqlx_err)?
    .ok_or_else(|| {
        ApplicationError::conflict("payment attempt status transition conflict or not found")
    })?;
    row_to_domain(row)
}

async fn set_outcome_once(
    pool: &PgPool,
    tenant_id: &TenantId,
    attempt_id: &str,
    status: PaymentAttemptStatus,
    disposition: &str,
    provider_ref: Option<&str>,
    now_unix_micros: i64,
) -> Result<PaymentAttempt, ApplicationError> {
    // Outcomes may only follow SUBMITTED or UNKNOWN (not PREPARED before I/O boundary).
    let row = sqlx::query_as::<_, AttemptRow>(&format!(
        "UPDATE tx4_infra.payment_attempts SET
            status = $3,
            outcome_disposition = $4,
            provider_ref = COALESCE($5, provider_ref),
            updated_at = TIMESTAMPTZ 'epoch' + (($6::bigint) * INTERVAL '1 microsecond')
         WHERE tenant_id = $1 AND attempt_id = $2
           AND status IN ('SUBMITTED', 'UNKNOWN')
         RETURNING {SELECT_COLS}"
    ))
    .bind(tenant_id.as_str())
    .bind(attempt_id)
    .bind(status.as_str())
    .bind(disposition)
    .bind(provider_ref)
    .bind(now_unix_micros)
    .fetch_optional(pool)
    .await
    .map_err(map_sqlx_err)?
    .ok_or_else(|| {
        ApplicationError::conflict("payment attempt outcome update conflict or invalid status")
    })?;
    row_to_domain(row)
}

async fn find_once(
    pool: &PgPool,
    tenant_id: &TenantId,
    attempt_id: Option<&str>,
    operation_id: Option<&OperationId>,
) -> Result<Option<PaymentAttempt>, ApplicationError> {
    let row = if let Some(aid) = attempt_id {
        sqlx::query_as::<_, AttemptRow>(&format!(
            "SELECT {SELECT_COLS} FROM tx4_infra.payment_attempts
             WHERE tenant_id = $1 AND attempt_id = $2"
        ))
        .bind(tenant_id.as_str())
        .bind(aid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
    } else if let Some(oid) = operation_id {
        sqlx::query_as::<_, AttemptRow>(&format!(
            "SELECT {SELECT_COLS} FROM tx4_infra.payment_attempts
             WHERE tenant_id = $1 AND operation_id = $2"
        ))
        .bind(tenant_id.as_str())
        .bind(oid.as_str())
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
    } else {
        return Err(ApplicationError::validation(
            "find requires attempt_id or operation_id",
        ));
    };
    match row {
        None => Ok(None),
        Some(r) => Ok(Some(row_to_domain(r)?)),
    }
}
