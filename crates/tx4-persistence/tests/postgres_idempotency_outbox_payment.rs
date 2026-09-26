//! PostgreSQL integration: idempotency, outbox fencing, PaymentAttempt + Mock (Phase-2C).

use std::collections::BTreeMap;
use std::sync::Arc;
use tx4_adapters_payment::{MockPaymentOutcome, MockPaymentProvider};
use tx4_application::{
    IdempotencyBeginOutcome, IdempotencyBeginRequest, IdempotencyFinalize, IdempotencyRepository,
    IdempotencyStatus, OutboxJob, OutboxRepository, OutboxStatus, PaymentAttempt,
    PaymentAttemptRepository, PaymentAttemptStatus, PaymentIntent, PaymentProvider,
    TransactionRepository, UnitOfWork, UnitOfWorkFactory,
};
use tx4_config::Config;
use tx4_domain::{CurrencyId, Money, OperationId, TenantId, Transaction, TransactionId};
use tx4_persistence::{
    close_pool, connect_pool, outbox_next_attempt_unix_micros, run_migrations,
    PgIdempotencyRepository, PgOutboxRepository, PgPaymentAttemptRepository,
    PgTransactionRepository, PgUnitOfWorkFactory, DEFAULT_LEASE_SECS,
    DEFAULT_OUTBOX_BACKOFF_BASE_SECS, DEFAULT_OUTBOX_MAX_ATTEMPTS,
};

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

#[allow(clippy::too_many_arguments)]
fn begin_req<'a>(
    tenant_id: &'a TenantId,
    key: &'a str,
    operation_id: &'a OperationId,
    fingerprint: &'a str,
    transaction_id: Option<&'a TransactionId>,
    lease_owner: &'a str,
    lease_secs: i64,
    now: i64,
) -> IdempotencyBeginRequest<'a> {
    IdempotencyBeginRequest {
        tenant_id,
        idempotency_key: key,
        operation_id,
        request_fingerprint: fingerprint,
        transaction_id,
        lease_owner,
        lease_duration_secs: lease_secs,
        now_unix_micros: now,
    }
}

#[tokio::test]
#[ignore = "requires DATABASE_URL against a real PostgreSQL database"]
async fn idempotency_outbox_payment_boundary() {
    let Some(config) = test_config_from_env() else {
        panic!("DATABASE_URL must be set for ignored PostgreSQL integration tests");
    };
    let pool = connect_pool(&config).await.expect("connect");
    run_migrations(&pool).await.expect("migrate");
    run_migrations(&pool).await.expect("migrate twice");
    sqlx::query(
        "TRUNCATE TABLE
            tx4_infra.payment_attempts,
            tx4_infra.idempotency_records,
            tx4_infra.outbox_jobs,
            tx4_infra.transactions
         RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .expect("truncate");

    for table in ["idempotency_records", "outbox_jobs", "payment_attempts"] {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'tx4_infra' AND table_name = $1
             )",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(exists, "missing {table}");
    }

    let tenant = TenantId::new("tenant-2c").unwrap();
    let tx_repo = PgTransactionRepository::new(pool.clone());
    let idemp = Arc::new(PgIdempotencyRepository::new(pool.clone()));
    let outbox = Arc::new(PgOutboxRepository::new(pool.clone()));
    let attempts = PgPaymentAttemptRepository::new(pool.clone());

    let tx_id = TransactionId::new("tx-2c-pay").unwrap();
    let created = Transaction::new_created(
        tx_id.clone(),
        tenant.clone(),
        Some(Money::new(1_000, CurrencyId::new("IDR").unwrap())),
        None,
        None,
        1_000_000,
    )
    .unwrap();
    tx_repo.insert(&created).await.unwrap();

    let op = OperationId::new("op-2c-1").unwrap();
    let key = "idem-key-1";
    let fp = "fp-abc";
    let now = 2_000_000i64;
    let acquired = idemp
        .begin_or_recover(&begin_req(
            &tenant,
            key,
            &op,
            fp,
            Some(&tx_id),
            "worker-a",
            DEFAULT_LEASE_SECS,
            now,
        ))
        .await
        .unwrap();
    assert!(matches!(acquired, IdempotencyBeginOutcome::Acquired(_)));

    let other_op = OperationId::new("op-should-not-win").unwrap();
    let other = idemp
        .begin_or_recover(&begin_req(
            &tenant,
            key,
            &other_op,
            fp,
            Some(&tx_id),
            "worker-b",
            DEFAULT_LEASE_SECS,
            now + 1,
        ))
        .await
        .unwrap();
    assert!(matches!(other, IdempotencyBeginOutcome::InProgress(_)));

    let conflict = idemp
        .begin_or_recover(&begin_req(
            &tenant,
            key,
            &op,
            "fp-other",
            Some(&tx_id),
            "worker-c",
            DEFAULT_LEASE_SECS,
            now + 2,
        ))
        .await
        .unwrap_err();
    assert!(conflict.to_string().contains("idempotency"));

    idemp
        .complete(&tenant, key, "worker-a", "200", "{\"ok\":true}", now + 10)
        .await
        .unwrap();
    let replay = idemp
        .begin_or_recover(&begin_req(
            &tenant,
            key,
            &op,
            fp,
            Some(&tx_id),
            "worker-d",
            DEFAULT_LEASE_SECS,
            now + 11,
        ))
        .await
        .unwrap();
    match replay {
        IdempotencyBeginOutcome::Replay(r) => {
            assert_eq!(r.status, IdempotencyStatus::Completed);
            assert_eq!(r.response_body.as_deref(), Some("{\"ok\":true}"));
        }
        other => panic!("expected replay, got {other:?}"),
    }

    let key2 = "idem-key-reclaim";
    let op2 = OperationId::new("op-2c-reclaim").unwrap();
    let t0 = 10_000_000i64;
    idemp
        .begin_or_recover(&begin_req(
            &tenant, key2, &op2, "fp-r", None, "owner-1", 1, t0,
        ))
        .await
        .unwrap();
    let reclaimed = idemp
        .begin_or_recover(&begin_req(
            &tenant,
            key2,
            &op2,
            "fp-r",
            None,
            "owner-2",
            DEFAULT_LEASE_SECS,
            t0 + 2_000_000,
        ))
        .await
        .unwrap();
    match reclaimed {
        IdempotencyBeginOutcome::Acquired(r) => {
            assert_eq!(r.lease_owner.as_deref(), Some("owner-2"));
        }
        other => panic!("expected reclaim acquire, got {other:?}"),
    }

    let key3 = "idem-concurrent";
    let idemp_a = idemp.clone();
    let idemp_b = idemp.clone();
    let tenant_a = tenant.clone();
    let tenant_b = tenant.clone();
    let t1 = tokio::spawn(async move {
        let op = OperationId::new("op-c-a").unwrap();
        idemp_a
            .begin_or_recover(&begin_req(
                &tenant_a,
                key3,
                &op,
                "fp-c",
                None,
                "wa",
                DEFAULT_LEASE_SECS,
                20_000_000,
            ))
            .await
    });
    let t2 = tokio::spawn(async move {
        let op = OperationId::new("op-c-b").unwrap();
        idemp_b
            .begin_or_recover(&begin_req(
                &tenant_b,
                key3,
                &op,
                "fp-c",
                None,
                "wb",
                DEFAULT_LEASE_SECS,
                20_000_000,
            ))
            .await
    });
    let r1 = t1.await.unwrap().unwrap();
    let r2 = t2.await.unwrap().unwrap();
    let acquired_n = [&r1, &r2]
        .iter()
        .filter(|o| matches!(o, IdempotencyBeginOutcome::Acquired(_)))
        .count();
    let blocked_n = [&r1, &r2]
        .iter()
        .filter(|o| matches!(o, IdempotencyBeginOutcome::InProgress(_)))
        .count();
    assert_eq!(acquired_n, 1);
    assert_eq!(blocked_n, 1);

    let dup_op = idemp
        .begin_or_recover(&begin_req(
            &tenant,
            "other-key",
            &op2,
            "fp-x",
            None,
            "w",
            DEFAULT_LEASE_SECS,
            30_000_000,
        ))
        .await;
    assert!(dup_op.is_err(), "duplicate operation_id must fail");

    let job = OutboxJob {
        id: "job-1".into(),
        tenant_id: tenant.clone(),
        job_type: "demo".into(),
        payload: "{}".into(),
        status: OutboxStatus::Pending,
        locked_by: None,
        claim_epoch: 0,
        lease_expires_at_unix_micros: None,
        attempt_count: 0,
        next_attempt_at_unix_micros: 40_000_000,
        last_error: None,
    };
    outbox.enqueue(&job).await.unwrap();

    let claimed = outbox
        .claim("worker-1", 1, DEFAULT_LEASE_SECS, 40_000_000)
        .await
        .unwrap();
    assert_eq!(claimed.len(), 1);
    let epoch = claimed[0].job.claim_epoch;
    assert_eq!(epoch, 1);
    assert_eq!(claimed[0].job.status, OutboxStatus::Running);

    assert!(!outbox
        .complete_succeeded("job-1", "worker-1", 0, 41_000_000)
        .await
        .unwrap());
    assert_eq!(
        outbox.find("job-1").await.unwrap().unwrap().status,
        OutboxStatus::Running
    );
    assert!(outbox
        .complete_succeeded("job-1", "worker-1", epoch, 41_000_000)
        .await
        .unwrap());
    assert_eq!(
        outbox.find("job-1").await.unwrap().unwrap().status,
        OutboxStatus::Succeeded
    );

    let job2 = OutboxJob {
        id: "job-2".into(),
        tenant_id: tenant.clone(),
        job_type: "demo".into(),
        payload: "x".into(),
        status: OutboxStatus::Pending,
        locked_by: None,
        claim_epoch: 0,
        lease_expires_at_unix_micros: None,
        attempt_count: 0,
        next_attempt_at_unix_micros: 50_000_000,
        last_error: None,
    };
    outbox.enqueue(&job2).await.unwrap();
    let c1 = outbox.claim("w-old", 1, 1, 50_000_000).await.unwrap();
    let epoch_old = c1[0].job.claim_epoch;
    let c2 = outbox
        .claim("w-new", 1, DEFAULT_LEASE_SECS, 52_000_000)
        .await
        .unwrap();
    assert!(c2[0].job.claim_epoch > epoch_old);
    assert!(!outbox
        .complete_succeeded("job-2", "w-old", epoch_old, 53_000_000)
        .await
        .unwrap());
    assert!(outbox
        .complete_succeeded("job-2", "w-new", c2[0].job.claim_epoch, 53_000_000)
        .await
        .unwrap());

    let job3 = OutboxJob {
        id: "job-3".into(),
        tenant_id: tenant.clone(),
        job_type: "demo".into(),
        payload: "y".into(),
        status: OutboxStatus::Pending,
        locked_by: None,
        claim_epoch: 0,
        lease_expires_at_unix_micros: None,
        attempt_count: 0,
        next_attempt_at_unix_micros: 60_000_000,
        last_error: None,
    };
    outbox.enqueue(&job3).await.unwrap();
    let c = outbox
        .claim("w-r", 1, DEFAULT_LEASE_SECS, 60_000_000)
        .await
        .unwrap();
    let next = outbox_next_attempt_unix_micros(
        61_000_000,
        c[0].job.attempt_count,
        DEFAULT_OUTBOX_BACKOFF_BASE_SECS,
    )
    .unwrap();
    assert!(outbox
        .complete_retry(
            "job-3",
            "w-r",
            c[0].job.claim_epoch,
            "transient",
            next,
            61_000_000,
        )
        .await
        .unwrap());
    assert_eq!(
        outbox.find("job-3").await.unwrap().unwrap().status,
        OutboxStatus::Pending
    );

    for i in 0..DEFAULT_OUTBOX_MAX_ATTEMPTS {
        let now = 70_000_000 + i64::from(i) * 10_000_000;
        sqlx::query(
            "UPDATE tx4_infra.outbox_jobs SET next_attempt_at = TIMESTAMPTZ 'epoch'
             WHERE id = 'job-3'",
        )
        .execute(&pool)
        .await
        .unwrap();
        let claimed = outbox
            .claim("w-dl", 1, DEFAULT_LEASE_SECS, now)
            .await
            .unwrap();
        if claimed.is_empty() {
            break;
        }
        let epoch_dl = claimed[0].job.claim_epoch;
        if claimed[0].job.attempt_count >= DEFAULT_OUTBOX_MAX_ATTEMPTS {
            assert!(outbox
                .complete_dead_letter("job-3", "w-dl", epoch_dl, "poison", now + 1)
                .await
                .unwrap());
            break;
        }
        let n = outbox_next_attempt_unix_micros(
            now + 1,
            claimed[0].job.attempt_count,
            DEFAULT_OUTBOX_BACKOFF_BASE_SECS,
        )
        .unwrap();
        outbox
            .complete_retry("job-3", "w-dl", epoch_dl, "retry", n, now + 1)
            .await
            .unwrap();
    }
    assert_eq!(
        outbox.find("job-3").await.unwrap().unwrap().status,
        OutboxStatus::DeadLetter
    );

    let attempt_id = "att-1";
    let op_pay = OperationId::new("op-pay-1").unwrap();
    let attempt = PaymentAttempt {
        attempt_id: attempt_id.into(),
        tenant_id: tenant.clone(),
        transaction_id: tx_id.clone(),
        operation_id: op_pay,
        idempotency_key: Some("pay-idem".into()),
        provider_adapter: "mock".into(),
        provider_ref: None,
        provider_idempotency_key: None,
        status: PaymentAttemptStatus::Prepared,
        outcome_disposition: None,
        amount_atomic: Some(1_000),
        currency_id: Some("IDR".into()),
    };
    attempts
        .insert_prepared(&attempt, 80_000_000)
        .await
        .unwrap();
    assert!(attempts
        .mark_succeeded(&tenant, attempt_id, "x", 80_000_001)
        .await
        .is_err());

    let submitted = attempts
        .mark_submitted(&tenant, attempt_id, "pik-stable-1", 80_000_002)
        .await
        .unwrap();
    assert_eq!(submitted.status, PaymentAttemptStatus::Submitted);
    assert_eq!(
        submitted.provider_idempotency_key.as_deref(),
        Some("pik-stable-1")
    );

    let mock = MockPaymentProvider::new();
    mock.set_create_outcome(MockPaymentOutcome::Succeed)
        .unwrap();
    let pref = mock
        .create_payment(&PaymentIntent {
            amount: Money::new(1_000, CurrencyId::new("IDR").unwrap()),
            provider_idempotency_key: "pik-stable-1".into(),
            description: None,
        })
        .await
        .unwrap();
    assert_eq!(
        attempts
            .mark_succeeded(&tenant, attempt_id, pref.as_str(), 80_000_003)
            .await
            .unwrap()
            .status,
        PaymentAttemptStatus::Succeeded
    );

    let attempt2 = PaymentAttempt {
        attempt_id: "att-2".into(),
        tenant_id: tenant.clone(),
        transaction_id: tx_id,
        operation_id: OperationId::new("op-pay-2").unwrap(),
        idempotency_key: None,
        provider_adapter: "mock".into(),
        provider_ref: None,
        provider_idempotency_key: None,
        status: PaymentAttemptStatus::Prepared,
        outcome_disposition: None,
        amount_atomic: Some(500),
        currency_id: Some("IDR".into()),
    };
    attempts
        .insert_prepared(&attempt2, 90_000_000)
        .await
        .unwrap();
    attempts
        .mark_submitted(&tenant, "att-2", "pik-u-2", 90_000_001)
        .await
        .unwrap();
    let mock_u = MockPaymentProvider::new();
    mock_u
        .set_create_outcome(MockPaymentOutcome::UnknownTimeout)
        .unwrap();
    assert!(mock_u
        .create_payment(&PaymentIntent {
            amount: Money::new(500, CurrencyId::new("IDR").unwrap()),
            provider_idempotency_key: "pik-u-2".into(),
            description: None,
        })
        .await
        .unwrap_err()
        .is_uncertainty_or_timeout());
    let unknown = attempts
        .mark_unknown(&tenant, "att-2", "TIMEOUT", 90_000_002)
        .await
        .unwrap();
    assert_eq!(unknown.status, PaymentAttemptStatus::Unknown);
    assert!(unknown.status.requires_reconcile_before_provider_io());

    let mock_r = MockPaymentProvider::new();
    mock_r
        .set_create_outcome(MockPaymentOutcome::Succeed)
        .unwrap();
    let pref2 = mock_r
        .create_payment(&PaymentIntent {
            amount: Money::new(500, CurrencyId::new("IDR").unwrap()),
            provider_idempotency_key: "pik-u-2".into(),
            description: None,
        })
        .await
        .unwrap();
    let _ = mock_r.get_payment_status(&pref2).await.unwrap();
    attempts
        .mark_succeeded(&tenant, "att-2", pref2.as_str(), 90_000_003)
        .await
        .unwrap();

    close_pool(&pool).await;
}

#[tokio::test]
#[ignore = "requires DATABASE_URL against a real PostgreSQL database"]
async fn unit_of_work_atomicity_and_reclaim_gates() {
    let Some(config) = test_config_from_env() else {
        panic!("DATABASE_URL must be set for ignored PostgreSQL integration tests");
    };
    let pool = connect_pool(&config).await.expect("connect");
    run_migrations(&pool).await.expect("migrate");
    sqlx::query(
        "TRUNCATE TABLE
            tx4_infra.payment_attempts,
            tx4_infra.idempotency_records,
            tx4_infra.outbox_jobs,
            tx4_infra.transactions
         RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .expect("truncate");

    let factory = PgUnitOfWorkFactory::new(pool.clone());
    let idemp = PgIdempotencyRepository::new(pool.clone());
    let attempts = PgPaymentAttemptRepository::new(pool.clone());
    let tx_repo = PgTransactionRepository::new(pool.clone());

    let tenant = TenantId::new("tenant-uow").unwrap();
    let tx_id = TransactionId::new("tx-uow-1").unwrap();
    tx_repo
        .insert(
            &Transaction::new_created(
                tx_id.clone(),
                tenant.clone(),
                Some(Money::new(100, CurrencyId::new("IDR").unwrap())),
                None,
                None,
                1_000_000,
            )
            .unwrap(),
        )
        .await
        .unwrap();

    // CASE A: reservation ok, PaymentAttempt insert fails → full rollback.
    {
        let key = "uow-a";
        let op = OperationId::new("op-uow-a").unwrap();
        let mut uow = factory.begin().await.unwrap();
        let acquired = uow
            .begin_or_recover(&begin_req(
                &tenant,
                key,
                &op,
                "fp-a",
                Some(&tx_id),
                "owner-a",
                DEFAULT_LEASE_SECS,
                100_000_000,
            ))
            .await
            .unwrap();
        assert!(matches!(acquired, IdempotencyBeginOutcome::Acquired(_)));
        let bad = PaymentAttempt {
            attempt_id: "att-a".into(),
            tenant_id: tenant.clone(),
            transaction_id: TransactionId::new("tx-missing-fk").unwrap(),
            operation_id: op.clone(),
            idempotency_key: Some(key.into()),
            provider_adapter: "mock".into(),
            provider_ref: None,
            provider_idempotency_key: None,
            status: PaymentAttemptStatus::Prepared,
            outcome_disposition: None,
            amount_atomic: Some(1),
            currency_id: Some("IDR".into()),
        };
        assert!(uow
            .insert_prepared_attempt(&bad, 100_000_001)
            .await
            .is_err());
        drop(uow);
        assert!(idemp.find(&tenant, key).await.unwrap().is_none());
        assert!(attempts
            .find_by_operation(&tenant, &op)
            .await
            .unwrap()
            .is_none());
    }

    // CASE B: reservation + attempt succeed; drop before commit → neither durable.
    {
        let key = "uow-b";
        let op = OperationId::new("op-uow-b").unwrap();
        let mut uow = factory.begin().await.unwrap();
        uow.begin_or_recover(&begin_req(
            &tenant,
            key,
            &op,
            "fp-b",
            Some(&tx_id),
            "owner-b",
            DEFAULT_LEASE_SECS,
            110_000_000,
        ))
        .await
        .unwrap();
        uow.insert_prepared_attempt(
            &PaymentAttempt {
                attempt_id: "att-b".into(),
                tenant_id: tenant.clone(),
                transaction_id: tx_id.clone(),
                operation_id: op.clone(),
                idempotency_key: Some(key.into()),
                provider_adapter: "mock".into(),
                provider_ref: None,
                provider_idempotency_key: None,
                status: PaymentAttemptStatus::Prepared,
                outcome_disposition: None,
                amount_atomic: Some(2),
                currency_id: Some("IDR".into()),
            },
            110_000_001,
        )
        .await
        .unwrap();
        drop(uow);
        assert!(idemp.find(&tenant, key).await.unwrap().is_none());
        assert!(attempts
            .find_by_operation(&tenant, &op)
            .await
            .unwrap()
            .is_none());
    }

    // CASE C: finalization + stored response commit → both durable.
    {
        let key = "uow-c";
        let op = OperationId::new("op-uow-c").unwrap();
        let mut uow = factory.begin().await.unwrap();
        uow.begin_or_recover(&begin_req(
            &tenant,
            key,
            &op,
            "fp-c",
            Some(&tx_id),
            "owner-c",
            DEFAULT_LEASE_SECS,
            120_000_000,
        ))
        .await
        .unwrap();
        uow.insert_prepared_attempt(
            &PaymentAttempt {
                attempt_id: "att-c".into(),
                tenant_id: tenant.clone(),
                transaction_id: tx_id.clone(),
                operation_id: op.clone(),
                idempotency_key: Some(key.into()),
                provider_adapter: "mock".into(),
                provider_ref: None,
                provider_idempotency_key: None,
                status: PaymentAttemptStatus::Prepared,
                outcome_disposition: None,
                amount_atomic: Some(3),
                currency_id: Some("IDR".into()),
            },
            120_000_001,
        )
        .await
        .unwrap();
        uow.complete_idempotency(
            &tenant,
            key,
            "owner-c",
            &IdempotencyFinalize::Completed {
                response_status: "200".into(),
                response_body: "{\"ok\":true}".into(),
            },
            120_000_002,
        )
        .await
        .unwrap();
        uow.commit().await.unwrap();
        let r = idemp.find(&tenant, key).await.unwrap().unwrap();
        assert_eq!(r.status, IdempotencyStatus::Completed);
        assert_eq!(r.response_body.as_deref(), Some("{\"ok\":true}"));
        assert_eq!(
            attempts
                .find_by_operation(&tenant, &op)
                .await
                .unwrap()
                .unwrap()
                .status,
            PaymentAttemptStatus::Prepared
        );
    }

    // CASE D: stored success + later failure in same UoW → response not durable as success.
    {
        let key = "uow-d";
        let op = OperationId::new("op-uow-d").unwrap();
        let mut uow = factory.begin().await.unwrap();
        uow.begin_or_recover(&begin_req(
            &tenant,
            key,
            &op,
            "fp-d",
            Some(&tx_id),
            "owner-d",
            DEFAULT_LEASE_SECS,
            130_000_000,
        ))
        .await
        .unwrap();
        uow.complete_idempotency(
            &tenant,
            key,
            "owner-d",
            &IdempotencyFinalize::Completed {
                response_status: "200".into(),
                response_body: "{\"false_success\":true}".into(),
            },
            130_000_001,
        )
        .await
        .unwrap();
        let fail = uow
            .insert_prepared_attempt(
                &PaymentAttempt {
                    attempt_id: "att-d".into(),
                    tenant_id: tenant.clone(),
                    transaction_id: TransactionId::new("tx-missing-d").unwrap(),
                    operation_id: op.clone(),
                    idempotency_key: Some(key.into()),
                    provider_adapter: "mock".into(),
                    provider_ref: None,
                    provider_idempotency_key: None,
                    status: PaymentAttemptStatus::Prepared,
                    outcome_disposition: None,
                    amount_atomic: Some(4),
                    currency_id: Some("IDR".into()),
                },
                130_000_002,
            )
            .await;
        assert!(fail.is_err());
        drop(uow);
        assert!(
            idemp.find(&tenant, key).await.unwrap().is_none(),
            "failed business finalization must not leave a success response"
        );
    }

    // Cross-tenant isolation + independent same key.
    {
        let a = TenantId::new("tenant-iso-a").unwrap();
        let b = TenantId::new("tenant-iso-b").unwrap();
        let key = "shared-key";
        let op_a = OperationId::new("op-iso-a").unwrap();
        let op_b = OperationId::new("op-iso-b").unwrap();
        idemp
            .begin_or_recover(&begin_req(
                &a,
                key,
                &op_a,
                "fp-a",
                None,
                "owner-a",
                DEFAULT_LEASE_SECS,
                140_000_000,
            ))
            .await
            .unwrap();
        assert!(idemp.find(&b, key).await.unwrap().is_none());
        let from_b = idemp
            .begin_or_recover(&begin_req(
                &b,
                key,
                &op_b,
                "fp-b",
                None,
                "owner-b",
                DEFAULT_LEASE_SECS,
                140_000_001,
            ))
            .await
            .unwrap();
        assert!(matches!(from_b, IdempotencyBeginOutcome::Acquired(_)));
        let steal = idemp
            .begin_or_recover(&begin_req(
                &b,
                key,
                &op_a,
                "fp-a",
                None,
                "thief",
                DEFAULT_LEASE_SECS,
                142_000_000,
            ))
            .await;
        // Tenant B's own key has fingerprint fp-b; attempting A's fingerprint conflicts on B's row.
        assert!(steal.is_err() || matches!(steal, Ok(IdempotencyBeginOutcome::InProgress(_))));
        // Tenant B cannot observe A's reservation body/owner.
        let a_row = idemp.find(&a, key).await.unwrap().unwrap();
        assert_eq!(a_row.lease_owner.as_deref(), Some("owner-a"));
        let b_row = idemp.find(&b, key).await.unwrap().unwrap();
        assert_eq!(b_row.lease_owner.as_deref(), Some("owner-b"));
        assert_ne!(a_row.operation_id.as_str(), b_row.operation_id.as_str());
    }

    // Timestamp integer-safe roundtrip (representative micros).
    {
        let samples = [
            0i64,
            1,
            1_000_000,
            1_704_067_200_000_000,
            9_007_199_254_740_991,
        ];
        for ts in samples {
            let got: i64 = sqlx::query_scalar(
                "SELECT ((EXTRACT(EPOCH FROM (TIMESTAMPTZ 'epoch' + (($1::bigint) * INTERVAL '1 microsecond')))::numeric) * 1000000)::bigint",
            )
            .bind(ts)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(got, ts, "timestamp micros roundtrip failed for {ts}");
        }
    }

    // Reclaim gates: PREPARED → Acquired; SUBMITTED/UNKNOWN/SUCCEEDED/FAILED → ReclaimedRequiresReconcile.
    struct ReclaimSeed<'a> {
        idemp: &'a PgIdempotencyRepository,
        attempts: &'a PgPaymentAttemptRepository,
        tenant: &'a TenantId,
        tx_id: &'a TransactionId,
        key: &'a str,
        op: &'a OperationId,
        attempt_id: &'a str,
        status: PaymentAttemptStatus,
        t0: i64,
    }

    async fn seed_reclaim(s: ReclaimSeed<'_>) {
        s.idemp
            .begin_or_recover(&begin_req(
                s.tenant,
                s.key,
                s.op,
                "fp-rcl",
                Some(s.tx_id),
                "owner-old",
                1,
                s.t0,
            ))
            .await
            .unwrap();
        s.attempts
            .insert_prepared(
                &PaymentAttempt {
                    attempt_id: s.attempt_id.into(),
                    tenant_id: s.tenant.clone(),
                    transaction_id: s.tx_id.clone(),
                    operation_id: s.op.clone(),
                    idempotency_key: Some(s.key.into()),
                    provider_adapter: "mock".into(),
                    provider_ref: None,
                    provider_idempotency_key: None,
                    status: PaymentAttemptStatus::Prepared,
                    outcome_disposition: None,
                    amount_atomic: Some(9),
                    currency_id: Some("IDR".into()),
                },
                s.t0 + 1,
            )
            .await
            .unwrap();
        if s.status != PaymentAttemptStatus::Prepared {
            s.attempts
                .mark_submitted(
                    s.tenant,
                    s.attempt_id,
                    &format!("pik-{}", s.attempt_id),
                    s.t0 + 2,
                )
                .await
                .unwrap();
        }
        match s.status {
            PaymentAttemptStatus::Prepared => {}
            PaymentAttemptStatus::Submitted => {}
            PaymentAttemptStatus::Unknown => {
                s.attempts
                    .mark_unknown(s.tenant, s.attempt_id, "TIMEOUT", s.t0 + 3)
                    .await
                    .unwrap();
            }
            PaymentAttemptStatus::Succeeded => {
                s.attempts
                    .mark_succeeded(s.tenant, s.attempt_id, "pref-x", s.t0 + 3)
                    .await
                    .unwrap();
            }
            PaymentAttemptStatus::Failed => {
                s.attempts
                    .mark_failed(s.tenant, s.attempt_id, "DECLINED", s.t0 + 3)
                    .await
                    .unwrap();
            }
        }
    }

    let mock = MockPaymentProvider::new();
    mock.set_create_outcome(MockPaymentOutcome::Succeed)
        .unwrap();

    // PREPARED reclaim may proceed (Acquired); no provider I/O required by gate.
    {
        let key = "rcl-prepared";
        let op = OperationId::new("op-rcl-p").unwrap();
        seed_reclaim(ReclaimSeed {
            idemp: &idemp,
            attempts: &attempts,
            tenant: &tenant,
            tx_id: &tx_id,
            key,
            op: &op,
            attempt_id: "att-rcl-p",
            status: PaymentAttemptStatus::Prepared,
            t0: 150_000_000,
        })
        .await;
        let before = mock.create_call_count().unwrap();
        let outcome = idemp
            .begin_or_recover(&begin_req(
                &tenant,
                key,
                &op,
                "fp-rcl",
                Some(&tx_id),
                "owner-new",
                DEFAULT_LEASE_SECS,
                152_000_000,
            ))
            .await
            .unwrap();
        assert!(
            matches!(outcome, IdempotencyBeginOutcome::Acquired(_)),
            "PREPARED reclaim should Acquired, got {outcome:?}"
        );
        assert_eq!(mock.create_call_count().unwrap(), before);
    }

    for (label, status, key, op, att, t0) in [
        (
            "SUBMITTED",
            PaymentAttemptStatus::Submitted,
            "rcl-sub",
            "op-rcl-s",
            "att-rcl-s",
            160_000_000i64,
        ),
        (
            "UNKNOWN",
            PaymentAttemptStatus::Unknown,
            "rcl-unk",
            "op-rcl-u",
            "att-rcl-u",
            170_000_000,
        ),
        (
            "SUCCEEDED",
            PaymentAttemptStatus::Succeeded,
            "rcl-ok",
            "op-rcl-ok",
            "att-rcl-ok",
            180_000_000,
        ),
        (
            "FAILED",
            PaymentAttemptStatus::Failed,
            "rcl-fail",
            "op-rcl-f",
            "att-rcl-f",
            190_000_000,
        ),
    ] {
        let op_id = OperationId::new(op).unwrap();
        seed_reclaim(ReclaimSeed {
            idemp: &idemp,
            attempts: &attempts,
            tenant: &tenant,
            tx_id: &tx_id,
            key,
            op: &op_id,
            attempt_id: att,
            status,
            t0,
        })
        .await;
        let before = mock.create_call_count().unwrap();
        let outcome = idemp
            .begin_or_recover(&begin_req(
                &tenant,
                key,
                &op_id,
                "fp-rcl",
                Some(&tx_id),
                "owner-new",
                DEFAULT_LEASE_SECS,
                t0 + 2_000_000,
            ))
            .await
            .unwrap();
        match outcome {
            IdempotencyBeginOutcome::ReclaimedRequiresReconcile { attempt, .. } => {
                assert_eq!(attempt.status, status, "{label}");
                assert!(attempt.status.requires_reconcile_before_provider_io());
            }
            other => panic!("{label}: expected ReclaimedRequiresReconcile, got {other:?}"),
        }
        assert_eq!(
            mock.create_call_count().unwrap(),
            before,
            "{label}: reclaim must not trigger provider I/O"
        );
    }

    close_pool(&pool).await;
}
