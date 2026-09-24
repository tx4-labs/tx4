# TX4 — Transaction Infrastructure

**Domain:** `tx4.xyz`
**Status:** DRAFT — Pre-Freeze
**Version:** 0.1
**Product Type:** Open-Source Transaction Infrastructure + Managed Cloud
**Primary Goal:** Production-grade, commercially ready, self-hostable transaction infrastructure.

---

## 1. Product Definition

**TX4 is an open-source transaction infrastructure platform for developers building transactional applications.**

TX4 provides reusable infrastructure for applications such as:

* ordering
* booking
* marketplaces
* service marketplaces
* event/ticketing
* closed-community commerce
* other vertical transactional applications

TX4 is infrastructure, not a vertical application.

### Core thesis

Developers should not need to rebuild:

* transaction lifecycle
* payment integration
* fees
* billing/metering
* ledger integration
* settlement
* reconciliation
* idempotency
* webhooks
* auditability

for every new transactional product.

TX4 provides these primitives behind a stable developer-facing API.

---

## 2. Product Architecture

TX4 consists of two primary layers:

```text
                    TX4
                     │
          ┌──────────┴──────────┐
          │                     │
       TX4 OSS              TX4 Cloud
       Core                  Managed
          │                     │
      Self-hosted          Hosted service
          │                     │
          └──────────┬──────────┘
                     │
                 Developer
                  API / SDK
                     │
             ┌───────┼───────┐
             ↓       ↓       ↓
          Canteen Booking Marketplace
```

---

## 3. OSS Boundary

The following are intended to be open source:

* transaction core
* domain primitives
* public API
* API specification
* database schema/migrations required for self-hosting
* payment abstraction
* payment adapters where legally distributable
* fee primitives
* usage/metering primitives
* settlement primitives
* reconciliation primitives
* webhook infrastructure
* idempotency infrastructure
* tenant isolation
* authentication/authorization primitives
* SDKs
* Docker deployment
* test suites
* documentation
* examples

The OSS distribution must be genuinely useful for production self-hosting.

TX4 must not intentionally cripple the OSS edition to force Cloud adoption.

---

## 4. Managed Cloud Boundary

TX4 Cloud is the commercial hosted offering.

The following may remain proprietary:

* cloud control plane
* hosted dashboard
* cloud orchestration
* managed deployment
* managed scaling
* managed backups
* operational automation
* hosted observability
* internal operations tooling
* commercial analytics
* cloud-specific automation

The Managed Cloud does not replace the OSS core.

It operates the same fundamental TX4 transaction model as the OSS distribution.

---

## 5. Core Design Principle

## Infrastructure ≠ Vertical Application

TX4 Core must remain vertical-neutral.

Forbidden architecture:

```text
if vertical == CANTEEN:
    ...
```

Preferred architecture:

```text
Generic primitive
        ↓
Vertical application
```

A feature may enter TX4 Core only when it represents a reusable infrastructure primitive across multiple transactional domains.

---

## 6. Initial Core Domains

Initial candidate domains:

```text
M01 Identity & Tenant
M02 Provider / Merchant
M03 Customer
M04 Catalog / Offering
M05 Transaction / Order
M06 Booking
M07 Payment
M08 Fee
M09 Usage & Metering
M10 Billing
M11 Ledger
M12 Settlement
M13 Refund / Adjustment
M14 Reconciliation
M15 Webhook
M16 Audit
M17 Rate Limit
M18 API Keys / OAuth
```

These modules are candidates, not automatically implementation-authorized.

Each module requires explicit specification before implementation.

---

## 7. Transaction Model

TX4 treats a transaction as a first-class domain object.

A transaction must have:

* immutable identity
* tenant ownership
* customer/provider relationship where applicable
* monetary representation
* currency
* lifecycle state
* timestamps
* idempotency semantics
* audit trail
* associated payment state
* associated completion state

The transaction lifecycle must be deterministic.

Example:

```text
CREATED
   ↓
PENDING
   ↓
PAID
   ↓
PROCESSING
   ↓
COMPLETED
```

Exceptional states may include:

```text
CANCELLED
EXPIRED
FAILED
REFUNDED
PARTIALLY_REFUNDED
```

Exact state machine is subject to dedicated specification and ADR.

---

## 8. Payment Boundary

TX4 Core must not become a payment processor.

Payment providers are external authorities.

Architecture:

```text
                  TX4 Payment
                       │
              PaymentProvider
                       │
        ┌──────────────┼──────────────┐
        ↓              ↓              ↓
     Xendit           DOKU         Midtrans
     Adapter          Adapter        Adapter
```

TX4 owns the application/payment orchestration state.

The external provider owns provider-side payment authorization and processing.

Provider-specific implementation must not leak into the generic domain API.

---

## 9. Financial Correctness

Money must never be represented using binary floating-point arithmetic.

The core must use an exact monetary representation.

Candidate implementation:

```text
integer minor units
+
explicit currency
```

Example:

```text
IDR 25,000
USD 10.50
```

Money calculations must have explicit invariants.

No implicit currency conversion is permitted.

---

## 10. Fees

Fees are first-class domain concepts.

A transaction may produce:

```text
Gross Amount
      ↓
Fee Calculation
      ↓
Net Amount
```

Fees must be:

* deterministic
* auditable
* versioned where necessary
* explicitly attributed
* reproducible

TX4 must distinguish platform fees from external payment-provider fees.

---

## 11. Billing & Usage

TX4 Managed Cloud uses transaction-based monetization.

### Primary billable unit

A completed commercial transaction is the initial candidate billable unit.

API requests are not automatically billable transactions.

Therefore:

```text
1 Order
   ↓
multiple API calls
   ↓
1 completed transaction
   ↓
1 billable transaction
```

Retries and webhook deliveries must not accidentally create additional transaction charges.

Exact billing semantics require a dedicated billing specification.

---

## 12. Subscription

Subscription is primarily a **capacity/rate-limit mechanism**.

Subscription does not automatically represent transaction quota.

Concept:

```text
PAYG
  ↓
transaction usage charge

SUBSCRIPTION
  ↓
higher capacity / rate limits
```

The exact capacity tiers remain OPEN until commercial validation.

---

## 13. Ledger

TX4 must maintain a clear financial source-of-truth boundary.

Candidate architecture:

```text
TX4 Domain
     │
     ├── Transaction state
     ├── Payment state
     ├── Fee state
     └── Settlement state
             │
             ↓
          Ledger
```

If an external ledger engine is used, it must be integrated behind a TX4 abstraction.

Provider-specific ledger concepts must not become the public TX4 API.

---

## 14. Settlement

Settlement represents movement of finalized financial obligations toward the appropriate recipient.

Settlement must support:

* deterministic state
* idempotency
* retries
* failure recovery
* audit trail
* reconciliation

Settlement must not be considered complete merely because an internal state changed.

External settlement authority must be verified where applicable.

---

## 15. Reconciliation

Reconciliation compares TX4 records against external financial/payment authorities.

Examples:

```text
TX4
  vs
Payment Provider

TX4
  vs
Ledger

TX4
  vs
Settlement Provider
```

Differences must become explicit reconciliation records.

Silent correction is prohibited.

---

## 16. Idempotency

Any externally repeatable operation must define idempotency semantics.

Examples:

* create transaction
* create payment
* refund
* settlement request
* webhook processing

Repeated requests must not create duplicate financial effects.

---

## 17. Webhooks

Webhook handling must support:

* signature verification
* idempotent processing
* replay protection where appropriate
* retries
* delivery state
* auditability
* dead-letter/error handling

Webhook events are evidence/input, not automatically unquestioned truth.

---

## 18. Multi-Tenancy

TX4 must be multi-tenant from the foundation.

Every tenant-owned object must have explicit tenant ownership.

Cross-tenant access must be prevented by design.

Authorization must not rely solely on frontend checks.

---

## 19. API

Primary developer interface:

```text
REST API
+
OpenAPI specification
```

API must provide:

* versioning
* consistent errors
* idempotency
* authentication
* authorization
* pagination
* filtering
* request correlation
* rate-limit headers where appropriate

The API is a first-class product artifact.

---

## 20. SDK

Initial SDK priority:

1. TypeScript

Additional SDKs may be introduced based on actual adoption:

* Python
* Go
* Kotlin
* Rust
* others

SDKs must be generated or maintained from the authoritative API contract where practical.

---

## 21. Core Runtime

### Candidate default

**Rust**

Candidate stack:

```text
Rust
├── Axum
├── Tokio
├── PostgreSQL
├── SQLx
├── exact monetary representation
├── tracing
└── OpenTelemetry-compatible observability
```

This decision remains subject to ADR-001 before implementation freeze.

Selection criteria:

* correctness
* memory safety
* concurrency
* financial domain modeling
* operational simplicity
* ecosystem maturity
* PostgreSQL support
* payment integration
* Docker deployment
* long-term OSS maintainability

---

## 22. Deployment

Self-hosting must be first-class.

Initial target:

```text
Docker
+
Docker Compose
+
PostgreSQL
```

Target developer experience:

```text
clone repository
      ↓
configure environment
      ↓
docker compose up
      ↓
TX4 available
      ↓
API request
      ↓
transaction
```

The first successful local transaction should be achievable through documented quickstart instructions.

---

## 23. Managed Cloud

TX4 Cloud provides:

* hosted TX4
* project management
* API keys
* usage visibility
* billing
* capacity management
* observability
* managed database
* backups
* deployment automation
* operational reliability

The exact cloud architecture remains OPEN.

---

## 24. Dogfood Applications

Dogfood applications are separate from TX4 Core.

Initial candidate:

### Canteen

Demonstrates:

```text
Customer
    ↓
Order
    ↓
Payment
    ↓
Fee
    ↓
Completion
```

Future dogfood:

```text
Booking
Service Marketplace
Marketplace
```

Dogfood applications must consume TX4 through the same public API available to external developers.

This is an important architectural validation requirement.

---

## 25. Security Principles

Security is part of the core product.

Required areas:

* authentication
* authorization
* tenant isolation
* secret management
* API key security
* webhook signature validation
* replay protection
* rate limiting
* input validation
* dependency security
* audit logging
* secure defaults
* vulnerability disclosure
* supply-chain security

Security claims must be supported by evidence.

---

## 26. Observability

Production deployments must provide:

* structured logs
* metrics
* traces
* request correlation
* health checks
* readiness checks
* error classification

Financial events must be observable without exposing unnecessary sensitive data.

---

## 27. OSS Governance

The public repository must include:

* LICENSE
* README
* CONTRIBUTING
* CODE_OF_CONDUCT
* SECURITY
* GOVERNANCE
* ROADMAP
* CHANGELOG
* issue templates
* pull-request template
* release process

Documentation must distinguish:

```text
FROZEN
OPEN
PROPOSED
IMPLEMENTED
DEPRECATED
```

---

## 28. Licensing

License selection must balance:

* genuine OSS adoption
* commercial sustainability
* contributor participation
* grant compatibility
* ecosystem compatibility
* dependency licenses
* Managed Cloud business model

Final license requires explicit ADR.

No source code may be copied or integrated from third-party projects without license compatibility review.

---

## 29. Commercial Model

Initial model:

```text
SELF-HOSTED
Free OSS

MANAGED CLOUD
PAYG
+
Capacity Subscription
```

PAYG is based on defined transaction events.

Subscription primarily unlocks higher capacity/rate limits.

The model must avoid double charging the same economic transaction because of internal API activity.

---

## 30. Non-Goals

TX4 is not initially intended to become:

* a consumer wallet
* a bank
* a payment processor
* a generic ERP
* a complete e-commerce storefront
* a consumer marketplace
* a vertical-specific SaaS
* a replacement for every accounting system
* a replacement for every payment provider
* a replacement for every billing platform

TX4 provides infrastructure that applications can build upon.

---

## 31. Production-Grade Requirement

No component is considered production-ready merely because:

```text
it compiles
```

Production readiness requires evidence for:

* correctness
* tests
* failure handling
* concurrency
* security
* observability
* migration safety
* backup/recovery
* operational procedures
* API stability
* documentation
* reproducible deployment

---

## 32. Grant & Fundraising Readiness

TX4 must be designed so that a reviewer can independently understand:

### Problem

What developer pain does TX4 solve?

### Technology

What infrastructure does TX4 provide?

### OSS

What is genuinely open?

### Commercial

How does Managed Cloud monetize usage?

### Adoption

Who can use TX4 and why?

### Evidence

Which real applications use TX4?

### Roadmap

What remains to reach production scale?

### Governance

How can external contributors trust the project?

No fabricated traction or unsupported market claims are permitted.

---

## 33. Quality Gates

Implementation must follow:

```text
SPECIFICATION
      ↓
ADR
      ↓
FREEZE
      ↓
IMPLEMENTATION
      ↓
TEST
      ↓
AUDIT
      ↓
REMEDIATION
      ↓
RE-AUDIT
      ↓
RELEASE
```

Cursor is an implementation assistant.

Cursor does not have authority to silently change frozen architecture or business rules.

Any material deviation requires explicit specification/ADR change.

---

## 34. Architecture Rule

The following hierarchy is authoritative:

```text
Master Specification
        ↓
Architecture Decision Records
        ↓
Domain Specifications
        ↓
API Contract
        ↓
Implementation
        ↓
Tests
        ↓
Runtime
```

Implementation must not silently redefine the specification.

Tests must enforce important invariants.

Runtime behavior must be deterministic wherever the domain requires deterministic behavior.

---

## 35. Initial Repository Objective

The initial GitHub repository must communicate:

> **This is a serious open-source infrastructure project intended for real production use.**

It must not look like:

* a research notebook
* a university project
* a prototype
* a vertical SaaS repository
* an abandoned boilerplate
* an AI-generated code dump

The repository must prioritize:

**clarity → correctness → reproducibility → security → maintainability → adoption.**

---

## 36. Current Status

```text
PRODUCT THESIS       = PROPOSED
OSS/CLOUD BOUNDARY   = PROPOSED
CORE DOMAIN          = PROPOSED
RUNTIME              = OPEN (Rust candidate)
LICENSE              = OPEN
API                  = PROPOSED
BILLING MODEL        = PROPOSED
DOGFOOD STRATEGY     = PROPOSED
REPOSITORY STANDARD  = PROPOSED
PRODUCTION TARGET    = FROZEN AS OBJECTIVE
```

No implementation is authorized by this document until the relevant specifications and ADRs are explicitly frozen.

---

## 37. Immediate Next Decisions

Before implementation:

1. **ADR-001 — Core Runtime Language**
2. **ADR-002 — OSS License**
3. **ADR-003 — Dependency / Third-Party License Policy**
4. **ADR-004 — Core Source-of-Truth Boundaries**
5. **ADR-005 — API Versioning Strategy**
6. **ADR-006 — Monetary Representation**
7. **ADR-007 — Transaction Lifecycle**
8. **ADR-008 — Managed Cloud / OSS Boundary**

After these decisions:

**TASK-002 — Repository Foundation**

Then:

**TASK-003 — Core Architecture Specification**

Then module-by-module implementation.

---

## 38. Definition of Success

TX4 succeeds when a developer can:

```text
discover TX4
    ↓
read documentation
    ↓
run TX4 locally
    ↓
create tenant/project
    ↓
create transaction
    ↓
process payment
    ↓
complete transaction
    ↓
observe fee
    ↓
observe settlement
    ↓
reconcile
```

without requiring the developer to understand TX4's internal implementation language.

The same API contract must support:

```text
Self-hosted OSS
       +
TX4 Managed Cloud
```

The first real-world application must use the same infrastructure rather than bypassing it.

---

## 39. Product Principle

> **Build the transaction infrastructure once. Let developers build the vertical applications.**

TX4 Core is the product.

Canteen, booking, marketplace, and other applications are validation surfaces.

Managed Cloud is the commercial delivery model.

OSS is the adoption and distribution layer.

Production reliability is the non-negotiable requirement.
