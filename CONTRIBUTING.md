# Contributing to TX4

Thank you for your interest in TX4.

TX4 is open-source transaction infrastructure for developers building transactional applications. Contributions are welcome when they improve correctness, clarity, security, documentation, or maintainability.

> Contributions that materially change architecture require an ADR or explicit maintainer decision.

---

## 1. Project Overview

TX4 provides reusable infrastructure primitives such as transaction lifecycle, payment orchestration, fees, billing/metering, ledger integration, settlement, reconciliation, idempotency, webhooks, and auditability.

TX4 Core must remain vertical-neutral. Applications (canteen, booking, marketplace, and others) consume TX4; they do not define it.

Read:

* [README.md](README.md)
* [docs/MASTER-SPEC.md](docs/MASTER-SPEC.md)
* [AGENTS.md](AGENTS.md)
* [GOVERNANCE.md](GOVERNANCE.md)

---

## 2. Development Philosophy

Optimize for:

```text
correctness
+
clarity
+
auditability
+
security
+
reproducibility
+
OSS maintainability
+
commercial viability
```

Do not optimize for speed of feature delivery at the expense of financial correctness or frozen architecture.

---

## 3. Repository Governance

Authority order:

```text
1. Explicit maintainer instruction
2. docs/MASTER-SPEC.md
3. Accepted / FROZEN ADRs
4. TASK specifications
5. AGENTS.md
6. Existing repository implementation
7. Implementation judgment
```

If sources conflict: stop, report the conflict, and do not silently resolve it.

---

## 4. Finding an Issue

* Prefer existing issues labeled for contribution when available.
* Feature requests are discussion, not automatic implementation authorization.
* Security issues follow [SECURITY.md](SECURITY.md), not public feature/bug templates when sensitive.

---

## 5. Branch Naming

```text
feat/<short-description>
fix/<short-description>
docs/<short-description>
test/<short-description>
refactor/<short-description>
chore/<short-description>
ci/<short-description>
security/<short-description>
```

Examples:

```text
feat/transaction-core
fix/idempotency-conflict
docs/payment-boundary
ci/github-actions-baseline
```

Direct development on `main` is discouraged. Prefer Pull Requests.

---

## 6. Conventional Commits

Allowed primary types:

```text
feat
fix
docs
test
refactor
perf
build
ci
chore
revert
security
```

Examples:

```text
feat: add transaction lifecycle model
fix: reject duplicate completion requests
docs: clarify payment boundary
test: add transaction idempotency coverage
refactor: isolate settlement domain
ci: add postgres integration job
chore: update development tooling
security: harden webhook verification
```

Avoid meaningless messages such as `update`, `wip`, `final`, or `asdf`.

One logical change per commit where practical.

---

## 7. Pull Requests

Use the repository Pull Request template.

Before opening a PR:

* Read relevant documentation
* Confirm the change does not silently modify a FROZEN decision
* Add or update tests where appropriate
* Ensure CI passes
* Confirm no secrets are included
* Avoid unnecessary dependencies
* Document API, migration, and security impact when relevant

---

## 8. Tests

Tests are evidence.

Never weaken or remove tests merely to make CI pass. Never disable checks, hide failures, or change expected behavior without documentation.

When runtime code exists, PRs that change behavior should include corresponding tests.

---

## 9. Documentation

Update documentation when behavior, contracts, governance, or developer workflows change.

Public documentation should use clear professional English. Avoid exaggerated claims and unsupported metrics.

Status labels used in documentation:

```text
FROZEN
OPEN
PROPOSED
IMPLEMENTED
DEPRECATED
PLANNED
IN PROGRESS
```

---

## 10. Architecture Decisions

Material architecture changes require:

1. Discussion (issue and/or draft ADR)
2. An ADR under `docs/adr/` when appropriate
3. Maintainer acceptance
4. Freeze before implementation when the quality gate requires it

Do not choose architecture because a library, tutorial, or AI suggestion makes it convenient.

---

## 11. Security

* Never commit secrets
* Follow [SECURITY.md](SECURITY.md) for vulnerability reporting
* Consider security implications in every PR
* Do not post sensitive vulnerability details in public issues

---

## 12. Dependency Policy

Future dependency rules (final compatibility claims await ADR-002 and ADR-003):

1. Every dependency must have a clear purpose.
2. Avoid unnecessary dependencies.
3. Check license compatibility.
4. Prefer actively maintained projects.
5. Review security history.
6. Pin / lock versions appropriately.
7. Review transitive dependency risk.
8. Avoid copying third-party code without license compliance.
9. Record material dependency decisions.
10. Never use AGPL/GPL/copyleft code in a way that creates unintended obligations for the project.

Do not make final licensing compatibility claims before ADR-002 and ADR-003.

---

## 13. Code Review

Reviewers should check:

* specification alignment
* absence of silent architecture drift
* financial safety where relevant
* tenant isolation and security
* test evidence
* documentation updates
* dependency necessity

Maintainers approve merges according to [GOVERNANCE.md](GOVERNANCE.md).

---

## 14. Release Process

Releases should be reproducible and documented in [CHANGELOG.md](CHANGELOG.md).

A formal release process will be defined after architecture freeze and when publishable artifacts exist. Until then, treat `main` as the integration branch for foundation work only.

---

## Future CI Layers

Current CI validates repository foundation hygiene only.

Intended future CI layers (not yet claimed as implemented):

```text
format
lint
compile
unit tests
integration tests
PostgreSQL integration
migration verification
API contract tests
dependency auditing
security scanning
container build
SBOM / supply-chain verification
release verification
```

Do not claim these checks exist until they are actually wired and passing.

---

## Code of Conduct

Participation is governed by [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
