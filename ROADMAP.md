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

Nothing below implies completion unless marked **IMPLEMENTED**.

---

## Phase 0 — Repository Foundation

**Status:** IN PROGRESS

* Git repository and `main` branch
* OSS governance documents
* Master Spec
* ADR directory skeleton
* Issue / PR templates
* CI baseline (foundation hygiene only)
* Contributor and agent rules

---

## Phase 1 — Architecture & ADR Freeze

**Status:** PLANNED

Required ADRs before implementation freeze:

| ADR | Topic | Status |
| --- | --- | --- |
| ADR-001 | Core Runtime Language | OPEN |
| ADR-002 | OSS License | OPEN |
| ADR-003 | Dependency / Third-Party License Policy | OPEN |
| ADR-004 | Core Source-of-Truth Boundaries | OPEN |
| ADR-005 | API Versioning Strategy | OPEN |
| ADR-006 | Monetary Representation | OPEN |
| ADR-007 | Transaction Lifecycle | OPEN |
| ADR-008 | Managed Cloud / OSS Boundary | OPEN |

---

## Phase 2 — Core Domain Foundation

**Status:** PROPOSED

* Tenant / identity primitives
* Domain module boundaries
* Persistence strategy after ADR freeze
* Observability baseline

---

## Phase 3 — Transaction Infrastructure

**Status:** PROPOSED

* Transaction as first-class domain object
* Deterministic lifecycle
* Idempotency semantics
* Audit trail foundations

---

## Phase 4 — Payment / Fee / Billing

**Status:** PROPOSED

* Payment provider abstraction
* Fee primitives
* Usage / metering
* Managed Cloud billable-event semantics

---

## Phase 5 — Ledger / Settlement / Reconciliation

**Status:** PROPOSED

* Ledger boundary
* Settlement state and recovery
* Explicit reconciliation records

---

## Phase 6 — API / SDK

**Status:** PROPOSED

* Versioned REST API + OpenAPI
* TypeScript SDK priority
* Additional SDKs based on adoption

---

## Phase 7 — Production Hardening

**Status:** PROPOSED

* Security evidence
* Migration safety
* Backup / recovery procedures
* Expanded CI (format, lint, tests, audits, containers, SBOM)

---

## Phase 8 — Managed Cloud

**Status:** OPEN

* Hosted TX4
* Control plane / dashboard (proprietary as applicable)
* Usage visibility and billing
* Operational reliability

Exact cloud architecture remains OPEN.

---

## Phase 9 — Dogfood Applications

**Status:** PROPOSED

* Canteen (initial candidate)
* Booking / service marketplace / marketplace (future)

Dogfood apps must consume the public API.

---

## Phase 10 — Public Adoption

**Status:** PROPOSED

* Public documentation maturity
* Contributor onboarding
* Release cadence
* Grant / diligence readiness materials based on evidence only

---

## Notes

* Do not fabricate users, deployments, volume, funding, or partnerships.
* Production readiness requires evidence, not compilation alone.
* See [docs/MASTER-SPEC.md](docs/MASTER-SPEC.md) for product status and non-goals.
