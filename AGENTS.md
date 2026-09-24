# AGENTS.md — Engineering Governance for TX4

This document defines how human contributors and coding agents (including Cursor) must work in the TX4 repository.

---

## Authority Model

```text
MASTER-SPEC.md defines WHAT TX4 is.

ADR defines WHY a specific architectural decision was made.

AGENTS.md defines HOW contributors and coding agents must work.
```

### Source of truth hierarchy

```text
1. Explicit user / maintainer instruction
2. docs/MASTER-SPEC.md
3. Accepted / FROZEN ADRs
4. TASK specifications
5. AGENTS.md
6. Existing repository implementation
7. Implementation judgment
```

If sources conflict:

```text
STOP
REPORT THE CONFLICT
DO NOT SILENTLY RESOLVE IT
```

Do not invent a new architectural decision merely to make implementation easier.

---

## Required Reading Before Related Work

Before implementing related functionality, agents MUST read:

1. `docs/MASTER-SPEC.md` (relevant sections)
2. Accepted / FROZEN ADRs under `docs/adr/`
3. This file (`AGENTS.md`)
4. The authorizing TASK specification, if any

---

## No Silent Architecture Drift

Agents and contributors MUST NOT:

* silently modify FROZEN decisions
* silently change financial semantics
* silently change source-of-truth boundaries
* silently change API contracts
* silently change transaction semantics
* silently change licensing strategy
* silently introduce vertical-specific architecture

If implementation conflicts with a specification:

```text
STOP
→ identify conflict
→ report affected document/section
→ explain technical conflict
→ request explicit decision
```

---

## No Architecture by Convenience

Do not choose architecture because:

* a library makes it easier
* generated code suggests it
* an AI model suggests it
* an existing tutorial uses it
* implementation would be faster

Architectural changes require explicit documentation (ADR and/or Master Spec update) and maintainer approval.

---

## Financial Safety

TX4 will handle transaction-related financial concepts.

Agents MUST assume:

```text
money is correctness-critical
```

Rules:

* Do not introduce floating-point monetary calculations without an explicitly approved ADR permitting it.
* Never silently convert exact monetary values into binary floating-point.
* Prefer exact representations (for example, integer minor units + explicit currency) once ADR-006 is frozen.
* Treat financial invariants as tests and evidence, not comments.

---

## Core Must Remain Generic

Core infrastructure MUST NOT contain logic such as:

```text
if vertical == "canteen"
if vertical == "booking"
if vertical == "marketplace"
```

Dogfood applications consume TX4 through the public API.

They do not define TX4.

A feature may enter TX4 Core only when it is a reusable infrastructure primitive across multiple transactional domains, as defined in the Master Spec.

---

## Tests Are Evidence

Never weaken or remove tests merely to make CI pass.

Never:

* delete failing tests without justification
* reduce assertions
* disable checks
* ignore compiler warnings
* skip security checks
* hide failures
* change expected behavior without documentation

A failing test is evidence of a problem until proven otherwise.

---

## Quality Gate Workflow

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

Coding agents are implementation assistants.

They do not have authority to silently change frozen architecture or business rules.

---

## Scope Discipline

* Implement only what the authorizing task permits.
* Do not create empty implementation directories for appearance.
* Do not invent remotes, licenses, runtimes, or security contacts.
* Record useful out-of-scope ideas under Follow-up / Deferred — do not implement them automatically.

---

## Secrets

Never commit:

* API keys
* cloud credentials
* payment provider secrets
* private keys
* database passwords
* JWT signing secrets
* personal access tokens

Use placeholders only. Prefer environment variables and local untracked files.

---

## Working Method

Agents MUST:

```text
read → inspect → plan → implement → validate → report
```

Not:

```text
generate → overwrite → assume
```

Before changing files:

1. Inspect the repository
2. Inspect existing documents
3. Identify conflicts
4. State intended changes
5. Implement only authorized scope
6. Validate
7. Report the exact result

---

## Related Documents

* [Master Specification](docs/MASTER-SPEC.md)
* [Architecture Decision Records](docs/adr/)
* [Contributing Guide](CONTRIBUTING.md)
* [Governance](GOVERNANCE.md)
* [Security Policy](SECURITY.md)
* [Roadmap](ROADMAP.md)
