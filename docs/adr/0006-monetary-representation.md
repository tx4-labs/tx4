# ADR-006 — Monetary Representation

| Field | Value |
| --- | --- |
| ADR | **006** |
| Title | Monetary Representation |
| Status | **FROZEN** |
| Date | 2026-09-24 |
| Frozen | 2026-09-25 (FREEZE-009; basis AUDIT-014 PASS) |
| Decision scope | Canonical TX4 monetary value model, exactness, currency identity, precision, rounding, allocation, overflow, serialization, comparison, and FX boundary |
| Related Master Spec | §9 Financial Correctness; §10 Fees; §11 Billing & Usage; §13 Ledger; §14 Settlement; §15 Reconciliation; §16 Idempotency; §19 API; §31 Production-Grade; §36 Current Status |
| Depends on | ADR-001 (Rust) — FROZEN; ADR-002 (Apache-2.0) — FROZEN; ADR-003 — FROZEN; ADR-004 (SoT / durability) — FROZEN; ADR-005 (API versioning) — FROZEN |
| Supersedes | None |

---

## 1. Status

**FROZEN**

Freeze path completed: TASK-009 → AUDIT-012 → TASK-012R → AUDIT-014 → FREEZE-009.

This ADR defines TX4’s **internal technical monetary representation and arithmetic rules**. It does **not** establish statutory accounting, tax compliance, regulated financial accounting, legal-tender treatment, or banking compliance.

This ADR does **not** select persistence technology, database types, Rust crates, payment providers, FX providers, fee business rules, tax rules, transaction lifecycle, or Managed Cloud / OSS boundary details.

---

## 2. Context

Master Spec §9 requires:

* no binary floating-point monetary authority
* exact monetary representation
* explicit currency
* no implicit currency conversion
* explicit financial invariants

Candidate guidance in the Master Spec:

```text
integer minor units
+
explicit currency
```

ADR-004 (FROZEN) requires durable financial records, idempotent effects, reconciliation, and TX4 ledger authority limited to TX4 internal financial records. Monetary representation remained OPEN for this ADR.

ADR-005 (FROZEN) defines public API versioning (`/v{N}/...`, initial `/v1/...`) but does not define money schemas.

Dogfood apps and external developers will exchange monetary values through the same public contract. Self-hosted OSS and Managed Cloud must share one monetary model to avoid split-brain financial meaning.

---

## 3. Decision

**TX4’s canonical authoritative monetary value is a signed integer count of a currency’s catalog-defined atomic unit, paired with an explicit currency identity.**

Conceptual model:

```text
Money {
  amount: signed integer   // count of atomic units
  currency: CurrencyId     // explicit currency identity
}
```

Where:

* `amount` is an exact integer (no fractional atomic units in the authoritative value)
* `currency` identifies the currency and selects catalog metadata (including atomic scale)
* display formatting, locale strings, and provider-specific encodings are **not** the authoritative value

### What this decides

* Authoritative money = **integer atomic units + explicit currency**
* Binary floating point is **prohibited** for authoritative monetary amounts and financial arithmetic
* Currency-specific precision comes from a **currency catalog**, not a single global scale
* Rounding, allocation, overflow, serialization, comparison, and FX boundaries follow the policies below

### What this does not decide

* Concrete Rust `Money` type / crate
* Database column types
* OpenAPI field names beyond serialization principles
* Fee schedules, tax rules, pricing formulas
* FX rates / FX providers
* Transaction / refund / settlement lifecycle (ADR-007)
* OSS / Managed Cloud product boundary (ADR-008)

---

## 4. Alternatives Considered

Qualitative comparison only. No numeric scores or unsupported benchmarks.

| Approach | Exactness | Determinism | Serialization risk | Multi-currency fit | Notes |
| --- | --- | --- | --- | --- | --- |
| **A. Binary floating point** (`f64`) | Fail | Fail | High | Unsafe | Prohibited by Master Spec §9 |
| **B. Decimal floating point** (IEEE-754 decimal / binary-coded decimal float) | Better | Possible | Medium | Possible | Exactness depends on mode; easy to misuse; heavier than needed for cash atomic units |
| **C. Fixed-point integer atomic units + currency** | Strong | Strong | Controllable | Strong with catalog | **Selected** — matches common payment rails and Master Spec candidate |
| **D. Arbitrary-precision decimal** | Strong | Strong | Controllable | Strong | Viable, but cash settlement usually ends on an atomic grid; adds complexity without replacing need for currency metadata |
| **E. Rational (p/q) as sole authority** | Strong | Strong | Heavier | Strong | Useful for **intermediates**; not chosen as the durable public money authority because settled cash values are atomic integers |

### Rationale for integer atomic units

1. **Exactness** — integers do not approximate common cash amounts.
2. **Deterministic arithmetic** — addition/subtraction of money in the same currency is exact.
3. **Reproducibility / auditability** — one integer + currency identity reconstructs the value.
4. **Payment-provider interoperability** — many providers already speak “minor units” / smallest currency units.
5. **API / persistence interoperability** — maps cleanly to exact integer storage and string serialization.
6. **Rust suitability** — checked integer arithmetic is idiomatic; no float money.
7. **Financial safety** — eliminates binary-float authority and many silent precision bugs.

Arbitrary-precision decimal remains acceptable for **non-authoritative intermediates** only when converted to canonical `Money` at an explicit rounding boundary (see §10–§11).

---

## 5. Money Model

### Authoritative fields

| Field | Meaning |
| --- | --- |
| `amount` | Signed integer count of the currency’s **atomic unit** |
| `currency` | Explicit currency identity selecting catalog metadata |

### Validity (conceptual)

A `Money` value is valid only if:

1. `currency` is present and identifies a supported catalog entry (or an explicitly documented unsupported/unknown handling path rejects the value), and
2. `amount` is a finite integer within the representable range policy (§13), and
3. no authoritative fractional atomic remainder remains (fractionals exist only as intermediates before rounding).

### Distinctions (must not be collapsed)

| Concept | Role |
| --- | --- |
| Authoritative `Money` | Exact integer atomic units + currency |
| Display string | Human presentation (symbols, separators, locales) — non-authoritative |
| Accounting narrative | Debit/credit labels, account names — domain/ledger concern (OPEN) |
| Provider encoding | Provider-specific amount formats — adapter mapping concern (OPEN) |

---

## 6. Currency Model

### Identity

* Currency identity is an explicit code (alphabetic), normalized to **uppercase ASCII** for comparison of standard codes.
* Unknown / empty / whitespace-only currency codes are **invalid** for authoritative money.
* Unsupported currencies must be **rejected** (or otherwise fail closed), not silently coerced to another currency.

### Catalog metadata (required concept)

Each supported currency has catalog metadata including at least:

* currency code
* **atomic scale** / atoms-per-major (or equivalent exact definition of the atomic unit)
* support status (supported / unsupported / experimental)

TX4 does **not** assume a fixed global scale such as “always 2 decimals.”

Examples (illustrative, not an exhaustive catalog):

| Currency | Typical atomic unit | Notes |
| --- | --- | --- |
| USD | 1/100 dollar (cent) | ISO minor exponent 2 |
| JPY | 1 yen | ISO minor exponent 0 |
| KWD | 1/1000 dinar (fils) | ISO minor exponent 3 |
| IDR | catalog-defined | Provider practice may differ from ISO tables; TX4 catalog is authoritative for TX4 |
| MGA / MRU | non-decimal historical subdivision | Catalog must define atomic unit explicitly; do not assume base-10 from ISO “minor unit” alone |

### ISO 4217 relationship

* TX4 **references** ISO 4217 alphabetic codes as the primary vocabulary for common currencies.
* ISO 4217 alone does **not** define every TX4 business rule (fees, taxes, provider quirks, acceptance lists).
* Currency **code validity**, acceptance, and atomic-unit metadata are a **TX4 currency catalog** concern.
* Catalog versions may evolve; breaking changes to atomic meaning for an existing code require explicit migration (and typically API major consideration under ADR-005).

Precious-metal / special codes (for example `XAU`) and non-currency placeholders (for example `XXX`) are not automatically supported for transactional money unless the catalog explicitly enables them.

---

## 7. Zero and Sign

### Zero

* Zero is a **valid** monetary amount.
* Zero still requires an explicit **currency** (there is no currency-less zero money).
* Authoritative representation does **not** distinguish “negative zero”; zero is a single value `amount = 0` for a currency.

### Negative values

* Negative `amount` values are **permitted** in the representation for signed financial effects (for example adjustments, credits, reversals as recorded by TX4).
* Interpreting a negative amount as a refund, debit, credit, or ledger posting is a **domain/lifecycle** concern and remains **OPEN** (ADR-007 / future ledger specs).
* Representation allows signed values; it does not define when negatives are business-valid.
* The **allocation primitive** (§11) does **not** accept negative `source.amount`. Negative adjustments/reversals must use separate explicit monetary operations; they must not silently reuse allocation semantics. This does not define refund/reversal lifecycle (ADR-007 remains OPEN).

---

## 8. Precision Policy

1. Authoritative monetary amounts are exact integers in atomic units.
2. **Binary floating point is prohibited** for authoritative monetary amounts and for financial arithmetic that produces authoritative money.
3. Lossy conversions into authoritative money are prohibited.
4. Implicit truncation (dropping fractional atomic remainders without a documented rounding step) is prohibited.
5. Currency precision is **catalog-defined**, not globally hard-coded.

---

## 9. Rounding Policy

### When rounding occurs

Rounding occurs **only at explicit boundaries**, when an exact intermediate result must be expressed as `Money` on a currency’s atomic grid.

Examples of boundaries:

* completing a percentage fee calculation
* applying tax/discount factors
* converting a rational intermediate into atomic units
* mapping a provider-observed amount that requires unit alignment (adapter rules)

### Intermediate arithmetic

* Prefer exact integer / rational arithmetic for intermediates.
* Do **not** round every intermediate step by default.
* Do **not** use binary floating point for intermediates that feed authoritative money.

### Canonical rounding mode

When mapping an exact rational / higher-precision intermediate onto atomic units, TX4’s **default canonical rounding mode** remains:

```text
half-even (round half to even) onto the currency atomic grid
```

This is a **mathematical** rule. It is not defined by any particular language, runtime, or library’s `round` function.

### Atomic-grid interpretation

Let `x` be an exact rational quantity already expressed in **atomic units** of the target currency (possibly with a fractional atomic part). Rounding produces an integer atomic count `r` such that the authoritative `Money.amount = r`.

Authoritative money has no fractional atomic part after rounding. Signed zero is not representable: if the mathematical result is zero, `r = 0` (a single zero; see §7).

### Exact half

Write the absolute value `|x|` as:

```text
|x| = i + f
```

where:

* `i` is the greatest integer ≤ `|x|` (non-negative integer part; unambiguous for `|x| ≥ 0`)
* `f` is the fractional part, `0 ≤ f < 1`

An **exact half** means `f = 1/2` exactly (as a rational), i.e. `|x|` is exactly halfway between `i` and `i + 1`.

### Normative half-even algorithm

Let `s = +1` if `x ≥ 0`, and `s = -1` if `x < 0`.
Let `|x| = i + f` as above.

1. If `f < 1/2`: let `u = i`
2. If `f > 1/2`: let `u = i + 1`
3. If `f = 1/2` (exact half / tie):
   * if `i` is **even**: let `u = i` (tie stays on the even retained atomic digit)
   * if `i` is **odd**: let `u = i + 1` (tie moves to the next even atomic integer)
4. Let `r = s × u`. If `r` would be “negative zero”, set `r = 0`.

`i` even/odd refers to ordinary integer parity (`i mod 2 = 0` ⇒ even).

### Examples (atomic units)

| Intermediate `x` | `i` | `f` | Result `r` | Note |
| --- | --- | --- | --- | --- |
| `2` | 2 | 0 | `2` | already integer |
| `2.4` | 2 | 0.4 | `2` | below half |
| `2.6` | 2 | 0.6 | `3` | above half |
| `2.5` | 2 | 0.5 | `2` | exact half; `i` even → stay |
| `3.5` | 3 | 0.5 | `4` | exact half; `i` odd → to even |
| `-2.4` | 2 | 0.4 | `-2` | sign applied after absolute half-even |
| `-2.6` | 2 | 0.6 | `-3` | |
| `-2.5` | 2 | 0.5 | `-2` | exact half; even `i` → `-2` |
| `-3.5` | 3 | 0.5 | `-4` | exact half; odd `i` → `-4` |
| `0.5` | 0 | 0.5 | `0` | exact half; `i=0` even → `0` |
| `-0.5` | 0 | 0.5 | `0` | maps to single zero (no negative zero) |

### Determinism requirements

* Two independent implementations MUST produce identical `r` for the same exact rational `x`.
* Do not delegate this definition to an implementation library.
* Binary floating-point evaluation of `x` is not an acceptable way to decide half-even ties.

### Explicit overrides

A specific domain rule (fee schedule, tax rule, contractual allocation rule) MAY select a different documented rounding mode **only if**:

1. the mode is explicit and versioned with that rule, and
2. audit records capture which mode was applied, and
3. the override does not silently change previously published results for the same rule version.

Absent an explicit override, half-even as defined above applies.

### Non-goals

This ADR does **not** define every fee, tax, discount, or pricing formula (M08 and related domain work remain future).

---

## 10. Fractional Calculations

Operations such as:

```text
amount × rate
tax
discount
pro-rata charge
```

MUST:

1. treat `amount` as integer atomic units
2. treat `rate` as an exact non-float quantity (for example rational, or integer fixed-point in documented units such as basis points / per-million)
3. compute an exact intermediate (integer or rational)
4. apply the rounding policy (§9) once at the defined boundary to produce `Money`

Never:

* `f64` fee math as authority
* locale-string parsing as authority
* silent truncation of fractional atomic remainders

---

## 11. Allocation Policy

### Purpose

The allocation primitive splits one authoritative `Money` into N recipient parts in the **same currency** without creating or destroying value.

Invariant (mandatory):

```text
sum(parts[i].amount) == source.amount
parts[i].currency == source.currency  for all i
```

This ADR defines the mathematical primitive only. Marketplace payout *business* rules remain out of scope.

### Inputs

* `source`: authoritative `Money`
* `weights`: sequence `w[0] … w[n-1]` of **exact non-negative integers**, in a **stable recipient order** (index order is part of the contract; do not depend on hash-map iteration or unspecified map order)

Equal N-way split is the special case `w[i] = 1` for all `i`.

### Rejection rules (fail closed)

Reject the allocation (do not partially apply) if any of:

1. `n < 1` (no recipients)
2. `source.amount < 0` (**negative source amounts are prohibited** in this primitive — Option B)
3. any `w[i] < 0`
4. any `w[i]` is non-integer / non-exact
5. `W = sum(w[i]) = 0` (all weights zero — including the empty-weight total)
6. currency identity would not be preserved on every part
7. any intermediate or output exceeds the representable range (§13) under checked arithmetic

Negative adjustments, reversals, or signed splits MUST use a separate explicit monetary operation; they MUST NOT silently reuse this allocation primitive. Refund/reversal lifecycle remains OPEN (ADR-007).

### Allowed source amounts

* `source.amount > 0` — allocate as defined below
* `source.amount = 0` — every `parts[i].amount = 0` (same currency); conservation holds

### Normative weighted / pro-rata algorithm

Use **exact integer arithmetic only**. Binary floating point is prohibited.

Let:

```text
S = source.amount          // integer ≥ 0
n = number of recipients   // n ≥ 1
w[i] ≥ 0                   // integer weights
W = sum_{i=0..n-1} w[i]    // W ≥ 1 after rejection rules
```

**Step 1 — exact proportional numerators**

```text
num[i] = S × w[i]          // exact integer; checked overflow → reject
```

**Step 2 — base shares (language-independent division)**

Because `S ≥ 0`, `w[i] ≥ 0`, and `W ≥ 1`, all quantities are non-negative. Define Euclidean (non-negative) quotient and remainder:

```text
base[i] = floor(num[i] / W)     // greatest integer ≤ num[i]/W
rem[i]  = num[i] - base[i] × W  // therefore 0 ≤ rem[i] < W
```

Equivalently: `num[i] = base[i] × W + rem[i]` with the constraints above. Do **not** use toward-zero vs floor distinctions from signed division; this algorithm never divides negative integers.

**Step 3 — conservation gap**

```text
T = sum_{i=0..n-1} base[i]
R = S - T                   // R is an integer, 0 ≤ R < n
```

`R` is the number of leftover atomic units that must still be distributed. Exact divisibility means `R = 0` (and typically all `rem[i] = 0` when each `num[i]` is divisible by `W`, but implementations MUST still compute `R` from `S - T`).

**Step 4 — deterministic remainder distribution (largest remainder; stable ties)**

If `R = 0`, set `parts[i].amount = base[i]` and finish.

If `R > 0`, assign exactly `R` additional `+1` atomic units as follows:

1. Build the list of recipient indices `i` where `w[i] > 0` (recipients with `w[i] = 0` keep `base[i] = 0` and never receive remainder units).
2. Sort that list by:
   * **primary key:** `rem[i]` descending (larger remainder first)
   * **tie-break:** smaller index `i` first (stable recipient order)
3. Give `+1` to the first `R` indices in that sorted list.
4. Set `parts[i].amount = base[i]` plus `1` if selected, else `base[i]`.
5. Set `parts[i].currency = source.currency`.

Recipients with `w[i] = 0` always receive `0` when `S ≥ 0` under this algorithm (including when others receive remainder).

### Single recipient

If `n = 1` and `w[0] ≥ 1`, then `parts[0].amount = S` (and `R = 0`). If `w[0] = 0`, reject via `W = 0`.

### Worked examples

**Equal weights — `S = 100`, `w = [1,1,1]`:**

* `W = 3`
* `num = [100,100,100]`
* `base = [33,33,33]`, `rem = [1,1,1]`
* `T = 99`, `R = 1`
* Ties on `rem`: indices ordered `0,1,2` → first `R=1` gets `+1`
* Result: `[34, 33, 33]` (sum 100)

**Unequal weights — `S = 100`, `w = [1,2,3]`:**

* `W = 6`
* `num = [100,200,300]`
* `base = [16,33,50]`, `rem = [4,2,0]`
* `T = 99`, `R = 1`
* Largest remainder is index `0` (`rem=4`) → `+1`
* Result: `[17, 33, 50]` (sum 100)

**Exact divisibility — `S = 100`, `w = [1,1]`:**

* Result: `[50, 50]`, `R = 0`

**Zero source — `S = 0`, `w = [2,5]`:**

* Result: `[0, 0]`

**Zero weight among others — `S = 10`, `w = [1,0,1]`:**

* `W = 2`
* `num = [10,0,10]`
* `base = [5,0,5]`, `rem = [0,0,0]`
* `R = 0` → `[5, 0, 5]`

### Determinism

Two independent implementations MUST produce identical `parts[i].amount` sequences for the same `S`, currency, and ordered weight vector. No binary float; no unspecified ordering.

### Non-goals

This ADR does **not** define marketplace-specific payout business rules beyond the conservation-preserving primitive above.

---

## 12. Overflow / Underflow Policy

Silent integer wraparound is **prohibited**.

Conceptual requirement:

* Arithmetic that exceeds the representable range MUST **fail checked** (error / rejection), not wrap.
* Implementations MAY use a wider intermediate type for multiply/divide intermediates, then check the final atomic result against the representable range.
* Arbitrary-precision intermediates are allowed for calculation safety; the **durable authoritative `Money` amount** remains a checked integer range (§13).

No Rust crate is selected by this ADR.

---

## 13. Range Policy

* Authoritative `Money.amount` uses a **bounded signed integer range** suitable for exact checked arithmetic and common payment magnitudes.
* Conceptual safety floor: the range MUST be at least as wide as signed 64-bit atomic units for supported currencies, unless a later ADR explicitly narrows/widens with migration rules.
* Values outside the representable range are **rejected** (checked failure), not clamped and not wrapped.
* This ADR does **not** invent commercial product maximums (for example “max order size”); product limits are separate domain/policy decisions layered on top of representation limits.

---

## 14. Serialization Policy

### Canonical machine representation

Across REST/JSON, SDKs, persistence, logs, and audit records, the canonical machine form of authoritative money is:

```text
{
  "amount": "<signed integer string of atomic units>",
  "currency": "<UPPERCASE currency code>"
}
```

Principles:

1. **`amount` is a decimal integer string** (optional leading `-`; no fractional point; no scientific notation; no locale grouping separators).
2. JSON **number** types MUST NOT be the sole authoritative transport for monetary amounts (IEEE/JSON number precision hazards for large magnitudes).
3. Trailing fractional zeros in display strings are irrelevant to authority; authoritative amount has no fractional atomic part.
4. Locale-dependent formatting (`"Rp 25.000,00"`, `"$10.50"`) is **display only** and MUST NOT be parsed as authority without an explicit, lossless parser that yields canonical `Money`.

Persistence technology remains OPEN; whatever store is chosen later MUST preserve exact integer amount + currency without float coercion.

API field naming and OpenAPI schemas remain future contract work under ADR-005 versioning; this ADR freezes the **semantic** serialization requirements, not endpoint shapes.

---

## 15. Display vs Authority

| Layer | May change formatting? | May change value? |
| --- | --- | --- |
| Authoritative `Money` | N/A | Only via explicit financial operations |
| Serialized canonical form | No (canonical) | No |
| Human display | Yes | **No** |

Display formatting MUST NOT alter authoritative value.

---

## 16. Comparison Policy

For `Money` values `a` and `b`:

1. If `a.currency != b.currency` → comparison for equality/order is **undefined as money comparison**; fail closed / require explicit conversion context. Never implicit FX.
2. If currencies match → compare `amount` integers exactly.
3. Zero compares equal only within the same currency (`0 USD` ≠ `0 IDR` as money values).

---

## 17. Currency Conversion (FX) Boundary

* **Implicit FX conversion is prohibited.**
* Any conversion between currencies requires an **explicit conversion context** (rate source identity, rate value, timestamp/version, rounding mode, resulting `Money`).
* FX provider selection, rate feeds, and FX product architecture remain **OPEN** (future ADR/domain work).
* Absence of an explicit conversion context means cross-currency operations MUST fail.

---

## 18. Fees Compatibility

* Fee amounts use the same authoritative `Money` model.
* Fee calculation MUST be deterministic, auditable, and reproducible (Master Spec §10).
* Fee math follows §9–§10 (exact intermediates + explicit rounding boundary).
* Complete fee schedules, attribution rules, and versioning of fee *business* rules remain future domain work (M08); this ADR supplies the monetary substrate only.

---

## 19. Payment Provider Compatibility

* External provider amounts MUST be mapped into TX4 `Money` without silent loss of financial meaning.
* Adapters MUST document unit assumptions (provider minor units vs TX4 atomic units) and currency-code mapping.
* Provider catalogs sometimes diverge from ISO tables for a given code (examples appear in industry docs for certain currencies). TX4’s catalog + explicit adapter mapping rules resolve meaning for TX4 records; disagreement becomes a reconciliation concern under ADR-004.
* This ADR does **not** select Xendit, DOKU, Midtrans, Stripe, or any provider/adapter implementation.

---

## 20. Ledger Compatibility

* TX4 ledger / internal financial records (when present) MUST use the same authoritative `Money` representation.
* ADR-004 preserved: TX4 ledger authority applies to **TX4 internal financial records only**; provider-side balances remain external.
* Ledger account chart, double-entry rules, and schema remain OPEN.

---

## 21. Refunds / Adjustments

* Exact signed `Money` can express adjustments and refunds as recorded by TX4.
* Refund / reversal **lifecycle** remains OPEN (ADR-007).

---

## 22. Reconciliation Compatibility

* Exact integer comparison of same-currency amounts supports reconciliation between TX4 records and external observations once mapped into `Money`.
* Cross-currency reconciliation requires explicit conversion context (§17) or separate per-currency comparison.
* Reconciliation algorithms remain OPEN (ADR-004).

---

## 23. Idempotency Interaction

* Monetary ambiguity (float noise, silent rounding, silent FX) MUST NOT create duplicate or divergent financial effects.
* Idempotency identity and retry semantics remain governed by ADR-004; this ADR removes numeric ambiguity as a cause of effect divergence.

---

## 24. Financial Invariants

TX4 monetary handling MUST uphold:

1. No binary-float authoritative money
2. No silent rounding
3. No silent overflow / wraparound
4. No implicit currency conversion
5. Explicit currency on every monetary value
6. Exact conservation for same-currency allocation (§11)
7. Deterministic arithmetic and rounding
8. Lossless authoritative value
9. Serialization round-trip preserves `amount` and `currency`
10. Display formatting does not mutate authority

---

## 25. Persistence Requirements (Technology OPEN)

Persistence MUST:

* store exact integer atomic amount + currency identity
* forbid float columns as monetary authority
* support exact read/write round-trip

**Not selected:** PostgreSQL, `NUMERIC`, `BIGINT`, `DECIMAL`, SQLx, migrations, or any database product.

---

## 26. API Relationship

* Public API monetary fields MUST obey §14 serialization semantics under ADR-005 versioned contracts (`/v1/...` initially).
* Concrete endpoint schemas remain future API contract work.
* Breaking changes to monetary meaning are breaking API changes under ADR-005.

---

## 27. Rust Suitability (No Crate Selected)

ADR-001 freezes Rust as core runtime. Integer checked arithmetic and explicit types are suitable for this model.

This ADR does **not** select a Rust money crate, decimal crate, or serialization crate (ADR-003 still governs any future dependency).

---

## 28. Security / Correctness Hazards

Reject or fail closed on:

* malformed amount strings (non-integer, empty, scientific notation, locale separators)
* overflow / underflow
* currency missing / unknown / unsupported
* currency confusion (same digits, different currency)
* ambiguous dual encoding (accepting both float and integer without a single authority)

Do not accept binary float payloads as authoritative money.

---

## 29. Auditability

Audit records that include monetary effects MUST store (or unambiguously reference) canonical `amount` + `currency`, and for derived values also enough context to reproduce rounding/allocation (rule version, rounding mode, inputs).

---

## 30. Self-Hosted vs Managed Cloud

The same monetary representation and invariants apply to self-hosted OSS and Managed Cloud. No cloud-only alternate money authority.

OSS / Managed Cloud product boundary remains ADR-008.

---

## 31. Consequences

### Positive

* Exact, deterministic, auditable money
* Aligns with Master Spec §9 candidate and common payment rails
* Clear FX and float prohibitions
* Explicit rounding and allocation conservation
* Safe serialization posture for JSON/API

### Costs / tradeoffs

* Requires a maintained currency catalog (scale/support)
* Non-decimal / special currencies need explicit catalog entries
* JSON string amounts are slightly less ergonomic than bare numbers
* Bounded integer range requires checked failure handling
* Half-even default may differ from some local commercial rounding customs (overridable only via explicit versioned rules)

---

## 32. Non-Decisions

Explicitly **OPEN** / out of scope:

* Persistence technology / database / schema
* HTTP framework / OpenAPI tooling / SDK implementation
* Authentication architecture
* Transaction lifecycle (ADR-007)
* Payment architecture / provider adapters / provider selection
* Settlement algorithms
* Fee business rules / tax rules (beyond monetary substrate)
* FX provider / FX product architecture
* Managed Cloud / OSS boundary details (ADR-008)
* Concrete Rust crates for money/decimal
* Commercial max-order / risk limits

---

## 33. Reconsideration Conditions

Revisit only if objective evidence shows:

1. Integer atomic units cannot represent a required supported currency without loss, even with catalog-defined atomic units
2. A different exact model is required by a FROZEN domain constraint not foreseeable here
3. Serialization string-integer approach is demonstrably incompatible with a FROZEN public API requirement (would require ADR-005-aware migration)

---

## 34. References

* Master Spec §9 Financial Correctness; §10 Fees
* ADR-004 Core Source-of-Truth Boundaries — FROZEN
* ADR-005 API Versioning Strategy — FROZEN
* ADR-001 / ADR-002 / ADR-003 — FROZEN
* ISO 4217 — currency code vocabulary reference (not a complete TX4 business-rule authority)
