# TX4 Implementation Architecture Specification

| Field | Value |
| --- | --- |
| Document | **IMPLEMENTATION-ARCHITECTURE** |
| Status | **PROPOSED** (awaiting AUDIT-021) |
| Date | 2026-09-25 |
| Authorizing task | TASK-018 |
| Baseline | `ec3b80dafd8328b363a89bbb5b7e4b9ea83508a9` |
| Authorization basis | AUDIT-020 PASS; Architecture Decision Phase COMPLETE |
| Scope | Implementation architecture specification only — **not** implementation authorization |

---

## 0. Authority and Purpose

### 0.1 Product principle

```text
Open-source transaction infrastructure for developers.
```

TX4 spans durable transactional infrastructure:

```text
Transaction → Payment → Completion → Fee → Usage/Metering
→ Billing → Ledger → Settlement → Reconciliation
```

TX4 is **not** a payment processor, accounting ERP, bank, regulated FI, billing-only SaaS, or vertical application. Infrastructure is the product. Dogfood apps are consumers.

### 0.2 Frozen ADR authorities (non-negotiable)

| ADR | Decision (summary) | Status |
| --- | --- | --- |
| ADR-001 | Core runtime = **Rust** | FROZEN |
| ADR-002 | OSS license = **Apache-2.0** | FROZEN |
| ADR-003 | Dependency / third-party license policy | FROZEN |
| ADR-004 | Source-of-truth, durability, idempotency, reconciliation boundaries | FROZEN |
| ADR-005 | URI path versioning `/v{N}/...`; initial `/v1/...` | FROZEN |
| ADR-006 | Signed integer atomic units + currency; JSON amount as integer string; half-even; deterministic allocation | FROZEN |
| ADR-007 | Eight-state lifecycle; Option C refunds-as-records; precedence; uncertainty; compensating intent | FROZEN |
| ADR-008 | Non-crippled OSS; Cloud = ops/UX/control plane; shared conceptual `/vN` core | FROZEN |

**Rule:** This document may select *implementation technologies* and *concrete mechanisms*. It MUST NOT reopen, weaken, or contradict ADR-001…008. If a conflict appears: STOP — do not rewrite ADRs.

### 0.3 Document status language

| Label | Meaning |
| --- | --- |
| **IA-FROZEN** | Selected by this specification for initial production implementation (subject to AUDIT-021) |
| **OPEN** | Explicitly deferred; must not be silently assumed in code |
| **CONCEPTUAL** | Domain concept without technology freeze |

### 0.4 What this document authorizes

- A concrete, production-grade **implementation architecture plan**
- Technology selections required to start implementation **after** AUDIT-021

### 0.5 What this document does **not** authorize

- Writing Rust code, crates, migrations, APIs, SDKs, Dockerfiles, Cloud, or dogfood apps
- Modifying ADRs, Master Spec, Cursor rules, or CI
- Selecting cloud vendors, pricing, or speculative distributed systems

---

## 1. Architecture Thesis

TX4 Core is a **durable domain engine** whose authoritative transaction and financial facts live in durable storage, with:

1. Deterministic lifecycle transitions (ADR-007)
2. Exact money arithmetic (ADR-006)
3. Explicit SoT boundaries vs providers and banks (ADR-004)
4. At-least-once delivery + durable idempotency + reconciliation (not universal exactly-once)
5. Independently runnable OSS without Cloud (ADR-008)

Correctness has priority over speculative throughput.

---

## 2. Technology Decision Records

Each selection below is an **implementation architecture** decision. ADR authorities remain frozen above them.

### 2.1 Core runtime language

| Field | Value |
| --- | --- |
| **DECISION** | Rust |
| **RATIONALE** | Already FROZEN by ADR-001; required for core. |
| **ALTERNATIVES** | Go, Kotlin/JVM (rejected by ADR-001) |
| **TRADE-OFFS** | Higher learning curve; strong compile-time invariants |
| **IMPACT** | Entire OSS core and workers in Rust |
| **REVERSIBILITY** | Extremely low (ADR-level) |
| **FROZEN_OR_OPEN** | **IA-FROZEN** (inherits ADR-001 FROZEN) |

### 2.2 Async runtime

| Field | Value |
| --- | --- |
| **DECISION** | **Tokio** |
| **RATIONALE** | TX4 requires concurrent API traffic, workers, webhook ingestion, and provider I/O with cancellation and timeouts. Tokio is the mature Rust async executor that Axum, SQLx, and `tracing`/OTel stacks integrate with without inventing a second concurrency model. A single executor family reduces dual-runtime hazards (blocking vs async deadlock classes). |
| **ALTERNATIVES** | `async-std` (smaller ecosystem for DB/HTTP today); OS threads-only (harder for many concurrent I/O waits); `smol` (less production ecosystem depth for this stack) |
| **TRADE-OFFS** | Ecosystem coupling to Tokio; must keep CPU-heavy work off the runtime or use `spawn_blocking` carefully |
| **IMPACT** | All async I/O paths use Tokio |
| **REVERSIBILITY** | Medium–high cost after deep integration |
| **FROZEN_OR_OPEN** | **IA-FROZEN** |

### 2.3 HTTP framework

| Field | Value |
| --- | --- |
| **DECISION** | **Axum** |
| **RATIONALE** | REST `/v1/...` handlers need composition of authn/authz, idempotency, validation, tracing, and rate-limit middleware without embedding domain logic in the framework. Axum’s Tower middleware model and extractor pattern keep HTTP concerns at the edge and map cleanly to application commands. Actix Web’s actor-centric model adds concurrency abstractions TX4 does not need when ADR-007 concurrency is enforced in the database transaction boundary. |
| **ALTERNATIVES** | Actix Web; Warp; Tide; Rocket |
| **TRADE-OFFS** | Axum evolves with Tower/hyper; less “batteries included” than some frameworks — intentional for explicit middleware |
| **IMPACT** | `tx4-api` / server crate depends on Axum |
| **REVERSIBILITY** | Medium (API edge only if domain stays decoupled) |
| **FROZEN_OR_OPEN** | **IA-FROZEN** |

### 2.4 Database technology

| Field | Value |
| --- | --- |
| **DECISION** | **PostgreSQL** as the durable persistence technology for TX4 domain state and related records |
| **RATIONALE** | ADR-004 requires durable authoritative state, unique operation identity, concurrent writers, crash recovery without memory-only authority, and auditability. PostgreSQL provides ACID transactions, row-level locks, unique constraints, CHECK constraints, and mature operational patterns for self-host and Managed Cloud wrapping. These properties directly implement lifecycle concurrency (ADR-007 §9), idempotency uniqueness (ADR-004 §10 / ADR-007 §8), and financial record immutability without inventing a custom storage engine. |
| **ALTERNATIVES** | MySQL/MariaDB (weaker default constraint/JSON/ops fit for this design); SQLite (insufficient multi-writer production target); document DB as primary (weaker relational invariants for money/idempotency); NewSQL (unnecessary complexity for solo-maintainer initial architecture) |
| **TRADE-OFFS** | Ops burden of a DB; connection pooling required; not “zero-ops” |
| **IMPACT** | All authoritative domain persistence targets PostgreSQL |
| **REVERSIBILITY** | Low after schema/data exist |
| **FROZEN_OR_OPEN** | **IA-FROZEN** (technology); SoT *semantics* remain ADR-004 |

**Critical distinction:**

```text
PostgreSQL = durable persistence technology
TX4 domain engine  = authority for TX4 domain facts (ADR-004)
Payment provider   = authority for provider-side payment facts
Bank/settlement rails = authority for actual external movement
```

PostgreSQL stores TX4’s durable representation; it does not make TX4 a payment processor or bank.

### 2.5 Database access layer

| Field | Value |
| --- | --- |
| **DECISION** | **SQLx** (async, PostgreSQL) |
| **RATIONALE** | Lifecycle and money correctness require explicit SQL, transactions, and constraints visible in review — not hidden ORM magic. SQLx is async-native on Tokio, supports transactions/`FOR UPDATE`, and optional compile-time query checking. Diesel’s sync-first/historical model fights the Tokio/Axum stack and encourages heavier schema abstraction. |
| **ALTERNATIVES** | Diesel; SeaORM; tokio-postgres alone; custom |
| **TRADE-OFFS** | SQL written/maintained by hand; compile-time check needs DB or offline data |
| **IMPACT** | Persistence crate uses SQLx |
| **REVERSIBILITY** | Medium if ports isolate repositories |
| **FROZEN_OR_OPEN** | **IA-FROZEN** |

### 2.6 Serialization

| Field | Value |
| --- | --- |
| **DECISION** | **Serde** (+ `serde_json` for JSON) |
| **RATIONALE** | ADR-006 requires JSON `amount` as integer **string**; API and events need explicit serde. Serde is the Rust ecosystem serialization foundation compatible with Axum. |
| **ALTERNATIVES** | Manual JSON; alternative codecs for internal only |
| **TRADE-OFFS** | Derive macros; must forbid float money serializers |
| **IMPACT** | All public JSON DTOs use Serde with money string codecs |
| **REVERSIBILITY** | Low for public JSON |
| **FROZEN_OR_OPEN** | **IA-FROZEN** |

### 2.7 Observability libraries

| Field | Value |
| --- | --- |
| **DECISION** | **`tracing`** + **OpenTelemetry-compatible** export interface |
| **RATIONALE** | ADR-001/Master Spec require structured logs, metrics, tracing. `tracing` is the Rust idiomatic structured span/event layer; OTel-compatible exporters keep vendor choice OPEN while enabling self-host and Cloud ops. |
| **ALTERNATIVES** | `log` only; proprietary APM SDKs in OSS core (rejected — vendor lock in OSS) |
| **TRADE-OFFS** | Exporter config complexity; must redact sensitive fields |
| **IMPACT** | Instrumentation in API/application/worker |
| **REVERSIBILITY** | Medium |
| **FROZEN_OR_OPEN** | **IA-FROZEN** (libraries); hosted vendor **OPEN** |

### 2.8 API style and contract

| Field | Value |
| --- | --- |
| **DECISION** | **REST** over HTTPS with **OpenAPI** as the public contract artifact for `/v1/...` |
| **RATIONALE** | ADR-005 freezes URI path versioning for the public REST API; OpenAPI is the maintainable contract source for SDKs and audits. |
| **ALTERNATIVES** | gRPC-first (not selected for initial public developer API); GraphQL (not selected — overfits query flexibility, underfits command/idempotency clarity) |
| **TRADE-OFFS** | Endpoint design still OPEN; must keep OpenAPI authoritative |
| **IMPACT** | `tx4-api` + OpenAPI artifacts in OSS |
| **REVERSIBILITY** | Low for public surface |
| **FROZEN_OR_OPEN** | **IA-FROZEN** (style); exact routes **OPEN** |

### 2.9 Migrations tool

| Field | Value |
| --- | --- |
| **DECISION** | **SQLx migrations** (ordered SQL files owned by persistence) |
| **RATIONALE** | Keeps schema evolution adjacent to SQLx; reviewable forward SQL; fits self-host `migrate` CLI. |
| **ALTERNATIVES** | Goose; Refinery; Flyway (JVM) |
| **TRADE-OFFS** | Rollback strategy is operational (see §16) |
| **IMPACT** | Migration files in OSS persistence |
| **REVERSIBILITY** | Medium |
| **FROZEN_OR_OPEN** | **IA-FROZEN** |

### 2.10 Container packaging

| Field | Value |
| --- | --- |
| **DECISION** | **Docker** images for API and worker; compose-style local stack with PostgreSQL |
| **RATIONALE** | ADR-008 requires self-host capability; containers are the minimum coherent distribution unit without selecting a cloud vendor or orchestrator. |
| **ALTERNATIVES** | Bare systemd binaries only; Kubernetes-first (deferred — ops complexity) |
| **TRADE-OFFS** | Image build/CI needed |
| **IMPACT** | Dockerfiles authorized only in later implementation tasks |
| **REVERSIBILITY** | High |
| **FROZEN_OR_OPEN** | **IA-FROZEN** (packaging approach); orchestrator **OPEN** |

### 2.11 Testing libraries

| Field | Value |
| --- | --- |
| **DECISION** | Rust built-in tests + **PostgreSQL integration tests** + **proptest** (property tests for money/allocation where valuable) |
| **RATIONALE** | Correctness-critical paths (lifecycle, money, concurrency, idempotency) cannot rely on mocks alone. |
| **ALTERNATIVES** | Mock-only (rejected for financial paths) |
| **TRADE-OFFS** | CI needs Postgres |
| **IMPACT** | Test architecture §21 |
| **REVERSIBILITY** | High |
| **FROZEN_OR_OPEN** | **IA-FROZEN** |

### 2.12 Explicitly not selected (remain OPEN)

| Concern | Status |
| --- | --- |
| Cloud vendor (AWS/GCP/Azure/…) | **OPEN** |
| Kubernetes / Nomad / etc. | **OPEN** |
| Kafka / NATS / RabbitMQ / Redis as required broker | **OPEN** (not required initially — see §9) |
| Specific payment provider as canonical | **OPEN** |
| Auth IdP (Auth0/Clerk/Keycloak/…) | **OPEN** |
| Hosted observability vendor | **OPEN** |
| Exact API endpoint catalog | **OPEN** |
| FX rate provider | **OPEN** |
| Full refund product architecture | **OPEN** (boundary frozen; product OPEN) |
| Event sourcing as sole authority | **NOT_SELECTED** (§4) |
| Distributed locks (ZooKeeper/etcd/Redis) | **NOT_SELECTED** initially |

---

## 3. System Context

```text
┌─────────────┐     HTTPS /v1/...      ┌──────────────────┐
│ Developers  │ ─────────────────────► │  tx4-server API  │
│ / SDKs      │ ◄───────────────────── │  (Axum)          │
└─────────────┘                        └────────┬─────────┘
                                                │
                     application commands       │
                                                ▼
                                       ┌──────────────────┐
                                       │  Application     │
                                       │  (use-cases)     │
                                       └────────┬─────────┘
                                                │
                                                ▼
                                       ┌──────────────────┐
                                       │  Domain          │
                                       │  (ADR-006/007)   │
                                       └────────┬─────────┘
                                                │ ports
                        ┌───────────────────────┼───────────────────────┐
                        ▼                       ▼                       ▼
               ┌────────────────┐    ┌────────────────┐      ┌────────────────┐
               │ PostgreSQL     │    │ PaymentProvider│      │ Outbox/Worker  │
               │ (SQLx)         │    │ adapters       │      │ (jobs)         │
               └────────────────┘    └───────┬────────┘      └────────────────┘
                                             │
                                             ▼
                                    External providers
                                    (Xendit/DOKU/Midtrans/Mock)
```

Managed Cloud (separate proprietary distribution) may host the same OSS binaries plus control-plane/dashboard; it MUST NOT become domain SoT (ADR-008).

---

## 4. Persistence Model

### 4.1 Decision

| Field | Value |
| --- | --- |
| **DECISION** | **Hybrid relational model**: (1) authoritative **current-state** relational rows for aggregates; (2) **append-only** tables for financial entries, provider observations, lifecycle audit, idempotency outcomes, outbox messages, reconciliation records |
| **EVENT_SOURCING** | **NOT_SELECTED** as sole authoritative model |
| **RATIONALE** | ADR-007 needs a clear current primary state and deterministic transitions under concurrency. A single current-state row (with version/lock) is the simplest authoritative representation that maps to row locks and unique constraints. Financial and audit facts need immutability — append-only records provide evidence without rewriting primary history (ADR-007 Option C). Full event sourcing would reconstruct state from events as authority, adding complexity, snapshot/rebuild machinery, and dual-representation risk without improving SoT clarity for the initial product. |
| **ALTERNATIVES** | Pure event sourcing; pure mutable state without audit; CQRS with dual write as dual authority (rejected) |
| **TRADE-OFFS** | Must keep append-only streams consistent with current state via same DB transaction; history is not a second writable authority |
| **FROZEN_OR_OPEN** | **IA-FROZEN** |

### 4.2 Authority map

| Data | Role |
| --- | --- |
| Transaction current primary state (+ version) | **Authoritative current state** |
| Payment attempt / outcome disposition rows | **Authoritative operation evidence** (TX4 view); provider remains provider-side SoT |
| Ledger entries (immutable) | **Authoritative TX4 internal financial records** |
| Materialized balances (if any) | **Derived cache** — must be rebuildable from entries |
| Lifecycle transition audit / event log | **Historical evidence** (not a competing writable state) |
| Idempotency records | **Authoritative duplicate-detection + stored response** |
| Outbox rows | **Durable intent to publish/work** — not domain SoT |
| Provider observation / webhook raw+normalized | **Observed evidence** |
| Reconciliation discrepancy/resolution | **Authoritative TX4 reconciliation decisions** |
| Cloud control-plane state | **Separate** (ADR-008) — not in OSS domain DB authority |

### 4.3 Rules

1. No competing authoritative balances for the same internal economic fact (ADR-004).
2. Do not store authoritative financial/transaction state only in process memory.
3. Append-only financial rows are never updated in place for “corrections”; corrections are new compensating entries.
4. Do not claim event-sourced reconstruction is required for correctness in v1.

---

## 5. Repository / Crate Architecture

### 5.1 Decision — Cargo workspace (OSS)

Minimum coherent structure (illustrative paths; created only by later implementation tasks):

```text
tx4/                          # OSS repository
  Cargo.toml                  # workspace
  crates/
    tx4-domain/               # pure domain: Money, lifecycle, invariants
    tx4-application/          # use-cases / commands / ports
    tx4-persistence/          # SQLx + migrations + repositories
    tx4-api/                  # Axum routes, DTOs, OpenAPI glue
    tx4-adapters-payment/     # PaymentProvider trait + Mock (+ later adapters)
    tx4-worker/               # outbox/job processor library or bin-shared
    tx4-observability/        # tracing/OTel bootstrap helpers
    tx4-config/               # config load/validate
  apps/
    tx4-server/               # API binary
    tx4-worker/               # worker binary
    tx4-cli/                  # migrate, health, admin self-host helpers
  docs/
    IMPLEMENTATION-ARCHITECTURE.md
    adr/
    MASTER-SPEC.md
```

**Cloud proprietary code** lives in a **separate repository or clearly separated proprietary workspace** that depends on published/OSS crates as libraries — never the reverse.

### 5.2 Dependency direction (normative)

```text
apps (server/worker/cli)
    ↓
tx4-api / tx4-worker
    ↓
tx4-application
    ↓
tx4-domain
    ↑
tx4-persistence, tx4-adapters-*  (implement ports defined in application/domain)
```

**Illegal:**

| From | To | Why |
| --- | --- | --- |
| `tx4-domain` | Axum / SQLx / payment SDK / cloud SDK | Domain purity |
| `tx4-domain` | `tx4-api` | Inversion |
| OSS core | Proprietary Cloud crates | ADR-008 non-crippled / independent build |
| Payment adapter | Domain mutation bypassing application | SoT / lifecycle bypass |

**Legal:**

- Application defines ports (`TransactionRepository`, `PaymentProvider`, `Outbox`, `Clock`, `IdGenerator`)
- Persistence/adapters implement ports
- API maps HTTP ↔ commands/queries

### 5.3 OSS / Cloud package boundary

| Layer | OSS | Cloud proprietary |
| --- | --- | --- |
| Domain + lifecycle + money | Yes | Consumes; does not fork semantics |
| Persistence + migrations | Yes | May operate managed Postgres |
| Public `/v1` API | Yes | Same conceptual core |
| Payment ports + Mock | Yes | Same |
| Worker/outbox | Yes | Same |
| Dashboard / fleet / billing of Cloud itself | No | Yes |
| Control-plane tenancy mapping | No (platform) | Yes — must not replace domain tenant model |

`OSS_CLOUD_IMPLEMENTATION_BOUNDARY` target: **PASS** when OSS builds and runs without proprietary crates.

---

## 6. Domain Module Classification

| Module | Class | Notes |
| --- | --- | --- |
| `money` / currency catalog | **CORE NOW** | ADR-006 |
| `transaction` lifecycle | **CORE NOW** | ADR-007 |
| `idempotency` | **CORE NOW** | ADR-004/007 |
| `tenant` / identity primitives | **CORE NOW** | Isolation boundary |
| `payment` (ports + attempts) | **CORE NOW** | Abstraction; providers later |
| `webhook` ingestion | **CORE NOW** | Durable observations |
| `audit` | **CORE NOW** | Evidence trail |
| `outbox` / jobs | **CORE NOW** | Async correctness |
| `fee` | **FOUNDATION** | After money + transaction |
| `ledger` | **FOUNDATION** | Immutable entries |
| `usage` / metering | **FOUNDATION** | Distinct from Cloud commercial billing |
| `billing` (app-level) | **FOUNDATION** | Not Cloud SKU pricing |
| `settlement` | **FOUNDATION** | Obligations + observations |
| `reconciliation` | **FOUNDATION** | Discrepancy records |
| `refund` records | **FOUNDATION** | Option C; product OPEN |
| `rate_limit` | **FOUNDATION** | Generic infra |
| `catalog` (products) | **LATER** | If reusable infra, not vertical |
| `customer` / `provider` (merchant) | **LATER** | Generic parties |
| `booking` | **REFERENCE/DOGFOOD ONLY** | Outside core |
| Canteen / marketplace verticals | **REFERENCE/DOGFOOD ONLY** | Outside core |

No `if vertical == ...` in core.

---

## 7. Transaction Engine (ADR-007 → Implementation)

### 7.1 Aggregate boundary

**Aggregate root:** `Transaction` (primary state + version + tenant + policy refs).

**Child / related records (same consistency boundary when mutating lifecycle):**

- payment attempts / outcome dispositions
- compensating-obligation intents (§6.1)
- transition audit entries
- outbox messages emitted by the transition

Refund/adjustment/ledger postings may attach by transaction id; they do **not** mutate primary state except via explicit allowed edges.

### 7.2 State persistence

- Table `transactions`: `id`, `tenant_id`, `primary_state`, `version`, expiration fields, attempt budget fields, timestamps, …
- Enum/`CHECK` constrained to ADR-007 states: `CREATED|PENDING|PAID|PROCESSING|COMPLETED|CANCELLED|EXPIRED|FAILED`
- Transition application = single PostgreSQL transaction that:
  1. Locks aggregate
  2. Validates matrix + predicates (C1–C3, §6.1, Terminal Failure Decision)
  3. Applies precedence if multi-candidate (normally one command per DB txn; see §8)
  4. Writes audit + compensating intent if required
  5. Upserts idempotency result
  6. Enqueues outbox if needed

### 7.3 Transition validation

Domain module encodes ADR-007 matrix as pure functions. Persistence commits only after domain accepts. Forbidden edges fail closed.

### 7.4 Terminal enforcement

Terminal states reject further primary progression. Refunds remain separate records.

### 7.5 Cancellation / expiration / failure

Implement ADR-007 §12–§14 literally:

- Cancel ≠ refund
- Expire only from `CREATED`/`PENDING`
- `TIMEOUT ≠ FAILED`
- `PENDING→FAILED` only via Terminal Failure Decision (`BUSINESS_REJECTED_TERMINAL` | `ATTEMPT_BUDGET_EXHAUSTED` | `DOMAIN_ABANDONED_TERMINAL`)

### 7.6 Compensating-obligation intent

For C1/C2/C3 and `PROCESSING→FAILED`, insert durable compensating-intent row **in the same DB transaction** as the state change or fail closed (ADR-007 §6.1).

---

## 8. Concurrency Architecture

### 8.1 Decision

| Field | Value |
| --- | --- |
| **DECISION** | **PostgreSQL transaction + row-level lock (`SELECT … FOR UPDATE`) on the transaction aggregate row**, plus **monotonic `version` column** for optimistic detection of lost updates; unique constraints for idempotency and operation identity |
| **RATIONALE** | ADR-007 §9 requires deterministic winners independent of thread scheduling. Serializing mutations per transaction id via `FOR UPDATE` makes “already-authoritative wins” trivial. When a single DB transaction evaluates a conflict set (rare multi-candidate batch), apply §9.3 precedence + §9.4 lexicographic operation-id tie-break in domain code before commit. Unique constraints prevent duplicate applies under races. Distributed locks are unnecessary for single-DB authority. |
| **ALTERNATIVES** | Optimistic-only version CAS (acceptable secondary; weaker under high contention without retry policy); advisory locks (optional supplement, not primary); Redis/etcd locks (rejected initially — extra SoT-ish failure domain) |
| **TRADE-OFFS** | Lock contention under hot transaction ids; keep critical section short (no provider I/O inside `FOR UPDATE` transaction) |
| **FROZEN_OR_OPEN** | **IA-FROZEN** |

### 8.2 Provider I/O boundary (critical)

```text
WRONG: BEGIN; FOR UPDATE; call provider; COMMIT;
RIGHT: prepare intent → call provider (outside lock) → BEGIN; FOR UPDATE;
       apply evidence/transition idempotently; COMMIT;
```

External calls happen **outside** the row lock. Uncertainty after timeout is recorded as disposition, not invented `FAILED`.

### 8.3 Concurrent duplicate requests

Two HTTP requests with the same idempotency key: unique constraint + lock ensures one execute path; the other reads stored result (see §9).

---

## 9. Idempotency Architecture

### 9.1 Decision

Durable **idempotency records** in PostgreSQL:

| Field | Purpose |
| --- | --- |
| `tenant_id` | Isolation boundary |
| `idempotency_key` | Client-supplied key (API header name OPEN; concept required) |
| `operation_id` | Logical operation identity (ADR-007 §8.2) |
| `request_fingerprint` | Hash of critical request body fields |
| `transaction_id` | Association when applicable |
| `status` | `IN_PROGRESS` / `COMPLETED` / `FAILED_CLOSED` |
| `response_status` + `response_body` | Stored result for replay |
| `created_at` / `expires_at` | Retention |

**Uniqueness:** `(tenant_id, idempotency_key)` unique. Optional unique `(tenant_id, operation_id)`.

### 9.2 Behavior

| Case | Outcome |
| --- | --- |
| First request | Insert `IN_PROGRESS`, execute, store result `COMPLETED` |
| Duplicate same fingerprint | Return stored result; no new financial effect |
| Same key, different fingerprint | **Conflict** (4xx business conflict); do not apply |
| Concurrent inserts | One wins unique constraint; loser re-reads |

### 9.3 Survival

Survives process restart, retry, network timeout, duplicate delivery, concurrent duplicates — because authority is PostgreSQL, not memory (ADR-004).

### 9.4 Retention

Idempotency rows are retained for a configurable window (default policy **OPEN** numerically; principle: long enough for client retries). Expired keys may be purged only if product policy allows; financial audit records are **not** purged by the same policy.

---

## 10. Crash Recovery Architecture

| Failure | Behavior |
| --- | --- |
| Process crash mid-request before commit | No durable transition; client retry with same idempotency key |
| Crash after commit | Resume from durable state; idempotent no-op on retry |
| DB transaction rollback | No partial authoritative write; safe retry |
| Provider timeout | Record `TIMEOUT`/`UNKNOWN` disposition; **not** auto-`FAILED`; schedule reconcile |
| Webhook late / duplicate / out-of-order | Durable observation + idempotent apply; ignore stale if state already advanced; never invent certainty |
| Recovery of uncertain operation | Query provider / reconcile; promote to `KNOWN_SUCCESS`/`KNOWN_FAILURE` only with evidence |

**Distinguish:**

```text
DB transaction rollback  ≠  external provider uncertainty
```

No universal exactly-once claim (ADR-004 §9). Desired invariant: **exactly-once business effect** via durable identity + idempotency + reconciliation.

---

## 11. Outbox / Async Processing

### 11.1 Decision

| Field | Value |
| --- | --- |
| **DECISION** | **Transactional outbox table + dedicated worker process** polling/claiming jobs in PostgreSQL |
| **QUEUE/BROKER** | **Not required** for initial architecture |
| **RATIONALE** | Async work (webhook delivery, provider status poll, reconciliation checks, expiration sweeps) must commit atomically with domain changes. A Postgres outbox preserves that atomicity without introducing Kafka/NATS operational surface or a second durability system. At-least-once worker delivery + idempotent handlers preserve correctness under duplicate/delay. |
| **ALTERNATIVES** | Kafka/NATS (defer until scale/ops justify); Redis queues (extra durability class); in-process only (fails crash recovery) |
| **TRADE-OFFS** | Polling latency; need careful `SKIP LOCKED` claim pattern |
| **FROZEN_OR_OPEN** | **IA-FROZEN** (outbox+worker); external broker **OPEN** |

### 11.2 Worker claim pattern

```text
UPDATE outbox SET locked_by, locked_at, attempts
WHERE id IN (
  SELECT id FROM outbox
  WHERE available_at <= now() AND status = 'PENDING'
  ORDER BY available_at
  FOR UPDATE SKIP LOCKED
  LIMIT N
)
```

Handlers must be idempotent. Poison messages → dead-letter status + alert; do not infinite-loop money effects.

---

## 12. Payment Architecture

### 12.1 Port

```text
PaymentProvider (port)
  - create_payment(intent) -> ProviderRef / Pending
  - capture(ref) -> ...
  - refund(ref, money) -> ...   // execution OPEN product-wise; port exists
  - get_payment_status(ref) -> ProviderObservation
```

### 12.2 Adapters

| Adapter | Class |
| --- | --- |
| Mock | **CORE NOW** (tests + local) |
| Xendit / DOKU / Midtrans | **LATER** (authorized separately; license review ADR-003) |

Provider SDKs stay in adapter crates; **never** leak provider enums into public `/v1` domain states.

### 12.3 Rules

- Provider authoritative for provider-side facts; TX4 authoritative for TX4 transaction state (ADR-004)
- `transaction_state ≠ provider_state`
- Timeouts → disposition uncertainty
- Webhooks verified, durably stored, applied idempotently
- Provider operation identity stored for reconciliation

---

## 13. Money Architecture (ADR-006 → Rust)

### 13.1 Types (conceptual)

```text
CurrencyId        // catalog identity (code + catalog version concept)
Money {
  amount: i64     // signed atomic units; range floor ≥ i64 per ADR-006 §13
  currency: CurrencyId
}
```

- Intermediates for multiply/allocate MAY use `i128` or big-int, then **checked** narrow to `i64`
- **No** `f32`/`f64` financial authority
- Serde: `amount` as **string**; reject JSON numbers as sole authority
- Half-even per ADR-006 §9; allocation per §11 (reject negative source)
- Overflow: fail checked, never wrap
- Comparison: same currency only; cross-currency compare forbidden without FX boundary
- FX: **out of money core** — conversion only at explicit FX boundary with documented rate identity (**OPEN** provider)

### 13.2 Module placement

`tx4-domain` owns Money primitives and property tests. Persistence stores `amount` as `BIGINT` + `currency` text/code; never `DOUBLE PRECISION` / `REAL` / `MONEY` float types.

---

## 14. Ledger Architecture

### 14.1 Decision

**Append-only ledger entries** as authoritative TX4 internal financial records:

- `entry_id`, `tenant_id`, `transaction_id` (nullable when not tied), `account_id`, `direction` (debit/credit **or** signed amount with explicit convention — choose one convention and document in implementation), `money`, `reason`, `created_at`, `idempotency/operation_id`

**Balances:** derived by summing entries (or materialized snapshot marked **derived**). Materialized balances MUST be rebuildable; never a second authority.

### 14.2 Non-goals

Not a full ERP (AR/AP/GL productization). No silent merge with Cloud credits (ADR-004 §5.8).

---

## 15. Settlement Architecture

TX4 records:

- settlement **obligations**
- settlement **records/state** as TX4 understands them
- **observations** of external movement

TX4 does **not** itself become the bank. External rails remain authoritative for actual movement (ADR-004 §5.5). Settlement execution integration remains **OPEN**.

---

## 16. Reconciliation Architecture

Records:

1. **Observation** (provider/bank feed normalized)
2. **Comparison** result vs TX4 expected
3. **Discrepancy** (open) when mismatch
4. **Resolution** (explicit decision record) — never fabricated certainty
5. Optional retry / manual review boundary (**policy OPEN**)

Reconciliation may update dispositions and evidence; MUST NOT silently revive terminal primary states into active ones (ADR-007 §11).

---

## 17. Usage / Billing Architecture

Separate concepts:

| Concept | Meaning |
| --- | --- |
| API request | Transport |
| Internal operation | Domain/work unit |
| Commercial transaction | ADR-007 aggregate |
| Completed transaction | Terminal success primary state |
| Billable usage | Metered infra usage events |
| Application customer billing | OSS billing primitives for *apps using TX4* |
| Cloud developer billing / credits | **Managed Cloud commercial** — separate DB/domain (ADR-004/008) |

Usage pipeline: emit → durable usage record → dedupe by operation identity → aggregate → invoice/billing run (**pricing OPEN**).

---

## 18. API Architecture (ADR-005)

### 18.1 Surface

- Canonical: `/v1/...`
- Unsupported `/v{N}` → explicit error (ADR-005 §15)
- No silent version remapping
- OpenAPI is contract authority for published shapes

### 18.2 Layers

```text
HTTP (Axum)
  → authn/authz + tenant context
  → idempotency middleware
  → validate DTO
  → application command
  → domain
  → map result/error → HTTP
```

### 18.3 Cross-cutting

- Idempotency header/field (**exact name OPEN**; mechanism §9)
- Correlation / request id
- Pagination strategy: cursor-based preferred for large lists (**details OPEN**)
- Webhook receiver endpoints under `/v1/...` with signature verification

### 18.4 Non-goal of this spec

Complete endpoint catalog. Routes designed in later API tasks under `/v1` additive evolution.

---

## 19. Tenancy / Authorization

### 19.1 Domain tenancy

Every tenant-scoped row carries `tenant_id`. Queries always scoped. `authenticated ≠ authorized`.

### 19.2 Authn boundary

Initial OSS: **API keys** as first-class primitive; OAuth/OIDC **OPEN** as additive. No mandatory third-party IdP for OSS correctness.

### 19.3 Cloud platform tenancy

Cloud org/project membership is control-plane state; mapping to domain `tenant_id` must be explicit and must not bypass domain authz (ADR-008 §10).

### 19.4 Service-to-service

Worker uses internal credentials with least privilege; cannot call provider “as tenant” without tenant context on domain commands.

---

## 20. Security Architecture (baseline)

| Control | Approach |
| --- | --- |
| Tenant isolation | Mandatory `tenant_id` constraints + authz checks |
| Secrets | Env/secret refs; never commit |
| Webhook verification | Provider signatures; reject invalid |
| Input validation | Edge + domain invariants |
| Replay | Idempotency + webhook event ids |
| Logging | Redact PANs/secrets; money logged as structured fields carefully |
| Dependencies | ADR-003 review + SBOM in later CI |
| Least privilege | DB roles: app vs migrator |

No claim of “secure product” until implementation + review evidence exists.

---

## 21. Observability Architecture

- Structured logs via `tracing`
- Spans across API → application → DB → adapter
- Metrics: request rates, transition counts, outbox lag, reconcile backlog
- IDs: `request_id`, `transaction_id`, `operation_id`, `provider_ref`, `tenant_id`
- Audit events for sensitive transitions
- Exporters OTel-compatible; **vendor OPEN**

---

## 22. Error Model

| Class | Domain/Application | Typical API |
| --- | --- | --- |
| Validation | `ValidationError` | 400 |
| Business rule rejection | `Rejected` | 422 or 409 per contract |
| Conflict / stale transition | `Conflict` | 409 |
| Idempotency conflict | `IdempotencyConflict` | 409 |
| Provider uncertainty | `Uncertainty` (not failure) | 202/200 with disposition or 503 policy — **exact mapping OPEN** but must not imply `FAILED` |
| Retryable infra | `Retryable` | 503 |
| Permanent internal | `Internal` | 500 (no leak) |

Map carefully so clients never treat timeout as terminal business failure.

---

## 23. Migrations Strategy

- Owned by `tx4-persistence`
- Ordered forward SQL via SQLx migrator
- Expand/contract for compatible upgrades where possible
- Destructive changes require explicit task authorization
- Self-host: `tx4-cli migrate`
- Cloud: same migrations applied by ops automation (proprietary runner OK; SQL remains OSS)
- App version ↔ schema version compatibility documented per release
- Rollback: prefer forward-fix; DB backup restore for catastrophic cases (**no** casual down-migrations for financial schema)

---

## 24. Configuration Architecture

- Env vars for secrets and runtime endpoints
- Optional config file for non-secret defaults
- Validate at startup (fail fast)
- Distinguish immutable process config vs future dynamic policy (**dynamic OPEN**)
- Examples: `DATABASE_URL`, `TX4_HTTP_BIND`, `TX4_ATTEMPT_BUDGET_DEFAULT` (values OPEN)

---

## 25. Data Retention Principles

| Record class | Principle |
| --- | --- |
| Transactions / financial / audit | Long-lived; deletion is explicit policy |
| Idempotency | Configurable TTL |
| Raw webhooks | Retain for reconcile window then archive policy |
| Usage | Retain for billing disputes window |
| Legal retention periods | **OPEN** (compliance policy later) |

Do not invent jurisdiction-specific retention days here.

---

## 26. Version Compatibility

| Artifact | Policy |
| --- | --- |
| Public API | ADR-005 `/v{N}`; additive in `v1` |
| DB schema | Migrated; app requires min schema |
| SDK | Tracks OpenAPI; major with API major |
| Cloud | May version control-plane separately; must not fork core `/vN` semantics |

---

## 27. Testing Architecture

| Layer | Required |
| --- | --- |
| 1 Domain unit | Lifecycle matrix, money, allocation, precedence |
| 2 Application | Command handlers with port fakes |
| 3 Persistence integration | **Real PostgreSQL** |
| 4 API contract | OpenAPI / HTTP tests |
| 5 Payment adapter contract | Mock + later provider sandboxes |
| 6 Concurrency | Parallel transitions on one txn id |
| 7 Idempotency | Duplicate/conflict cases |
| 8 Crash/recovery | Kill mid-flight; restart worker |
| 9 Reconciliation | Disagreement fixtures |
| 10 Financial property | proptest on round/allocate |
| 11 Migration | Fresh + upgrade paths |
| 12 End-to-end | API + DB + worker + mock provider |

Correctness-critical paths MUST use real PostgreSQL in CI.

---

## 28. Failure Injection Strategy

Test harnesses must be able to inject:

- DB failure / rollback
- Process crash (API and worker)
- Provider timeout / unknown
- Duplicate HTTP and webhook
- Delayed / out-of-order webhook
- Concurrent conflicting transitions
- Partial outbox processing
- Attempt budget exhaustion

Pass criteria: no duplicated financial effects; no `TIMEOUT→FAILED`; durable recovery to a valid state.

---

## 29. Performance Goals (non-benchmark claims)

Architectural goals (not invented SLAs):

- Correctness under contention > raw throughput
- Connection pooling (e.g. SQLx pool) required
- Indexes on `(tenant_id, id)`, idempotency unique keys, outbox `available_at`, webhook event ids
- Avoid holding row locks across provider I/O
- Pagination for list APIs
- Worker concurrency configurable

Premature sharding/multi-region **OPEN** / out of initial scope.

---

## 30. Deployment Architecture

### 30.1 Initial self-host (minimal)

```text
┌────────────┐   ┌────────────┐   ┌──────────────┐
│ tx4-server │   │ tx4-worker │   │ PostgreSQL   │
└────────────┘   └────────────┘   └──────────────┘
        \             / 
         \           /  
        reverse proxy (optional; nginx/caddy OPEN)
```

Single-node acceptable for small self-host; split API/worker processes even on one host for blast isolation.

### 30.2 Environments

| Env | Components |
| --- | --- |
| Local dev | Docker Compose: Postgres + server + worker |
| CI | Ephemeral Postgres + tests |
| Self-host prod | Same binaries + managed/ops Postgres |
| Managed Cloud later | Same OSS binaries + proprietary control plane (**vendor OPEN**) |

---

## 31. Implementation Phases

> Phases are planning only. Each requires a later authorizing task. No code in TASK-018.

### Phase 1 — Foundation / domain primitives

- **Prereqs:** AUDIT-021 PASS; implementation task authorized
- **Deliverables:** workspace, `tx4-domain` Money + lifecycle pure logic, unit/property tests
- **Invariants:** ADR-006/007 encoded; no float money
- **Exit:** domain tests green; no persistence yet optional

### Phase 2 — Persistence / transaction engine

- **Deliverables:** PostgreSQL schema/migrations, repositories, `FOR UPDATE` transitions, idempotency table, outbox table
- **Invariants:** durable state; concurrency precedence; compensating intent atomicity
- **Tests:** integration concurrency/idempotency/recovery
- **Exit:** engine proof on Mock clock/provider ports

### Phase 3 — API / authentication baseline

- **Deliverables:** Axum `/v1` skeleton, OpenAPI bootstrap, API keys, tenant context, error mapping
- **Invariants:** ADR-005 path versioning; authz checks
- **Exit:** contract tests for initial routes

### Phase 4 — Payment adapters + webhooks

- **Deliverables:** PaymentProvider port, Mock, webhook ingest, disposition model
- **Invariants:** timeout≠failed; provider≠transaction state
- **Exit:** e2e mock payment uncertainty tests

### Phase 5 — Financial primitives (fee/ledger foundations)

- **Deliverables:** ledger entries, fee hooks as generic infra
- **Invariants:** append-only; no competing balances
- **Exit:** ledger invariant tests

### Phase 6 — Usage / billing foundations

- **Deliverables:** usage records, dedupe, aggregation jobs
- **Invariants:** Cloud credit ≠ app billing
- **Exit:** metering idempotency tests

### Phase 7 — Settlement / reconciliation foundations

- **Deliverables:** obligation/observation/discrepancy/resolution records
- **Invariants:** no fabricated certainty
- **Exit:** reconcile fixtures pass

### Phase 8 — Production hardening

- **Deliverables:** Docker packaging, CLI migrate, observability exporters, security baseline, failure injection suite, docs
- **Invariants:** OSS runnable without Cloud; ADR-008 boundary intact
- **Exit:** Definition of Done checklist (§32) satisfied for initial production cut

Dependency graph: 1→2→3→4; 5–7 after 2; 8 continuous after 3.

---

## 32. Definition of Done (Initial Production Implementation)

Initial TX4 production OSS cut is architecture-complete only when evidence shows:

1. Durable authoritative state in PostgreSQL  
2. Deterministic ADR-007 lifecycle + precedence  
3. Durable idempotency  
4. Concurrency safety under parallel writers  
5. Crash recovery without invented failures  
6. Exact ADR-006 money (no float authority)  
7. Provider uncertainty model  
8. Reconciliation records path  
9. Tenant isolation + authz  
10. `/v1` API contract + OpenAPI  
11. Observability baseline  
12. Security baseline (secrets, webhook verify, validation)  
13. Automated tests including real Postgres + failure injection  
14. Migrations + self-host deploy path (Docker)  
15. OSS builds without proprietary Cloud code  

`compiles ≠ done`. `tests pass ≠ architecture correct` — but both are required evidence.

---

## 33. OPEN vs IA-FROZEN Summary

### IA-FROZEN by this specification (pending AUDIT-021)

- Tokio, Axum, PostgreSQL, SQLx, Serde, tracing+OTel-compatible, REST+OpenAPI, SQLx migrations, Docker packaging, transactional outbox+worker, hybrid relational persistence (not event sourcing), `FOR UPDATE`+version concurrency, durable idempotency table, Money as `i64` atomic + string JSON

### Remain OPEN

- Cloud vendor, K8s, brokers, auth IdP product, hosted OTel backend, exact routes/fields, pricing, payment provider canon, FX provider, refund product details, numeric retention TTLs, attempt-budget defaults, multi-region, dogfood architectures, SDK languages beyond eventual TS-first intent (SDK implementation **OPEN**)

---

## 34. Conflict Check vs Frozen ADRs

| ADR | Compatibility |
| --- | --- |
| ADR-001 Rust | Compatible — Rust selected |
| ADR-002 Apache-2.0 | Compatible — no license change; deps must follow ADR-003 |
| ADR-003 | Compatible — selections are permissive-ecosystem; concrete crate license review at add-time |
| ADR-004 SoT | Compatible — Postgres is technology; authorities unchanged |
| ADR-005 | Compatible — `/v1` REST |
| ADR-006 | Compatible — integer atomic + string JSON; half-even; allocation |
| ADR-007 | Compatible — matrix/precedence/uncertainty implemented, not replaced |
| ADR-008 | Compatible — OSS independent; Cloud separated |

**ARCHITECTURE_CONFLICTS:** NONE identified at specification time.

---

## 35. Explicit Non-Actions of TASK-018

This document does **not**:

- modify ADR-001…008 or Master Spec
- add `src/`, `crates/`, migrations, or dependencies
- authorize coding

Next gate: **AUDIT-021** (independent audit of this specification).

---

## Document history

| Date | Event |
| --- | --- |
| 2026-09-25 | TASK-018 created PROPOSED implementation architecture |
