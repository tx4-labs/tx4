# ADR-003 — Dependency / Third-Party License Policy

| Field | Value |
| --- | --- |
| ADR | **003** |
| Title | Dependency / Third-Party License Policy |
| Status | **FROZEN** |
| Date | 2026-09-24 |
| Frozen | 2026-09-24 (FREEZE-006; basis AUDIT-006 PASS) |
| Decision scope | Engineering governance policy for third-party dependency licensing, attribution, inventory, SBOM expectations, and review/exception process |
| Related Master Spec | §3 OSS Boundary; §4 Managed Cloud Boundary; §25 Security; §27 OSS Governance; §28 Licensing; §29 Commercial Model |
| Depends on | ADR-001 (Rust) — FROZEN; ADR-002 (Apache-2.0) — FROZEN |
| Supersedes | None |

---

## 1. Status

**FROZEN**

Freeze path completed: TASK-006 → AUDIT-006 → FREEZE-006.

This ADR is an **engineering governance control**. It is **not legal advice**. Compatibility, distribution obligations, linking models, and jurisdiction-specific effects may require professional legal review before relying on this policy in production commercial distribution.

---

## 2. Context

TX4’s project-level OSS license is **FROZEN** as **Apache License 2.0 (Apache-2.0)** (ADR-002). The repository root `LICENSE` contains the Apache-2.0 text.

TX4’s core runtime language is **FROZEN** as **Rust** (ADR-001). Future implementation will introduce Cargo crates and transitive dependency graphs. No application dependency inventory exists yet.

Master Spec §28 requires license compatibility review before integrating third-party code. CONTRIBUTING / AGENTS dependency rules require purpose, license checks, and avoidance of unintended copyleft obligations.

This ADR defines the policy future implementation, CI, and releases must follow. It does **not** introduce scanners, SBOMs, Cargo manifests, or dependencies in this task.

### Current repository reality

* No Cargo application/library dependency tree
* No license scanner
* No SBOM generation
* No third-party attribution inventory
* No `NOTICE` file currently required by existing audits

Do not invent fictional TX4 dependencies.

---

## 3. Decision

**Adopt the dependency and third-party license policy defined in this ADR.**

Core principles:

```text
PROJECT LICENSE (Apache-2.0)
        ↓
Every dependency license must be identified
        ↓
Compatibility review against Apache-2.0 distribution model
        ↓
Attribution / NOTICE obligations assessed
        ↓
Distribution implications assessed (source / binary / container)
        ↓
Approved, rejected, or exception-documented
        ↓
CI/governance enforcement (implementation later)
```

“Open source” does **not** automatically mean compatible with TX4’s Apache-2.0 project license and distribution model.

This policy **preserves** ADR-002. It does not reinterpret, weaken, dual-license, or replace Apache-2.0.

---

## 4. Dependency License Policy

### 4.1 Applicability

This policy applies to:

* direct dependencies
* transitive dependencies
* build-time / runtime / development dependencies that are redistributed with TX4 artifacts
* vendored/copied third-party source, if ever introduced
* third-party code embedded in release artifacts (binaries, containers, packages)

Dev-only tools that are **not** redistributed with TX4 OSS artifacts still require license identification, but may follow a lighter review path if they cannot affect distributed artifact obligations. Ambiguity defaults to full review.

### 4.2 Required identification

Before adding or upgrading a dependency, record at minimum:

| Field | Description |
| --- | --- |
| Name | Package / crate / component |
| Version | Exact version or version constraint under review |
| Direct / Transitive | Relationship |
| License | Declared license name(s) |
| License expression | SPDX expression where available |
| Source | Registry / repository URL |
| Attribution required | Yes / No / Unknown |
| NOTICE required | Yes / No / Unknown |
| Review status | Proposed / Approved / Rejected / Exception |
| Exception | Reference if any |

Exact inventory file format and tooling remain **OPEN** (implementation later).

### 4.3 Compatibility baseline

Evaluate dependencies for compatibility with:

* Apache-2.0 project licensing (ADR-002)
* intended OSS source distribution
* future binary / container distribution
* self-hosted OSS use
* Managed Cloud proprietary components that incorporate or ship OSS artifacts

Where compatibility depends on linking model, distribution form, or legal interpretation, mark **Requires legal review** rather than silently approving.

---

## 5. Allowed / Review / Restricted Categories

Categories below are **engineering governance classifications**, not universal legal determinations.

### 5.1 Generally acceptable (permissive / Apache-compatible posture)

Typically acceptable for Apache-2.0 projects when metadata is clear and attribution/notice obligations are met:

* **Apache-2.0**
* **MIT**
* **BSD-2-Clause**
* **BSD-3-Clause**
* **ISC**
* Other clearly permissive OSI-approved licenses with similar attribution-only obligations

Still require: license identification, inventory entry, and attribution handling as applicable.

### 5.2 Conditional / review-required

Require explicit review (and legal review when obligations are material):

* **MPL-2.0** (file-level weak copyleft; distribution of modified MPL files has obligations)
* **LGPL-family** (dynamic/static linking and modification/distribution nuances)
* Dual-licensed components (see §11)
* Public-domain / **CC0**-style dedications (confirm dedication scope and patent/disclaimer posture)
* Licenses with additional notices, advertising clauses, or unusual conditions
* Any license whose SPDX metadata is incomplete, conflicting, or non-standard

### 5.3 Restricted / default-prohibited for OSS-distributed TX4 artifacts

Default **reject** for code redistributed as part of TX4 OSS artifacts unless an explicit, documented exception is approved after legal review:

* **GPL-family** (GPLv2, GPLv3, and similar strong copyleft)
* **AGPL-family** (AGPLv3 and similar network copyleft)
* Proprietary / non-OSS licenses incompatible with Apache-2.0 OSS distribution
* Custom licenses that restrict field-of-use, commercial use, SaaS use, or redistribution in ways that conflict with TX4’s Apache-2.0 OSS goals
* Unknown / unascertainable licenses

Rationale: strong copyleft and network copyleft can impose redistribution or source-offer obligations that conflict with TX4’s intended Apache-2.0 OSS distribution and commercial Managed Cloud posture. This is a **policy default**, not a claim that such combinations are always legally impossible.

### 5.4 Unknown / ambiguous

If license metadata is missing, contradictory, non-SPDX, or only “custom”:

1. Do not add the dependency
2. Attempt to obtain authoritative license text from upstream
3. If still ambiguous → treat as restricted until legal review + maintainer exception

---

## 6. Direct and Transitive Dependencies

* **Direct** dependencies are the requester’s responsibility to identify and justify.
* **Transitive** dependencies are in scope; a direct dependency may be rejected because of its transitive closure.
* Review must consider the dependency tree that will be present in the released/distributed artifact, not only the top-level declaration.
* When tooling later exists, CI should fail closed on newly introduced restricted licenses in the resolved tree (implementation later).

---

## 7. Attribution / NOTICE

### 7.1 Principles

* Apache-2.0 does **not** require every project to ship a `NOTICE` file.
* A `NOTICE` file is required/useful when the Work includes a NOTICE as part of its distribution, when third-party licenses require attribution notices, or when TX4 chooses to publish a consolidated attribution inventory for release clarity.
* Do not create empty ceremonial `NOTICE` files.

### 7.2 When to create / update NOTICE or attribution inventory

Create or update `NOTICE` and/or a third-party attribution inventory when any of the following become true:

* A dependency’s license requires retention/reproduction of copyright or attribution notices in distributions
* Third-party code is vendored/bundled into TX4 source or release artifacts
* TX4 distributes binaries or containers that include third-party components with notice obligations
* Legal review or release policy requires a consolidated attribution file

### 7.3 Current stage

Based on current repository state (no dependency inventory): **no NOTICE is required today**.

Future dependency additions must reassess NOTICE / attribution before merge and before release.

### 7.4 Managed Cloud-only dependencies

Proprietary Managed Cloud components may use additional dependencies not shipped in OSS artifacts. Those still require license review for the Cloud distribution model, but do not automatically impose OSS `NOTICE` contents. Do not use Cloud-only deps to bypass OSS policy for code that is actually distributed in OSS.

---

## 8. License Inventory

TX4 intends to maintain a license inventory for dependencies included in distributed artifacts.

Recommended fields (format OPEN):

```text
Dependency
Version
Direct/Transitive
License
License Expression
Source
Attribution Required
NOTICE Required
Review Status
Exception
```

Inventory may live in-repo (for example under `docs/` or `.license/`) once implementation is authorized. This ADR does not create that file now.

---

## 9. SBOM

### 9.1 Intent

TX4 intends to produce and retain a **Software Bill of Materials (SBOM)** for **released/distributed artifacts** once releasable artifacts exist.

SBOM expectations apply to:

* release source archives (as applicable)
* release binaries
* container images
* other distributed packages

Repository `main` during pre-implementation foundation may not yet have an SBOM.

### 9.2 SBOM contents (logical)

SBOM should support at least:

* component identity and version
* dependency provenance where tooling allows
* license metadata where available

### 9.3 Tooling

Concrete SBOM tool/format (CycloneDX, SPDX, cargo-specific generators, etc.) remains an **implementation decision** after freeze. This ADR sets the requirement that released artifacts should have reproducible license/component inventory, not a specific generator.

---

## 10. Automation / CI Policy

### 10.1 Intended lifecycle

```text
Developer proposes dependency
        ↓
License metadata identified
        ↓
Compatibility review (this policy)
        ↓
Attribution / NOTICE implications reviewed
        ↓
Security/provenance review where applicable
        ↓
Approved (or exception recorded)
        ↓
Dependency added
        ↓
CI verifies policy (fail closed)
```

### 10.2 CI expectations (future)

Once implementation exists, CI SHOULD:

* detect dependency manifest changes
* resolve or inspect license metadata for the dependency set
* fail on newly introduced restricted/unknown licenses without an approved exception
* fail on missing required inventory/attribution updates when policy requires them

### 10.3 Out of scope for TASK-006

* Implementing scanners
* Adding CI jobs
* Adding Cargo.toml / lockfiles
* Selecting scanner vendors

---

## 11. Dual-Licensed Dependencies

If a dependency is dual-licensed (or multi-licensed):

1. Prefer the option that is compatible with Apache-2.0 OSS distribution under this policy (typically a permissive option when lawfully available)
2. Record which license option TX4 elects to use
3. Do not assume “any option” without documenting the election
4. If no compatible option exists → treat as restricted

---

## 12. Vendored Dependencies

If third-party source is ever copied/vendored into the TX4 repository:

* Record upstream identity, version, license, and attribution
* Preserve required copyright/license notices
* Update NOTICE / attribution inventory as required
* Prefer package-managed dependencies over vendoring unless there is a documented reason

Vendoring does not escape this policy.

---

## 13. Exceptions

### 13.1 Authority

Exceptions require **Project Maintainer** approval (see GOVERNANCE.md). Material exceptions SHOULD also obtain legal review.

### 13.2 Required documentation

An exception record must include:

* dependency identity and version
* license(s) at issue
* why the dependency is necessary
* alternatives considered
* distribution surfaces affected (OSS source / binary / container / Cloud-only)
* risk assessment
* approver
* date
* review/expiry date (default: re-review within 12 months or at next major release, whichever first)

### 13.3 Storage

Exceptions must be explicit in repository governance records (for example under `docs/adr/exceptions/` or an approved inventory field). Silent overrides are prohibited.

### 13.4 Ambiguity

Ambiguous licensing cannot be “approved by silence.” Default action: do not merge.

---

## 14. License Changes

If any of the following occur, **re-review before upgrade/release**:

* Dependency changes license
* Permissive → copyleft or restricted
* Transitive dependency introduces a restricted license
* License metadata disappears or becomes ambiguous
* Upstream is relicensed
* TX4 begins distributing binaries/containers that change obligation surface

Changed licensing conditions must not be silently accepted via routine version bumps.

If an in-tree dependency becomes non-compliant:

1. Block release of affected artifacts
2. Replace, remove, or obtain an approved exception
3. Update inventory / NOTICE / SBOM as applicable

---

## 15. Source / Binary / Container Distribution

Obligations may differ by distribution form:

| Surface | Policy emphasis |
| --- | --- |
| OSS source repository | License identification; preserve notices; inventory for declared deps when present |
| Source release archives | Same + attribution/NOTICE as required by included components |
| Binaries | Include required notices; SBOM for release; verify transitive closure |
| Containers | Treat image contents as distributed components; SBOM + license review of image layers/deps |

Do not assume source-only compliance covers binary/container releases.

---

## 16. Managed Cloud Considerations

Preserve:

```text
TX4 OSS = Apache-2.0
TX4 Managed Cloud = proprietary hosted service (boundary = ADR-008)
```

* OSS-distributed components must follow this dependency policy strictly.
* Managed Cloud may include additional proprietary code and Cloud-only dependencies, still subject to license review for those components.
* Cloud-only dependencies must not be copied into OSS without meeting OSS policy.
* This ADR does **not** redefine the OSS/Managed Cloud product boundary (ADR-008).

---

## 17. Security / Provenance (adjacent)

License review is necessary but not sufficient. Dependency proposals SHOULD also consider:

* known vulnerability posture (process matured later)
* maintainer/provenance trust
* unnecessary dependency expansion (see AGENTS / CONTRIBUTING)

Detailed security-scanning tool selection remains outside this ADR.

---

## 18. Consequences

### Positive

* Clear gate before Rust/Cargo dependencies appear
* Protects Apache-2.0 OSS distribution posture (ADR-002)
* Defines NOTICE/SBOM/inventory expectations without fake current deps
* Fail-closed stance for unknown/restricted licenses
* Explicit exception governance

### Negative / tradeoffs

* Higher friction when adding dependencies
* Some popular copyleft crates may be unavailable for OSS artifacts without exception
* Requires future tooling investment (scanner, SBOM, CI)
* Legal review still needed for edge cases

### Risks

* Treating this policy as legal advice
* Approving transitive risk by reviewing only direct deps
* Creating empty NOTICE files “for appearance”
* Cloud-only deps leaking into OSS
* Silent acceptance of upstream relicensing via automated updates

---

## 19. Non-Decisions

Explicitly **OPEN** / out of scope:

* Concrete license scanner product
* Concrete SBOM tool/format
* Inventory file path/format
* CI workflow implementation
* API / database / persistence / durability architecture
* Transaction lifecycle (ADR-007)
* Monetary representation (ADR-006)
* Payment / ledger / settlement architecture
* API versioning (ADR-005)
* Source-of-truth boundaries (ADR-004)
* OSS/Managed Cloud boundary details (ADR-008)
* Trademark policy, CLA/DCO instruments, commercial contracts
* Runtime language (ADR-001 already FROZEN)
* Project license (ADR-002 already FROZEN)

---

## 20. Future Implementation Tasks

After this ADR is audited and frozen, later authorized tasks may:

1. Add license inventory template/location
2. Wire CI license policy checks for Cargo
3. Generate SBOMs for release artifacts
4. Establish NOTICE/attribution update checklist in release process
5. Document exception record format/path

None of those tasks are authorized by TASK-006.

---

## 21. Legal Disclaimer

This policy is an engineering governance control for TX4 contributors and maintainers. It does **not** constitute legal advice and does **not** guarantee license compliance, patent safety, or freedom from third-party claims. Before commercial distribution or when classifying conditional/restricted licenses, obtain professional legal review appropriate to TX4’s jurisdictions and distribution model.

---

## 22. References

* ADR-002 — OSS License (Apache-2.0) — FROZEN
* ADR-001 — Core Runtime Language (Rust) — FROZEN
* Repository `LICENSE` — Apache License 2.0
* Master Spec §28 Licensing
* Apache License 2.0: https://www.apache.org/licenses/LICENSE-2.0
* SPDX License List (for identifiers): https://spdx.org/licenses/
