# ADR-001 — Core Runtime Language

| Field | Value |
| --- | --- |
| ADR | **001** |
| Title | Core Runtime Language |
| Status | **PROPOSED** |
| Date | 2026-09-24 |
| Decision scope | Core runtime language for TX4 only |
| Related Master Spec | §5 Core Design Principle; §21 Core Runtime; §22 Deployment; §25 Security; §26 Observability; §31 Production-Grade Requirement; §36 Current Status (`RUNTIME = OPEN`) |
| Supersedes | None |

---

## Context

TX4 is durable, open-source transaction infrastructure for developers building transactional applications.

The core must support production transactional workloads where:

* domain state must survive process failure
* operations must be recoverable
* retries must be safe
* idempotency matters
* financial effects must not be duplicated
* external payment providers may timeout or deliver duplicate events
* transaction state must remain durable
* reconciliation must remain possible

The Master Spec lists **Rust** as a candidate default for the core runtime, subject to this ADR before implementation freeze (`RUNTIME` remains **OPEN** until this ADR is independently audited and frozen).

An explicit language decision is required before Core Foundation implementation because it constrains:

* how domain invariants are encoded
* concurrency and failure handling styles
* deployment artifact shape for self-hosted OSS and Managed Cloud
* contributor tooling and long-term maintainability

This ADR decides **only** the core runtime language. It does not select frameworks, drivers, persistence architecture, license, monetary representation, or transaction lifecycle.

---

## Requirements

Relevant TX4 requirements for this decision:

1. **Domain correctness support** — strong ability to model explicit states, discourage invalid combinations, and keep business logic deterministic. Language choice does not by itself guarantee financial correctness.
2. **Durable transaction infrastructure suitability** — suitable for implementing durable state, crash recovery, retry-safe processing, idempotency, background work, webhooks, and reconciliation. Durability architecture itself remains OPEN.
3. **Concurrency** — concurrent API traffic, payment/webhook events, workers, settlement, and reconciliation jobs.
4. **PostgreSQL ecosystem fit** — mature drivers, pooling, transactions, async/database integration options, migrations, and production patterns. Specific libraries remain OPEN.
5. **API infrastructure fit** — REST, OpenAPI, authn/authz, validation, idempotency, webhooks, versioning. HTTP framework remains OPEN.
6. **Deployment** — container-friendly, operationally simple self-hosting, and suitable for Managed Cloud operation.
7. **Observability** — structured logs, metrics, tracing, OpenTelemetry-compatible tooling.
8. **Performance characteristics** — predictable resource use under concurrent transactional load. Not the sole criterion; no unsupported benchmark claims.
9. **Security assistance** — memory safety where applicable, reduced classes of memory bugs, dependency/tooling maturity. Language choice alone does not make TX4 secure.
10. **Developer productivity** — solo-developer suitability, testing/debugging, learning curve, contributor accessibility.
11. **Ecosystem / maintainability** — language maturity, libraries, OSS patterns, long-term maintainability.
12. **SDK separation** — core language is independent of developer-facing SDKs (REST/OpenAPI; TypeScript SDK first; others later).

---

## Options Considered

### Option A — Rust

Systems language with strong static typing, ownership/borrowing, and memory safety without a garbage collector. Commonly used for infrastructure services, CLI/tools, and network services. Candidate default in Master Spec §21.

### Option B — Go

Statically typed, garbage-collected language with a simple concurrency model (`goroutines`), fast compile times, and a large cloud-native / infrastructure ecosystem. Strong operational familiarity for many teams.

### Option C — Kotlin / JVM

Statically typed language on the JVM with mature tooling, strong typing, coroutines, and a large enterprise/library ecosystem. Deployed as JVM applications or native images (GraalVM) with additional operational choices.

Other languages (for example TypeScript/Node, Python, C#) were not treated as primary candidates: they are less aligned with TX4’s combination of durable financial/transaction infrastructure, self-hosted single-artifact operational preference, and correctness-oriented systems boundaries, without offering a clearly superior fit on the criteria above.

---

## Comparative Analysis

Qualitative comparison only. No numeric scores, weights, rankings, or winner/loser ordering.

| Criterion | Rust | Go | Kotlin/JVM |
| --- | --- | --- | --- |
| Domain modeling / type safety | Strong; ownership + enums/ADTs help encode states | Good; simpler type system, fewer compile-time invariants | Strong; rich type system, mature null-safety options |
| Financial-domain suitability | Good fit for explicit invariants; correctness still system-level | Good fit with discipline; GC pauses usually fine at API scale | Good fit; mature decimal/money libraries exist — representation still OPEN |
| Durable transaction infra | Strong fit for long-running services and careful failure handling | Mature fit; many durable workers/services in production | Mature fit; JVM ops patterns well understood |
| Concurrency | Predictable async/OS-thread models; higher design discipline | Strong; simple concurrency, easy worker fan-out | Strong; threads + coroutines; mature executors |
| PostgreSQL ecosystem | Mature drivers/tooling available; specific stack OPEN | Mature drivers/pooling/migrations; specific stack OPEN | Very mature JDBC/R2DBC/ecosystem; specific stack OPEN |
| API / OpenAPI ecosystem | Mature enough; fewer batteries than JVM/Go in places | Strong HTTP/OpenAPI ecosystem | Very strong HTTP/OpenAPI/enterprise API ecosystem |
| Deployment / self-hosting | Strong; native binary, small runtime surface | Strong; single static binary common | Trade-off; JVM runtime or native-image complexity |
| Observability | Good OpenTelemetry / tracing ecosystem | Mature cloud-native observability | Mature JVM metrics/tracing ecosystems |
| Performance characteristics | Strong predictability / low overhead potential | Strong enough for most API workloads | Strong throughput; heavier default runtime footprint |
| Memory safety / bug classes | Strong memory safety without GC | GC safety; different classes of concurrency bugs remain | JVM memory safety; large runtime attack/config surface |
| Developer productivity | Higher learning curve / compile-time friction | High productivity; fast feedback | High productivity for JVM-experienced developers |
| Solo-developer maintainability | Trade-off; more upfront discipline, fewer runtime surprises | Strong; simplicity aids solo ops | Trade-off; runtime/tooling surface larger |
| Contributor accessibility | Trade-off; fewer casual contributors, higher bar | Strong accessibility for many infra engineers | Strong where JVM talent is common |
| SDK separation impact | Neutral — clients use REST/TS SDKs | Neutral | Neutral |

### Supporting discussion

**Rust** aligns with TX4’s need for explicit state encoding, memory safety, and a native deployable artifact that fits self-hosted OSS plus Managed Cloud operations. Costs are real: learning curve, compile times, and a smaller pool of casual contributors.

**Go** optimizes for implementation speed and operational familiarity. It remains a credible alternative for durable services. Relative to TX4’s emphasis on compile-time invalid-state prevention and minimal runtime surface for self-hosting, it offers fewer structural guardrails than Rust’s type/ownership model, while still requiring the same system-level financial discipline.

**Kotlin/JVM** offers excellent API and PostgreSQL ecosystems and strong typing. The default operational model (JVM process, GC tuning, larger artifact/runtime) conflicts more with TX4’s preference for simple self-hosted deployment unless native-image complexity is accepted. That operational trade-off is material for an OSS self-host target.

---

## Decision

**Select Rust as the core runtime language for TX4.**

Status of this decision: **PROPOSED** (not FROZEN).

Implementation of TX4 core in Rust is **not authorized** until this ADR passes independent audit and is explicitly frozen.

---

## Rationale

Rust is proposed because it best matches TX4’s combination of:

1. **Correctness-oriented boundaries** — strong static typing and exhaustive enum/match patterns support explicit transaction and payment state machines once ADR-007 (and related specs) exist. This assists invalid-state prevention; it does not guarantee financial correctness.
2. **Infrastructure suitability** — well suited to long-running services that must handle concurrent requests, webhooks, workers, and careful failure paths without a heavy managed runtime.
3. **Memory safety without GC** — reduces entire classes of memory-safety bugs relevant to network-facing infrastructure, while leaving authentication, tenancy, and secret hygiene as system responsibilities.
4. **Deployment shape** — native executables favor Docker/Compose self-hosting and Managed Cloud packaging with a smaller default runtime dependency set than a JVM service.
5. **Ecosystem adequacy** — PostgreSQL access, HTTP/API services, and OpenTelemetry-compatible observability are achievable without selecting specific libraries in this ADR.
6. **SDK independence** — external developers interact via REST/OpenAPI and SDKs (TypeScript first). Core language need not match client languages.

Performance is a supporting characteristic, not the sole argument.

---

## Consequences

### Positive consequences

* Clear implementation language for future Core Foundation work after freeze
* Encourages explicit domain modeling and failure handling at compile time
* Favorable self-hosting and container packaging characteristics
* Aligns with Master Spec’s documented Rust candidacy without treating candidacy as a freeze
* Keeps client SDK strategy independent of core language

### Negative consequences / costs

* Steeper learning curve than Go for many contributors
* Longer compile times and heavier local iteration than Go in typical setups
* Smaller casual-contributor pool than Go or JVM ecosystems
* More ceremony for some everyday tasks (error types, async bounds, trait design)
* Hiring/contributor accessibility trade-offs for a solo-maintained OSS project

### Risks

* Over-engineering or premature abstraction in Rust idioms if solo-developer principle is ignored
* Ecosystem gaps requiring more custom code than JVM for some enterprise integrations
* Contributor friction slowing review velocity if onboarding docs are weak
* Mistaking Rust’s type system for completed financial/security correctness

### Operational implications

* CI and release tooling will eventually center on Rust toolchains (not authorized by this ADR)
* Container images can ship a single binary plus minimal OS base once implementation exists
* Runtime diagnostics will rely on structured logging/tracing libraries chosen in later decisions

### Contributor implications

* Contributors to core must be willing to work in Rust
* Documentation and examples must invest in onboarding
* Non-Rust developers remain first-class via API/SDK surfaces

---

## Explicit Non-Decisions

This ADR does **not** decide:

* HTTP framework (for example Axum, Actix Web, or others)
* Async runtime architecture details
* PostgreSQL driver or query library (for example SQLx, Diesel, or others)
* Database schema or migrations
* Durability architecture (event sourcing, outbox, queues, WAL strategy, etc.)
* PostgreSQL as final authoritative source of truth (ADR-004)
* API versioning (ADR-005)
* Monetary representation (ADR-006)
* Transaction lifecycle / state machine (ADR-007)
* OSS / Managed Cloud boundary details (ADR-008)
* OSS license (ADR-002)
* Dependency / third-party license policy (ADR-003)
* Payment providers or adapters
* Ledger, billing, settlement, or reconciliation implementations
* SDK implementation languages beyond the existing Master Spec priority (TypeScript first)
* Cloud orchestration, Kubernetes, or Terraform

Master Spec §21 candidate stack names (Axum, Tokio, SQLx, etc.) remain **illustrative candidates**, not decisions of this ADR.

---

## Reconsideration Conditions

Revisit this PROPOSED (or later FROZEN) decision only if objective evidence shows one or more of:

1. Critical ecosystem gap that blocks production-grade PostgreSQL, observability, or secure deployment for TX4’s scope
2. Sustained inability to maintain the core with available contributors despite reasonable onboarding investment
3. Demonstration that another candidate materially reduces operational risk for both self-hosted OSS and Managed Cloud without regressing correctness or safety properties TX4 requires
4. Master Spec product scope changes so that core language requirements materially change
5. Legal/compliance constraints that make Rust tooling or dependency reality incompatible (evaluated with ADR-002/ADR-003 as applicable)

Preference or familiarity alone is not sufficient to reopen the decision after freeze.

---

## Relationship to Future ADRs

Once this ADR is **FROZEN**:

* Later ADRs and implementation tasks must treat Rust as the core runtime language
* Framework, driver, and durability ADRs must be expressed within a Rust implementation context unless this ADR is formally superseded
* OPEN decisions (license, money, lifecycle, SoT, API versioning, OSS/Cloud boundary) remain independently decidable and must not be smuggled into Rust “defaults”

Until freeze:

* No Rust application implementation is authorized by this document
* `RUNTIME` in the Master Spec remains **OPEN** pending audit/freeze gates

---

## Notes for Reviewers

* This ADR proposes Rust; it does **not** claim Rust guarantees correctness or security.
* Comparative analysis is qualitative and requirement-driven.
* Independent audit (AUDIT-003) must verify scope, non-decisions, and absence of unauthorized freezes before any freeze gate.
