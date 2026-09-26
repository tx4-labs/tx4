//! Shared PostgreSQL persistence helpers (timestamps, retries, SQL error mapping).

use std::time::Duration;
use tx4_application::ApplicationError;

/// OPEN numeric default (IA §9/§11): lease duration seconds for idempotency/outbox.
pub const DEFAULT_LEASE_SECS: i64 = 30;
/// OPEN numeric default: max outbox processing attempts before DEAD_LETTER.
pub const DEFAULT_OUTBOX_MAX_ATTEMPTS: i32 = 8;
/// OPEN numeric default: base backoff seconds for outbox retry scheduling.
pub const DEFAULT_OUTBOX_BACKOFF_BASE_SECS: i64 = 1;

pub(crate) const MAX_DB_RETRIES: u32 = 5;
pub(crate) const RETRY_BASE_MS: u64 = 10;

pub(crate) async fn sleep_backoff(attempt: u32) {
    let delay = RETRY_BASE_MS.saturating_mul(u64::from(attempt));
    tokio::time::sleep(Duration::from_millis(delay)).await;
}

pub(crate) fn map_sqlx_err(err: sqlx::Error) -> ApplicationError {
    if is_retryable_db_err(&err) {
        return ApplicationError::retryable("postgresql deadlock or serialization failure");
    }
    if let sqlx::Error::Database(db) = &err {
        if db.code().as_deref() == Some("23505") {
            return ApplicationError::conflict("unique constraint violation");
        }
    }
    ApplicationError::permanent_internal("database query failed")
}

pub(crate) fn is_retryable_db_err(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db) => matches!(db.code().as_deref(), Some("40001") | Some("40P01")),
        _ => false,
    }
}

pub(crate) fn u64_from_i64(value: i64, label: &str) -> Result<u64, ApplicationError> {
    if value < 0 {
        return Err(ApplicationError::permanent_internal(format!(
            "negative {label} in database; fail closed"
        )));
    }
    u64::try_from(value).map_err(|_| {
        ApplicationError::permanent_internal(format!("{label} mapping failed; fail closed"))
    })
}

pub(crate) fn i64_from_u64(value: u64, label: &str) -> Result<i64, ApplicationError> {
    i64::try_from(value).map_err(|_| {
        ApplicationError::permanent_internal(format!(
            "{label} exceeds BIGINT persistence range; fail closed"
        ))
    })
}

/// Deterministic outbox backoff: base_secs * attempt_count (OPEN numeric policy).
pub fn outbox_next_attempt_unix_micros(
    now_unix_micros: i64,
    attempt_count: i32,
    base_secs: i64,
) -> Result<i64, ApplicationError> {
    let delay_secs = base_secs.saturating_mul(i64::from(attempt_count.max(1)));
    now_unix_micros
        .checked_add(delay_secs.saturating_mul(1_000_000))
        .ok_or_else(|| ApplicationError::permanent_internal("outbox backoff overflow"))
}
