# Governance

TX4 uses role-based governance. Individual names are not invented here.

## Authority Hierarchy

```text
Project Maintainer
        ↓
Architecture / ADR authority
        ↓
Code review
        ↓
CI
        ↓
Release
```

### Project Maintainer

* Owns repository administration and release authority
* Approves material governance changes
* Appoints or delegates architecture / ADR authority when needed
* Ensures security reports are handled according to [SECURITY.md](SECURITY.md)

### Architecture / ADR Authority

* Accepts, rejects, or requests changes to ADRs
* Freezes architectural decisions when ready
* Ensures Master Spec and ADRs remain consistent
* Must not silently rewrite FROZEN decisions without an explicit change process

### Code Review

* Reviews Pull Requests for correctness, security, documentation, and scope
* Confirms changes do not silently modify FROZEN decisions
* May request tests, ADRs, or documentation updates

### CI

* Provides automated evidence for repository hygiene and, later, build/test/security checks
* CI failure blocks merge under the intended branch protection policy
* CI must not be weakened to force a green status

### Release

* Releases should be reproducible
* Notable changes are recorded in [CHANGELOG.md](CHANGELOG.md)
* Formal release artifacts will be defined after architecture freeze

---

## Decision Status Labels

Documentation and ADRs should use explicit status labels:

```text
PROPOSED
OPEN
PLANNED
IN PROGRESS
FROZEN
IMPLEMENTED
DEPRECATED
```

---

## Architecture Change Rules

* **FROZEN** architecture requires an explicit change (updated ADR / Master Spec and maintainer approval).
* Contributors may **propose** changes via issues, draft ADRs, or pull requests that only add proposal documents.
* Maintainers **approve** material changes.
* Coding agents and contributors must not silently change financial semantics, API contracts, licensing strategy, or source-of-truth boundaries.

See [AGENTS.md](AGENTS.md) and [docs/MASTER-SPEC.md](docs/MASTER-SPEC.md).

---

## Security

Security issues follow [SECURITY.md](SECURITY.md).

Public issues must not contain sensitive vulnerability details.

---

## Releases

* Releases should be reproducible.
* Release process details will mature with the project.
* Do not claim production readiness without evidence listed in the Master Spec quality gates.

---

## Evolving Governance

Governance itself may evolve through documented changes (pull request updating this file and related documents), subject to maintainer approval.

---

## Required GitHub Branch Protection (Desired)

These settings should be configured on GitHub when repository administration is available. They are **documented requirements**, not a claim that they are already configured:

```text
main:
- Pull Request required
- CI required to pass
- Branch must be up to date before merge
- Force push disabled
- Branch deletion disabled
```

If these settings are not yet applied on the remote, treat them as required follow-up for repository administrators.
