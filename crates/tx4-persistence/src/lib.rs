//! TX4 persistence adapters (PostgreSQL / SQLx).
//!
//! Phase-1C: connection pool, migration runner, and connectivity health.
//! Phase-2A: transaction aggregate schema foundation (`tx4_infra.transactions`).
//! Phase-2B: durable transaction repository + `SELECT … FOR UPDATE` concurrency.
//! Phase-2C: idempotency reservations, transactional outbox fencing, PaymentAttempt.

#![forbid(unsafe_code)]

mod error;
mod health;
mod idempotency_repo;
mod migrate;
mod outbox_repo;
mod payment_attempt_repo;
mod pool;
mod sql_support;
mod transaction_repo;

pub use error::PersistenceError;
pub use health::{check_connectivity, check_schema_foundation};
pub use idempotency_repo::PgIdempotencyRepository;
pub use migrate::run_migrations;
pub use outbox_repo::PgOutboxRepository;
pub use payment_attempt_repo::PgPaymentAttemptRepository;
pub use pool::{close_pool, connect_pool};
pub use sql_support::{
    outbox_next_attempt_unix_micros, DEFAULT_LEASE_SECS, DEFAULT_OUTBOX_BACKOFF_BASE_SECS,
    DEFAULT_OUTBOX_MAX_ATTEMPTS,
};
pub use transaction_repo::PgTransactionRepository;

pub use sqlx::PgPool;

pub use tx4_application;
pub use tx4_domain;

/// Marker that the persistence crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_tx4_persistence() {
        assert_eq!(crate_name(), "tx4-persistence");
    }
}
