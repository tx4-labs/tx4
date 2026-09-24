# ADR-002 — OSS License

| Field | Value |
| --- | --- |
| ADR | **002** |
| Title | OSS License |
| Status | **FROZEN** |
| Date | 2026-09-24 |
| Frozen | 2026-09-24 (FREEZE-004; basis AUDIT-004 PASS) |
| Decision scope | Project-level open-source license for TX4 OSS distribution only |
| Related Master Spec | §3 OSS Boundary; §4 Managed Cloud Boundary; §28 Licensing; §29 Commercial Model; §32 Grant & Fundraising Readiness; §36 Current Status |
| Depends on | ADR-001 (Core Runtime Language = Rust) — FROZEN |
| Supersedes | None |

---

## Status

**FROZEN**

**TX4 project-level OSS license = Apache License 2.0 (Apache-2.0).**

This freeze selects the **project-level OSS license decision only**.

It does **not** update the repository root `LICENSE` file. A separate authorized task must replace the placeholder `LICENSE` with the official Apache-2.0 text and evaluate `NOTICE` as applicable.

It does **not** decide dependency/third-party license policy (ADR-003), OSS/Managed Cloud product boundary (ADR-008), trademark policy, CLA policy, commercial contract terms, documentation/SDK licensing schemes, or Managed Cloud proprietary licensing.

Freeze path completed: TASK-004 → AUDIT-004 → FREEZE-004.

---

## Decision

**TX4 project-level OSS license = Apache License 2.0 (Apache-2.0).**

Status of this decision: **FROZEN**.

Until an authorized LICENSE-file application task completes, the repository root `LICENSE` placeholder remains in place and must not be treated as the applied Apache-2.0 grant text.

---

## Context

TX4 is intended to be:

* production-grade open-source transaction infrastructure
* publicly usable and self-hostable
* commercially viable alongside a Managed Cloud offering
* grant-ready and fundraising-ready
* contributor-friendly
* suitable as serious OSS infrastructure

Master Spec §28 requires an explicit ADR for license selection balancing OSS adoption, commercial sustainability, contributor participation, grant compatibility, ecosystem compatibility, dependency licenses, and the Managed Cloud business model.

The current repository `LICENSE` file is an intentional placeholder (`Status: OPEN`).

ADR-001 has frozen **Rust** as the core runtime language. That does not select a license.

---

## Requirements

Evaluate candidate licenses against:

1. **Commercial use** — ability for users and companies to use TX4 OSS commercially under documented license terms
2. **Self-hosting** — ability to run modified or unmodified TX4 OSS privately
3. **Modification and redistribution** — clarity of permissions/obligations when modifying or redistributing
4. **SaaS / hosted use** — practical implications when TX4 OSS is operated as a service (including by TX4 Managed Cloud or third parties)
5. **Enterprise adoption** — familiarity and friction for enterprise legal review
6. **Contributor adoption** — clarity and accessibility for external contributors
7. **Patent-related provisions** — whether the license text includes explicit patent grants/terminations (not a claim of eliminating patent risk)
8. **Downstream / ecosystem interoperability** — practical compatibility posture with common OSS ecosystems
9. **Grant / funding expectations** — whether the license is commonly acceptable in open-source infrastructure and funding contexts
10. **Fundraising / commercial positioning** — ability to sustain proprietary Managed Cloud components without requiring the OSS license alone to “protect” the business
11. **OSS Core vs proprietary Managed Cloud separation** — whether the license is compatible with keeping cloud control-plane components proprietary (product boundary remains ADR-008)
12. **Clarity for external developers** — understandable permissions for integrators and dogfood consumers

This document describes documented license characteristics and practical implications. It is **not legal advice**. Material legal interpretation requires professional review before freeze and before relying on the license in production commercial contracts.

---

## Options Considered

### Option A — Apache License 2.0

Permissive OSI-approved license with an explicit patent license grant and patent termination provisions in the license text. Widely used by infrastructure and cloud-native projects. Requires preservation of copyright/license notices and a NOTICE file when applicable.

### Option B — MIT License

Short permissive OSI-approved license. Broad reuse permissions with attribution. Does **not** include an explicit patent grant section comparable to Apache-2.0’s patent language.

### Option C — Mozilla Public License 2.0 (MPL-2.0)

Weak/file-level copyleft OSI-approved license. Modified MPL-covered files generally must remain under MPL when distributed; larger works may combine with proprietary code under MPL’s terms. More complex contribution and compliance story than Apache-2.0/MIT for many teams.

### Option D — GNU GPLv3

Strong copyleft. Redistribution of combined/derivative works generally requires GPL terms. Commonly creates higher friction for proprietary products and some enterprise adoption paths. Does not by itself define network/SaaS obligations the way AGPL does.

### Option E — GNU AGPLv3

Strong copyleft with an additional network interaction provision in the license text intended to address remote use. Often increases legal/compliance friction for hosted offerings, enterprise adoption, and some commercial partnership paths.

Other licenses (BSL, SSPL, proprietary “source available”) are out of scope: they are not treated here as conventional OSS choices for a genuine open-source infrastructure distribution as described in the Master Spec.

---

## Option Analysis

Qualitative comparison only. No numeric scores, weights, rankings, or “best license” declaration.

| Consideration | Apache-2.0 | MIT | MPL-2.0 | GPLv3 | AGPLv3 |
| --- | --- | --- | --- | --- | --- |
| Commercial use of OSS | Permissive under license terms | Permissive under license terms | Permissive with file-level copyleft obligations | Use allowed; redistribution of derivatives constrained by copyleft | Use allowed; network provision adds hosted-use compliance considerations |
| Self-hosting / private modification | Compatible | Compatible | Compatible; distribution of modified MPL files has obligations | Compatible privately; distribution triggers copyleft | Compatible privately; network use can trigger source obligations |
| Redistribution clarity | Mature; notice requirements | Very simple attribution | More complex file-level rules | Strong copyleft obligations | Strong copyleft + network provision |
| Offering as a service | Generally no OSS-license obligation to open proprietary surrounding services solely because of hosting (product/compliance still matter) | Similar practical posture to other permissive licenses | Similar for non-MPL files; MPL files keep file-level rules on distribution | Does not itself encode AGPL-style network clause | Network interaction provision is material for hosted offerings |
| Enterprise legal familiarity | High for many infra/cloud contexts | High | Moderate | Mixed; higher friction in some orgs | Often higher friction |
| Contributor simplicity | Good | Very good | Trade-off; more rules to teach | Trade-off | Trade-off |
| Explicit patent grant language | Yes (in license text) | No equivalent dedicated patent grant section | Includes patent provisions in license text | Includes patent-related terms | Includes patent-related terms |
| Ecosystem interoperability (practical) | Strong with many permissive ecosystems | Strong | Good but more nuanced | More constrained combinations | More constrained combinations |
| Grant / infra-OSS positioning | Commonly used | Commonly used | Used; less default for infra cores | Used; less common for permissive infra cores | Used; often controversial for SaaS-adjacent infra |
| Proprietary Managed Cloud components | Compatible with keeping separate proprietary control plane (boundary = ADR-008) | Compatible | Compatible if boundary/compliance handled carefully | Higher coupling risk if cloud distributes GPL-combined works | Higher friction for hosted model |
| Clarity for external developers | Clear, widely documented | Clearest short text | Needs more explanation | Needs more explanation | Needs more explanation |

### Apache-2.0

Fits TX4’s intent to be genuinely open for self-hosting and commercial use while remaining compatible with a separate proprietary Managed Cloud control plane. Explicit patent grant language is material for an infrastructure project used by enterprises. Notice obligations are manageable.

### MIT

Attractive for simplicity and contributor accessibility. Relative to Apache-2.0, the absence of an explicit patent grant section is a material trade-off for TX4’s enterprise/infrastructure posture. Often paired with Apache-2.0 in dual-license ecosystems; dual-licensing is **not** proposed here to keep the decision single and clear.

### MPL-2.0

Can support mixed proprietary/open architectures via file-level copyleft. Adds compliance and contributor education cost. Does not clearly improve TX4’s stated goals over Apache-2.0 for a generic infrastructure core intended for broad adoption.

### GPLv3 / AGPLv3

Provide strong reciprocity on distribution (and AGPL on network use). That can be desirable for some projects seeking maximal share-alike outcomes, but increases friction for enterprise adoption, some commercial partnerships, and a Managed Cloud business that keeps proprietary operational components. Master Spec requires genuine OSS usefulness without using licensing alone as an artificial commercial moat; copyleft is not required to meet that principle and introduces adoption trade-offs TX4 prefers to avoid at this stage.

---

## Decision Rationale

**Apache License 2.0** was selected because it best matches TX4’s combination of:

1. **Genuine OSS usability** — permissive terms support self-hosting, modification, and redistribution with well-understood notice obligations
2. **Commercial and enterprise posture** — familiar to many enterprise reviewers for infrastructure software
3. **Patent grant language** — Apache-2.0 includes an explicit patent license grant and related termination provisions in the license text (this does **not** eliminate all patent risk)
4. **Managed Cloud coexistence** — compatible with maintaining proprietary hosted control-plane/operations components as separate works, subject to ADR-008 boundary design and ordinary compliance
5. **Contributor and ecosystem clarity** — widely understood; avoids teaching file-level or AGPL network rules as the default contributor experience
6. **Grant / fundraising readiness** — commonly acceptable in OSS infrastructure contexts without relying on marketing claims about funding outcomes

Apache-2.0 is **not** proposed as a guarantee of adoption, fundraising success, competitive protection, or legal safety.

---

## OSS / Managed Cloud Implications

Intended model (product boundary details remain **ADR-008**):

```text
TX4 OSS (Apache-2.0 — FROZEN project license; LICENSE file application pending)
  → self-hostable core and related OSS artifacts

TX4 Managed Cloud (proprietary components as designed)
  → hosted control plane, dashboard, orchestration, managed ops, commercial analytics, etc.
```

Under a permissive license such as Apache-2.0:

* Third parties may generally host TX4 OSS under the license terms
* TX4’s commercial differentiation is expected to come from Managed Cloud operations, reliability, and product experience — not from OSS license lock-in
* The OSS edition must remain genuinely useful (Master Spec §3); licensing must not be used to intentionally cripple OSS

This ADR does **not** redefine which components are OSS vs proprietary. That remains ADR-008.

---

## Commercial Implications

Distinguish carefully:

| Topic | Implication under proposed Apache-2.0 |
| --- | --- |
| Commercial use of OSS | Generally permitted under Apache-2.0 terms |
| Hosted / SaaS use of OSS | Generally does not, by itself, impose Apache-2.0 source-distribution obligations merely for running a service; compliance, trademarks, and product boundaries still matter |
| Redistribution | Permitted with license/notice conditions |
| Modification | Permitted; distribution of modified works must meet Apache-2.0 conditions |
| Proprietary Managed Cloud | Separate proprietary works remain possible; do not treat the OSS license as a substitute for ADR-008 boundary design or commercial contracts |

“Open source” does **not** mean identical commercial permissions across all licenses; AGPL/GPL differ materially from Apache-2.0/MIT on redistribution and (for AGPL) network use.

---

## Patent Considerations

Apache-2.0 includes explicit patent license grant and patent termination provisions in its text.

This ADR does **not** claim:

* that Apache-2.0 eliminates patent risk
* that contributors or users are fully protected against all third-party patents
* that patent provisions substitute for legal counsel

MIT lacks a comparable dedicated patent grant section; that difference is a reason Apache-2.0 is preferred for TX4’s infrastructure/enterprise context.

---

## Contributor / Ecosystem Implications

* Contributors should expect contributions to be licensed under the project license once ADR-002 is frozen and `LICENSE` is updated
* Until freeze, the placeholder `LICENSE` warns not to assume a named license
* Downstream forks and commercial integrators can generally reuse Apache-2.0 code under its terms with attribution/notice
* Package ecosystems commonly accept Apache-2.0 artifacts
* Exact contribution mechanics (CLA vs DCO vs inbound=outbound) remain **OPEN** and are not decided here

---

## Compatibility Considerations

Project-level license choice affects, but does not fully determine, dependency policy.

* ADR-003 must define how third-party dependency licenses are reviewed for compatibility with the chosen project license
* This ADR must **not** silently approve or ban specific dependency licenses
* Combining Apache-2.0 project code with copyleft dependencies can create obligations; those rules belong in ADR-003
* Rust ecosystem crates have heterogeneous licenses; compatibility review remains future work under ADR-003

---

## Consequences

### Positive

* Clear proposed license for public OSS distribution after freeze
* Aligns with permissive infrastructure-OSS norms
* Supports self-hosting and commercial use under documented terms
* Compatible with proprietary Managed Cloud components when boundaries are designed correctly (ADR-008)
* Explicit patent grant language relative to MIT

### Negative / tradeoffs

* Permissive licensing allows third parties to host competing services using TX4 OSS without share-alike obligations
* Notice/`NOTICE` compliance must be maintained in distributions
* Does not create copyleft leverage against proprietary forks
* Requires careful ADR-003 work so dependencies do not undermine the intended licensing posture

### Risks

* Treating Apache-2.0 as “legal safety” without counsel review
* Blurring OSS and proprietary Cloud code without ADR-008
* Updating marketing claims (“guarantees funding/adoption”) that the license cannot support
* Freezing the license before ADR-003 clarifies dependency constraints

---

## Non-Decisions

Explicitly **OPEN** / out of scope:

* Dependency / third-party license policy → **ADR-003**
* Managed Cloud / OSS boundary → **ADR-008**
* Trademark policy
* CLA / DCO / inbound contribution legal instruments
* Governance changes beyond license selection
* Commercial terms, pricing, support contracts
* Separate documentation or API licensing schemes
* SDK licensing exceptions (default expectation: same project license unless a future decision says otherwise — not decided here)
* Database, persistence, durability, payment, or ledger architecture
* Contents of the repository `LICENSE` file update (authorized only at/after freeze)
* Dual-licensing

---

## Reconsideration Conditions

Revisit this **FROZEN** decision only if objective evidence shows:

1. Professional legal review identifies a material incompatibility with TX4’s documented OSS + Managed Cloud model that Apache-2.0 cannot reasonably address
2. ADR-003 analysis shows the chosen project license cannot support a viable dependency posture for the Rust ecosystem TX4 needs
3. Master Spec commercial/OSS goals change materially
4. Granting bodies or critical partners impose a conflicting, non-negotiable license constraint that still preserves genuine OSS distribution as defined by the Master Spec

Preference alone is insufficient after freeze.

---

## References / Legal Notice

* Apache License 2.0: https://www.apache.org/licenses/LICENSE-2.0
* MIT License: https://opensource.org/licenses/MIT
* MPL 2.0: https://www.mozilla.org/MPL/2.0/
* GPLv3: https://www.gnu.org/licenses/gpl-3.0.html
* AGPLv3: https://www.gnu.org/licenses/agpl-3.0.html
* Master Spec §28 Licensing; §3–§4 OSS/Cloud boundaries
* Current placeholder: repository root `LICENSE`

**Legal notice:** This ADR is an architecture/product decision record. It is not legal advice. Before commercial reliance and before/as part of applying the Apache-2.0 text to the repository `LICENSE` file, obtain professional legal review appropriate to TX4’s jurisdictions and business model.
