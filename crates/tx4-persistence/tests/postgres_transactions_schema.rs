//! PostgreSQL integration tests for Phase-2A transaction schema foundation.
//!
//! Requires a real PostgreSQL instance (`DATABASE_URL`) and `--ignored`.

use std::collections::BTreeMap;
use tx4_config::Config;
use tx4_domain::TransactionState;
use tx4_persistence::{close_pool, connect_pool, run_migrations};

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

async fn table_exists(pool: &sqlx::PgPool, schema: &str, table: &str) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.tables
            WHERE table_schema = $1 AND table_name = $2
         )",
    )
    .bind(schema)
    .bind(table)
    .fetch_one(pool)
    .await
    .expect("table existence query")
}

#[tokio::test]
#[ignore = "requires DATABASE_URL against a real PostgreSQL database"]
async fn transaction_schema_foundation() {
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

    assert!(
        table_exists(&pool, "tx4_infra", "transactions").await,
        "tx4_infra.transactions must exist after Phase-2A migration"
    );

    // Phase-1C foundation schema remains.
    let schema_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'tx4_infra')",
    )
    .fetch_one(&pool)
    .await
    .expect("schema existence query");
    assert!(schema_exists);

    // Unauthorized business tables still absent (schema + public).
    for forbidden in [
        "payments",
        "payment_attempts",
        "idempotency_records",
        "outbox_jobs",
        "ledger_entries",
        "settlements",
        "reconciliation_records",
    ] {
        assert!(
            !table_exists(&pool, "public", forbidden).await,
            "forbidden public table present: {forbidden}"
        );
        assert!(
            !table_exists(&pool, "tx4_infra", forbidden).await,
            "forbidden tx4_infra table present: {forbidden}"
        );
    }

    // Required columns.
    for column in [
        "id",
        "tenant_id",
        "primary_state",
        "version",
        "amount_atomic",
        "currency_id",
        "expires_at",
        "attempt_budget",
        "created_at",
        "updated_at",
    ] {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'tx4_infra'
                  AND table_name = 'transactions'
                  AND column_name = $1
             )",
        )
        .bind(column)
        .fetch_one(&pool)
        .await
        .expect("column existence query");
        assert!(exists, "missing column: {column}");
    }

    // amount_atomic / version must be bigint (no float money authority).
    let amount_udt: String = sqlx::query_scalar(
        "SELECT udt_name FROM information_schema.columns
         WHERE table_schema = 'tx4_infra' AND table_name = 'transactions'
           AND column_name = 'amount_atomic'",
    )
    .fetch_one(&pool)
    .await
    .expect("amount udt");
    assert_eq!(amount_udt, "int8");

    let version_udt: String = sqlx::query_scalar(
        "SELECT udt_name FROM information_schema.columns
         WHERE table_schema = 'tx4_infra' AND table_name = 'transactions'
           AND column_name = 'version'",
    )
    .fetch_one(&pool)
    .await
    .expect("version udt");
    assert_eq!(version_udt, "int8");

    // Insert with domain lifecycle Display strings — no independent SQL state machine.
    for state in TransactionState::ALL {
        let state_s = state.to_string();
        sqlx::query(
            "INSERT INTO tx4_infra.transactions
                (id, tenant_id, primary_state, version, amount_atomic, currency_id)
             VALUES ($1, $2, $3, 0, $4, $5)",
        )
        .bind(format!("tx-{state_s}"))
        .bind("tenant-a")
        .bind(&state_s)
        .bind(i64::MAX)
        .bind("IDR")
        .execute(&pool)
        .await
        .unwrap_or_else(|e| panic!("insert state {state_s} should succeed: {e}"));
    }

    // Money extremes are lossless as BIGINT.
    let roundtrip: i64 = sqlx::query_scalar(
        "SELECT amount_atomic FROM tx4_infra.transactions
         WHERE tenant_id = 'tenant-a' AND id = 'tx-CREATED'",
    )
    .fetch_one(&pool)
    .await
    .expect("amount roundtrip");
    assert_eq!(roundtrip, i64::MAX);

    sqlx::query(
        "INSERT INTO tx4_infra.transactions
            (id, tenant_id, primary_state, version, amount_atomic, currency_id)
         VALUES ('tx-min', 'tenant-a', 'CREATED', 0, $1, 'USD')",
    )
    .bind(i64::MIN)
    .execute(&pool)
    .await
    .expect("i64::MIN insert");

    let min_rt: i64 = sqlx::query_scalar(
        "SELECT amount_atomic FROM tx4_infra.transactions
         WHERE tenant_id = 'tenant-a' AND id = 'tx-min'",
    )
    .fetch_one(&pool)
    .await
    .expect("min roundtrip");
    assert_eq!(min_rt, i64::MIN);

    // NOT NULL / CHECK: empty tenant rejected.
    let err = sqlx::query(
        "INSERT INTO tx4_infra.transactions (id, tenant_id, primary_state, version)
         VALUES ('tx-x', '', 'CREATED', 0)",
    )
    .execute(&pool)
    .await;
    assert!(err.is_err(), "empty tenant_id must fail");

    // Invalid lifecycle state rejected.
    let err = sqlx::query(
        "INSERT INTO tx4_infra.transactions (id, tenant_id, primary_state, version)
         VALUES ('tx-bad', 'tenant-a', 'REFUNDED', 0)",
    )
    .execute(&pool)
    .await;
    assert!(err.is_err(), "undocumented state REFUNDED must fail");

    // Money pair invariant: amount without currency rejected.
    let err = sqlx::query(
        "INSERT INTO tx4_infra.transactions
            (id, tenant_id, primary_state, version, amount_atomic)
         VALUES ('tx-money', 'tenant-a', 'CREATED', 0, 100)",
    )
    .execute(&pool)
    .await;
    assert!(err.is_err(), "amount without currency must fail");

    // Primary key / tenant isolation: duplicate (tenant_id, id) rejected.
    let err = sqlx::query(
        "INSERT INTO tx4_infra.transactions (id, tenant_id, primary_state, version)
         VALUES ('tx-CREATED', 'tenant-a', 'PENDING', 1)",
    )
    .execute(&pool)
    .await;
    assert!(err.is_err(), "duplicate PK must fail");

    // Same id under different tenant is allowed (tenant ownership boundary).
    sqlx::query(
        "INSERT INTO tx4_infra.transactions (id, tenant_id, primary_state, version)
         VALUES ('tx-CREATED', 'tenant-b', 'CREATED', 0)",
    )
    .execute(&pool)
    .await
    .expect("same id different tenant should succeed");

    // Negative version rejected.
    let err = sqlx::query(
        "INSERT INTO tx4_infra.transactions (id, tenant_id, primary_state, version)
         VALUES ('tx-neg-ver', 'tenant-a', 'CREATED', -1)",
    )
    .execute(&pool)
    .await;
    assert!(err.is_err(), "negative version must fail");

    close_pool(&pool).await;
}
