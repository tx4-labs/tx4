# ADR-004 — Core Source-of-Truth Boundaries

| Field | Value |
| --- | --- |
| ADR | **004** |
| Title | Core Source-of-Truth Boundaries |
| Status | **PROPOSED** |
| Date | 2026-09-24 |
| Decision scope | Authority boundaries, durability/crash-recovery requirements, idempotency and reconciliation principles for TX4 durable transaction infrastructure |
| Related Master Spec | §7 Transaction Model; §8 Payment Boundary; §9 Financial Correctness; §10 Fees; §11 Billing & Usage; §13 Ledger; §14 Settlement; §15 Reconciliation; §16 Idempotency; §17 Webhooks; §18 Multi-Tenancy; §31 Production-Grade; §36 Current Status |
| Depends on | ADR-001 (Rust) — FROZEN; ADR-002 (Apache-2.0) — FROZEN; ADR-003 (Dependency license policy) — FROZEN |
| Supersedes | None |

---

## 1. Status

**PROPOSED** — not FROZEN.

Freeze requires independent audit and an explicit freeze task.

This ADR defines **authority and durability properties**. It does **not** select persistence technology, event-sourcing adoption, queue/workflow engines, monetary representation, or the exact transaction state machine.

---

## 2. Context

TX4 is durable, open-source transaction infrastructure for developers building transactional applications.

Master Spec requires deterministic transaction semantics, payment-provider boundaries, ledger/settlement/reconciliation primitives, idempotency, and production-grade failure handling.

Transaction correctness MUST NOT depend on:

* process memory
* local process-only state
* ephemeral local filesystem as sole authority
* a single running worker
* successful completion of an in-flight HTTP request
* external provider callbacks arriving exactly once
* external provider callbacks arriving in order
* a single application instance remaining alive

A process crash MUST NOT create an unrecoverable authoritative financial state.

PostgreSQL appears in the Master Spec as a **conceptual deployment candidate**. Persistence implementation remains **OPEN**. This ADR must not silently freeze PostgreSQL, SQLx, Kafka, Redis, outbox/inbox, event sourcing, or any other storage technology.

---

## 3. Critical Distinctions

ADR-004 separates these concepts:

| Concept | Meaning |
| --- | --- |
| **Authority** | Which system is authoritative for a fact? |
| **Durability** | Can TX4 recover that fact after process failure? |
| **Reconciliation** | How does TX4 resolve disagreement between TX4 durable state and an external system? |
| **Delivery** | How are commands/events/callbacks retried safely? |
| **Auditability** | Can resulting state and financial effects be explained later? |

These must not be collapsed into one concept.

---

## 4. Decision

**Adopt the source-of-truth, durability, recovery, idempotency, and reconciliation boundary principles defined in this ADR.**

Summary:

1. TX4 is authoritative for TX4 domain facts it owns.
2. External payment providers remain authoritative for provider-side payment execution facts.
3. TX4 maintains durable records of its own state and of observed external interactions.
4. Authoritative TX4 state must be durable across crashes and restarts.
5. Uncertainty must be representable; timeouts must not be silently interpreted as definitive provider outcomes.
6. Prefer at-least-once delivery + idempotent processing + durable unique operation identity + reconciliation over claims of universal exactly-once transport.
7. Cloud billing credit is separate from transactional financial balances.
8. Persistence **technology** and event-sourcing **pattern** remain OPEN; future designs must satisfy the properties herein.

---

## 5. Source-of-Truth Boundaries

### 5.1 TX4 domain state (TX4 authoritative)

TX4 is authoritative for its own domain model, including applicable:

* tenant / project ownership and configuration
* merchant / provider configuration **as modeled in TX4**
* customer records **as modeled in TX4**
* catalog / offering state **as modeled in TX4**
* TX4 transaction / order lifecycle
* booking state where applicable **as modeled in TX4**
* TX4-generated fees
* usage records produced by TX4 metering rules
* TX4 billing state for Managed Cloud commercial usage (distinct from customer payment balances)
* TX4 settlement obligations and settlement records
* TX4 reconciliation records
* TX4 audit evidence

Distinguish carefully:

```text
TX4's durable record of an external fact
        ≠
the external system's authority over that external fact
```

TX4 may store observed provider status; the provider remains authoritative for provider-side execution truth until reconciled.

### 5.2 Payment provider authority

External payment providers remain authoritative for provider-side facts such as:

* provider transaction / payment identifiers
* provider-side payment status
* provider-side payment execution / authorization outcomes
* provider-side reversal / refund status
* provider-side settlement information (as reported by the provider)

TX4 MUST NOT pretend to be the payment provider or overwrite provider authority with assumptions.

TX4 MUST maintain its own durable representation of:

* requests it sent
* responses it received
* callbacks / webhooks it observed
* uncertainty when outcome is unknown
* reconciliation outcomes relative to provider observations

### 5.3 Transaction authority

```text
TX4 transaction state
        ≠
payment provider state
```

* A provider observation of “paid” does **not** automatically mean every downstream TX4 operation has completed.
* A TX4 timeout does **not** mean the provider failed to execute.
* TX4 is authoritative for the TX4 transaction lifecycle once defined by a later ADR (ADR-007), subject to evidence rules in this ADR.
* Exact lifecycle states remain **OPEN** (ADR-007).

Uncertainty must be representable (for example PENDING, UNKNOWN, RECONCILIATION_REQUIRED, FAILED, CANCELLED) where appropriate. Exact enums remain OPEN.

### 5.4 Ledger authority

If / when TX4 maintains a ledger:

* The TX4 ledger is authoritative for **TX4 internal financial records**, including fee effects, internal movements, settlement obligations as recorded by TX4, and adjustments/refunds as recorded by TX4.
* The TX4 ledger is **not** authoritative over an external provider’s bank/provider-side balances.
* TX4 must avoid multiple competing authoritative balances for the same internal economic fact.
* Exact ledger account model remains OPEN.

```text
provider-side money authority
        ≠
TX4 internal financial record
```

### 5.5 Settlement authority

TX4 is authoritative for:

* settlement instructions it creates
* settlement records it stores
* settlement status **as modeled by TX4**
* settlement obligations it computes

External banking / payment infrastructure remains authoritative for actual external fund movement.

TX4 must durably record what it instructed, observed, and reconciled.

### 5.6 Reconciliation authority

Reconciliation is a **TX4-controlled** process for detecting and resolving disagreement between:

```text
TX4 durable state
        vs
external provider / infrastructure observations
```

Principles:

* Disagreement must be detectable.
* Uncertainty must be representable.
* Reconciliation must be repeatable.
* Retries must be idempotent.
* Reconciliation must not fabricate certainty.
* Irreversible financial effects must not be duplicated.
* Silent correction of financial records without an explicit reconciliation record is prohibited (aligned with Master Spec §15).

Concrete reconciliation algorithms remain OPEN.

### 5.7 Usage / billing authority

Conceptual chain:

```text
Commercial transaction
        ↓
billable usage (when rules say so)
        ↓
TX4 usage record
        ↓
Managed Cloud billing
```

Distinctions:

| Concept | Notes |
| --- | --- |
| API request | Not automatically a billable transaction |
| Internal operation / retry / webhook delivery | Must not accidentally create duplicate billable charges |
| TX4 transaction | Domain object; lifecycle OPEN (ADR-007) |
| Completed commercial transaction | Candidate billable unit per Master Spec §11 |
| Billable usage event | Derived according to billing rules (exact algorithm OPEN) |

TX4 is authoritative for usage records it emits under its metering rules. Exact billing semantics remain OPEN.

### 5.8 Developer cloud credit boundary

Preserve separation:

```text
Developer Managed Cloud credit
        ≠
Customer payment balance
        ≠
Merchant balance
        ≠
TX4 financial ledger
```

Prepaid cloud credits (if used) are a **commercial billing mechanism** for Managed Cloud capacity/usage. They must not be merged with transactional payment/merchant balances or treated as interchangeable ledger money.

OSS/Managed Cloud product boundary details remain ADR-008.

---

## 6. Authority Matrix

| Fact | Authoritative System | TX4 Durable Record Required | Reconciliation Needed |
| --- | --- | --- | --- |
| Tenant / project ownership | TX4 | YES | Rarely (internal) |
| TX4 transaction lifecycle | TX4 | YES | Sometimes (when tied to external evidence) |
| Provider payment identifiers / status / execution | External payment provider | YES (observed + interaction history) | YES |
| TX4 fee calculation result | TX4 | YES | Rule-dependent (recompute/audit) |
| TX4 ledger / internal financial records | TX4 | YES | Internal controls + vs external where applicable |
| External settlement / fund movement execution | External infrastructure | YES (instructions + observations) | YES |
| TX4 settlement obligation / instruction record | TX4 | YES | Yes where external confirmation required |
| Reconciliation difference records | TX4 | YES | N/A (is the reconciliation output) |
| Usage / billable usage records | TX4 | YES | Rule-dependent |
| Managed Cloud credit / cloud billing | Managed Cloud commercial system | YES (cloud-side) | Commercial billing process |
| Audit evidence of TX4 operations | TX4 | YES | N/A |

---

## 7. Durability Requirements

### 7.1 Durable state

Authoritative TX4 state must survive:

* process restart
* application crash
* worker restart
* machine restart
* routine deployment / restart

Authoritative financial or transaction state MUST NOT exist only in process memory.

### 7.2 Crash recovery

After restart, TX4 must be able to:

```text
load durable state
        ↓
identify incomplete / uncertain work
        ↓
resume, query, retry, and/or reconcile
        ↓
reach a valid authoritative state
```

TX4 is **not** required to automatically complete every interrupted operation.

The system may legitimately represent incomplete or uncertain outcomes.

Critical requirement:

> no silent loss of authoritative state and no duplicated financial effect.

---

## 8. Recovery Model

Conceptual recovery lifecycle:

```text
NORMAL OPERATION
      ↓
INTERRUPTION / CRASH
      ↓
DURABLE STATE RELOADED
      ↓
INCOMPLETE / UNCERTAIN WORK IDENTIFIED
      ↓
RETRY / RESUME / QUERY PROVIDER / RECONCILE
      ↓
VALID AUTHORITATIVE STATE
```

Recovery MUST NOT rely on:

* guessing
* process-local memory as authority
* assuming duplicate execution is harmless
* assuming timeout means provider failure
* assuming callbacks are complete or unique

---

## 9. Exactly-Once Claims

Do **not** claim external distributed systems provide universal exactly-once execution.

Preferred composition:

```text
at-least-once delivery
+
idempotent processing
+
durable state
+
unique operation identity
+
reconciliation
```

Distinguish:

| Claim | Meaning |
| --- | --- |
| Exactly-once **message delivery** | Transport property; generally not assumed |
| Exactly-once **external provider execution** | Provider property; not controlled by TX4 |
| Exactly-once **business effect** | Desired invariant for financial effects, achieved via durable identity + idempotency + reconciliation |

Exactly-once business effects are an **invariant goal**, not a transport-level guarantee.

---

## 10. Idempotency Boundary

Idempotency is a correctness boundary for externally repeatable operations, including:

* repeated API requests
* repeated payment callbacks / webhooks
* repeated provider status observations
* worker retries
* timeout followed by retry
* reconciliation reruns
* duplicate settlement instruction attempts

Repeated processing must not create duplicate financial effects.

API idempotency key / header syntax remains OPEN (later API ADR). Transaction lifecycle specifics remain OPEN (ADR-007).

---

## 11. Event Ordering and External Observations

Do not assume external events arrive exactly once, in order, without duplication, or without delay.

Authoritative TX4 state must remain correct under:

```text
duplicate
late
out-of-order
missing
retried
```

external observations.

Webhook / provider events are evidence/input, not automatically unquestioned truth (Master Spec §17).

---

## 12. External Provider Failure Scenarios

The system must support uncertainty and subsequent reconciliation for scenarios including:

```text
TX4 request → provider request → provider may process → TX4 times out
```

Also:

* provider callback lost
* callback duplicated
* callback delayed
* provider status endpoint unavailable
* provider returns conflicting observations
* TX4 crashes after sending a provider request
* TX4 crashes after receiving a provider response but before persisting the result

TX4 must **not** infer `timeout = provider did not execute`.

Concrete recovery implementation remains OPEN.

---

## 13. Financial Safety Invariants (High-Level)

1. No duplicate financial effect from retries.
2. No authoritative financial state stored only in process memory.
3. No silent loss of durable transaction state.
4. No implicit transition from uncertainty to success/failure without recorded evidence.
5. External provider authority is not overwritten by TX4 assumptions.
6. TX4 internal financial records remain internally consistent.
7. Reconciliation can detect divergence.
8. Recovery is deterministic with respect to durable state and recorded external observations.

Complete ledger/accounting invariant sets belong to later financial specifications.

Monetary representation remains OPEN (ADR-006).

---

## 14. Persistence Properties (Technology OPEN)

Future persistence architecture MUST satisfy:

* durability of authoritative state
* transactional integrity where multi-fact consistency is required
* recoverability after crash
* support for idempotency keys / unique operation identity
* concurrency safety for concurrent requests and workers
* auditability of state changes and evidence
* ability to represent uncertainty and reconciliation outcomes

### Explicitly NOT decided by this ADR

* PostgreSQL (or any database) as frozen technology
* SQLx / ORM / driver choice
* Event sourcing vs current-state tables vs hybrid
* Kafka / Redis / queues / workflow engines
* Outbox / inbox / CDC / WAL strategies
* Schema design

Event sourcing is **not** assumed. Current-state persistence, append-only evidence, event sourcing, or hybrids may all be evaluated later against these properties.

---

## 15. Auditability

Durable evidence must be sufficient to answer later:

* what operation occurred?
* which transaction did it belong to?
* which tenant initiated or owned it?
* what external provider interaction occurred?
* what was observed?
* what state transition resulted?
* what financial effect occurred?
* was the operation retried?
* was reconciliation performed?
* why did the final state occur?

Audit-log implementation technology remains OPEN.

---

## 16. Consequences

### Positive

* Clear authority boundaries between TX4 and external providers
* Crash recoverability as a product requirement
* Safer retries and reduced duplicate financial effects
* Explicit representation of uncertainty
* Stronger reconciliation and audit posture
* Separation of cloud credits from transactional balances

### Costs / tradeoffs

* More durable state management responsibility
* Idempotency and unique-operation identity requirements
* Reconciliation complexity
* More explicit state modeling
* Operational procedures for incomplete/uncertain work
* Higher implementation discipline before “happy path” shortcuts

---

## 17. Non-Decisions

Explicitly **OPEN** / out of scope:

* Exact database / persistence technology
* Exact persistence schema
* Event-sourcing adoption
* Queue / workflow technology
* API idempotency key format
* Monetary representation (ADR-006)
* Exact transaction lifecycle states (ADR-007)
* Exact payment provider adapter contracts
* Exact ledger account model
* Exact settlement algorithm
* Exact billing algorithm
* API versioning (ADR-005)
* Managed Cloud implementation architecture / OSS boundary details (ADR-008)
* Runtime language (ADR-001 already FROZEN)
* Project license / dependency policy (ADR-002 / ADR-003 already FROZEN)

---

## 18. Reconsideration Conditions

Revisit only if objective evidence shows:

1. A required TX4 domain fact cannot be assigned a coherent authority without product-scope change
2. Durability/recovery requirements conflict with a later frozen persistence ADR in a way that cannot be remediated
3. Master Spec changes the payment-provider or financial source-of-truth model materially

Preference alone is insufficient after freeze.

---

## 19. Relationship to Future ADRs

Once FROZEN, later ADRs and implementation must respect these boundaries.

* ADR-005+ must not place authoritative financial state solely in process memory
* Persistence ADRs must satisfy durability/idempotency/recovery properties herein
* ADR-006/007/008 remain independently decidable within these boundaries

---

## 20. References

* Master Spec §§7–18, §31, §36
* ADR-001 Rust — FROZEN
* ADR-002 Apache-2.0 — FROZEN
* ADR-003 Dependency license policy — FROZEN
* `.cursor/rules/05-boundaries.mdc` (conceptual SoT guidance; this ADR is the specification authority once frozen)
