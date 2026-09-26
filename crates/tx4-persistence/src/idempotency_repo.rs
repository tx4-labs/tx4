//! PostgreSQL idempotency reservation repository (IA §9).

use sqlx::PgPool;
use tx4_application::{
    ApplicationError, IdempotencyBeginOutcome, IdempotencyBeginRequest, IdempotencyRepository,
    IdempotencyReservation, IdempotencyStatus,
};
use tx4_domain::TenantId;

use crate::sql_support::{map_sqlx_err, sleep_backoff, MAX_DB_RETRIES};
use crate::unit_of_work::{begin_or_recover_tx, finalize_idempotency_on, find_idempotency_on};

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

async fn begin_once(
    pool: &PgPool,
    req: &IdempotencyBeginRequest<'_>,
) -> Result<IdempotencyBeginOutcome, ApplicationError> {
    let mut db_tx = pool.begin().await.map_err(map_sqlx_err)?;
    let outcome = begin_or_recover_tx(&mut db_tx, req).await?;
    db_tx.commit().await.map_err(map_sqlx_err)?;
    Ok(outcome)
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
    let reservation = finalize_idempotency_on(
        &mut db_tx,
        tenant_id,
        idempotency_key,
        lease_owner,
        status,
        response_status,
        response_body,
        now_unix_micros,
    )
    .await?;
    db_tx.commit().await.map_err(map_sqlx_err)?;
    Ok(reservation)
}

async fn find_once(
    pool: &PgPool,
    tenant_id: &TenantId,
    idempotency_key: &str,
) -> Result<Option<IdempotencyReservation>, ApplicationError> {
    let mut db_tx = pool.begin().await.map_err(map_sqlx_err)?;
    let found = find_idempotency_on(&mut db_tx, tenant_id, idempotency_key).await?;
    db_tx.commit().await.map_err(map_sqlx_err)?;
    Ok(found)
}
