# ADR-008 — Managed Cloud / OSS Boundary

| Field | Value |
| --- | --- |
| ADR | **008** |
| Title | Managed Cloud / OSS Boundary |
| Status | **FROZEN** |
| Date | 2026-09-25 |
| Frozen | 2026-09-25 (FREEZE-011; basis AUDIT-019 PASS) |
| Decision scope | Boundary between TX4 Open Source transaction infrastructure and TX4 Managed Cloud (hosted proprietary experience / operations) |
| Related Master Spec | §3 OSS Boundary; §4 Managed Cloud Boundary; §5 Core Design Principle; §11 Billing & Usage; §19 API; §20 SDK; §23 Managed Cloud; §24 Dogfood Applications; §27 OSS Governance; §36 Current Status |
| Depends on | ADR-001 (Rust) — FROZEN; ADR-002 (Apache-2.0) — FROZEN; ADR-003 — FROZEN; ADR-004 (SoT / durability) — FROZEN; ADR-005 (API versioning) — FROZEN; ADR-006 (monetary representation) — FROZEN; ADR-007 (transaction lifecycle) — FROZEN |
| Supersedes | None |

---

## 1. Status

**FROZEN**

Freeze path completed: TASK-017 → AUDIT-019 → FREEZE-011.

This ADR defines the **product and distribution boundary** between TX4 OSS and TX4 Managed Cloud. It does **not** select cloud providers, databases, frameworks, payment providers, deployment platforms, pricing packages, or implementation topologies.

---

## 2. Context

TX4’s product principle:

```text
Open-source transaction infrastructure for developers.
```

TX4 consists of two primary layers (Master Spec):

```text
TX4 OSS  — self-hostable transaction infrastructure
TX4 Cloud — proprietary managed operation and hosted developer experience
```

Core commercial model:

```text
OPEN-SOURCE THE INFRASTRUCTURE.
PROPRIETIZE THE HOSTED EXPERIENCE.
```

Dogfood applications (canteen, booking, marketplaces) **consume** TX4; they do not define TX4 Core or this boundary.

Frozen ADRs already constrain the core:

* ADR-001 Rust runtime
* ADR-002 Apache-2.0 OSS license
* ADR-003 dependency / third-party license policy
* ADR-004 SoT, durability, cloud-credit separation
* ADR-005 URI path API versioning (`/v{N}/...`, initial `/v1/...`)
* ADR-006 monetary representation
* ADR-007 transaction lifecycle / uncertainty / idempotency / recovery

ADR-008 must place Cloud around that core without reopening it.

---

## 3. Problem

Without an explicit boundary, TX4 risks:

1. Crippling OSS to force Cloud adoption
2. Leaking proprietary Cloud operations into OSS
3. Requiring Cloud for basic correctness (lifecycle, durability, money safety)
4. Creating a second authoritative domain model in Cloud that conflicts with OSS
5. Confusing Cloud commercial billing/credits with transactional ledgers and customer payments
6. Letting dogfood apps or Cloud UX dictate core semantics
7. Silently selecting infrastructure technologies under the guise of “Cloud needs”

---

## 4. Decision

**Adopt the Managed Cloud / OSS boundary principles in this ADR.**

Summary:

1. **TX4 OSS** is the self-hostable, production-grade transaction infrastructure distribution.
2. **TX4 Managed Cloud** is the proprietary hosted control plane, managed operations, and commercial hosted experience around the same conceptual infrastructure.
3. OSS remains independently usable for basic durable transaction correctness without Cloud.
4. Cloud must not redefine core transaction, money, or SoT semantics.
5. Self-hosted and Cloud share the same conceptual public API contract for core TX4 (ADR-005).
6. Cloud-only capabilities are operational/control-plane/commercial, clearly separated from domain authority.
7. Dogfood apps remain outside Core and do not set this boundary.
8. Technology selections (DB, cloud vendor, frameworks, payment providers, etc.) remain OPEN.

---

## 5. OSS Boundary

### 5.1 What TX4 OSS is

TX4 OSS is the **open-source transaction infrastructure** that a developer can inspect, run, and self-host to obtain the core TX4 domain capabilities.

### 5.2 OSS scope candidates (boundary intent, not implementation authorization)

The following are **intended OSS boundary candidates** when/if implemented under later authorized tasks:

* transaction core and domain primitives
* public API and OpenAPI/contract artifacts
* API versioning behavior per ADR-005
* persistence schemas/migrations **required for self-hosting** (technology OPEN)
* payment abstraction and provider adapter **interfaces** (and legally distributable adapters when authorized)
* fee, usage/metering, billing, ledger, settlement, and reconciliation **primitives** (as generic infrastructure, not Cloud packaging)
* webhook, idempotency, tenant-isolation, authn/authz, API-key/OAuth, audit, and generic rate-limit primitives
* SDKs, Docker/self-host packaging, configuration, tests, local tooling, docs, examples/reference integrations

### 5.3 OSS must include correctness

When the corresponding domain capability exists in TX4, the OSS distribution MUST include the ability to provide:

* durable transaction lifecycle semantics (ADR-007)
* financial representation safety (ADR-006)
* SoT / durability / idempotency / recovery principles (ADR-004)
* versioned public API contracts (ADR-005)

These correctness properties are **not** Cloud-exclusive.

### 5.4 What OSS is not

OSS is not:

* the proprietary hosted dashboard
* the Managed Cloud control plane
* TX4’s internal fleet/ops automation
* a vertical dogfood application
* a demo-only shell of the real product

---

## 6. Managed Cloud Boundary

### 6.1 What TX4 Managed Cloud is

TX4 Managed Cloud is the **commercial hosted offering**: proprietary operation of TX4 for customers who prefer managed infrastructure and hosted developer experience.

### 6.2 Managed Cloud scope candidates (boundary intent, not implementation authorization)

The following may remain **proprietary Managed Cloud** concerns:

* hosted control plane
* proprietary hosted dashboard / project UX
* managed deployment, scaling, backups
* cloud orchestration / fleet management
* hosted observability and operational automation
* commercial analytics for Cloud operation
* managed support tooling
* Cloud billing infrastructure for TX4 Cloud itself (commercial metering of Cloud usage)
* Cloud-specific operational controls and automation

### 6.3 What Cloud must not be

Cloud must not become:

* a second conflicting domain authority for TX4 transaction facts
* a required dependency for OSS correctness
* a place where vertical dogfood logic redefines Core
* a stealth freeze of database/cloud-vendor/payment architecture

### 6.4 Cloud may wrap OSS

Cloud may:

* host and operate OSS-compatible TX4 infrastructure
* add proprietary operational services around it
* expose the same conceptual developer API for core TX4
* add clearly separated Cloud operational/control-plane surfaces

---

## 7. Shared Contract Principle

```text
Self-hosted:  Developer → TX4 OSS API (/v{N}/...)
Managed Cloud: Developer → TX4 Cloud API (/v{N}/... conceptual core)
```

Principles:

1. Both expose the **same conceptual transaction infrastructure** for core TX4 capabilities.
2. ADR-005 remains authoritative for API versioning (`/v{N}/...`, initial `/v1/...`).
3. Self-hosted OSS and Managed Cloud MUST NOT intentionally diverge the public `/vN` **core** contract for the same major.
4. Cloud-only operational / control-plane endpoints, if any, MUST be clearly separated and MUST NOT silently redefine core TX4 `/vN` domain semantics (aligns with ADR-005 §13).
5. This ADR does **not** design routes, schemas, or SDKs.

---

## 8. Non-Crippled OSS Principle

### 8.1 Definition

“Not intentionally crippled” means TX4 MUST NOT remove or withhold **essential transaction infrastructure capabilities** from OSS solely to create artificial Cloud dependency.

### 8.2 Prohibited crippling patterns

TX4 MUST NOT:

* remove essential transaction primitives from OSS solely to force Cloud adoption
* intentionally disable correctness features in OSS
* withhold basic self-hosting capability
* make OSS a demo-only implementation of the real product
* require proprietary Cloud services for durable transaction correctness
* require proprietary Cloud services for basic transaction lifecycle (ADR-007)
* make core financial safety (ADR-006 / ADR-004) dependent on Cloud

### 8.3 Legitimate commercial differentiation

Commercial differentiation SHOULD primarily come from:

* managed operations and convenience
* hosted infrastructure and automation
* operational visibility / scaling / backups
* hosted dashboard and developer UX
* commercial support/services
* Cloud-specific operational capabilities

This ADR does **not** define pricing tiers or package SKUs.

### 8.4 What “essential” means here

Essential = capabilities required for a self-hosted deployment to run TX4 as production-grade **transaction infrastructure** for the domain features TX4 claims to provide, including correctness properties frozen in ADR-004/006/007 for those features.

It does **not** mean Cloud must open-source its control plane, dashboard, or ops automation.

---

## 9. Authority and Source-of-Truth Boundary

Compatible with ADR-004:

1. TX4 domain authority (transaction lifecycle, TX4 financial records, etc.) remains TX4 Core semantics — not replaced by Cloud marketing state.
2. External payment providers remain authoritative for provider-side payment execution facts.
3. **Cloud control-plane state** (org membership, fleet placement, Cloud subscription, operational flags) MUST be distinguishable from **TX4 domain state**.
4. Cloud MUST NOT silently create a second authoritative domain model that conflicts with the OSS core for the same economic/domain facts.
5. Developer Managed Cloud credit / Cloud commercial billing remains separate from:
   * customer payment balances
   * merchant balances
   * TX4 financial ledger  
   (ADR-004 §5.8 preserved.)
6. Cloud commercial billing MUST NOT be confused with application-level customer billing primitives that TX4 OSS may provide as infrastructure.

```text
Cloud control-plane / fleet state
        ≠
TX4 domain transaction state
        ≠
provider payment state
        ≠
Cloud commercial credit / Cloud billing
        ≠
customer/merchant transactional balances
```

---

## 10. Security and Tenancy Boundary

1. **OSS / domain tenancy primitives** (tenant isolation for TX4 domain objects) are Core infrastructure concerns.
2. **Cloud platform tenancy** (Cloud organization / account / project / fleet membership) is a platform/control-plane concern.
3. Cloud platform tenancy MUST NOT silently replace or redefine the transaction-domain tenant model.
4. Mapping between Cloud accounts and domain tenants, if any, must be explicit and must not weaken isolation.
5. No authentication provider, secret manager, cloud IAM product, or database is selected by this ADR.
6. Least privilege and isolation principles apply on both sides; Cloud ops access must not imply domain authorization shortcuts that bypass Core checks.

---

## 11. Durability and Correctness Boundary

ADR-004 / ADR-006 / ADR-007 correctness is part of **OSS infrastructure**, not a Cloud exclusive.

Therefore the following MUST NOT be Cloud-only for the Core product claims:

* durable transaction semantics
* crash-recovery correctness
* idempotency correctness
* lifecycle correctness
* monetary / financial safety

Cloud MAY provide managed operational mechanisms (managed DB, backups, ops automation) that **support** these properties in a hosted environment, but the **domain guarantees themselves** belong to the transaction infrastructure boundary and must remain achievable in self-hosted OSS without proprietary Cloud services.

---

## 12. Licensing Boundary

ADR-002 and ADR-003 remain authoritative.

1. OSS artifacts follow **Apache-2.0** and ADR-003 dependency/third-party policy.
2. Proprietary Cloud artifacts are **outside** the OSS distribution boundary.
3. This boundary must not be used to smuggle license-incompatible dependencies into OSS.
4. Proprietary Cloud code MUST NOT be required to compile or run OSS core.
5. This ADR does **not** introduce a new project license or dual-license scheme.

---

## 13. Commercial Boundary

1. OSS is the adoption and distribution layer for transaction infrastructure.
2. Managed Cloud is the commercial delivery model for hosted convenience/operations.
3. Cloud monetization (usage of Cloud, credits, invoices) is commercial Cloud concern; exact pricing remains OPEN.
4. Application-level fees/billing/ledger primitives in OSS are infrastructure for **developers’ applications**, not the same thing as TX4 Cloud’s own commercial billing.
5. This ADR is not a pricing specification and creates no SLA by itself.

---

## 14. Compatibility and Evolution

1. Core domain semantics (money, lifecycle, SoT, API major contracts) evolve under existing ADR rules and versioning (ADR-005/006/007).
2. Cloud operational features may evolve independently if they do not redefine Core contracts.
3. Breaking Core API meaning requires ADR-005-compatible major versioning — Cloud hosting does not grant a silent exception.
4. Self-hosted upgrades and Cloud-hosted upgrades may differ operationally; Core contract compatibility principles remain shared.
5. Documentation MUST make OSS vs Cloud boundaries discoverable (what is open, what is hosted/proprietary).

---

## 15. Documentation Expectations

Public documentation SHOULD clearly state:

* what is in OSS
* what is Managed Cloud / proprietary
* that OSS is self-hostable for core infrastructure
* that Cloud is optional for basic correctness
* how the shared `/vN` conceptual API relates to both
* that dogfood apps are separate from Core

Exact docs site/tooling remains OPEN.

---

## 16. Open Decisions

Explicitly **OPEN** (unless already frozen elsewhere):

* database technology / persistence implementation / event sourcing vs CRUD
* Rust HTTP framework / crates beyond ADR-001 language freeze
* queue/broker, deployment topology, cloud provider, storage provider
* payment providers / payment architecture / refund / settlement / reconciliation **implementations**
* observability implementation / authentication provider
* exact Cloud pricing, commercial packaging, Cloud billing implementation details
* SDK implementation details / API endpoint design
* operational architecture / fleet topology

ADR-008 does **not** freeze any of the above.

---

## 17. Non-Goals

Out of scope for this ADR:

* implementing OSS or Cloud
* selecting vendors or frameworks
* designing dashboard UX
* defining price lists or SLAs
* designing dogfood applications
* reopening ADR-001…007
* changing LICENSE text or project license

---

## 18. Consequences

### Positive

* Clear OSS vs Cloud product split
* Protects non-crippled self-hosting and correctness-in-OSS
* Preserves shared Core API concept (ADR-005)
* Protects SoT and cloud-credit separations (ADR-004)
* Prevents dogfood/Cloud UX from capturing Core
* Keeps technology choices OPEN

### Costs / tradeoffs

* Requires ongoing discipline to keep Cloud features from leaking into Core
* Some Cloud UX conveniences may need dual paths (ops API vs domain API)
* Maintainers must document boundary carefully to avoid customer confusion

---

## 19. Alternatives Considered

| Alternative | Why rejected |
| --- | --- |
| Cloud-required core correctness | Contradicts self-hostable / non-crippled OSS thesis |
| Fully open-source Cloud control plane by default | Optional later; not required for this boundary; commercial hosted experience may remain proprietary |
| Separate incompatible Cloud domain API | Creates split-brain; conflicts with ADR-005 shared-contract intent |
| Let dogfood apps define Core boundary | Violates infrastructure ≠ vertical principle |
| Freeze DB/cloud vendor inside ADR-008 | Implementation leakage; remains OPEN |

---

## 20. Compatibility With Existing ADRs

* **ADR-001:** Cloud/OSS boundary does not change Rust as core runtime.
* **ADR-002 / ADR-003:** OSS stays Apache-2.0 + dependency policy; Cloud proprietary stays outside OSS.
* **ADR-004:** Domain SoT preserved; Cloud credit ≠ transactional balances; Cloud control-plane ≠ domain state.
* **ADR-005:** Shared `/vN` core contract principle reinforced; Cloud-only ops surfaces separated.
* **ADR-006:** Money model remains Core; not Cloud-exclusive; no second money authority.
* **ADR-007:** Lifecycle/correctness remains Core/OSS-capable; not Cloud-exclusive.

---

## 21. Auditability Requirements

Boundary-relevant decisions and distributions SHOULD remain auditable enough to answer:

* Is this artifact OSS or proprietary Cloud?
* Does a Cloud feature redefine Core semantics? (must be no, unless a superseding ADR)
* Are Cloud credits kept separate from transactional money?
* Can OSS run Core correctness without proprietary Cloud code?

Exact audit tooling remains OPEN.

---

## 22. Reconsideration Conditions

Revisit only if:

1. Objective evidence shows the boundary prevents a required product capability that cannot be expressed as Cloud-around-OSS
2. A superseding ADR authorizes a justified Core/Cloud API divergence
3. License/commercial model changes via explicit ADR-002-compatible process

---

## 23. References

* Master Spec §3 OSS Boundary; §4 Managed Cloud Boundary; §23 Managed Cloud; §24 Dogfood
* ADR-004 Core Source-of-Truth Boundaries — FROZEN (incl. cloud-credit separation)
* ADR-005 API Versioning Strategy — FROZEN
* ADR-006 Monetary Representation — FROZEN
* ADR-007 Transaction Lifecycle — FROZEN
* ADR-001 / ADR-002 / ADR-003 — FROZEN
