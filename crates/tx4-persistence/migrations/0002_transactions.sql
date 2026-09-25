-- Phase-2A: durable transaction aggregate current-state table (IA §7.2 / §4.1.1A).
-- Mutable operational authoritative primary lifecycle state + version.
-- No idempotency, outbox, PaymentAttempt, ledger, or repository logic in this migration.

-- Money columns use BIGINT atomic units + explicit currency text (ADR-006).
-- Mapping: Rust Money.amount (i64) ↔ amount_atomic BIGINT (lossless for full i64 range).
-- Mapping: Rust Version (u64) ↔ version BIGINT with CHECK (version >= 0).
--   Application layer must fail closed if a version exceeds BIGINT / i64-safe persistence
--   bounds when reading/writing; typical monotonic versions remain well within range.
-- primary_state values match ADR-007 / tx4_domain::TransactionState Display strings exactly.
-- No FLOAT / REAL / DOUBLE PRECISION monetary columns.

CREATE TABLE tx4_infra.transactions (
    id              TEXT        NOT NULL,
    tenant_id       TEXT        NOT NULL,
    primary_state   TEXT        NOT NULL,
    version         BIGINT      NOT NULL,
    amount_atomic   BIGINT,
    currency_id     TEXT,
    expires_at      TIMESTAMPTZ,
    attempt_budget  INTEGER,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT transactions_pkey PRIMARY KEY (tenant_id, id),

    CONSTRAINT transactions_id_nonempty
        CHECK (char_length(btrim(id)) > 0),

    CONSTRAINT transactions_tenant_nonempty
        CHECK (char_length(btrim(tenant_id)) > 0),

    CONSTRAINT transactions_primary_state_check
        CHECK (primary_state IN (
            'CREATED',
            'PENDING',
            'PAID',
            'PROCESSING',
            'COMPLETED',
            'CANCELLED',
            'EXPIRED',
            'FAILED'
        )),

    CONSTRAINT transactions_version_nonneg
        CHECK (version >= 0),

    CONSTRAINT transactions_money_pair
        CHECK (
            (amount_atomic IS NULL AND currency_id IS NULL)
            OR (
                amount_atomic IS NOT NULL
                AND currency_id IS NOT NULL
                AND char_length(btrim(currency_id)) > 0
            )
        ),

    CONSTRAINT transactions_attempt_budget_check
        CHECK (attempt_budget IS NULL OR attempt_budget >= 1)
);

COMMENT ON TABLE tx4_infra.transactions IS
  'Authoritative mutable TX4 transaction aggregate current state (Phase-2A schema foundation).';

COMMENT ON COLUMN tx4_infra.transactions.amount_atomic IS
  'ADR-006 signed integer atomic units (i64). NULL when no amount recorded yet.';

COMMENT ON COLUMN tx4_infra.transactions.currency_id IS
  'Explicit ADR-006 currency identity; required when amount_atomic is set.';

COMMENT ON COLUMN tx4_infra.transactions.version IS
  'Monotonic aggregate version for later FOR UPDATE / CAS workflows (not implemented in Phase-2A).';

CREATE INDEX transactions_tenant_state_idx
    ON tx4_infra.transactions (tenant_id, primary_state);
