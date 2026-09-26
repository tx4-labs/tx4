//! PostgreSQL transactional outbox repository with claim_epoch fencing (IA §11).

use sqlx::PgPool;
use tx4_application::{
    ApplicationError, ClaimedOutboxJob, OutboxJob, OutboxRepository, OutboxStatus,
};
use tx4_domain::TenantId;

use crate::sql_support::{i64_from_u64, map_sqlx_err, sleep_backoff, u64_from_i64, MAX_DB_RETRIES};

/// SQLx-backed durable outbox.
#[derive(Clone, Debug)]
pub struct PgOutboxRepository {
    pool: PgPool,
}

impl PgOutboxRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl OutboxRepository for PgOutboxRepository {
    async fn enqueue(&self, job: &OutboxJob) -> Result<(), ApplicationError> {
        with_retry(|| enqueue_once(&self.pool, job)).await
    }

    async fn claim(
        &self,
        worker_id: &str,
        limit: i64,
        lease_duration_secs: i64,
        now_unix_micros: i64,
    ) -> Result<Vec<ClaimedOutboxJob>, ApplicationError> {
        with_retry(|| {
            claim_once(
                &self.pool,
                worker_id,
                limit,
                lease_duration_secs,
                now_unix_micros,
            )
        })
        .await
    }

    async fn complete_succeeded(
        &self,
        job_id: &str,
        worker_id: &str,
        claim_epoch: u64,
        now_unix_micros: i64,
    ) -> Result<bool, ApplicationError> {
        with_retry(|| {
            complete_once(
                &self.pool,
                job_id,
                worker_id,
                claim_epoch,
                OutboxStatus::Succeeded,
                None,
                None,
                now_unix_micros,
            )
        })
        .await
    }

    async fn complete_retry(
        &self,
        job_id: &str,
        worker_id: &str,
        claim_epoch: u64,
        last_error: &str,
        next_attempt_at_unix_micros: i64,
        now_unix_micros: i64,
    ) -> Result<bool, ApplicationError> {
        with_retry(|| {
            complete_once(
                &self.pool,
                job_id,
                worker_id,
                claim_epoch,
                OutboxStatus::Pending,
                Some(last_error),
                Some(next_attempt_at_unix_micros),
                now_unix_micros,
            )
        })
        .await
    }

    async fn complete_dead_letter(
        &self,
        job_id: &str,
        worker_id: &str,
        claim_epoch: u64,
        last_error: &str,
        now_unix_micros: i64,
    ) -> Result<bool, ApplicationError> {
        with_retry(|| {
            complete_once(
                &self.pool,
                job_id,
                worker_id,
                claim_epoch,
                OutboxStatus::DeadLetter,
                Some(last_error),
                None,
                now_unix_micros,
            )
        })
        .await
    }

    async fn find(&self, job_id: &str) -> Result<Option<OutboxJob>, ApplicationError> {
        with_retry(|| find_once(&self.pool, job_id)).await
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

fn select_cols() -> String {
    format!(
        "id, tenant_id, job_type, payload, status, locked_by, claim_epoch,
    {lease} AS lease_expires_at_unix_micros,
    attempt_count,
    {next} AS next_attempt_at_unix_micros,
    last_error",
        lease = crate::sql_support::ts_unix_micros_expr("lease_expires_at"),
        next = crate::sql_support::ts_unix_micros_expr("next_attempt_at"),
    )
}

#[derive(Debug, sqlx::FromRow)]
struct OutboxRow {
    id: String,
    tenant_id: String,
    job_type: String,
    payload: String,
    status: String,
    locked_by: Option<String>,
    claim_epoch: i64,
    lease_expires_at_unix_micros: Option<i64>,
    attempt_count: i32,
    next_attempt_at_unix_micros: i64,
    last_error: Option<String>,
}

fn row_to_domain(row: OutboxRow) -> Result<OutboxJob, ApplicationError> {
    Ok(OutboxJob {
        id: row.id,
        tenant_id: TenantId::new(row.tenant_id)
            .map_err(|e| ApplicationError::validation(e.to_string()))?,
        job_type: row.job_type,
        payload: row.payload,
        status: OutboxStatus::parse(&row.status)?,
        locked_by: row.locked_by,
        claim_epoch: u64_from_i64(row.claim_epoch, "claim_epoch")?,
        lease_expires_at_unix_micros: row.lease_expires_at_unix_micros,
        attempt_count: row.attempt_count,
        next_attempt_at_unix_micros: row.next_attempt_at_unix_micros,
        last_error: row.last_error,
    })
}

async fn enqueue_once(pool: &PgPool, job: &OutboxJob) -> Result<(), ApplicationError> {
    let claim_epoch = i64_from_u64(job.claim_epoch, "claim_epoch")?;
    let result = sqlx::query(
        "INSERT INTO tx4_infra.outbox_jobs (
            id, tenant_id, job_type, payload, status, locked_by, claim_epoch,
            lease_expires_at, attempt_count, next_attempt_at, last_error, created_at, updated_at
         ) VALUES (
            $1, $2, $3, $4, $5, NULL, $6, NULL, $7,
            TIMESTAMPTZ 'epoch' + (($8::bigint) * INTERVAL '1 microsecond'),
            NULL,
            TIMESTAMPTZ 'epoch' + (($9::bigint) * INTERVAL '1 microsecond'),
            TIMESTAMPTZ 'epoch' + (($9::bigint) * INTERVAL '1 microsecond')
         )",
    )
    .bind(&job.id)
    .bind(job.tenant_id.as_str())
    .bind(&job.job_type)
    .bind(&job.payload)
    .bind(OutboxStatus::Pending.as_str())
    .bind(claim_epoch)
    .bind(job.attempt_count)
    .bind(job.next_attempt_at_unix_micros)
    .bind(job.next_attempt_at_unix_micros)
    .execute(pool)
    .await
    .map_err(map_sqlx_err)?;
    if result.rows_affected() != 1 {
        return Err(ApplicationError::permanent_internal(
            "outbox enqueue affected unexpected row count",
        ));
    }
    Ok(())
}

async fn claim_once(
    pool: &PgPool,
    worker_id: &str,
    limit: i64,
    lease_duration_secs: i64,
    now_unix_micros: i64,
) -> Result<Vec<ClaimedOutboxJob>, ApplicationError> {
    if limit < 1 {
        return Err(ApplicationError::validation("claim limit must be >= 1"));
    }
    if lease_duration_secs < 1 {
        return Err(ApplicationError::validation(
            "lease_duration_secs must be >= 1",
        ));
    }
    let lease_expires = now_unix_micros
        .checked_add(lease_duration_secs.saturating_mul(1_000_000))
        .ok_or_else(|| ApplicationError::permanent_internal("outbox lease overflow"))?;

    let cols = select_cols();
    let mut db_tx = pool.begin().await.map_err(map_sqlx_err)?;
    let rows = sqlx::query_as::<_, OutboxRow>(&format!(
        "UPDATE tx4_infra.outbox_jobs AS o SET
            status = 'RUNNING',
            locked_by = $1,
            lease_expires_at = TIMESTAMPTZ 'epoch' + (($2::bigint) * INTERVAL '1 microsecond'),
            claim_epoch = o.claim_epoch + 1,
            attempt_count = o.attempt_count + 1,
            updated_at = TIMESTAMPTZ 'epoch' + (($3::bigint) * INTERVAL '1 microsecond')
         WHERE o.id IN (
            SELECT id FROM tx4_infra.outbox_jobs
            WHERE (status = 'PENDING' AND next_attempt_at <= TIMESTAMPTZ 'epoch' + (($3::bigint) * INTERVAL '1 microsecond'))
               OR (status = 'RUNNING' AND lease_expires_at < TIMESTAMPTZ 'epoch' + (($3::bigint) * INTERVAL '1 microsecond'))
            ORDER BY next_attempt_at ASC NULLS FIRST, id ASC
            FOR UPDATE SKIP LOCKED
            LIMIT $4
         )
         RETURNING {cols}"
    ))
    .bind(worker_id)
    .bind(lease_expires)
    .bind(now_unix_micros)
    .bind(limit)
    .fetch_all(&mut *db_tx)
    .await
    .map_err(map_sqlx_err)?;

    db_tx.commit().await.map_err(map_sqlx_err)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(ClaimedOutboxJob {
            job: row_to_domain(row)?,
        });
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
async fn complete_once(
    pool: &PgPool,
    job_id: &str,
    worker_id: &str,
    claim_epoch: u64,
    next_status: OutboxStatus,
    last_error: Option<&str>,
    next_attempt_at: Option<i64>,
    now_unix_micros: i64,
) -> Result<bool, ApplicationError> {
    let epoch = i64_from_u64(claim_epoch, "claim_epoch")?;
    let next_attempt = next_attempt_at.unwrap_or(now_unix_micros);
    let result = sqlx::query(
        "UPDATE tx4_infra.outbox_jobs SET
            status = $1,
            locked_by = CASE WHEN $1 = 'PENDING' THEN NULL ELSE locked_by END,
            lease_expires_at = CASE WHEN $1 = 'PENDING' THEN NULL ELSE lease_expires_at END,
            last_error = COALESCE($2, last_error),
            next_attempt_at = TIMESTAMPTZ 'epoch' + (($3::bigint) * INTERVAL '1 microsecond'),
            updated_at = TIMESTAMPTZ 'epoch' + (($4::bigint) * INTERVAL '1 microsecond')
         WHERE id = $5
           AND claim_epoch = $6
           AND locked_by = $7
           AND status = 'RUNNING'
           AND lease_expires_at > TIMESTAMPTZ 'epoch' + (($4::bigint) * INTERVAL '1 microsecond')",
    )
    .bind(next_status.as_str())
    .bind(last_error)
    .bind(next_attempt)
    .bind(now_unix_micros)
    .bind(job_id)
    .bind(epoch)
    .bind(worker_id)
    .execute(pool)
    .await
    .map_err(map_sqlx_err)?;

    // Zero rows → stale worker no-op (IA §11.4).
    Ok(result.rows_affected() == 1)
}

async fn find_once(pool: &PgPool, job_id: &str) -> Result<Option<OutboxJob>, ApplicationError> {
    let cols = select_cols();
    let row = sqlx::query_as::<_, OutboxRow>(&format!(
        "SELECT {cols} FROM tx4_infra.outbox_jobs WHERE id = $1"
    ))
    .bind(job_id)
    .fetch_optional(pool)
    .await
    .map_err(map_sqlx_err)?;
    match row {
        None => Ok(None),
        Some(r) => Ok(Some(row_to_domain(r)?)),
    }
}
