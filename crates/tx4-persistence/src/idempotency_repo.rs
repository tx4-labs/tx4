//! PostgreSQL idempotency reservation repository (IA §9).

use sqlx::{PgPool, Postgres, Transaction as DbTxn};
use tx4_application::{
    ApplicationError, IdempotencyBeginOutcome, IdempotencyBeginRequest, IdempotencyRepository,
    IdempotencyReservation, IdempotencyStatus,
};
use tx4_domain::{OperationId, TenantId, TransactionId};

use crate::sql_support::{map_sqlx_err, sleep_backoff, MAX_DB_RETRIES};

/// SQLx-backed durable idempotency reservations.
#[derive(Clone, Debug)]
pub struct PgIdempotencyRepository {
    pool: PgPool,
}

impl PgIdempotencyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl IdempotencyRepository for PgIdempotencyRepository {
    async fn begin_or_recover(
        &self,
        req: &IdempotencyBeginRequest<'_>,
    ) -> Result<IdempotencyBeginOutcome, ApplicationError> {
        let mut attempt = 0u32;
        loop {
            match begin_once(&self.pool, req).await {
                Ok(v) => return Ok(v),
                Err(err) if err.is_retryable_infrastructure() && attempt + 1 < MAX_DB_RETRIES => {
                    attempt += 1;
                    sleep_backoff(attempt).await;
                }
                Err(err) => return Err(err),
            }
        }
    }

    async fn complete(
        &self,
        tenant_id: &TenantId,
        idempotency_key: &str,
        lease_owner: &str,
        response_status: &str,
        response_body: &str,
        now_unix_micros: i64,
    ) -> Result<IdempotencyReservation, ApplicationError> {
        finalize(
            &self.pool,
            tenant_id,
            idempotency_key,
            lease_owner,
            IdempotencyStatus::Completed,
            response_status,
            response_body,
            now_unix_micros,
        )
        .await
    }

    async fn fail_closed(
        &self,
        tenant_id: &TenantId,
        idempotency_key: &str,
        lease_owner: &str,
        response_status: &str,
        response_body: &str,
        now_unix_micros: i64,
    ) -> Result<IdempotencyReservation, ApplicationError> {
        finalize(
            &self.pool,
            tenant_id,
            idempotency_key,
            lease_owner,
            IdempotencyStatus::FailedClosed,
            response_status,
            response_body,
            now_unix_micros,
        )
        .await
    }

    async fn find(
        &self,
        tenant_id: &TenantId,
        idempotency_key: &str,
    ) -> Result<Option<IdempotencyReservation>, ApplicationError> {
        let mut attempt = 0u32;
        loop {
            match find_once(&self.pool, tenant_id, idempotency_key).await {
                Ok(v) => return Ok(v),
                Err(err) if err.is_retryable_infrastructure() && attempt + 1 < MAX_DB_RETRIES => {
                    attempt += 1;
                    sleep_backoff(attempt).await;
                }
                Err(err) => return Err(err),
            }
        }
    }
}

const SELECT_COLS: &str = "tenant_id, idempotency_key, operation_id, request_fingerprint,
    transaction_id, status, lease_owner,
    (EXTRACT(EPOCH FROM lease_expires_at) * 1000000)::bigint AS lease_expires_at_unix_micros,
    response_status, response_body";

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

fn row_to_domain(row: IdempotencyRow) -> Result<IdempotencyReservation, ApplicationError> {
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

async fn begin_once(
    pool: &PgPool,
    req: &IdempotencyBeginRequest<'_>,
) -> Result<IdempotencyBeginOutcome, ApplicationError> {
    let tenant_id = req.tenant_id;
    let idempotency_key = req.idempotency_key;
    let operation_id = req.operation_id;
    let request_fingerprint = req.request_fingerprint;
    let transaction_id = req.transaction_id;
    let lease_owner = req.lease_owner;
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

    let mut db_tx = pool.begin().await.map_err(map_sqlx_err)?;

    // First-writer insert; unique losers re-read under FOR UPDATE.
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
         RETURNING {SELECT_COLS}"
    ))
    .bind(tenant_id.as_str())
    .bind(idempotency_key)
    .bind(operation_id.as_str())
    .bind(request_fingerprint)
    .bind(transaction_id.map(TransactionId::as_str))
    .bind(lease_owner)
    .bind(lease_expires)
    .bind(now_unix_micros)
    .fetch_optional(&mut *db_tx)
    .await
    .map_err(map_sqlx_err)?;

    if let Some(row) = inserted {
        db_tx.commit().await.map_err(map_sqlx_err)?;
        return Ok(IdempotencyBeginOutcome::Acquired(row_to_domain(row)?));
    }

    let row = lock_row(&mut db_tx, tenant_id, idempotency_key).await?;
    let current = row_to_domain(row)?;

    if current.request_fingerprint != request_fingerprint {
        return Err(ApplicationError::idempotency_conflict(
            "idempotency key reused with different request fingerprint",
        ));
    }

    match current.status {
        IdempotencyStatus::Completed | IdempotencyStatus::FailedClosed => {
            db_tx.commit().await.map_err(map_sqlx_err)?;
            Ok(IdempotencyBeginOutcome::Replay(current))
        }
        IdempotencyStatus::InProgress => {
            let expired = current
                .lease_expires_at_unix_micros
                .map(|exp| exp < now_unix_micros)
                .unwrap_or(true);
            if !expired {
                db_tx.commit().await.map_err(map_sqlx_err)?;
                return Ok(IdempotencyBeginOutcome::InProgress(current));
            }
            // Crash reclaim: refresh lease under lock (IA §9.2(5)).
            let updated = sqlx::query_as::<_, IdempotencyRow>(&format!(
                "UPDATE tx4_infra.idempotency_records SET
                    lease_owner = $3,
                    lease_expires_at = TIMESTAMPTZ 'epoch' + (($4::bigint) * INTERVAL '1 microsecond'),
                    updated_at = TIMESTAMPTZ 'epoch' + (($5::bigint) * INTERVAL '1 microsecond')
                 WHERE tenant_id = $1 AND idempotency_key = $2 AND status = 'IN_PROGRESS'
                 RETURNING {SELECT_COLS}"
            ))
            .bind(tenant_id.as_str())
            .bind(idempotency_key)
            .bind(lease_owner)
            .bind(lease_expires)
            .bind(now_unix_micros)
            .fetch_one(&mut *db_tx)
            .await
            .map_err(map_sqlx_err)?;
            db_tx.commit().await.map_err(map_sqlx_err)?;
            Ok(IdempotencyBeginOutcome::Acquired(row_to_domain(updated)?))
        }
    }
}

async fn lock_row(
    db_tx: &mut DbTxn<'_, Postgres>,
    tenant_id: &TenantId,
    idempotency_key: &str,
) -> Result<IdempotencyRow, ApplicationError> {
    sqlx::query_as::<_, IdempotencyRow>(&format!(
        "SELECT {SELECT_COLS}
         FROM tx4_infra.idempotency_records
         WHERE tenant_id = $1 AND idempotency_key = $2
         FOR UPDATE"
    ))
    .bind(tenant_id.as_str())
    .bind(idempotency_key)
    .fetch_optional(&mut **db_tx)
    .await
    .map_err(map_sqlx_err)?
    .ok_or_else(|| ApplicationError::not_found("idempotency reservation not found"))
}

#[allow(clippy::too_many_arguments)]
async fn finalize(
    pool: &PgPool,
    tenant_id: &TenantId,
    idempotency_key: &str,
    lease_owner: &str,
    status: IdempotencyStatus,
    response_status: &str,
    response_body: &str,
    now_unix_micros: i64,
) -> Result<IdempotencyReservation, ApplicationError> {
    let mut attempt = 0u32;
    loop {
        match finalize_once(
            pool,
            tenant_id,
            idempotency_key,
            lease_owner,
            status,
            response_status,
            response_body,
            now_unix_micros,
        )
        .await
        {
            Ok(v) => return Ok(v),
            Err(err) if err.is_retryable_infrastructure() && attempt + 1 < MAX_DB_RETRIES => {
                attempt += 1;
                sleep_backoff(attempt).await;
            }
            Err(err) => return Err(err),
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn finalize_once(
    pool: &PgPool,
    tenant_id: &TenantId,
    idempotency_key: &str,
    lease_owner: &str,
    status: IdempotencyStatus,
    response_status: &str,
    response_body: &str,
    now_unix_micros: i64,
) -> Result<IdempotencyReservation, ApplicationError> {
    let mut db_tx = pool.begin().await.map_err(map_sqlx_err)?;
    let current = row_to_domain(lock_row(&mut db_tx, tenant_id, idempotency_key).await?)?;
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
         RETURNING {SELECT_COLS}"
    ))
    .bind(tenant_id.as_str())
    .bind(idempotency_key)
    .bind(status.as_str())
    .bind(response_status)
    .bind(response_body)
    .bind(now_unix_micros)
    .fetch_optional(&mut *db_tx)
    .await
    .map_err(map_sqlx_err)?
    .ok_or_else(|| ApplicationError::conflict("idempotency finalize lost race"))?;

    db_tx.commit().await.map_err(map_sqlx_err)?;
    row_to_domain(row)
}

async fn find_once(
    pool: &PgPool,
    tenant_id: &TenantId,
    idempotency_key: &str,
) -> Result<Option<IdempotencyReservation>, ApplicationError> {
    let row = sqlx::query_as::<_, IdempotencyRow>(&format!(
        "SELECT {SELECT_COLS}
         FROM tx4_infra.idempotency_records
         WHERE tenant_id = $1 AND idempotency_key = $2"
    ))
    .bind(tenant_id.as_str())
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await
    .map_err(map_sqlx_err)?;
    match row {
        None => Ok(None),
        Some(r) => Ok(Some(row_to_domain(r)?)),
    }
}
