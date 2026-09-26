# Roadmap

Status labels:

```text
PROPOSED
OPEN
PLANNED
IN PROGRESS
FROZEN
IMPLEMENTED
```

Nothing below implies completion unless marked **IMPLEMENTED** or **FROZEN** with an explicit freeze/implementation baseline.

### Authority note (Phase numbering)

For **implementation sequencing**, frozen [Implementation Architecture](docs/IMPLEMENTATION-ARCHITECTURE.md) §31 is the **canonical** phase numbering for Phase 1–8.

This Roadmap is realigned to that numbering. Historical Roadmap text that called “Phase 3 — Transaction Infrastructure” described work that is already covered by IA Phase 1 (domain/lifecycle) and Phase 2A/2B/2C (persistence, idempotency, outbox, PaymentProvider/Mock) and is **not** the next coding milestone.

---

## Phase 0 — Repository Foundation

**Status:** IMPLEMENTED (foundation complete; ongoing hygiene allowed)

* Git repository and `main` branch
* OSS governance documents
* Master Spec
* ADR directory
* Issue / PR templates
* CI baseline
* Contributor and agent rules

---

## Phase 1 — Foundation / domain primitives + ADR freeze

**Status:** FROZEN / IMPLEMENTED (ADR-001…008 FROZEN; Phase-1 lettered runtime/domain foundations complete per IA §31 / project freeze records)

Required ADRs (all **FROZEN**):

| ADR | Topic | Status |
| --- | --- | --- |
| ADR-001 | Core Runtime Language | **FROZEN** |
| ADR-002 | OSS License | **FROZEN** |
| ADR-003 | Dependency / Third-Party License Policy | **FROZEN** |
| ADR-004 | Core Source-of-Truth Boundaries | **FROZEN** |
| ADR-005 | API Versioning Strategy | **FROZEN** |
| ADR-006 | Monetary Representation | **FROZEN** |
| ADR-007 | Transaction Lifecycle | **FROZEN** |
| ADR-008 | Managed Cloud / OSS Boundary | **FROZEN** |

Also includes workspace runtime foundations (config, persistence pool/migrations foundation, observability, application ports, server/worker bootstrap, domain Money + lifecycle) as recorded under Phase-1 lettered freezes.

---

## Phase 2 — Persistence / transaction engine

**Status:** FROZEN (lettered milestones 2A / 2B / 2C COMPLETE / FROZEN)

Canonical decomposition (IA §31):

* **2A** — PostgreSQL transaction schema foundation
* **2B** — Durable transaction repository + `FOR UPDATE` concurrency
* **2C** — Durable idempotency, outbox lease/fencing, PaymentProvider port + Mock, PaymentAttempt durable boundary

Historical note: earlier Roadmap wording “Phase 3 — Transaction Infrastructure” (transaction object, lifecycle, idempotency, audit foundations) maps here and to Phase 1 domain work — **not** to current Phase 3.

---

## Phase 3 — API / authentication baseline

**Status:** SCOPE FROZEN (implementation **not** authorized until `PHASE-3-IMPLEMENTATION-AUTHORIZATION`)

**Canonical title:** API / authentication baseline

**Objective:** Public OSS `/v1` API authentication, authorization, tenant context, OpenAPI bootstrap, and consistent error mapping — on top of frozen Phase-2C durability — without SDKs, real providers, ledger, Cloud, or dogfood.

**Deliverables (ONLY):** see IA §31 Phase 3 (authoritative detail).

Summary:

* Axum `/v1` skeleton (ADR-005)
* OpenAPI bootstrap
* API key authentication primitive (OSS)
* Tenant context + authz checks
* Consistent HTTP error mapping
* Request correlation wiring
* HTTP ↔ application wiring that preserves Phase-2C guarantees

**Explicitly not Phase 3:** TypeScript/other SDKs; OAuth/external IdP; real payment providers/webhooks; ledger/settlement/reconciliation/billing; Cloud; dogfood; full endpoint catalog.

**Next gate:** `PHASE-3-IMPLEMENTATION-AUTHORIZATION`

---

## Phase 4 — Payment adapters + webhooks

**Status:** PROPOSED

* Real provider adapter integrations (when authorized)
* Production webhook ingest
* PaymentAttempt/disposition wiring beyond Mock
* Does **not** redefine the PaymentProvider port (owned since Phase 2C)

---

## Phase 5 — Financial primitives (fee / ledger foundations)

**Status:** PROPOSED

* Ledger entries / fee hooks as generic infrastructure
* Requires ledger posting convention freeze before implementation (IA §14.3 OPEN until that gate)
* No competing authoritative balances (ADR-004 / ADR-006)

---

## Phase 6 — Usage / billing foundations

**Status:** PROPOSED

* Usage records, dedupe, aggregation
* Application billing primitives ≠ Cloud developer billing (ADR-008)

---

## Phase 7 — Settlement / reconciliation foundations

**Status:** PROPOSED

* Settlement obligations/observations
* Explicit reconciliation discrepancy/resolution records
* No fabricated certainty

---

## Phase 8 — Production hardening

**Status:** PROPOSED

* Docker packaging maturity, security evidence, migration safety
* Expanded CI, failure injection, operational docs
* Backup / recovery procedures

---

## Phase 9 — Managed Cloud

**Status:** OPEN

* Hosted TX4
* Control plane / dashboard (proprietary as applicable)
* Usage visibility and Cloud billing
* Operational reliability

Exact cloud architecture remains OPEN (ADR-008).

---

## Phase 10 — Dogfood Applications

**Status:** PROPOSED

* Canteen (initial candidate)
* Booking / service marketplace / marketplace (future)

Dogfood apps must consume the **same public API** as external developers (Master Spec §24).

---

## Phase 11 — Public Adoption

**Status:** PROPOSED

* Public documentation maturity
* Contributor onboarding
* Release cadence
* Grant / diligence readiness materials based on evidence only

---

## SDK packaging (not a Phase-3 deliverable)

**Status:** PROPOSED (after Phase-3 API/auth baseline exists)

* TypeScript SDK priority (Master Spec §20)
* Additional SDKs based on adoption
* Generated or maintained from authoritative OpenAPI where practical

---

## Notes

* Do not fabricate users, deployments, volume, funding, or partnerships.
* Production readiness requires evidence, not compilation alone.
* See [docs/MASTER-SPEC.md](docs/MASTER-SPEC.md) for product status and non-goals.
* See [docs/IMPLEMENTATION-ARCHITECTURE.md](docs/IMPLEMENTATION-ARCHITECTURE.md) §31 for canonical implementation-phase scope freezes.
* Preserve: at-least-once processing + durable state + idempotency + unique operation identity + reconciliation (**no** universal exactly-once claim).
