//! PostgreSQL integration tests for the persistence foundation.
//!
//! These tests require a real PostgreSQL instance.
//! Set `DATABASE_URL` and run with `--ignored`, for example:
//!
//! ```text
//! DATABASE_URL=postgres://tx4:tx4@127.0.0.1:54329/tx4 \
//!   cargo test -p tx4-persistence --test postgres_foundation -- --ignored
//! ```

use std::collections::BTreeMap;
use tx4_config::Config;
use tx4_persistence::{
    check_connectivity, close_pool, connect_pool, run_migrations, PersistenceError,
};

fn test_config_from_env() -> Option<Config> {
    let url = std::env::var("DATABASE_URL").ok()?;
    if url.trim().is_empty() {
        return None;
    }
    let mut m = BTreeMap::new();
    m.insert("DATABASE_URL".to_owned(), url);
    m.insert("TX4_DATABASE_MAX_CONNECTIONS".to_owned(), "5".to_owned());
    m.insert("TX4_DATABASE_MIN_CONNECTIONS".to_owned(), "0".to_owned());
    m.insert(
        "TX4_DATABASE_ACQUIRE_TIMEOUT_SECS".to_owned(),
        "10".to_owned(),
    );
    Some(Config::from_map(&m).expect("valid DATABASE_URL config"))
}

#[tokio::test]
#[ignore = "requires DATABASE_URL against a real PostgreSQL database"]
async fn migrate_twice_is_idempotent_and_health_ok() {
    let Some(config) = test_config_from_env() else {
        panic!("DATABASE_URL must be set for ignored PostgreSQL integration tests");
    };

    let pool = connect_pool(&config)
        .await
        .expect("connect_pool should succeed");

    run_migrations(&pool)
        .await
        .expect("first migrate should succeed");
    run_migrations(&pool)
        .await
        .expect("second migrate should succeed (idempotent)");

    check_connectivity(&pool)
        .await
        .expect("SELECT 1 health check should succeed");

    // Foundation marker schema must exist; business tables must not.
    let schema_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'tx4_infra')",
    )
    .fetch_one(&pool)
    .await
    .expect("schema existence query");
    assert!(schema_exists);

    for forbidden in [
        "transactions",
        "payments",
        "payment_attempts",
        "idempotency_records",
        "outbox_jobs",
        "ledger_entries",
        "settlements",
        "reconciliation_records",
    ] {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1)",
        )
        .bind(forbidden)
        .fetch_one(&pool)
        .await
        .expect("table existence query");
        assert!(!exists, "forbidden business table present: {forbidden}");
    }

    close_pool(&pool).await;
}

#[tokio::test]
async fn connect_to_closed_port_fails_closed() {
    let mut m = BTreeMap::new();
    m.insert(
        "DATABASE_URL".to_owned(),
        "postgres://tx4:secret@127.0.0.1:1/tx4".to_owned(),
    );
    m.insert("TX4_DATABASE_MAX_CONNECTIONS".to_owned(), "1".to_owned());
    m.insert(
        "TX4_DATABASE_ACQUIRE_TIMEOUT_SECS".to_owned(),
        "1".to_owned(),
    );
    let config = Config::from_map(&m).unwrap();
    let err = connect_pool(&config).await.unwrap_err();
    assert_eq!(err, PersistenceError::Connect);
}
