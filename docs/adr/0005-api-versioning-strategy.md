# ADR-005 — API Versioning Strategy

| Field | Value |
| --- | --- |
| ADR | **005** |
| Title | API Versioning Strategy |
| Status | **FROZEN** |
| Date | 2026-09-24 |
| Frozen | 2026-09-24 (FREEZE-008; basis AUDIT-010 PASS) |
| Decision scope | Canonical public REST API versioning strategy for TX4 (identifier placement, compatibility, deprecation, sunset, SDK/OpenAPI relationship) |
| Related Master Spec | §19 API; §20 SDK; §24 Dogfood Applications; §31 Production-Grade; §34 Architecture Rule; §36 Current Status (`API = PROPOSED`) |
| Depends on | ADR-001 (Rust) — FROZEN; ADR-002 (Apache-2.0) — FROZEN; ADR-003 — FROZEN; ADR-004 (SoT / durability) — FROZEN |
| Supersedes | None |

---

## 1. Status

**FROZEN**

Freeze path completed: TASK-008 → AUDIT-010 → FREEZE-008.

This ADR does **not** select HTTP frameworks, OpenAPI generators, API gateways, authentication mechanisms, databases, payment adapters, transaction lifecycle enums, or monetary representation.

This policy is an engineering contract-evolution rule. It does **not** by itself create commercial SLA or support-duration obligations.

---

## 2. Context

TX4’s public API is a first-class product artifact (Master Spec §19): REST + OpenAPI, with versioning, consistent errors, idempotency, authn/authz, pagination, filtering, request correlation, and rate-limit headers where appropriate.

The same API contract must support:

```text
Self-hosted OSS
       +
TX4 Managed Cloud
```

Dogfood applications must consume the same public API as external developers.

API evolution must remain compatible with ADR-004 durability and financial-safety discipline: breaking changes that affect payment, transaction state, financial fields, idempotency, authentication, or authorization require heightened care.

---

## 3. Decision

**Use URI path versioning for the TX4 public REST API.**

Canonical form:

```text
/v{N}/...
```

Initial public API version:

```text
/v1/...
```

Where `{N}` is a positive integer major API contract version.

### What this decides

* Path-based major version identifiers are the **canonical** public version mechanism.
* Clients and documentation address a version by path.
* Self-hosted OSS and Managed Cloud expose the **same** versioning strategy and must not diverge into split-brain contracts.

### What this does not decide

* Whether `/v2` ever ships
* Exact route trees, schemas, or error codes
* HTTP framework / middleware
* OpenAPI tooling
* SDK packaging strategy beyond version relationship principles

---

## 4. Versioning Unit

### Versioned

The **public API contract**, including:

* endpoint / resource availability and paths under a version prefix
* request and response schemas
* field presence, types, and documented meanings
* documented error contract for that version
* documented authn/authz interaction as part of the public surface
* idempotency semantics as exposed by the public API

### Not the same as (and not versioned by this ADR)

| Concern | Versioned by |
| --- | --- |
| Internal Rust crate / binary release | Application release process (OPEN) |
| Database schema / migrations | Persistence / migration policy (OPEN) |
| Dependency versions | ADR-003 + package tooling |
| Domain rule / fee schedule versions | Domain specifications (OPEN) |
| SDK package versions | SDK release process (related but distinct) |

A TX4 server release may support one or more API contract versions simultaneously.

---

## 5. Alternatives Considered

Qualitative comparison only. No numeric scores or rankings.

| Approach | Discoverability | Debugging / logs | Docs / SDK gen | Proxy / gateway fit | Ops simplicity | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| **URI path `/vN`** | Strong | Strong (visible in URL) | Strong | Strong | Strong | Selected |
| Header versioning | Weaker for casual discovery | Harder without capturing headers | Good if documented | Variable | Trade-off | Easy to omit; harder for browsers/curl demos |
| Media-type (`Accept`) | Weak for many clients | Harder | Possible | Variable | Higher complexity | Powerful but less ergonomic for infra quickstarts |
| Query `?version=` | Moderate | Moderate | Uneven | Weaker caching/semantics | Tempting but brittle | Easy to ignore; ugly for resource identity |

### Rationale for URI path versioning

1. **Developer discoverability** — version is visible in every example URL and quickstart.
2. **Debugging** — access logs and traces show the contract version without extra header capture.
3. **Documentation & SDK generation** — OpenAPI can be scoped per `/vN` surface cleanly.
4. **Self-hosted + Managed Cloud** — same path convention works behind reverse proxies without mandatory header rewrite rules.
5. **Operational simplicity** — solo-maintainer-friendly; matches common infrastructure API practice.
6. **Long-lived contracts** — major versions are explicit and hard to accidentally mix.

Header or media-type versioning remains possible as a **non-canonical** extension only if a later ADR explicitly authorizes it; they are not the default public contract mechanism.

---

## 6. Initial Version

* The first public API contract version is **`v1`**, exposed under `/v1/...`.
* Do not invent `/v2` until a breaking change requires a new major contract.
* Pre-release / unstable experimental surfaces, if any, must be explicitly labeled and are outside the stable `vN` compatibility promises of this ADR (details OPEN).

---

## 7. Breaking Change Definition

A **breaking change** to a published API version includes, at minimum:

* removing an endpoint
* removing a response or request field that was part of the published contract
* changing the meaning/semantics of an existing field or endpoint
* making a previously optional request field required
* incompatible enum changes (removing/renaming values, changing meaning)
* incompatible changes to error shapes or status-code contracts that clients are documented to rely on
* incompatible changes to authentication/authorization requirements of the public surface
* changing idempotency semantics in a way that alters client-visible guarantees

### Generally non-breaking (compatible) when done carefully

* adding optional request fields with safe defaults
* adding response fields (clients must ignore unknown fields)
* adding new endpoints under the same major version
* adding new enum values **only if** clients are documented to tolerate unknown values; otherwise treat as breaking
* additive error codes **only if** clients are documented to tolerate unknown codes; otherwise breaking
* documentation clarifications that do not change behavior

Ambiguous cases default to **breaking** until maintainer review decides otherwise.

---

## 8. Compatibility Policy

Within a published major version `vN`:

1. **Additive evolution is preferred** over breaking changes.
2. Clients SHOULD ignore unknown response fields.
3. Servers MUST NOT remove or redefine published fields/endpoints without a new major version or an explicit deprecation+sunset path that ends in removal only after sunset.
4. Behavior changes that alter financial, payment, transaction-state, idempotency, authn, or authz outcomes are treated as high-risk and presumed breaking unless proven compatible.
5. TX4 does **not** promise indefinite backward compatibility across all future majors.
6. Multiple majors MAY be supported concurrently for a transition period; exact calendars are not frozen here.

This ADR does **not** create a contractual SLA for support duration.

---

## 9. Deprecation

When a major version (or a subset of its surface) is deprecated:

1. Mark it **deprecated** in API documentation and OpenAPI descriptions.
2. Provide migration guidance to the replacement version/surface.
3. Keep the deprecated version **operational** until sunset, unless a critical security issue forces earlier action (maintainer authority; document the exception).
4. Prefer advance notice in CHANGELOG / release notes.
5. Avoid inventing fixed multi-year SLA promises in this ADR.

Minimum expectation: deprecation is **explicit and documented** before removal.

---

## 10. Sunset

A major version may be **removed** only when:

1. It has been marked deprecated with published migration guidance, and
2. Maintainers authorize sunset (release notes / changelog), and
3. Removal does not silently strand the only supported public contract without a successor (except emergency security cases documented as exceptions).

After sunset, requests to the removed version SHOULD fail with a clear unsupported-version error (conceptual; exact status/body OPEN).

---

## 11. SDK Relationship

* SDKs target a **specific API major version** (or document which majors they support).
* An SDK package version is **not** identical to the API major version, but the mapping must be documented.
* Breaking API majors typically require a new SDK major (or explicit dual-major support).
* Initial SDK priority remains TypeScript (Master Spec); this ADR does not freeze SDK languages or generators.

---

## 12. OpenAPI Relationship

* Each supported public major version SHOULD have a machine-readable OpenAPI description of its contract.
* OpenAPI is the authoritative **documentation/contract artifact** for that version’s HTTP surface, subject to the Master Spec hierarchy (spec/ADR → API contract → implementation → tests).
* OpenAPI generator / publisher tooling remains **OPEN**.

---

## 13. Self-Hosted vs Managed Cloud

```text
Same versioning strategy
Same major contract semantics for a given /vN
```

* Self-hosted OSS and Managed Cloud MUST NOT intentionally diverge the public `/vN` contract for the same major.
* Cloud-only operational endpoints, if any, must be clearly separated and not silently redefine core TX4 `/vN` semantics.
* OSS/Managed Cloud product boundary remains ADR-008.

---

## 14. Version Negotiation

Canonical selection rule:

1. The version is determined by the URI path prefix `/v{N}/`.
2. No implicit “latest” path without a version is part of the stable public contract (unless a later ADR explicitly adds a redirect/alias policy).
3. Optional headers/media types must not override the path version for the canonical public API.

---

## 15. Unsupported Versions

If a client requests an unsupported or unknown `/v{N}/...`:

* The server SHOULD reject the request with a clear, consistent error indicating the version is unsupported.
* Exact HTTP status code and error schema remain OPEN (to be defined with the API error contract).
* Do not silently remap to another major version.

---

## 16. Documentation

Supported and deprecated API majors must be discoverable in public API documentation, including:

* which majors are supported
* which are deprecated
* migration notes
* link to OpenAPI for each supported major (when published)

---

## 17. Security / Financial Safety Interaction

API changes affecting payment, transaction state, financial fields, idempotency, authentication, or authorization:

* require heightened review
* are presumed breaking if client-visible guarantees change
* must respect ADR-004 (no fabricated certainty, durable effects, idempotent retries)

ADR-005 does not define authn/authz architecture or payment adapters.

---

## 18. Consequences

### Positive

* Clear, discoverable public contract versions
* Simpler docs, logs, and SDK targeting
* Coherent self-hosted + Managed Cloud surface
* Controlled breaking-change path via new majors
* Explicit deprecation/sunset authority

### Costs / tradeoffs

* Path prefixes add URL verbosity
* Parallel majors increase maintenance while overlapping
* Header-based advanced negotiation is non-canonical
* Requires discipline to avoid “quiet” breaking changes inside `v1`

---

## 19. Non-Decisions

Explicitly **OPEN**:

* HTTP framework (Axum or otherwise)
* API gateway / reverse proxy product
* OpenAPI generator / publisher
* SDK implementation languages and generators
* Authentication / authorization architecture
* Database / persistence technology
* Transaction lifecycle (ADR-007)
* Monetary representation (ADR-006)
* Payment architecture / provider adapters
* Exact error schema and status codes
* Exact idempotency key header/field names
* Support-duration SLAs
* Managed Cloud / OSS boundary details (ADR-008)

---

## 20. Reconsideration Conditions

Revisit only if objective evidence shows:

1. Path versioning materially blocks a required deployment/proxy environment that header/media-type versioning uniquely solves
2. A dual-canonical versioning scheme becomes necessary and is authorized by a superseding ADR
3. Master Spec changes the public API product model materially

---

## 21. References

* Master Spec §19 API; §20 SDK
* ADR-004 Core Source-of-Truth Boundaries — FROZEN
* ADR-001 / ADR-002 / ADR-003 — FROZEN
