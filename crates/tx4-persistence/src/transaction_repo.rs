//! PostgreSQL transaction repository with `SELECT … FOR UPDATE` concurrency (Phase-2B).

use sqlx::{PgPool, Postgres, Transaction as DbTxn};
use std::time::Duration;
use tx4_application::{ApplicationError, TransactionRepository};
use tx4_domain::{
    CurrencyId, DomainError, LifecycleCommand, LifecycleContext, Money, TenantId, Transaction,
    TransactionId, TransactionState, Version,
};

/// Bounded retries for PostgreSQL deadlock / serialization failures (IA §8.5; numeric default OPEN).
const MAX_DB_RETRIES: u32 = 5;
const RETRY_BASE_MS: u64 = 10;

/// SQLx-backed durable transaction repository.
#[derive(Clone, Debug)]
pub struct PgTransactionRepository {
    pool: PgPool,
}

impl PgTransactionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl TransactionRepository for PgTransactionRepository {
    async fn insert(&self, tx: &Transaction) -> Result<(), ApplicationError> {
        let mut attempt = 0u32;
        loop {
            match insert_once(&self.pool, tx).await {
                Ok(()) => return Ok(()),
                Err(err) if err.is_retryable_infrastructure() && attempt + 1 < MAX_DB_RETRIES => {
                    attempt += 1;
                    sleep_backoff(attempt).await;
                }
                Err(err) => return Err(err),
            }
        }
    }

    async fn find(
        &self,
        tenant_id: &TenantId,
        id: &TransactionId,
    ) -> Result<Option<Transaction>, ApplicationError> {
        let mut attempt = 0u32;
        loop {
            match find_once(&self.pool, tenant_id, id).await {
                Ok(v) => return Ok(v),
                Err(err) if err.is_retryable_infrastructure() && attempt + 1 < MAX_DB_RETRIES => {
                    attempt += 1;
                    sleep_backoff(attempt).await;
                }
                Err(err) => return Err(err),
            }
        }
    }

    async fn apply_transition(
        &self,
        tenant_id: &TenantId,
        id: &TransactionId,
        expected_version: Version,
        command: &LifecycleCommand,
        ctx: &LifecycleContext,
        updated_at_unix_micros: i64,
    ) -> Result<Transaction, ApplicationError> {
        let mut attempt = 0u32;
        loop {
            match apply_once(
                &self.pool,
                tenant_id,
                id,
                expected_version,
                command,
                ctx,
                updated_at_unix_micros,
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
}

async fn sleep_backoff(attempt: u32) {
    let delay = RETRY_BASE_MS.saturating_mul(u64::from(attempt));
    tokio::time::sleep(Duration::from_millis(delay)).await;
}

async fn insert_once(pool: &PgPool, tx: &Transaction) -> Result<(), ApplicationError> {
    let amount = tx.amount_atomic();
    let currency = tx.currency_id().map(|c| c.as_str().to_owned());
    let result = sqlx::query(
        "INSERT INTO tx4_infra.transactions (
            id, tenant_id, primary_state, version,
            amount_atomic, currency_id, expires_at, attempt_budget,
            created_at, updated_at
         ) VALUES (
            $1, $2, $3, $4,
            $5, $6,
            CASE WHEN $7::bigint IS NULL THEN NULL
                 ELSE TIMESTAMPTZ 'epoch' + (($7::bigint) * INTERVAL '1 microsecond')
            END,
            $8,
            TIMESTAMPTZ 'epoch' + (($9::bigint) * INTERVAL '1 microsecond'),
            TIMESTAMPTZ 'epoch' + (($10::bigint) * INTERVAL '1 microsecond')
         )",
    )
    .bind(tx.id().as_str())
    .bind(tx.tenant_id().as_str())
    .bind(tx.primary_state().to_string())
    .bind(version_to_i64(tx.version())?)
    .bind(amount)
    .bind(currency)
    .bind(tx.expires_at_unix_micros())
    .bind(tx.attempt_budget())
    .bind(tx.created_at_unix_micros())
    .bind(tx.updated_at_unix_micros())
    .execute(pool)
    .await
    .map_err(map_sqlx_err)?;

    if result.rows_affected() != 1 {
        return Err(ApplicationError::permanent_internal(
            "insert affected unexpected row count",
        ));
    }
    Ok(())
}

async fn find_once(
    pool: &PgPool,
    tenant_id: &TenantId,
    id: &TransactionId,
) -> Result<Option<Transaction>, ApplicationError> {
    let row = sqlx::query_as::<_, TransactionRow>(LOAD_SQL)
        .bind(tenant_id.as_str())
        .bind(id.as_str())
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

    match row {
        None => Ok(None),
        Some(r) => Ok(Some(row_to_domain(r)?)),
    }
}

async fn apply_once(
    pool: &PgPool,
    tenant_id: &TenantId,
    id: &TransactionId,
    expected_version: Version,
    command: &LifecycleCommand,
    ctx: &LifecycleContext,
    updated_at_unix_micros: i64,
) -> Result<Transaction, ApplicationError> {
    let mut db_tx = pool.begin().await.map_err(map_sqlx_err)?;

    let current = lock_and_load(&mut db_tx, tenant_id, id).await?;
    if current.version() != expected_version {
        return Err(ApplicationError::conflict(
            "transaction version conflict: expected version is stale",
        ));
    }

    let next = current
        .apply(command, ctx, updated_at_unix_micros)
        .map_err(map_domain_err)?;

    // IdempotentNoOp (e.g. ReplayDuplicate): durable row unchanged — return the locked
    // aggregate so callers never observe a non-persisted updated_at (Phase-2B P2-1).
    if next.version() == current.version() && next.primary_state() == current.primary_state() {
        db_tx.commit().await.map_err(map_sqlx_err)?;
        return Ok(current);
    }

    persist_update(&mut db_tx, &next, expected_version).await?;
    db_tx.commit().await.map_err(map_sqlx_err)?;
    Ok(next)
}

const LOAD_SQL: &str = "SELECT
    id, tenant_id, primary_state, version,
    amount_atomic, currency_id, attempt_budget,
    (EXTRACT(EPOCH FROM expires_at) * 1000000)::bigint AS expires_at_unix_micros,
    (EXTRACT(EPOCH FROM created_at) * 1000000)::bigint AS created_at_unix_micros,
    (EXTRACT(EPOCH FROM updated_at) * 1000000)::bigint AS updated_at_unix_micros
 FROM tx4_infra.transactions
 WHERE tenant_id = $1 AND id = $2";

const LOCK_SQL: &str = "SELECT
    id, tenant_id, primary_state, version,
    amount_atomic, currency_id, attempt_budget,
    (EXTRACT(EPOCH FROM expires_at) * 1000000)::bigint AS expires_at_unix_micros,
    (EXTRACT(EPOCH FROM created_at) * 1000000)::bigint AS created_at_unix_micros,
    (EXTRACT(EPOCH FROM updated_at) * 1000000)::bigint AS updated_at_unix_micros
 FROM tx4_infra.transactions
 WHERE tenant_id = $1 AND id = $2
 FOR UPDATE";

#[derive(Debug, sqlx::FromRow)]
struct TransactionRow {
    id: String,
    tenant_id: String,
    primary_state: String,
    version: i64,
    amount_atomic: Option<i64>,
    currency_id: Option<String>,
    attempt_budget: Option<i32>,
    expires_at_unix_micros: Option<i64>,
    created_at_unix_micros: i64,
    updated_at_unix_micros: i64,
}

async fn lock_and_load(
    db_tx: &mut DbTxn<'_, Postgres>,
    tenant_id: &TenantId,
    id: &TransactionId,
) -> Result<Transaction, ApplicationError> {
    let row = sqlx::query_as::<_, TransactionRow>(LOCK_SQL)
        .bind(tenant_id.as_str())
        .bind(id.as_str())
        .fetch_optional(&mut **db_tx)
        .await
        .map_err(map_sqlx_err)?;

    match row {
        None => Err(ApplicationError::not_found(
            "transaction not found for tenant",
        )),
        Some(r) => row_to_domain(r),
    }
}

async fn persist_update(
    db_tx: &mut DbTxn<'_, Postgres>,
    next: &Transaction,
    expected_version: Version,
) -> Result<(), ApplicationError> {
    let expected_i64 = version_to_i64(expected_version)?;
    let next_i64 = version_to_i64(next.version())?;
    let result = sqlx::query(
        "UPDATE tx4_infra.transactions SET
            primary_state = $1,
            version = $2,
            amount_atomic = $3,
            currency_id = $4,
            expires_at = CASE WHEN $5::bigint IS NULL THEN NULL
                              ELSE TIMESTAMPTZ 'epoch' + (($5::bigint) * INTERVAL '1 microsecond')
                         END,
            attempt_budget = $6,
            updated_at = TIMESTAMPTZ 'epoch' + (($7::bigint) * INTERVAL '1 microsecond')
         WHERE tenant_id = $8 AND id = $9 AND version = $10",
    )
    .bind(next.primary_state().to_string())
    .bind(next_i64)
    .bind(next.amount_atomic())
    .bind(next.currency_id().map(|c| c.as_str().to_owned()))
    .bind(next.expires_at_unix_micros())
    .bind(next.attempt_budget())
    .bind(next.updated_at_unix_micros())
    .bind(next.tenant_id().as_str())
    .bind(next.id().as_str())
    .bind(expected_i64)
    .execute(&mut **db_tx)
    .await
    .map_err(map_sqlx_err)?;

    if result.rows_affected() != 1 {
        return Err(ApplicationError::conflict(
            "transaction version conflict during update",
        ));
    }
    Ok(())
}

fn row_to_domain(row: TransactionRow) -> Result<Transaction, ApplicationError> {
    let id = TransactionId::new(row.id).map_err(map_domain_err)?;
    let tenant_id = TenantId::new(row.tenant_id).map_err(map_domain_err)?;
    let primary_state = TransactionState::parse_str(&row.primary_state).map_err(map_domain_err)?;
    let version = version_from_i64(row.version)?;
    let money = match (row.amount_atomic, row.currency_id) {
        (None, None) => None,
        (Some(amount), Some(code)) => {
            let currency = CurrencyId::new(code).map_err(map_domain_err)?;
            Some(Money::new(amount, currency))
        }
        _ => {
            return Err(ApplicationError::permanent_internal(
                "corrupt money pair in transaction row",
            ));
        }
    };

    Transaction::reconstitute(
        id,
        tenant_id,
        primary_state,
        version,
        money,
        row.expires_at_unix_micros,
        row.attempt_budget,
        row.created_at_unix_micros,
        row.updated_at_unix_micros,
    )
    .map_err(map_domain_err)
}

fn version_to_i64(version: Version) -> Result<i64, ApplicationError> {
    i64::try_from(version.get()).map_err(|_| {
        ApplicationError::permanent_internal(
            "version exceeds BIGINT persistence range; fail closed",
        )
    })
}

fn version_from_i64(value: i64) -> Result<Version, ApplicationError> {
    if value < 0 {
        return Err(ApplicationError::permanent_internal(
            "negative version in database; fail closed",
        ));
    }
    u64::try_from(value)
        .map(Version::new)
        .map_err(|_| ApplicationError::permanent_internal("version mapping failed; fail closed"))
}

fn map_domain_err(err: DomainError) -> ApplicationError {
    match err {
        DomainError::ForbiddenTransition
        | DomainError::ConditionalPredicateUnsatisfied
        | DomainError::CompensatingIntentRequired
        | DomainError::MissingKnownSuccessEvidence
        | DomainError::StaleTransition
        | DomainError::ConflictingTransition
        | DomainError::TerminalStateImmutable
        | DomainError::TimeoutIsNotFailure
        | DomainError::NoEligibleConflictCandidate
        | DomainError::AmbiguousConflictTie => {
            ApplicationError::business_rejection(err.to_string())
        }
        DomainError::InvalidVersion => ApplicationError::permanent_internal(err.to_string()),
        other => ApplicationError::validation(other.to_string()),
    }
}

fn map_sqlx_err(err: sqlx::Error) -> ApplicationError {
    if is_retryable_db_err(&err) {
        return ApplicationError::retryable("postgresql deadlock or serialization failure");
    }
    if let sqlx::Error::Database(db) = &err {
        if db.code().as_deref() == Some("23505") {
            return ApplicationError::conflict("transaction identity already exists");
        }
    }
    ApplicationError::permanent_internal("database query failed")
}

fn is_retryable_db_err(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db) => matches!(db.code().as_deref(), Some("40001") | Some("40P01")),
        _ => false,
    }
}
