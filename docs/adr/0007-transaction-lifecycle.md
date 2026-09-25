# ADR-007 — Transaction Lifecycle

| Field | Value |
| --- | --- |
| ADR | **007** |
| Title | Transaction Lifecycle |
| Status | **PROPOSED** |
| Date | 2026-09-25 |
| Decision scope | Canonical TX4 transaction lifecycle state machine: states, transitions, uncertainty, idempotency, concurrency, crash recovery, terminal semantics, cancellation, expiration, failure/retry, and refund/adjustment boundary |
| Related Master Spec | §7 Transaction Model; §8 Payment Boundary; §13 Ledger; §14 Settlement; §15 Reconciliation; §16 Idempotency; §17 Webhooks; §19 API; §31 Production-Grade; §36 Current Status |
| Depends on | ADR-001 (Rust) — FROZEN; ADR-002 — FROZEN; ADR-003 — FROZEN; ADR-004 (SoT / durability) — FROZEN; ADR-005 (API versioning) — FROZEN; ADR-006 (monetary representation) — FROZEN |
| Supersedes | None |

---

## 1. Status

**PROPOSED** — not FROZEN.

Freeze requires independent audit and an explicit freeze task.

This ADR defines the **canonical TX4 transaction lifecycle** as a durable, deterministic, auditable state machine. It does **not** select persistence technology, payment providers, queues, locking strategies, workflow engines, HTTP frameworks, or SDK languages.

---

## 2. Context

Master Spec §7 requires a deterministic transaction lifecycle and lists candidate states including CREATED, PENDING, PAID, PROCESSING, COMPLETED and exceptional candidates CANCELLED, EXPIRED, FAILED, REFUNDED, PARTIALLY_REFUNDED. Exact state machine was deferred to this ADR.

ADR-004 (FROZEN) requires:

* TX4 owns authoritative TX4 transaction domain state
* provider state ≠ TX4 transaction state
* timeout ≠ provider failure
* uncertainty must remain representable
* durable state across crash/restart
* recovery must not fabricate certainty
* at-least-once + idempotency + durable identity + reconciliation (not universal exactly-once)

ADR-005 (FROZEN): public API uses `/v{N}/...` (initial `/v1/...`); unsupported versions must not silently remap.

ADR-006 (FROZEN): monetary values use signed integer atomic units + explicit currency; no float authority; no implicit FX. Lifecycle must not invent a competing money model or a competing financial ledger.

---

## 3. Decision

**Adopt the canonical TX4 transaction lifecycle defined in this ADR.**

Summary:

1. A TX4 **Transaction** is a first-class durable domain object with a single primary lifecycle **state**.
2. Primary states are: `CREATED`, `PENDING`, `PAID`, `PROCESSING`, `COMPLETED`, `CANCELLED`, `EXPIRED`, `FAILED`.
3. **Refunds and partial refunds are not primary transaction states.** They are separate financial records (Option C).
4. **External uncertainty is modeled orthogonally** via payment/operation **outcome disposition**, not by equating timeout with `FAILED`.
5. Transitions are explicit, durable, idempotent under duplicate delivery, and concurrency-safe under a deterministic conflict rule.
6. Terminal primary states forbid further primary-lifecycle progression; they do **not** forbid later refund/adjustment/reconciliation **records**.
7. Persistence technology, provider adapters, and API schemas remain OPEN.

---

## 4. Core Distinctions

| Concept | Meaning |
| --- | --- |
| **Transaction** | TX4 domain object with immutable identity and primary lifecycle state |
| **Primary lifecycle state** | Exactly one of the eight states in §5 |
| **Lifecycle transition** | Durable change of primary state under an allowed edge |
| **Payment / external operation** | Interaction with an external authority (provider, etc.) |
| **Outcome disposition** | Known success / known failure / timeout / unknown / reconciliation-required for an operation |
| **Refund / adjustment record** | Compensating financial fact linked to a transaction; not a primary state |
| **Provider state** | External authority’s view (ADR-004); never silently equals TX4 state |

```text
TX4 transaction.state
        ≠
provider payment state
        ≠
refund/adjustment records
        ≠
ledger postings (when present)
```

---

## 5. Primary State Model

### 5.1 Canonical states

| State | Meaning (testable) | Terminal? |
| --- | --- | --- |
| `CREATED` | Transaction identity exists; not yet opened for payment/fulfillment progression | No |
| `PENDING` | Open for payment (or awaiting resolution of a payment attempt). Primary path before paid confirmation | No |
| `PAID` | TX4 has durably accepted **known payment success** for this transaction under TX4 evidence rules | No |
| `PROCESSING` | Post-payment fulfillment / domain processing is in progress under TX4 rules | No |
| `COMPLETED` | TX4 has durably recorded successful completion of the transaction’s primary commercial intent | Yes |
| `CANCELLED` | TX4 has durably cancelled the transaction’s primary progression before completion | Yes |
| `EXPIRED` | TX4 has durably marked the transaction expired under an explicit expiration rule before completion | Yes |
| `FAILED` | TX4 has durably recorded a **terminal business failure** of the primary progression | Yes |

### 5.2 Refund candidates rejected as primary states

Master Spec exceptional candidates `REFUNDED` / `PARTIALLY_REFUNDED` are **not** primary lifecycle states.

**Chosen model (Option C):** refunds and adjustments are **explicit financial records** associated with a transaction (and monetary amounts per ADR-006). They may coexist with terminal primary states such as `COMPLETED` without rewriting historical primary state.

Rationale:

* preserves immutable primary history
* avoids conflating payment/refund architecture with transaction progression
* aligns with ADR-004 ledger/refund authority as records, not provider overwrite
* leaves full refund architecture OPEN

### 5.3 Per-state definition

#### CREATED

* **Entry:** durable creation of a new transaction identity.
* **Predecessors:** none (initial).
* **Successors:** `PENDING`, `CANCELLED`, `EXPIRED`.
* **Retryable:** creation itself must be idempotent by client/operation identity (ADR-004); state is not a “retry of payment.”
* **Externally observable:** yes (once created).
* **Financial effects:** may record intended monetary amounts (ADR-006) but must not treat creation alone as payment success or settlement.
* **Recovery:** resume from `CREATED` if durable; do not invent `PAID`/`COMPLETED`.

#### PENDING

* **Entry:** transaction opened for payment / awaiting payment-attempt resolution.
* **Predecessors:** `CREATED`.
* **Successors:** `PAID`, `CANCELLED`, `EXPIRED`, `FAILED` (only under §7.3 Terminal Failure Decision — not automatic from operation `KNOWN_FAILURE`).
* **Retryable:** payment **operations** may retry while remaining `PENDING` when outcome disposition is timeout/unknown/retryable or operation-level `KNOWN_FAILURE` (§7–§8). Primary state does not flip to `FAILED` solely due to timeout or a single operation failure.
* **Externally observable:** yes.
* **Financial effects:** payment attempts may be in flight; authoritative payment success is not assumed until transition to `PAID`.
* **Recovery:** remain `PENDING` with accurate disposition; never fabricate provider failure from timeout.

#### PAID

* **Entry:** TX4 durably accepts known payment success evidence for this transaction (ADR-004 evidence rules).
* **Predecessors:** `PENDING`.
* **Successors:** `PROCESSING`, `CANCELLED` (conditional), `FAILED` (conditional).
* **Retryable:** duplicate “payment succeeded” deliveries must be idempotent (stay `PAID`).
* **Externally observable:** yes.
* **Financial effects:** payment-accepted effects as modeled by TX4 may be recorded; must use ADR-006 money; must not duplicate on retry.
* **Recovery:** resume `PAID`; do not regress to `PENDING` because a worker crashed after persistence.

#### PROCESSING

* **Entry:** post-payment fulfillment/processing started under TX4 rules.
* **Predecessors:** `PAID`.
* **Successors:** `COMPLETED`, `FAILED`, `CANCELLED` (conditional).
* **Retryable:** processing **operations** may retry while remaining `PROCESSING` if incomplete and non-terminal; must be idempotent.
* **Externally observable:** yes.
* **Financial effects:** fees/usage side-effects may occur only under explicit rules; no float money; no duplicate effects on retry.
* **Recovery:** resume `PROCESSING`; do not invent `COMPLETED`.

#### COMPLETED

* **Entry:** primary commercial intent successfully completed under TX4 rules.
* **Predecessors:** `PROCESSING`.
* **Successors:** none (primary lifecycle).
* **Terminal:** yes.
* **Retryable:** duplicate completion deliveries are no-ops (remain `COMPLETED`).
* **Externally observable:** yes.
* **Financial effects:** completion may authorize billable/usage effects per future billing rules; later refunds/adjustments are separate records (§15).
* **Recovery:** remain `COMPLETED`.

#### CANCELLED

* **Entry:** durable cancel of primary progression before `COMPLETED`.
* **Predecessors:** `CREATED`, `PENDING`, `PAID` (conditional), `PROCESSING` (conditional).
* **Successors:** none (primary lifecycle).
* **Terminal:** yes.
* **Financial effects:** cancellation does **not** imply refund. If funds were accepted (`PAID`/`PROCESSING`), any return of funds requires explicit refund/adjustment records (§12, §15).
* **Recovery:** remain `CANCELLED`.

#### EXPIRED

* **Entry:** durable expiration under an explicit expiration rule before `COMPLETED`.
* **Predecessors:** `CREATED`, `PENDING`.
* **Successors:** none (primary lifecycle).
* **Terminal:** yes.
* **Distinct from timeout:** expiration is a TX4 domain decision that the transaction’s offer/window ended; timeout is an operation observation about an external call (§7, §13).
* **Revival:** expired transactions are **not** revived into active primary progression. A new business attempt requires a **new transaction identity** (unless a future ADR explicitly authorizes a superseding rule).
* **Financial effects:** expiration does not imply refund; if payment was somehow accepted contrary to expected order, reconciliation + compensating records apply (must not silently rewrite history).
* **Recovery:** remain `EXPIRED`.

#### FAILED

* **Entry:** durable **terminal business failure** of primary progression under §7.3 (from `PENDING`) or matrix-allowed post-pay failure (not mere timeout / not automatic operation `KNOWN_FAILURE`).
* **Predecessors:** `PENDING`, `PAID` (conditional), `PROCESSING`.
* **Successors:** none (primary lifecycle).
* **Terminal:** yes.
* **Retryable:** the **transaction** does not leave `FAILED` back to active states. A new commercial attempt uses a new transaction identity. Transient/retryable conditions must remain in `PENDING`/`PROCESSING` with disposition, not `FAILED`.
* **Financial effects:** failure does not silently create refunds; compensating records explicit if needed.
* **Recovery:** remain `FAILED`.

---

## 6. Transition Matrix

Legend:

* **A** = ALLOWED
* **F** = FORBIDDEN
* **C** = CONDITIONAL (allowed only when stated predicates hold)

Rows = from; columns = to.

| From \ To | CREATED | PENDING | PAID | PROCESSING | COMPLETED | CANCELLED | EXPIRED | FAILED |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CREATED | F* | A | F | F | F | A | A | F |
| PENDING | F | F* | A | F | F | A | A | A |
| PAID | F | F | F* | A | F | C1 | F | C2 |
| PROCESSING | F | F | F | F* | A | C3 | F | A |
| COMPLETED | F | F | F | F | F* | F | F | F |
| CANCELLED | F | F | F | F | F | F* | F | F |
| EXPIRED | F | F | F | F | F | F | F* | F |
| FAILED | F | F | F | F | F | F | F | F* |

\* Self-transition of primary state via duplicate delivery of an already-applied transition is handled as **idempotent no-op** (remain in state), not as a new edge. See §8.

### Conditional predicates

**C1 — `PAID` → `CANCELLED`:** allowed only when an explicit cancel-after-payment decision is recorded for this transaction **and** the compensating-obligation rule in §6.1 is satisfied. Cancellation still does **not** itself execute a refund or move money.

**C2 — `PAID` → `FAILED`:** allowed only for a documented terminal business inability to proceed after payment acceptance (not timeout) **and** the compensating-obligation rule in §6.1 is satisfied. Must not silently drop paid funds from history.

**C3 — `PROCESSING` → `CANCELLED`:** allowed only under an explicit cancel-during-processing decision **and** the compensating-obligation rule in §6.1 is satisfied. Must not imply silent refund execution.

### 6.1 Compensating-obligation intent (normative for C1/C2/C3)

**Trigger:** Any primary transition that leaves `PAID` or `PROCESSING` for `CANCELLED` or `FAILED` (C1, C2, C3, and `PROCESSING` → `FAILED`) occurs **after funds/value acceptance has already been established** in primary state `PAID` (or while still post-acceptance in `PROCESSING`).

For every such transition, TX4 MUST durably record a **compensating-obligation intent** as part of the authoritative transition semantics, before or atomically with the primary-state change (implementation mechanism OPEN), such that the transition cannot imply that the accepted financial consequence simply disappeared.

Minimum intent contents (conceptual; schema OPEN):

* transaction identity
* logical transition / operation identity (idempotent association)
* previous primary state and resulting primary state
* reason/category for the compensating need
* monetary amount + currency when known, using ADR-006 representation (or an explicit “amount to be determined by later refund rules” marker that remains auditable)
* intent status distinct from refund execution (for example `INTENT_RECORDED`)

Rules:

1. **Cancel ≠ refund.** Recording intent does not execute a refund.
2. **Failed ≠ refund.** Same boundary.
3. Intent must be auditable and idempotently tied to the transition identity (§8).
4. Later refund/adjustment **execution** remains OPEN architecture; historical primary state is not rewritten solely to represent compensation (§11, §15).
5. If compensating intent cannot be durably recorded, the primary transition MUST fail closed (do not apply C1/C2/C3 / `PROCESSING`→`FAILED`).

### Explicitly forbidden examples (normative)

* `PENDING` → `PROCESSING` or `COMPLETED` without `PAID`
* `PROCESSING` → `PENDING` (no regression to payment-waiting)
* `COMPLETED` → any other primary state
* `EXPIRED` → `PENDING` / revival
* `FAILED` → `PENDING` / `PAID` / `PROCESSING`
* Any transition that treats timeout alone as `FAILED`

---

## 7. Uncertainty Model (Mandatory)

### 7.1 Outcome disposition (orthogonal)

For external payment (and similar) operations, TX4 MUST record an **outcome disposition** distinct from primary transaction state:

| Disposition | Meaning |
| --- | --- |
| `KNOWN_SUCCESS` | TX4 has accepted definitive success evidence |
| `KNOWN_FAILURE` | TX4 has accepted definitive failure evidence |
| `TIMEOUT` | TX4 timed out waiting; provider outcome unknown |
| `UNKNOWN` | Outcome unknown for reasons other than a classified timeout |
| `RECONCILIATION_REQUIRED` | Disagreement or unresolved uncertainty requires reconciliation |

### 7.2 Hard rules

1. **`TIMEOUT ≠ FAILED`.** Timeout must not by itself transition the transaction to `FAILED`.
2. **`UNKNOWN ≠ FAILED`.**
3. While disposition is `TIMEOUT` / `UNKNOWN` / `RECONCILIATION_REQUIRED`, primary state typically remains `PENDING` (or stays where it was if already past payment), and recovery/reconciliation proceeds per ADR-004.
4. Transition `PENDING` → `PAID` requires disposition path equivalent to accepted `KNOWN_SUCCESS` under TX4 evidence rules (including when reconciliation later promotes an uncertain operation to accepted `KNOWN_SUCCESS` while still `PENDING`).
5. Exactly-once external execution is **not** assumed (ADR-004).

### 7.3 Operation-level `KNOWN_FAILURE` vs transaction `FAILED`

#### Operation-level `KNOWN_FAILURE`

A payment (or similar) **operation** may record disposition `KNOWN_FAILURE` while the transaction remains **`PENDING`** when all of the following hold:

1. the failure is attributable to that attempted operation (not a fabricated whole-transaction conclusion from timeout/unknown), and
2. the failure does **not** by itself establish that the transaction’s primary commercial progression is terminally finished, and
3. another permitted payment operation, explicit terminal decision, cancel, or expiration remains possible under domain policy.

Therefore:

```text
operation KNOWN_FAILURE
        ≠
automatic PENDING → FAILED
```

Retryable payment declines / known provider-side operation failures are modeled as operation dispositions + optional new attempts while remaining `PENDING`, not as automatic primary terminalization.

#### Transaction-level `PENDING` → `FAILED`

`PENDING` → `FAILED` is **ALLOWED only when** a durable **Terminal Failure Decision** is recorded for the transaction, with a reason category in:

| Reason category | Meaning |
| --- | --- |
| `BUSINESS_REJECTED_TERMINAL` | Explicit domain/business rejection that the transaction must not proceed further (not a single retryable operation failure) |
| `ATTEMPT_BUDGET_EXHAUSTED` | A configured maximum payment-attempt budget `M` (`M ≥ 1`) is set for the transaction (or its type/policy), and the count of completed payment attempts that ended in `KNOWN_FAILURE` (without intervening `KNOWN_SUCCESS`) has reached `M` |
| `DOMAIN_ABANDONED_TERMINAL` | Explicit abandonment decision that ends primary progression without using `CANCELLED`/`EXPIRED` |

`PENDING` → `FAILED` is **FORBIDDEN** when the only evidence is a single (or non-budget-exhausting) operation `KNOWN_FAILURE`, `TIMEOUT`, or `UNKNOWN`.

#### No indefinite `PENDING` solely from operation failures

A conforming system MUST ensure every `PENDING` transaction has at least one defined exit other than “wait forever on repeated operation failures”:

* an expiration rule enabling `PENDING` → `EXPIRED`, and/or
* cancel enabling `PENDING` → `CANCELLED`, and/or
* a configured attempt budget enabling `ATTEMPT_BUDGET_EXHAUSTED`, and/or
* an explicit terminal-failure / abandonment decision path

Exact default policy values (which exits are enabled, numeric `M`) remain domain/configuration OPEN, but “leave forever in `PENDING` because operations keep returning `KNOWN_FAILURE` with no exit path” is **non-conforming**.

### 7.4 Representation boundary

Outcome disposition may be stored as operation/payment attempt metadata rather than as a ninth primary transaction state. That is intentional: uncertainty and operation failure are about **external operations**, while primary state is about **TX4 transaction progression**.

---

## 8. Idempotency

### 8.1 Invariant

Repeated delivery of the same logical lifecycle transition MUST NOT:

* apply the transition twice
* duplicate financial effects
* fork into conflicting durable primary states

### 8.2 Logical transition identity

Each attempted transition has a durable logical identity including at least:

* transaction identity
* target transition type (from→to + transition kind)
* idempotency / operation key (durable unique identity per ADR-004)
* actor/source category (API client, worker, webhook, recovery)

Exact API header names remain OPEN (ADR-004).

### 8.3 Handling rules

| Case | Required outcome |
| --- | --- |
| Duplicate of already-applied transition | No-op success; remain in resulting state; no new financial effect |
| Transition already applied with same effect | Treat as success (idempotent) |
| Conflicting transition (different target while first applied) | Reject deterministically; do not partially apply |
| Stale transition (source state no longer matches) | Reject; do not mutate; may record ignored/stale attempt for audit |
| Transition into terminal from terminal | Forbidden / no-op depending on duplicate vs conflict (§6, §11) |

---

## 9. Concurrency

Concurrent transition attempts on one transaction MUST yield a deterministic durable outcome that is **independent of worker timing, thread scheduling, network arrival order, database product, queue product, lock product, Rust runtime behavior, and provider-specific racing**.

### 9.1 Already-authoritative state wins

If a primary transition is **already durably committed**, later competing operations are **not** concurrent winners:

* duplicate of the committed transition → idempotent no-op (§8)
* different transition targeting a now-invalid source state → reject / stale (§8)
* must not reopen terminal states (§6, §11)

### 9.2 Concurrent conflict set (before commitment)

When two or more **distinct eligible** primary transitions compete for the same transaction **before** any of them is durably authoritative, form the conflict set of candidates that are matrix-allowed (including conditionals whose predicates are satisfied at evaluation time).

Exactly one candidate becomes the durable winner. All others MUST be rejected (not partially applied).

### 9.3 Normative precedence (total order)

Assign each candidate a **precedence class** by **target primary state** of the transition:

| Precedence (higher number wins) | Target state | Rationale |
| --- | ---: | --- |
| 60 | `PAID` | Accepted payment success is a fund/value-acceptance fact; must not lose a race to abandon paths when `KNOWN_SUCCESS` evidence makes `→PAID` eligible |
| 50 | `COMPLETED` | Established fulfillment/completion of primary commercial intent outranks abort paths from `PROCESSING` when completion is eligible |
| 40 | `PROCESSING` | Progress into fulfillment outranks abandon-from-`PAID` only when both are concurrently eligible (rare; usually sequential) |
| 30 | `FAILED` | Explicit terminal business failure outranks cancel/expire when concurrently eligible |
| 20 | `CANCELLED` | Intentional cancel outranks mere expiration |
| 10 | `EXPIRED` | Time-window end is the weakest abandon path in a conflict set |
| 0 | `PENDING` | Opening progression (from `CREATED`) is lowest among listed targets |

**Why not “success always beats cancel” as a slogan:** precedence is derived from **which fact is being established**. Fund-acceptance (`PAID`) and fulfillment-completion (`COMPLETED`) outrank abort transitions because dropping those facts in a race would erase stronger established outcomes. Abort transitions still win when they are the only eligible candidates, or when a higher-precedence candidate is not eligible.

### 9.4 Deterministic tie-break

If two candidates share the same precedence class (same target state) or otherwise tie on precedence:

1. Compare durable logical **operation identity** strings (§8.2) using **lexicographically ascending** byte/code-point order.
2. The **smaller** operation identity wins.
3. If identities are equal, they are the same logical transition → idempotent single apply (§8).

This tie-break is language-independent and must not use wall-clock arrival order.

### 9.5 Required covered races

The rule above yields a single prescribed winner for at least:

| Race | Winner if both eligible |
| --- | --- |
| `PROCESSING`→`COMPLETED` vs `PROCESSING`→`CANCELLED` | `COMPLETED` |
| `PENDING`→`PAID` vs `PENDING`→`EXPIRED` | `PAID` |
| `PENDING`→`PAID` vs `PENDING`→`CANCELLED` | `PAID` |
| `PROCESSING`→`COMPLETED` vs `PROCESSING`→`FAILED` | `COMPLETED` |
| `PENDING`→`FAILED` vs `PENDING`→`CANCELLED` | `FAILED` |
| `PENDING`→`CANCELLED` vs `PENDING`→`EXPIRED` | `CANCELLED` |
| Any other distinct eligible pair from the same predecessor | higher precedence class, else §9.4 |

Same logical transition delivered twice: one apply, one no-op (§8) — not a §9.3 race.

### 9.6 Non-decision

Database locks, optimistic versioning, queues, CAS, actors, and workflow engines remain **OPEN**. Implementations may use any mechanism that realizes §9.1–§9.5.

---

## 10. Crash Recovery

Compatible with ADR-004:

| Scenario | Required behavior |
| --- | --- |
| Crash before transition persistence | No primary state change; retry may re-attempt with same logical identity |
| Crash after transition persistence | Resume from durable new state; do not re-apply effects |
| Crash during external provider interaction | Do not infer failure; retain/set disposition `TIMEOUT`/`UNKNOWN` as appropriate; reconcile |
| Restart after timeout | Continue uncertainty handling; not auto-`FAILED` |
| Duplicate recovery attempt | Idempotent per §8 |

Recovery MUST NOT fabricate `PAID`, `COMPLETED`, or `FAILED` without evidence rules.

---

## 11. Terminal Semantics

Terminal primary states: `COMPLETED`, `CANCELLED`, `EXPIRED`, `FAILED`.

### Why terminal

No further **primary lifecycle progression** edges are allowed (§6).

### What terminal does **not** mean

Terminal does **not** mean “nothing financial may ever happen.”

Allowed after terminal primary state (as separate concerns):

* refund / adjustment **records** (ADR-006 money; refund architecture OPEN)
* ledger postings derived from those records (ledger model OPEN)
* reconciliation records comparing TX4 vs provider observations
* audit/observability updates

### Reconciliation vs primary state

Reconciliation may update dispositions, evidence, and reconciliation records. It MUST NOT silently rewrite a terminal primary state to an active state. If evidence shows a catastrophic inconsistency, record disagreement explicitly; remediation policy is OPEN but must not fabricate certainty or duplicate money effects.

---

## 12. Cancellation

| Situation | Rule |
| --- | --- |
| Before payment (`CREATED`/`PENDING`) | `→ CANCELLED` allowed; no refund implied |
| After payment (`PAID`) | `→ CANCELLED` only under **C1** + §6.1 compensating-obligation intent; refund execution not implied |
| During processing | `→ CANCELLED` only under **C3** + §6.1; refund execution not implied |
| After completion | Forbidden as primary transition; use refund/adjustment records if funds must return |
| Under external uncertainty | Do not cancel solely because of timeout; resolve disposition/reconcile first unless an explicit domain cancel rule applies independently of provider timeout |

**Cancellation ≠ refund.** Boundary is mandatory.

---

## 13. Expiration

* **Cause:** explicit domain expiration rule (offer window, payment window, etc.). Scheduling infrastructure OPEN.
* **Automatic vs triggered:** either is allowed conceptually; must be an explicit TX4 decision recorded as `→ EXPIRED`.
* **Revival:** not allowed (§5.3 `EXPIRED`).
* **After payment:** `PAID`/`PROCESSING`/`COMPLETED` → `EXPIRED` is **FORBIDDEN**. Post-payment time limits use cancel/fail/complete rules + refunds as applicable — not expiration.
* **Timeout vs expiration:** timeout = operation observation; expiration = transaction primary-state decision.

---

## 14. Failure and Retry

| Category | Primary state effect |
| --- | --- |
| Transient operational failure / retryable processing | Remain in `PENDING` or `PROCESSING`; retry operations idempotently |
| External uncertainty (timeout/unknown) | Remain; disposition updated; not auto-`FAILED` |
| Operation-level `KNOWN_FAILURE` | Remain in `PENDING` (for payment attempts) per §7.3; do **not** auto-transition `PENDING`→`FAILED` |
| Terminal business failure | `→ FAILED` only when §7.3 Terminal Failure Decision (or matrix-allowed post-pay failure with §6.1) applies |
| Retry after `FAILED` | New **transaction** identity (or future explicitly authorized supersession ADR) — not a primary edge out of `FAILED` |

`FAILED` is **terminal** for this transaction’s primary lifecycle.

---

## 15. Refund / Adjustment Boundary

```text
Transaction primary lifecycle
        +
Refund / adjustment records (separate)
        +
Ledger effects (when present; OPEN model)
        +
Settlement effects (OPEN)
```

Rules:

1. Do not encode refund progress as `REFUNDED` / `PARTIALLY_REFUNDED` primary states.
2. Prefer **immutable primary history** + **compensating records** over rewriting `COMPLETED`/`PAID` history.
3. Refund amounts use ADR-006 monetary representation.
4. Full refund product architecture remains OPEN.
5. Payment provider selection remains OPEN.
6. Lifecycle state must not become a competing financial ledger (ADR-004 / ADR-006).

---

## 16. Financial Safety Invariants

1. Duplicate lifecycle delivery must not duplicate financial effects.
2. Transitions must not alter monetary meaning or introduce float/FX authority.
3. `COMPLETED` has a deterministic primary-success boundary.
4. `CANCELLED` does not imply refund.
5. Refunds do not rewrite primary historical state.
6. Lifecycle state is not a ledger substitute.
7. `PENDING` → `PAID` requires known-success evidence path, not timeout.
8. Money on the transaction and related records obeys ADR-006.

---

## 17. Auditability

Each durable primary transition MUST be auditable with at least:

* transaction identity
* previous primary state
* resulting primary state
* logical transition / operation identity
* reason/category (domain-coded; exact enum OPEN beyond categories herein)
* actor/source category (client, worker, webhook, recovery, admin policy — exact authn OPEN)
* timestamp and/or durable ordering information
* correlation / external reference identities where applicable
* outcome disposition references when the transition depends on external evidence

Schema/storage technology remains OPEN.

---

## 18. Observability Boundary

Eventually observable (technology OPEN):

* current primary state
* transition history (or equivalent audit trail)
* failure category when `FAILED`
* uncertainty / outcome disposition for relevant operations
* retry status of operations (not primary-state thrash)
* reconciliation-required indicators

Do not define metrics backends or log stacks here.

---

## 19. API Boundary

Lifecycle semantics herein may be exposed under ADR-005 versioned contracts (`/v1/...` initially).

This ADR does **not** define:

* HTTP routes
* request/response schemas
* error body shapes
* webhook payload schemas

Unsupported API versions must not silently remap (ADR-005). Breaking lifecycle meaning is a breaking API change.

---

## 20. Non-Goals / Non-Decisions

Explicitly **OPEN** / out of scope:

* payment provider architecture and selection
* persistence technology / DB schema / SQL vs document
* event sourcing vs CRUD
* queue / bus / workflow engine
* locking / concurrency mechanism
* Rust HTTP framework / crates beyond ADR-001 language freeze
* API and SDK implementation
* refund, settlement, reconciliation **implementations**
* Managed Cloud / OSS boundary (ADR-008)
* pricing / billing algorithms
* exact authn/authz mechanisms

---

## 21. Consequences

### Positive

* Deterministic, testable primary lifecycle
* Compatible with ADR-004 uncertainty and durability
* Avoids refund-state proliferation and history rewrite
* Clear cancel ≠ refund and timeout ≠ failure
* API-versionable semantics without freezing HTTP shapes

### Costs / tradeoffs

* Clients must understand refunds as separate records
* Conditional cancel-after-pay requires careful product rules later
* Orthogonal disposition model adds conceptual surface vs a single mega-enum

---

## 22. Rejected Alternatives

| Alternative | Why rejected |
| --- | --- |
| `REFUNDED` / `PARTIALLY_REFUNDED` as primary states | Mutates/conflates progression with compensation; conflicts with immutable-history preference |
| Timeout → `FAILED` | Violates ADR-004 |
| Provider webhook state as TX4 primary state | Violates SoT boundary |
| `PROCESSING` → `PENDING` regression | Ambiguous money/payment semantics; hides failures |
| Non-terminal `FAILED` with auto-reopen | Duplicate-effect risk; prefer new transaction identity |
| Universal exactly-once transport claim | Forbidden by ADR-004 |
| Selecting Postgres/queues/locks in this ADR | Implementation leakage |

---

## 23. Open Questions (Non-blocking for proposal)

1. Exact public enum naming / serialization for states and dispositions (API contract later).
2. Precise evidence policy for accepting provider “paid” into `PAID` (provider adapter ADR later).
3. Whether cancel-after-pay (C1) is enabled for all verticals by default or policy-gated.
4. Billing trigger exactness relative to `COMPLETED` (billing OPEN).

These do not unblock defining the state machine itself.

---

## 24. Relationship to Previous ADRs

* **ADR-004:** SoT, durability, timeout≠failure, idempotency, reconciliation — obeyed.
* **ADR-005:** lifecycle is versionable API meaning under `/v{N}`; no routes defined.
* **ADR-006:** all monetary amounts/effects use frozen money model.
* **ADR-001:** Rust runtime assumed for future impl; no framework selected.
* **ADR-008:** remains OPEN; same lifecycle for self-hosted and Managed Cloud consumption of core semantics.

---

## 25. Reconsideration Conditions

Revisit only if:

1. A FROZEN domain requirement proves eight primary states insufficient without violating ADR-004/006
2. Refund-as-primary-state becomes mandatory for a frozen public contract (would require explicit supersession)
3. Master Spec materially changes transaction product model

---

## 26. References

* Master Spec §7 Transaction Model
* ADR-004 Core Source-of-Truth Boundaries — FROZEN
* ADR-005 API Versioning Strategy — FROZEN
* ADR-006 Monetary Representation — FROZEN
* ADR-001 / ADR-002 / ADR-003 — FROZEN
