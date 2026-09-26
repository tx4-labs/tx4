//! PostgreSQL integration: idempotency, outbox fencing, PaymentAttempt + Mock (Phase-2C).

use std::collections::BTreeMap;
use std::sync::Arc;
use tx4_adapters_payment::{MockPaymentOutcome, MockPaymentProvider};
use tx4_application::{
    IdempotencyBeginOutcome, IdempotencyBeginRequest, IdempotencyRepository, IdempotencyStatus,
    OutboxJob, OutboxRepository, OutboxStatus, PaymentAttempt, PaymentAttemptRepository,
    PaymentAttemptStatus, PaymentIntent, PaymentProvider, TransactionRepository,
};
use tx4_config::Config;
use tx4_domain::{CurrencyId, Money, OperationId, TenantId, Transaction, TransactionId};
use tx4_persistence::{
    close_pool, connect_pool, outbox_next_attempt_unix_micros, run_migrations,
    PgIdempotencyRepository, PgOutboxRepository, PgPaymentAttemptRepository,
    PgTransactionRepository, DEFAULT_LEASE_SECS, DEFAULT_OUTBOX_BACKOFF_BASE_SECS,
    DEFAULT_OUTBOX_MAX_ATTEMPTS,
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
    mock.set_create_outcome(MockPaymentOutcome::Succeed);
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
    mock_u.set_create_outcome(MockPaymentOutcome::UnknownTimeout);
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
    mock_r.set_create_outcome(MockPaymentOutcome::Succeed);
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
