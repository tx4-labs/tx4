-- Phase-2C: durable idempotency reservation, transactional outbox, PaymentAttempt.
-- Normative: IA §9 / §11 / §12. No ledger, settlement, reconciliation, billing, or real providers.

-- ---------------------------------------------------------------------------
-- Idempotency reservations (IA §9)
-- ---------------------------------------------------------------------------
CREATE TABLE tx4_infra.idempotency_records (
    tenant_id           TEXT        NOT NULL,
    idempotency_key     TEXT        NOT NULL,
    operation_id        TEXT        NOT NULL,
    request_fingerprint TEXT        NOT NULL,
    transaction_id      TEXT,
    status              TEXT        NOT NULL,
    lease_owner         TEXT,
    lease_expires_at    TIMESTAMPTZ,
    response_status     TEXT,
    response_body       TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at          TIMESTAMPTZ,

    CONSTRAINT idempotency_records_pkey
        PRIMARY KEY (tenant_id, idempotency_key),

    CONSTRAINT idempotency_records_operation_unique
        UNIQUE (tenant_id, operation_id),

    CONSTRAINT idempotency_records_tenant_nonempty
        CHECK (char_length(btrim(tenant_id)) > 0),
    CONSTRAINT idempotency_records_key_nonempty
        CHECK (char_length(btrim(idempotency_key)) > 0),
    CONSTRAINT idempotency_records_operation_nonempty
        CHECK (char_length(btrim(operation_id)) > 0),
    CONSTRAINT idempotency_records_fingerprint_nonempty
        CHECK (char_length(btrim(request_fingerprint)) > 0),

    CONSTRAINT idempotency_records_status_check
        CHECK (status IN ('IN_PROGRESS', 'COMPLETED', 'FAILED_CLOSED')),

    CONSTRAINT idempotency_records_lease_pair
        CHECK (
            (status <> 'IN_PROGRESS')
            OR (
                lease_owner IS NOT NULL
                AND char_length(btrim(lease_owner)) > 0
                AND lease_expires_at IS NOT NULL
            )
        )
);

COMMENT ON TABLE tx4_infra.idempotency_records IS
  'Durable client/command idempotency reservation (Phase-2C / IA §9).';

CREATE INDEX idempotency_records_tenant_status_idx
    ON tx4_infra.idempotency_records (tenant_id, status);

-- ---------------------------------------------------------------------------
-- Transactional outbox / jobs (IA §11)
-- ---------------------------------------------------------------------------
CREATE TABLE tx4_infra.outbox_jobs (
    id                  TEXT        NOT NULL,
    tenant_id           TEXT        NOT NULL,
    job_type            TEXT        NOT NULL,
    payload             TEXT        NOT NULL,
    status              TEXT        NOT NULL,
    locked_by           TEXT,
    claim_epoch         BIGINT      NOT NULL DEFAULT 0,
    lease_expires_at    TIMESTAMPTZ,
    attempt_count       INTEGER     NOT NULL DEFAULT 0,
    next_attempt_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error          TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT outbox_jobs_pkey PRIMARY KEY (id),

    CONSTRAINT outbox_jobs_id_nonempty
        CHECK (char_length(btrim(id)) > 0),
    CONSTRAINT outbox_jobs_tenant_nonempty
        CHECK (char_length(btrim(tenant_id)) > 0),
    CONSTRAINT outbox_jobs_type_nonempty
        CHECK (char_length(btrim(job_type)) > 0),

    CONSTRAINT outbox_jobs_status_check
        CHECK (status IN ('PENDING', 'RUNNING', 'SUCCEEDED', 'DEAD_LETTER')),

    CONSTRAINT outbox_jobs_claim_epoch_nonneg
        CHECK (claim_epoch >= 0),
    CONSTRAINT outbox_jobs_attempt_count_nonneg
        CHECK (attempt_count >= 0),

    CONSTRAINT outbox_jobs_running_lease
        CHECK (
            (status <> 'RUNNING')
            OR (
                locked_by IS NOT NULL
                AND char_length(btrim(locked_by)) > 0
                AND lease_expires_at IS NOT NULL
            )
        )
);

COMMENT ON TABLE tx4_infra.outbox_jobs IS
  'Transactional outbox / async job table with claim_epoch fencing (Phase-2C / IA §11).';

CREATE INDEX outbox_jobs_claim_eligible_idx
    ON tx4_infra.outbox_jobs (next_attempt_at, id)
    WHERE status IN ('PENDING', 'RUNNING');

CREATE INDEX outbox_jobs_tenant_status_idx
    ON tx4_infra.outbox_jobs (tenant_id, status);

-- ---------------------------------------------------------------------------
-- PaymentAttempt (IA §12.4) — Mock-bound durability; not TX4 primary lifecycle SoT
-- ---------------------------------------------------------------------------
CREATE TABLE tx4_infra.payment_attempts (
    attempt_id                  TEXT        NOT NULL,
    tenant_id                   TEXT        NOT NULL,
    transaction_id              TEXT        NOT NULL,
    operation_id                TEXT        NOT NULL,
    idempotency_key             TEXT,
    provider_adapter            TEXT        NOT NULL,
    provider_ref                TEXT,
    provider_idempotency_key    TEXT,
    status                      TEXT        NOT NULL,
    outcome_disposition         TEXT,
    amount_atomic               BIGINT,
    currency_id                 TEXT,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT payment_attempts_pkey PRIMARY KEY (tenant_id, attempt_id),

    CONSTRAINT payment_attempts_operation_unique
        UNIQUE (tenant_id, operation_id),

    CONSTRAINT payment_attempts_tenant_nonempty
        CHECK (char_length(btrim(tenant_id)) > 0),
    CONSTRAINT payment_attempts_attempt_nonempty
        CHECK (char_length(btrim(attempt_id)) > 0),
    CONSTRAINT payment_attempts_transaction_nonempty
        CHECK (char_length(btrim(transaction_id)) > 0),
    CONSTRAINT payment_attempts_operation_nonempty
        CHECK (char_length(btrim(operation_id)) > 0),
    CONSTRAINT payment_attempts_adapter_nonempty
        CHECK (char_length(btrim(provider_adapter)) > 0),

    CONSTRAINT payment_attempts_status_check
        CHECK (status IN (
            'PREPARED',
            'SUBMITTED',
            'UNKNOWN',
            'SUCCEEDED',
            'FAILED'
        )),

    CONSTRAINT payment_attempts_money_pair
        CHECK (
            (amount_atomic IS NULL AND currency_id IS NULL)
            OR (
                amount_atomic IS NOT NULL
                AND currency_id IS NOT NULL
                AND char_length(btrim(currency_id)) > 0
            )
        ),

    CONSTRAINT payment_attempts_submitted_key
        CHECK (
            (status = 'PREPARED')
            OR (
                provider_idempotency_key IS NOT NULL
                AND char_length(btrim(provider_idempotency_key)) > 0
            )
        ),

    CONSTRAINT payment_attempts_transaction_fk
        FOREIGN KEY (tenant_id, transaction_id)
        REFERENCES tx4_infra.transactions (tenant_id, id)
);

COMMENT ON TABLE tx4_infra.payment_attempts IS
  'Durable PaymentAttempt operational record (Phase-2C / IA §12.4). Not TX4 primary lifecycle authority.';

CREATE INDEX payment_attempts_tenant_tx_idx
    ON tx4_infra.payment_attempts (tenant_id, transaction_id);

CREATE INDEX payment_attempts_tenant_status_idx
    ON tx4_infra.payment_attempts (tenant_id, status);
