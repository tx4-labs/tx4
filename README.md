# TX4

**Transaction Infrastructure**

> Open-source transaction infrastructure for developers building transactional applications.

**Status:** Early development — repository foundation only
**Domain:** [tx4.xyz](https://tx4.xyz)
**License:** [Apache License 2.0](LICENSE) (ADR-002 FROZEN; LICENSE file applied)

---

## Why TX4

Developers building ordering, booking, marketplace, ticketing, and similar products repeatedly rebuild the same infrastructure:

* transaction lifecycle
* payment orchestration
* fees
* billing / metering
* ledger integration
* settlement
* reconciliation
* idempotency
* webhooks
* auditability

TX4 aims to provide these primitives once, behind a stable developer-facing API.

---

## What TX4 Is

TX4 is **infrastructure**, not a vertical application.

```text
Developer
    ↓
TX4 API / SDK
    ↓
Transaction Infrastructure
    ├── transactions
    ├── payments
    ├── fees
    ├── billing
    ├── ledger
    ├── settlement
    ├── reconciliation
    └── webhooks
```

Applications such as canteen ordering, booking, or marketplaces are expected to consume TX4 — not to become part of TX4 Core.

---

## What TX4 Is Not

TX4 is not initially intended to become:

* a consumer wallet
* a bank
* a payment processor
* a generic ERP
* a complete e-commerce storefront
* a consumer marketplace
* a vertical-specific SaaS
* a replacement for every accounting system, payment provider, or billing platform

See [docs/MASTER-SPEC.md](docs/MASTER-SPEC.md) for the full non-goals list.

---

## Architecture

TX4 has two delivery layers:

| Layer | Description |
| --- | --- |
| **TX4 OSS** | Self-hostable open-source core |
| **TX4 Managed Cloud** | Commercial hosted offering |

Both are intended to share the same fundamental transaction model and public API contract.

Core design principle: **infrastructure ≠ vertical application**. Vertical-specific logic does not belong in TX4 Core.

---

## OSS vs Managed Cloud

**OSS** is intended to include the transaction core, public API, schema/migrations for self-hosting, payment abstraction, fee/usage/settlement/reconciliation primitives, webhooks, idempotency, tenant isolation, SDKs, Docker-oriented deployment, tests, docs, and examples — once those components exist and are frozen for release.

**Managed Cloud** may retain proprietary control plane, hosted dashboard, orchestration, managed operations, and commercial analytics.

The OSS edition must remain genuinely useful for production self-hosting. It must not be intentionally crippled to force Cloud adoption.

Exact OSS / Cloud boundaries remain **PROPOSED** until ADR-008.

---

## Dogfood Applications

Dogfood applications (for example, a canteen ordering app) are validation surfaces. They must use the same public API available to external developers.

They are **not** part of this repository foundation and are not implemented yet.

---

## Current Status

This repository currently contains governance, documentation, and CI baseline only.

```text
PRODUCT THESIS       = PROPOSED
OSS/CLOUD BOUNDARY   = PROPOSED
CORE DOMAIN          = PROPOSED
RUNTIME              = OPEN
LICENSE              = OPEN
API                  = PROPOSED
BILLING MODEL        = PROPOSED
DOGFOOD STRATEGY     = PROPOSED
REPOSITORY STANDARD  = PROPOSED
PRODUCTION TARGET    = FROZEN AS OBJECTIVE
```

No runtime, schema, API server, payment integration, or SDK implementation is present yet.

---

## Roadmap

See [ROADMAP.md](ROADMAP.md).

High-level sequence:

1. Repository foundation (this phase)
2. Architecture & ADR freeze
3. Core domain and transaction infrastructure
4. Payment / fee / billing
5. Ledger / settlement / reconciliation
6. API / SDK
7. Production hardening
8. Managed Cloud
9. Dogfood applications
10. Public adoption

---

## Development

```text
main                 protected / releasable
feature branches     feat/*, fix/*, docs/*, chore/*, ...
```

* Conventional Commits required
* Meaningful changes should go through Pull Requests
* Architecture changes require an ADR or explicit maintainer decision

See [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md).

---

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening issues or pull requests.

Contributions that materially change architecture require an ADR or explicit maintainer decision.

---

## Security

Please report security issues according to [SECURITY.md](SECURITY.md).

Do not post sensitive vulnerability details in public GitHub issues.

---

## Documentation

* [Master Specification](docs/MASTER-SPEC.md)
* [Architecture Decision Records](docs/adr/)
* [Governance](GOVERNANCE.md)
* [Code of Conduct](CODE_OF_CONDUCT.md)
* [Changelog](CHANGELOG.md)

---

## License

License selection is **OPEN** and must be decided by **ADR-002**.

Until then, see [LICENSE](LICENSE) for the current placeholder statement. Do not assume any particular open-source license applies.
