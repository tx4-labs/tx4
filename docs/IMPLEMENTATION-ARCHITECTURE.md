# TX4 Implementation Architecture Specification

| Field | Value |
| --- | --- |
| Document | **IMPLEMENTATION-ARCHITECTURE** |
| Status | **FROZEN** |
| Date | 2026-09-25 |
| Frozen | 2026-09-25 (FREEZE-012; basis AUDIT-023 PASS; audited HEAD `230144f813a3bd8c69880ca3e4fc0035ea7ede0b`) |
| Authorizing path | TASK-018 → AUDIT-021 → TASK-018R → AUDIT-022 → TASK-018RR → AUDIT-023 → FREEZE-012 |
| Baseline (TASK-018) | `ec3b80dafd8328b363a89bbb5b7e4b9ea83508a9` |
| Audited content HEAD | `230144f813a3bd8c69880ca3e4fc0035ea7ede0b` |
| Authorization basis | AUDIT-020 PASS; Architecture Decision Phase COMPLETE; AUDIT-023 PASS / FREEZE_READY |
| Scope | Frozen implementation architecture — **not** implementation authorization |

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
| **FROZEN** (document) | This specification is frozen by FREEZE-012; implementation must treat it as authoritative |
| **IA-FROZEN** | Selected by this specification for initial production implementation |
| **OPEN** | Explicitly deferred; must not be silently assumed in code |
| **CONCEPTUAL** | Domain concept without technology freeze |

### 0.4 What this frozen document authorizes

- Authoritative **implementation architecture** for future authorized implementation tasks
- Technology selections and durability/idempotency/payment/outbox protocols frozen herein

### 0.5 What this document does **not** authorize

- Writing Rust code, crates, migrations, APIs, SDKs, Dockerfiles, Cloud, or dogfood apps **by this freeze alone**
- Modifying ADRs, Master Spec, Cursor rules, or CI
- Selecting cloud vendors, pricing, or speculative distributed systems
- Silently converting remaining **OPEN** decisions into frozen ones

### 0.6 Freeze conflict rule

```text
IMPLEMENTATION
      ↓
CONFLICT WITH FROZEN ARCHITECTURE / ADR
      ↓
STOP → REPORT → SPEC/ADR CHANGE GATE
```

Never silent workaround or architecture drift.

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
| **DECISION** | **Hybrid relational model**: (1) authoritative **mutable current-state** relational rows for aggregates and operational processing; (2) **append-only** tables for financial entries, provider observations, lifecycle audit evidence, and reconciliation decision records |
| **EVENT_SOURCING** | **NOT_SELECTED** as sole authoritative model |
| **RATIONALE** | ADR-007 needs a clear current primary state and deterministic transitions under concurrency. A single current-state row (with version/lock) is the simplest authoritative representation that maps to row locks and unique constraints. Financial and audit facts need immutability — append-only records provide evidence without rewriting primary history (ADR-007 Option C). Full event sourcing would reconstruct state from events as authority, adding complexity, snapshot/rebuild machinery, and dual-representation risk without improving SoT clarity for the initial product. |
| **ALTERNATIVES** | Pure event sourcing; pure mutable state without audit; CQRS with dual write as dual authority (rejected) |
| **TRADE-OFFS** | Must keep append-only evidence consistent with current state via same DB transaction when both are written; history is not a second writable authority |
| **FROZEN_OR_OPEN** | **IA-FROZEN** |

### 4.1.1 Taxonomy (ARCHITECTURAL REQUIREMENT)

**Durable does NOT imply append-only.**  
**Append-only does NOT mean event sourcing.**

#### A. Mutable operational state (durable, updatable)

These rows are durable in PostgreSQL and **may be updated in place**. They MUST NOT be described as append-only.

| Record | Mutability | Authority role | Purpose |
| --- | --- | --- | --- |
| Transaction current primary state (+ version) | **Mutable** | **Authoritative** current TX4 lifecycle state | Current aggregate state |
| Idempotency reservation | **Mutable** | **Authoritative** for duplicate detection + stored API/command result | Lease, status, replay |
| Outbox / job record | **Mutable** | **Authoritative** for async work scheduling only — **not** domain SoT | Claim, lease, fencing, retry, completion |
| Worker lease / claim metadata (on outbox/job) | **Mutable** | Operational processing state | Prevent stuck work; enable reclaim |
| PaymentAttempt current status / disposition fields | **Mutable** (status) | **Authoritative** TX4 view of that attempt’s progress; provider remains provider-side SoT | Bind provider I/O; uncertainty; reconcile |
| Retry / processing counters on operational rows | **Mutable** | Operational | Backoff, attempt budgets at row level |
| Materialized balances (if any) | **Mutable cache** | **Derived only** — rebuildable from ledger entries; **non-authoritative** | Performance; never second authority |

#### B. Append-only evidence / financial records

These rows are **immutable after insert** (corrections = new rows). They are **not** event-sourced reconstruction of aggregate authority.

| Record | Mutability | Authority role | Purpose |
| --- | --- | --- | --- |
| Ledger entries | **Append-only** | **Authoritative** TX4 internal financial records | Money movements as TX4 records them |
| Lifecycle transition audit / evidence log | **Append-only** | **Historical evidence** (not competing writable state) | Who/what transitioned when |
| Provider observations (raw + normalized) | **Append-only** | **Observed evidence** | Webhooks/polls as received |
| Reconciliation discrepancy / resolution records | **Append-only** (new resolution rows) | **Authoritative** TX4 reconciliation decisions | Detect/resolve without fabricating certainty |
| Compensating-obligation intent records | **Append-only** (intent row; later refund execution separate) | **Authoritative** intent evidence per ADR-007 §6.1 | Fail-closed post-pay exits |

Cloud control-plane state remains **separate** (ADR-008) and is not OSS domain DB authority.

### 4.2 Authority map (summary)

| Data | Mutable / append-only | Role |
| --- | --- | --- |
| Transaction current primary state (+ version) | Mutable | **Authoritative current state** |
| PaymentAttempt | Mutable status + appendable observations | **Authoritative attempt evidence** (TX4 view); provider remains provider-side SoT |
| Ledger entries | Append-only | **Authoritative TX4 internal financial records** |
| Materialized balances (if any) | Derived mutable cache | **Derived** — rebuildable |
| Lifecycle transition audit | Append-only | **Historical evidence** |
| Idempotency reservation | Mutable operational | **Authoritative** duplicate-detection + stored result |
| Outbox / job | Mutable operational | Durable work intent — **not** domain SoT |
| Provider observations | Append-only | **Observed evidence** |
| Reconciliation decisions | Append-only | **Authoritative** TX4 reconcile decisions |

### 4.3 Rules

1. No competing authoritative balances for the same internal economic fact (ADR-004).
2. Do not store authoritative financial/transaction state only in process memory.
3. Append-only financial/evidence rows are never updated in place for “corrections”; corrections are new compensating entries.
4. Do not claim event-sourced reconstruction is required for correctness in v1.
5. Do not label idempotency or outbox/job tables as append-only.

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
    tx4-adapters-payment/     # implements PaymentProvider port (+ Mock; later adapters)
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

**Port location (IA-FROZEN):** Provider and infrastructure **ports/interfaces are defined in `tx4-application`** (the application boundary). Infrastructure adapters depend **inward** on those ports. Domain types/invariants live in `tx4-domain`; application may depend on domain; adapters must not invert that.

```text
apps (server/worker/cli)
        ↓
tx4-api / tx4-worker
        ↓
tx4-application   ←── ports defined here (TransactionRepository, PaymentProvider,
        ↓                 Outbox, Clock, IdGenerator, …)
tx4-domain

tx4-persistence ───────────┐
tx4-adapters-payment ──────┤── depend on tx4-application ports (+ tx4-domain types)
                           │   implement adapters inward
```

**Illegal:**

| From | To | Why |
| --- | --- | --- |
| `tx4-domain` | Axum / SQLx / payment SDK / cloud SDK | Domain purity |
| `tx4-domain` | `tx4-api` | Inversion |
| OSS core | Proprietary Cloud crates | ADR-008 non-crippled / independent build |
| Payment adapter | Domain mutation bypassing application | SoT / lifecycle bypass |

**Legal:**

- `tx4-application` defines ports
- `tx4-persistence` / `tx4-adapters-*` implement those ports
- API maps HTTP ↔ application commands/queries

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
RIGHT: durable PaymentAttempt PREPARED + idempotency reservation
       → assign provider_idempotency_key; PREPARED → SUBMITTED; COMMIT (§12.4.1)
       → call provider (outside locks; same attempt identity)
       → BEGIN; FOR UPDATE (lock order §8.4); apply evidence/transition; COMMIT;
```

External calls happen **outside** database row locks and **only after** committed `SUBMITTED`. Uncertainty after timeout is recorded as disposition on the existing PaymentAttempt, not invented transaction `FAILED`.

### 8.3 Concurrent duplicate requests

Two HTTP requests with the same idempotency key: §9 reservation protocol ensures only one acquires execution; the other follows non-executing duplicate behavior.

### 8.4 Lock acquisition order (IA-FROZEN)

Commands that touch multiple authoritative/operational rows in one PostgreSQL transaction MUST acquire locks in this **global order**:

1. **Idempotency reservation** row (`tenant_id`, `idempotency_key`) when the command is idempotent
2. **Transaction aggregate** row (`tenant_id`, `transaction_id`)
3. **Related child / attempt rows** for that aggregate (e.g. PaymentAttempt being updated), ordered by primary key ascending when multiple

Outbox enqueue within the same DB transaction locks/inserts outbox rows **after** the aggregate lock is held (inserts do not require a pre-existing lock order beyond unique constraints).

Never acquire locks in the reverse of this order in a different code path.

### 8.5 Transient database conflict retry (IA-FROZEN)

| Condition | Classification | Behavior |
| --- | --- | --- |
| Deadlock detected by PostgreSQL | Retryable infrastructure | Abort txn; **bounded retry** of the same idempotent command path |
| Serialization / concurrent update conflict | Retryable infrastructure | Same as above |
| Permanent business rejection / matrix forbid / fingerprint conflict | **Not** retryable as infra | Return business/conflict error; do not loop |

**Bounded retry:** finite attempts with backoff (exact numeric defaults **OPEN**; architecture requires a documented upper bound). After exhaustion → retryable/infra error to client (e.g. 503), not silent success.

Retries MUST re-enter through the **same idempotent command path** (§9). Do **not** retry permanently failed business operations. Do **not** perform external provider I/O while database locks are held.

---

## 9. Idempotency Architecture

### 9.0 Distinction (ARCHITECTURAL REQUIREMENT)

```text
Idempotency reservation  ≠  Business operation attempt (e.g. PaymentAttempt / lifecycle transition attempt)
```

| Concept | Role |
| --- | --- |
| **Idempotency reservation** | Client/command duplicate gate: lease, fingerprint, stored HTTP/command result |
| **Business operation attempt** | Domain/provider-facing durable attempt with stable `operation_id` (see §12.4) |

Both are durable. Reclaim of an idempotency lease MUST NOT erase historical evidence (audit, observations, attempts).

### 9.1 Decision — mutable operational reservation

Durable **idempotency reservation** rows in PostgreSQL (**mutable operational state**, §4.1.1A):

| Field | Purpose |
| --- | --- |
| `tenant_id` | Isolation boundary |
| `idempotency_key` | Client-supplied key (API header name **OPEN**; concept required) |
| `operation_id` | Logical operation identity (ADR-007 §8.2); **distinct** from client idempotency key |
| `request_fingerprint` | Hash of critical request body fields |
| `transaction_id` | Association when applicable |
| `status` | `IN_PROGRESS` / `COMPLETED` / `FAILED_CLOSED` |
| `lease_owner` | Executor identity holding the reservation |
| `lease_expires_at` | Bounded lease end for `IN_PROGRESS` |
| `response_status` + `response_body` (or result reference) | Stored result for replay when finalized |
| `created_at` / `updated_at` | Timestamps |
| `expires_at` | Retention window for the reservation row (TTL policy) |

**Uniqueness (IA-FROZEN):**

- `UNIQUE (tenant_id, idempotency_key)`
- `UNIQUE (tenant_id, operation_id)` — normative; prevents duplicate logical operation application (ADR-007). Equivalent uniqueness on a durable transition-attempt table is acceptable **only if** it is the single enforcement point and still unique per tenant.

### 9.2 Normative protocol (IA-FROZEN)

**Statuses:** `IN_PROGRESS` | `COMPLETED` | `FAILED_CLOSED`

1. **Reservation.** The reservation MUST be durable in PostgreSQL before side effects that must not duplicate.

2. **Atomicity.** Where the command creates a business operation attempt (e.g. PaymentAttempt) under the same database authority, the idempotency reservation (`IN_PROGRESS` + lease) and that attempt row MUST be inserted/updated in the **same PostgreSQL transaction**.

3. **Duplicate while active (selected initial behavior).** If another request observes an **active** `IN_PROGRESS` reservation (lease not expired) for the same key + same fingerprint: it MUST **NOT** execute the operation. It MUST return a **deterministic in-progress result** (exact HTTP status code **OPEN**; behavior class fixed: non-executing in-progress/conflict). Different fingerprint → fingerprint **conflict**; do not apply.

4. **Lease.** Every `IN_PROGRESS` MUST carry `lease_owner` + `lease_expires_at` (bounded; numeric default **OPEN**).

5. **Crash reclaim.** When `status = IN_PROGRESS` and `lease_expires_at < now()`, a claimant MAY reclaim **only** by: (a) locking the reservation row, (b) verifying expiry, (c) setting new `lease_owner` / `lease_expires_at`, (d) **not** blindly re-running external side effects — see (7).

6. **External side-effect protection (normative).** Before **any** provider I/O, a durable PaymentAttempt MUST already exist **and** its `PREPARED → SUBMITTED` transition MUST already be **committed** with a stable `provider_idempotency_key` (§12.4.1). Existence of `PREPARED` alone is **not** sufficient to call the provider.

7. **Reclaim predicate (normative).** On reclaim of an expired `IN_PROGRESS` reservation, the reclaiming executor MUST inspect the bound PaymentAttempt (when the command is payment-related).

   - If attempt status is `SUBMITTED`, `UNKNOWN`, `SUCCEEDED`, or `FAILED`: MUST **reconcile/query** provider state for that **same** attempt identity first; MUST NOT initiate a **new** external payment side effect merely because the local request lease expired.
   - If attempt status is `PREPARED` with **no** durable evidence of submission (`SUBMITTED` never committed): MAY proceed with first submission **only** via §12.4.1 (assign key → `SUBMITTED` commit → then I/O).
   - **Lease expiry is not evidence that provider I/O did not occur.**
   - Process crash is not evidence that the provider request did not happen.
   - Forbidden crash-recovery strategy: `SUBMITTED` (or later) → create a new PaymentAttempt → blind provider call.

8. **Completion.** On success, durably set `COMPLETED` with stored result in the same DB transaction as the business result being finalized (or immediately after under the same command’s commit boundary without releasing the reservation to another executor mid-finalize).

9. **Failure.** Permanent business failure of the command may finalize `FAILED_CLOSED` with stored error result per domain rules. Do not use `FAILED_CLOSED` for mere provider timeout/uncertainty.

10. **Restart.** Process restart loses only in-memory work; reservations and attempts in PostgreSQL remain authoritative.

11. **Concurrent first writers.** Only one insert wins `UNIQUE (tenant_id, idempotency_key)`; losers re-read and follow (3) or completed/failed replay.

12. **Lease expiry** updates reservation lease/status mechanics only; it MUST NOT delete audit, PaymentAttempt, observation, or ledger evidence.

### 9.3 Behavior summary

| Case | Outcome |
| --- | --- |
| First request | Insert `IN_PROGRESS` + lease (+ attempt if needed) atomically; execute; finalize `COMPLETED` / `FAILED_CLOSED` |
| Duplicate, `COMPLETED`, same fingerprint | Return stored result; no new financial effect |
| Duplicate, active `IN_PROGRESS`, same fingerprint | Non-executing deterministic in-progress/conflict |
| Same key, different fingerprint | Conflict; do not apply |
| Expired `IN_PROGRESS` | Reclaim per §9.2(5–7); apply PaymentAttempt reclaim predicate (§9.2(7) / §12.4.2) |

### 9.4 Retention

Idempotency reservation rows are retained for a configurable TTL (numeric default **OPEN**; long enough for client retries). Purging a reservation MUST NOT purge financial/audit/attempt evidence. Lease expiration ≠ retention purge.

---

## 10. Crash Recovery Architecture

| Failure | Behavior |
| --- | --- |
| Process crash mid-request before commit | No durable transition; client retry with same idempotency key; expired `IN_PROGRESS` reclaim per §9 |
| Crash after `SUBMITTED` commit, before/during provider I/O | Attempt remains `SUBMITTED` (or later `UNKNOWN`); reclaim MUST reconcile — not blind new attempt (§12.4.3) |
| Crash after commit of business result | Resume from durable state; idempotent no-op on retry |
| DB transaction rollback | No partial authoritative write; safe retry |
| Provider timeout | Mark existing PaymentAttempt `UNKNOWN` / timeout disposition; **not** auto-`FAILED` transaction; schedule reconcile |
| Webhook late / duplicate / out-of-order | Durable observation + idempotent apply against PaymentAttempt; ignore stale if state already advanced; never invent certainty |
| Recovery of uncertain operation | Query provider / reconcile **existing** attempt; promote to success/failure only with evidence |
| Worker crash after claim | Outbox lease expires; job reclaimable with new `claim_epoch` (§11); handlers idempotent; stale worker fenced |
| Worker crash before ack | Same — lease reclaim; at-least-once + fencing |
| Outbox remains PENDING | Eligible when `next_attempt_at <= now` |
| Reconciliation finds external success | Apply evidence to existing attempt/transaction per ADR-007; do not fabricate |

### 10.1 PaymentAttempt crash matrix (IA-FROZEN / ARCHITECTURAL REQUIREMENT)

| Durable attempt state before crash | Provider I/O possibility | Recovery |
| --- | --- | --- |
| `PREPARED`, before committed `SUBMITTED` | **No** provider I/O permitted yet | May proceed with **first** submission only via §12.4.1 |
| `SUBMITTED` | **May** have occurred | Reconcile/query first; do not create a new attempt for blind retry |
| `UNKNOWN` | **May** have occurred | Reconcile/query first |
| `SUCCEEDED` | Already completed | Do not duplicate external effect; finalize idempotency from evidence |
| `FAILED` | Follow provider/domain failure semantics | Do not blindly recreate external effect; new commercial attempt needs new identity when policy requires |

A process crash does **not** prove provider failure. Lease expiry does **not** prove provider I/O did not occur.

**Distinguish:**

```text
DB transaction rollback  ≠  external provider uncertainty
crash / lease expiry     ≠  provider did not execute
```

No universal exactly-once claim (ADR-004 §9). Desired invariant: **exactly-once business effect** via durable identity + idempotency + reconciliation (at-least-once delivery + idempotent processing).

---

## 11. Outbox / Async Processing

### 11.1 Decision

| Field | Value |
| --- | --- |
| **DECISION** | **Transactional outbox / job table + dedicated worker process** polling/claiming jobs in PostgreSQL (**mutable operational state**, §4.1.1A) |
| **QUEUE/BROKER** | **Not required** for initial architecture |
| **RATIONALE** | Async work (webhook delivery, provider status poll, reconciliation checks, expiration sweeps) must commit atomically with domain changes. A Postgres outbox preserves that atomicity without introducing Kafka/NATS operational surface or a second durability system. At-least-once worker delivery + idempotent handlers + **fencing** preserve correctness under duplicate/delay/stale workers. |
| **ALTERNATIVES** | Kafka/NATS (defer until scale/ops justify); Redis queues (extra durability class); in-process only (fails crash recovery) |
| **TRADE-OFFS** | Polling latency; requires lease reclaim **and** fencing (SKIP LOCKED alone is insufficient) |
| **FROZEN_OR_OPEN** | **IA-FROZEN** (outbox+worker); external broker **OPEN** |

### 11.2 Job lifecycle (IA-FROZEN)

**States:** `PENDING` | `RUNNING` | `SUCCEEDED` | `DEAD_LETTER`

| Field | Purpose |
| --- | --- |
| `status` | Lifecycle above |
| `locked_by` / worker identity | Claim holder while `RUNNING` |
| `claim_epoch` (or `fencing_token`) | Monotonic ownership generation; incremented on every successful claim/reclaim |
| `lease_expires_at` | Visibility timeout; expired `RUNNING` is reclaimable |
| `attempt_count` | Processing attempts |
| `next_attempt_at` | Eligibility time (`PENDING` when `next_attempt_at <= now`) |
| `last_error` (optional metadata) | Durable error context for retries / dead-letter |

### 11.3 Normative worker behavior (IA-FROZEN)

1. `PENDING` jobs become eligible when `next_attempt_at <= now`.
2. Worker claims inside a PostgreSQL transaction using `SELECT … FOR UPDATE SKIP LOCKED` on eligible rows.
3. Claim MUST set `RUNNING`, `locked_by`, a future `lease_expires_at`, and **increment `claim_epoch`** (or issue a new `fencing_token`) so the claim has a unique ownership generation.
4. Worker crash MUST NOT permanently stall the job: when `RUNNING` and `lease_expires_at < now()`, the job is **reclaimable** (exclusive lease-gated reclaim that also advances `claim_epoch`).
5. Expired `RUNNING` leases MUST become reclaimable under (4).
6. Reclaim is **at-least-once**: handlers MUST be idempotent; no exactly-once worker assumption.
7. **Retryable failure:** increment `attempt_count`, set `last_error`, schedule `next_attempt_at` with bounded backoff, set `PENDING` — **only if** the writer still owns the current `claim_epoch` / fencing identity (§11.4).
8. **Permanent failure / exhausted attempts:** transition to `DEAD_LETTER` under the same fencing condition; preserve evidence; do not silently delete.
9. **Success:** durable transition to `SUCCEEDED` under fencing (§11.4).
10. Duplicate execution (reclaim after partial work) MUST be safe via idempotent handlers **and** fencing.
11. Handlers that can cause external or financial side effects MUST be idempotent and keyed by durable operation/job identity.
12. No assumption of exactly-once worker execution.

### 11.4 Fencing / conditional completion (IA-FROZEN / ARCHITECTURAL REQUIREMENT)

Every claimed job MUST carry a unique ownership generation (`claim_epoch` or equivalent `fencing_token`).

Transitions:

```text
RUNNING → SUCCEEDED
RUNNING → DEAD_LETTER
RUNNING → PENDING   (retry path)
```

MUST be **conditional** on current ownership. Conceptually:

```text
UPDATE outbox
SET status = $next, ...
WHERE id = $job_id
  AND claim_epoch = $epoch_held_by_this_worker
  AND locked_by = $this_worker
  AND lease_expires_at > now()   -- still valid under this claim
```

If the conditional update matches zero rows, the completion MUST **fail closed** as a **no-op** (stale writer).

**Stale worker rule:** A worker whose lease/claim has been superseded MUST NOT mutate authoritative job state. Idempotent handlers remain required; fencing is an **additional** operational safety boundary. `FOR UPDATE SKIP LOCKED` alone does not prevent stale completion after lease loss.

**Claim sketch (illustrative):**

```text
UPDATE outbox
SET status = 'RUNNING',
    locked_by = $worker,
    lease_expires_at = now() + lease,
    claim_epoch = claim_epoch + 1,
    attempt_count = attempt_count + 1
WHERE id IN (
  SELECT id FROM outbox
  WHERE (status = 'PENDING' AND next_attempt_at <= now())
     OR (status = 'RUNNING' AND lease_expires_at < now())
  ORDER BY next_attempt_at NULLS FIRST, id
  FOR UPDATE SKIP LOCKED
  LIMIT N
)
RETURNING id, claim_epoch
```

---

## 12. Payment Architecture

### 12.1 Port

Ports are defined in `tx4-application` (§5.2):

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
| Mock | **CORE NOW** (tests + local); Phase 2 may ship Mock against the port |
| Xendit / DOKU / Midtrans | **LATER** / Phase 4 (authorized separately; license review ADR-003) |

Provider SDKs stay in adapter crates; **never** leak provider enums into public `/v1` domain states.

### 12.3 Rules

- Provider authoritative for provider-side facts; TX4 authoritative for TX4 transaction state (ADR-004)
- `transaction_state ≠ provider_state`
- Timeouts → disposition uncertainty on PaymentAttempt; not automatic transaction `FAILED`
- Webhooks verified, durably stored as observations, applied idempotently against PaymentAttempt
- Provider operation identity stored for reconciliation
- This protocol strengthens safety under at-least-once delivery; it is **not** a claim of universal exactly-once external execution (ADR-004)

### 12.4 PaymentAttempt (IA-FROZEN durable protocol)

**PaymentAttempt** is a first-class durable record (**mutable operational status** + linked append-only observations). It is **not** TX4 transaction lifecycle authority.

**Minimum fields:**

| Field | Purpose |
| --- | --- |
| `attempt_id` | Stable TX4 identity |
| `tenant_id` | Isolation |
| `transaction_id` | Parent aggregate |
| `operation_id` | Logical operation identity (unique with tenant per §9.1) |
| `idempotency_key` / reservation link | When created under an idempotent command |
| `provider_adapter` | Adapter identity (e.g. `mock`, later `xendit`) — not a public domain enum leak |
| `provider_ref` | Provider-side reference when known |
| `provider_idempotency_key` | Stable key for the provider call; assigned **before** `SUBMITTED` commit |
| `status` | Attempt lifecycle below |
| `outcome_disposition` | Aligns with ADR-007 uncertainty (`TIMEOUT` / `UNKNOWN` / …) where applicable |
| timestamps | created/updated |

**Attempt statuses (IA-FROZEN):**

| Status | Meaning |
| --- | --- |
| `PREPARED` | Durably inserted; **provider I/O MUST NOT occur** until `SUBMITTED` is committed |
| `SUBMITTED` | Durable marker that provider submission is **potentially in flight**; committed **before** provider I/O |
| `UNKNOWN` | Timeout or lost response; external outcome uncertain |
| `SUCCEEDED` | TX4 has evidence of provider-side success for this attempt |
| `FAILED` | TX4 has evidence of known provider/operation failure for this attempt (≠ transaction `FAILED` automatically) |

Refund execution product remains **OPEN**; do not encode full refund product states here. Later refund attempts may be separate records.

### 12.4.1 Durable submission boundary (IA-FROZEN / ARCHITECTURAL REQUIREMENT)

**Invariant:** Before **any** provider I/O for a payment attempt, `PREPARED → SUBMITTED` MUST be durably persisted and **committed**. `SUBMITTED` establishes that provider submission is considered potentially in-flight.

Normative sequence:

```text
BEGIN DB TX
  create/validate PaymentAttempt (PREPARED) if needed
  assign stable provider_idempotency_key (same logical attempt identity)
  transition PREPARED → SUBMITTED
COMMIT
ONLY NOW → provider I/O (outside DB locks; reuse provider_idempotency_key)
```

Retrying the **same** logical provider operation MUST reuse the same durable PaymentAttempt / `operation_id` / `provider_idempotency_key` — not create an unrelated external side effect. Creating a **new** PaymentAttempt for crash recovery while an existing attempt is `SUBMITTED`/`UNKNOWN`/`SUCCEEDED`/`FAILED` is **forbidden** as a blind-retry strategy.

### 12.4.2 Reclaim integration with idempotency (normative)

When an idempotency reservation is reclaimed (§9.2(7)), apply this predicate to the bound PaymentAttempt:

| Attempt status | Allowed action |
| --- | --- |
| `PREPARED` (never committed `SUBMITTED`) | First submission via §12.4.1 only |
| `SUBMITTED` / `UNKNOWN` / `SUCCEEDED` / `FAILED` | Reconcile/query first; no new blind provider side effect |

**Lease expiry is not evidence that provider I/O did not occur.**

### 12.4.3 Additional normative rules

1. PaymentAttempt MUST be durably created in `PREPARED` before the submission boundary of §12.4.1.
2. PaymentAttempt MUST have a stable `attempt_id` / `operation_id`.
3. MUST link `tenant_id`, `transaction_id`, `operation_id`, and idempotency context when applicable (same DB txn as reservation per §9.2).
4. Provider requests MUST carry/use the stable `provider_idempotency_key` where the provider abstraction supports idempotency.
5. Timeout → `UNKNOWN` (or equivalent disposition); MUST NOT alone move transaction to `FAILED`.
6. Recovery MUST reconcile/query the **existing** attempt before creating a new external side effect when status is not purely pre-submission `PREPARED`.
7. Duplicate webhooks resolve against the durable attempt + observation uniqueness.
8. PaymentAttempt history MUST NOT replace or silently rewrite TX4 primary lifecycle authority (ADR-007).

---
## 13. Money Architecture (ADR-006 → Rust)

### 13.1 Types (IA-FROZEN)

```text
CurrencyId        // catalog identity (code + catalog version concept)
Money {
  amount: i64     // signed atomic units; range floor ≥ i64 per ADR-006 §13
  currency: CurrencyId
}
```

| Layer | Type | Rule |
| --- | --- | --- |
| Authoritative atomic amount | **`i64`** | Storage, domain value, JSON integer-string parsing target |
| Multiply / allocation intermediates | **`i128`** | Checked arithmetic only |
| Narrowing | checked `i128` → `i64` | **Fail closed** on overflow / out-of-range |
| Arbitrary big-int | **OPEN / out of initial scope** | Not selected for initial implementation |
| `f32` / `f64` | **Forbidden** as financial authority | ADR-006 |

Additional rules:

- Serde: `amount` as **string**; reject JSON numbers as sole authority
- Half-even per ADR-006 §9; allocation per §11 (reject negative source)
- Overflow: fail checked, never wrap
- Comparison: same currency only; cross-currency compare forbidden without FX boundary
- FX: **out of money core** — conversion only at explicit FX boundary (**OPEN** provider)

### 13.2 Module placement

`tx4-domain` owns Money primitives and property tests. Persistence stores `amount` as `BIGINT` + `currency` text/code; never `DOUBLE PRECISION` / `REAL` / float-typed money.

---

## 14. Ledger Architecture

### 14.1 Decision

**Append-only ledger entries** as authoritative TX4 internal financial records:

- `entry_id`, `tenant_id`, `transaction_id` (nullable when not tied), `account_id`, posting fields (see §14.3), `money`, `reason`, `created_at`, `operation_id`

**Balances:** derived by summing entries (or materialized snapshot marked **derived**). Materialized balances MUST be rebuildable; never a second authority.

### 14.2 Non-goals

Not a full ERP (AR/AP/GL productization). No silent merge with Cloud credits (ADR-004 §5.8).

### 14.3 Posting convention status

**Status: OPEN.**

Ledger debit/credit vs signed-amount posting convention is **not** selected by this specification. It MUST be selected and frozen in an authorized task **before** ledger implementation (Phase 5). Do not leave the choice to silent coding judgment.

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
- Diagnostic “audit-style” log events for sensitive transitions (telemetry only)
- Exporters OTel-compatible; **vendor OPEN**

**Authority boundary:** Logs, traces, and metrics are diagnostic/operational telemetry only. They are **not** authoritative financial, lifecycle, or audit records. Durable transaction, ledger, audit, observation, and reconciliation records remain authoritative according to their defined source-of-truth roles (§4).

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

### 30.3 Operational contract (IA-FROZEN)

1. **Migration.** Database migrations MUST complete successfully before the application is considered ready to serve production traffic (`tx4-cli migrate` or equivalent before ready).
2. **Startup.** Server and worker MUST validate configuration before serving or claiming work.
3. **Readiness.** Server readiness MUST require successful database connectivity and required initialization (including schema readiness).
4. **Liveness.** Liveness MUST distinguish process health from dependency readiness. Liveness MUST NOT permanently fail merely because PostgreSQL is temporarily unavailable if the process itself remains healthy.
5. **Graceful server shutdown.** Stop accepting new work; drain in-flight application work within a bounded shutdown procedure (numeric timeout **OPEN**).
6. **Graceful worker shutdown.** Stop claiming new jobs; allow active work to finish or become safely reclaimable via outbox lease (§11).
7. **Database reconnect.** Transient database connection failures MUST use bounded retry/reconnect behavior (numeric defaults **OPEN**).
8. **Restart.** A restarted server/worker MUST resume from durable PostgreSQL state. No claim of zero-loss or exactly-once external execution.

---
## 31. Implementation Phases

> Phases are planning only. Each requires a later authorizing task. No code in TASK-018.

### Phase 1 — Foundation / domain primitives

- **Prereqs:** AUDIT-021 PASS; implementation task authorized
- **Deliverables:** workspace, `tx4-domain` Money + lifecycle pure logic, unit/property tests
- **Invariants:** ADR-006/007 encoded; no float money
- **Exit:** domain tests green; no persistence yet optional

### Phase 2 — Persistence / transaction engine

Phase 2 remains the aggregate planning phase for the durable transaction engine. Its original deliverables are unchanged in intent:

- PostgreSQL schema/migrations
- repositories
- `FOR UPDATE` transitions
- idempotency reservation protocol (§9)
- outbox lease protocol (§11)
- **PaymentProvider port + Mock adapter** (§12; interface ownership stays in `tx4-application`; Mock for foundational tests)

- **Invariants:** durable state; concurrency precedence; compensating intent atomicity; no blind re-execution
- **Tests:** integration concurrency/idempotency/recovery/outbox reclaim
- **Exit:** engine proof on Mock clock/provider ports

To make implementation gates auditable, Phase 2 is decomposed into the following **lettered milestones**. Lettered milestones do **not** invent new subsystems; they only bound already-frozen Phase-2 requirements. Normative semantics remain in §8–§12 (and related sections); this subsection only assigns milestone ownership.

#### Phase 2A — PostgreSQL Transaction Engine Schema Foundation

- **Status:** **COMPLETE / FROZEN** (implementation HEAD `22567a13a1af3bb6661eec91e3b0f41144b17884`)
- **Deliverables:** PostgreSQL schema/migrations for the transaction aggregate foundation under `crates/tx4-persistence/migrations/`
- **Does not include:** repositories, `FOR UPDATE` mutation paths, idempotency, outbox, PaymentProvider/Mock, API business workflows
- **Exit:** migration validation + schema foundation tests against real PostgreSQL

#### Phase 2B — Durable Transaction Repository & Concurrency Boundary

- **Status:** **COMPLETE / FROZEN** (implementation HEAD `366855fd6448d3b729ddaa2918383c0fd7e80211`; P2-1 ReplayDuplicate durable-return remediated)
- **Deliverables:** durable Transaction Repository; PostgreSQL aggregate mapping; `SELECT … FOR UPDATE` mutation boundary; version/concurrency control; tenant-scoped load/mutate; Phase-1F lifecycle integration at the repository boundary; atomic persistence; PostgreSQL concurrency/integration tests
- **Does not include:** idempotency reservation, outbox lease/fencing, PaymentProvider execution, real providers, webhooks, ledger/settlement/reconciliation/billing
- **Exit:** repository + concurrency invariants proven on real PostgreSQL; cargo fmt/check/test/clippy green

#### Phase 2C — Durable Idempotency, Outbox & Payment Boundary

- **Status:** **COMPLETE / FROZEN** (implementation HEAD `7174a227067a2085238899d744a1000226baa7a0`; final independent re-audit PASS / FREEZE_READY)
- **Freeze path:** scope baseline `fdbc9b6fddc1dc63f4a9e53c4d178239cd0b25e5` → original implementation tip `0ae9419e308f994cd30669be172947c044ed0ea5` → remediation `89dfe8771d84791ff9a0f150a139d2079d2b2bf9` → TOCTOU remediation / freeze baseline `7174a227067a2085238899d744a1000226baa7a0` → PHASE-2C-FINAL-INDEPENDENT-RE-AUDIT PASS
- **Verified at freeze:** atomic PostgreSQL unit-of-work; durable idempotency; idempotency concurrency safety; idempotency recovery; durable outbox; outbox lease/fencing; outbox recovery; PaymentProvider port; MockPaymentProvider; PaymentAttempt durable boundary; reconcile-first recovery; PaymentAttempt row locking; TOCTOU protection; lock ordering; deadlock safety; PostgreSQL persistence; migrations; crash recovery; concurrency; financial safety; durability; dependency direction; Phase-2B ReplayDuplicate regression preserved
- **Findings at freeze:** P0 NONE; P1 NONE; P2 NONE; P3 NONE; BLOCKERS NONE; ARCHITECTURE_DRIFT NONE; SCOPE_DRIFT NONE; UNAUTHORIZED_IMPLEMENTATION NONE
- **Semantics preserved:** at-least-once processing + durable state + idempotency + unique operation identity + reconciliation (**no** universal exactly-once claim)
- **Prereqs:** Phase 2A frozen; Phase 2B frozen
- **Deliverables (ONLY):**
  1. **Durable idempotency reservation protocol** as already specified in §9 (tenant-scoped key, fingerprint, stored response contract, `IN_PROGRESS` / completed / failed-closed semantics, lease/expiry, crash reclaim, duplicate/fingerprint-conflict behavior, concurrency, atomicity with the business operation, `operation_id` uniqueness, no blind re-execution under a valid lease, restart durability)
  2. **Durable outbox lease protocol** as already specified in §11 (`PENDING` / `RUNNING` / `SUCCEEDED` / `DEAD_LETTER`, lease/visibility timeout, `claim_epoch` fencing, conditional completion, stale-worker no-op, reclaim after crash, retry/backoff, poison/dead-letter handling, idempotent handler boundary, `FOR UPDATE SKIP LOCKED` where specified, durable worker recovery). **No external message broker.**
  3. **PaymentProvider port + Mock adapter** as already specified in §12 (`createPayment`, `capture`, `refund`, `getPaymentStatus`; Mock deterministic test behavior; application-level interface ownership). **No real providers** (Xendit/DOKU/Midtrans remain Phase 4).
- **Explicitly out of scope for Phase 2C:** real payment provider integrations; provider webhooks; full production payment execution workflows; PaymentAttempt production execution beyond what is strictly required to establish the generic PaymentProvider/Mock boundary; ledger posting; settlement execution; reconciliation engine; commercial/usage billing; subscriptions; Cloud control plane; managed dashboard; OAuth/external IdP; advanced analytics; dogfood apps; vertical-specific logic; frontend; SDK; production Cloud deployment
- **Tests:** PostgreSQL integration; concurrency; crash/recovery; idempotency; outbox fencing/reclaim; deterministic Mock behavior; atomic unit-of-work; reclaim TOCTOU / reconcile-first locking
- **Exit:** satisfied — all Phase-2C deliverables above proven; cargo fmt/check/test/clippy green; no Phase-4/5/6/7/8 functionality pulled forward

Lettering dependency inside Phase 2: **2A → 2B → 2C**. Lettered milestones **2A, 2B, and 2C are COMPLETE / FROZEN**. Aggregate Phase-2 lettered exit is satisfied; Phase 3 remains separately defined and is **not** authorized by this freeze.

### Phase 3 — API / authentication baseline

- **Deliverables:** Axum `/v1` skeleton, OpenAPI bootstrap, API keys, tenant context, error mapping
- **Invariants:** ADR-005 path versioning; authz checks
- **Exit:** contract tests for initial routes

### Phase 4 — Payment adapters + webhooks

- **Deliverables:** Real provider adapter integrations (when authorized), production webhook ingest, disposition/PaymentAttempt wiring beyond Mock
- **Does not redefine** the PaymentProvider port (owned since Phase 2)
- **Invariants:** timeout≠failed; provider≠transaction state
- **Exit:** e2e payment uncertainty tests (Mock remains; real providers as authorized)

### Phase 5 — Financial primitives (fee/ledger foundations)

- **Prereq:** Ledger posting convention MUST be selected/frozen before this phase (§14.3 remains **OPEN** until that gate)
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

Dependency graph: 1→2→3→4; inside Phase 2: 2A→2B→2C; 5–7 after 2; 8 continuous after 3.

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

### IA-FROZEN by this specification (**document FROZEN** — FREEZE-012)

- Tokio, Axum, PostgreSQL, SQLx, Serde, tracing+OTel-compatible, REST+OpenAPI, SQLx migrations, Docker packaging
- Hybrid relational persistence with explicit **mutable operational** vs **append-only evidence/financial** taxonomy (not event sourcing)
- `FOR UPDATE` + version concurrency; global lock order; bounded deadlock/serialization retry
- Durable idempotency reservation protocol with lease/reclaim + PaymentAttempt reclaim predicate
- PaymentAttempt durable submission boundary: committed `PREPARED → SUBMITTED` **before** provider I/O
- Outbox/job lifecycle `PENDING|RUNNING|SUCCEEDED|DEAD_LETTER` with lease reclaim + **claim_epoch fencing** + conditional completion
- `UNIQUE (tenant_id, operation_id)` (and idempotency key uniqueness)
- Money: authoritative `i64` + checked `i128` intermediates; string JSON amounts; no float authority
- Minimal deployment operational contract (migrate-before-ready, readiness/liveness, graceful shutdown, reconnect)

### Remain OPEN

- Cloud vendor, K8s, brokers, auth IdP product, hosted OTel backend, exact routes/fields, pricing, payment provider canon, FX provider, refund product details, numeric retention TTLs / lease durations / retry bounds / attempt-budget defaults, multi-region, dogfood architectures, SDK languages beyond eventual TS-first intent (SDK implementation **OPEN**)
- **Ledger posting convention** (debit/credit vs signed) — OPEN until before Phase 5 (§14.3)
- Arbitrary big-int money intermediates — out of initial scope

Freeze does **not** convert remaining OPEN decisions into frozen ones.

---

## 34. Conflict Check vs Frozen ADRs

| ADR | Compatibility |
| --- | --- |
| ADR-001 Rust | Compatible — Rust selected |
| ADR-002 Apache-2.0 | Compatible — no license change; deps must follow ADR-003 |
| ADR-003 | Compatible — selections are permissive-ecosystem; concrete crate license review at add-time |
| ADR-004 SoT | Compatible — Postgres is technology; authorities unchanged; submission boundary ≠ universal exactly-once claim |
| ADR-005 | Compatible — `/v1` REST |
| ADR-006 | Compatible — integer atomic + string JSON; half-even; allocation; i128 intermediates |
| ADR-007 | Compatible — matrix/precedence/uncertainty implemented, not replaced |
| ADR-008 | Compatible — OSS independent; Cloud separated |

**ARCHITECTURE_CONFLICTS:** NONE identified at freeze time.

---

## 35. Freeze Non-Actions

FREEZE-012 does **not**:

- modify ADR-001…008 or Master Spec
- add `src/`, `crates/`, migrations, or dependencies
- authorize coding or Phase 1 implementation

Next gate: **IMPLEMENTATION-AUTHORIZATION** (explicit task required before code).

---

## Document history

| Date | Event |
| --- | --- |
| 2026-09-25 | TASK-018 created PROPOSED implementation architecture |
| 2026-09-25 | TASK-018R remediated AUDIT-021 findings (persistence taxonomy, idempotency/outbox leases, money intermediates, locks, PaymentAttempt, ops contract) |
| 2026-09-25 | TASK-018RR remediated AUDIT-022 findings (SUBMITTED-before-I/O, outbox fencing, taxonomy, worktree integrity) |
| 2026-09-25 | FREEZE-012 froze implementation architecture (basis AUDIT-023 PASS) |
| 2026-09-26 | PHASE-2C-SCOPE-DEFINITION: §31 decomposes Phase 2 into lettered milestones 2A (frozen), 2B (frozen), 2C (defined, not authorized/implemented); normative §9/§11/§12 unchanged |
| 2026-09-26 | PHASE-2C-FREEZE: Phase 2C COMPLETE / FROZEN at HEAD `7174a227067a2085238899d744a1000226baa7a0` (final independent re-audit PASS / FREEZE_READY; P0–P3 NONE; no architecture/scope drift); normative §9/§11/§12 unchanged |
