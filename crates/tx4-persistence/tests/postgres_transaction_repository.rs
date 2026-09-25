//! PostgreSQL integration tests for Phase-2B transaction repository + concurrency.

use std::collections::BTreeMap;
use std::sync::Arc;
use tx4_application::TransactionRepository;
use tx4_config::Config;
use tx4_domain::{
    CurrencyId, LifecycleCommand, LifecycleContext, Money, TenantId, Transaction, TransactionId,
    TransactionState, Version,
};
use tx4_persistence::{close_pool, connect_pool, run_migrations, PgTransactionRepository};

fn test_config_from_env() -> Option<Config> {
    let url = std::env::var("DATABASE_URL").ok()?;
    if url.trim().is_empty() {
        return None;
    }
    let mut m = BTreeMap::new();
    m.insert("DATABASE_URL".to_owned(), url);
    m.insert("TX4_DATABASE_MAX_CONNECTIONS".to_owned(), "10".to_owned());
    m.insert("TX4_DATABASE_MIN_CONNECTIONS".to_owned(), "0".to_owned());
    m.insert(
        "TX4_DATABASE_ACQUIRE_TIMEOUT_SECS".to_owned(),
        "10".to_owned(),
    );
    Some(Config::from_map(&m).expect("valid DATABASE_URL config"))
}

fn money_idr(amount: i64) -> Money {
    Money::new(amount, CurrencyId::new("IDR").unwrap())
}

#[tokio::test]
#[ignore = "requires DATABASE_URL against a real PostgreSQL database"]
async fn repository_round_trip_and_concurrency() {
    let Some(config) = test_config_from_env() else {
        panic!("DATABASE_URL must be set for ignored PostgreSQL integration tests");
    };
    let pool = connect_pool(&config).await.expect("connect");
    run_migrations(&pool).await.expect("migrate");
    run_migrations(&pool).await.expect("migrate twice");

    let repo = Arc::new(PgTransactionRepository::new(pool.clone()));
    let tenant = TenantId::new("tenant-2b").unwrap();
    let id = TransactionId::new("tx-2b-1").unwrap();

    let created = Transaction::new_created(
        id.clone(),
        tenant.clone(),
        Some(money_idr(12_345)),
        Some(1_700_000_000_000_000),
        Some(3),
        1_700_000_000_000_000,
    )
    .unwrap();

    repo.insert(&created).await.expect("insert");

    let loaded = repo
        .find(&tenant, &id)
        .await
        .expect("find")
        .expect("present");
    assert_eq!(loaded.id(), &id);
    assert_eq!(loaded.tenant_id(), &tenant);
    assert_eq!(loaded.primary_state(), TransactionState::Created);
    assert_eq!(loaded.version(), Version::zero());
    assert_eq!(loaded.amount_atomic(), Some(12_345));
    assert_eq!(loaded.currency_id().map(|c| c.as_str()), Some("IDR"));
    assert_eq!(loaded.attempt_budget(), Some(3));

    // Tenant isolation: other tenant cannot see the row.
    let other = TenantId::new("tenant-other").unwrap();
    assert!(repo.find(&other, &id).await.unwrap().is_none());

    // Valid mutation CREATED → PENDING.
    let pending = repo
        .apply_transition(
            &tenant,
            &id,
            Version::zero(),
            &LifecycleCommand::OpenPending,
            &LifecycleContext::empty(),
            1_700_000_000_100_000,
        )
        .await
        .expect("open pending");
    assert_eq!(pending.primary_state(), TransactionState::Pending);
    assert_eq!(pending.version(), Version::new(1));

    // Stale version conflict.
    let stale = repo
        .apply_transition(
            &tenant,
            &id,
            Version::zero(),
            &LifecycleCommand::AcceptPaymentSuccess,
            &LifecycleContext {
                known_success_evidence: true,
                ..LifecycleContext::empty()
            },
            1_700_000_000_200_000,
        )
        .await;
    assert!(stale.unwrap_err().to_string().contains("conflict"));

    // Invalid lifecycle: PENDING → PROCESSING forbidden.
    let bad = repo
        .apply_transition(
            &tenant,
            &id,
            Version::new(1),
            &LifecycleCommand::StartProcessing,
            &LifecycleContext::empty(),
            1_700_000_000_300_000,
        )
        .await;
    assert!(bad.is_err());

    // Happy path to PAID then PROCESSING then COMPLETED (terminal).
    let paid = repo
        .apply_transition(
            &tenant,
            &id,
            Version::new(1),
            &LifecycleCommand::AcceptPaymentSuccess,
            &LifecycleContext {
                known_success_evidence: true,
                ..LifecycleContext::empty()
            },
            1_700_000_000_400_000,
        )
        .await
        .unwrap();
    assert_eq!(paid.primary_state(), TransactionState::Paid);

    let processing = repo
        .apply_transition(
            &tenant,
            &id,
            paid.version(),
            &LifecycleCommand::StartProcessing,
            &LifecycleContext::empty(),
            1_700_000_000_500_000,
        )
        .await
        .unwrap();
    assert_eq!(processing.primary_state(), TransactionState::Processing);

    let completed = repo
        .apply_transition(
            &tenant,
            &id,
            processing.version(),
            &LifecycleCommand::Complete,
            &LifecycleContext::empty(),
            1_700_000_000_600_000,
        )
        .await
        .unwrap();
    assert_eq!(completed.primary_state(), TransactionState::Completed);
    assert!(completed.primary_state().is_terminal());

    // Terminal cannot progress.
    let term_err = repo
        .apply_transition(
            &tenant,
            &id,
            completed.version(),
            &LifecycleCommand::Cancel,
            &LifecycleContext::empty(),
            1_700_000_000_700_000,
        )
        .await;
    assert!(term_err.is_err());

    // Money extremes round-trip.
    let id_max = TransactionId::new("tx-money-max").unwrap();
    let max_tx = Transaction::new_created(
        id_max.clone(),
        tenant.clone(),
        Some(money_idr(i64::MAX)),
        None,
        None,
        1,
    )
    .unwrap();
    repo.insert(&max_tx).await.unwrap();
    let loaded_max = repo.find(&tenant, &id_max).await.unwrap().unwrap();
    assert_eq!(loaded_max.amount_atomic(), Some(i64::MAX));

    // Concurrent mutation: two tasks with same expected version — exactly one commits.
    let id_c = TransactionId::new("tx-concurrent").unwrap();
    let base =
        Transaction::new_created(id_c.clone(), tenant.clone(), None, None, None, 10).unwrap();
    repo.insert(&base).await.unwrap();
    // Move to PENDING first so Cancel and Expire are both eligible from PENDING.
    let at_pending = repo
        .apply_transition(
            &tenant,
            &id_c,
            Version::zero(),
            &LifecycleCommand::OpenPending,
            &LifecycleContext::empty(),
            11,
        )
        .await
        .unwrap();
    assert_eq!(at_pending.version(), Version::new(1));

    let repo_a = repo.clone();
    let repo_b = repo.clone();
    let tenant_a = tenant.clone();
    let tenant_b = tenant.clone();
    let id_a = id_c.clone();
    let id_b = id_c.clone();

    let t1 = tokio::spawn(async move {
        repo_a
            .apply_transition(
                &tenant_a,
                &id_a,
                Version::new(1),
                &LifecycleCommand::Cancel,
                &LifecycleContext::empty(),
                20,
            )
            .await
    });
    let t2 = tokio::spawn(async move {
        repo_b
            .apply_transition(
                &tenant_b,
                &id_b,
                Version::new(1),
                &LifecycleCommand::Expire,
                &LifecycleContext::empty(),
                21,
            )
            .await
    });

    let r1 = t1.await.unwrap();
    let r2 = t2.await.unwrap();
    let wins = [r1.is_ok(), r2.is_ok()].iter().filter(|x| **x).count();
    let losses = [r1.is_err(), r2.is_err()].iter().filter(|x| **x).count();
    assert_eq!(wins, 1, "exactly one concurrent mutation must commit");
    assert_eq!(losses, 1, "exactly one concurrent mutation must conflict");

    let final_tx = repo.find(&tenant, &id_c).await.unwrap().unwrap();
    assert_eq!(final_tx.version(), Version::new(2));
    assert!(matches!(
        final_tx.primary_state(),
        TransactionState::Cancelled | TransactionState::Expired
    ));

    close_pool(&pool).await;
}
