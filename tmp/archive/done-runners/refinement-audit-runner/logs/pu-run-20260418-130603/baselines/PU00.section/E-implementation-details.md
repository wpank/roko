# E — Implementation Details (Docs 18-22)

Audit-aligned parity notes for `docs/00-architecture/18-decay-tier-matrix.md` through
`22-error-handling-recovery.md`.

This arc needs strict separation between what ships, what is guidance-only, and what is still a
future-state design. Without that split, the docs read like speculative abstractions are blocked
implementation work instead of proposal material.

---

## Baseline Facts

- 36 workspace members
- 322,088 Rust LOC
- `roko-learn`: 42 modules, 35,847 LOC
- audit test baseline: 3,761 tests

These facts matter because earlier parity wording simultaneously understated the size of the repo
and overstated the maturity of speculative mechanisms.

Use the full `36 workspace members / 322,088 Rust LOC` phrasing when correcting stale scale claims.

## Shipped Today

| Doc | Status | Current truth |
|-----|--------|---------------|
| 18 — Decay x Knowledge Tier Matrix | KEEP + REWRITE | the code already uses decay behavior plus knowledge tiers |
| 20 — Configuration Schema | KEEP | configuration is real and substantial |
| 22 — Error Handling / Recovery | KEEP + NARROW | retry policy and circuit-breaker machinery are real |

## Partial / Guidance-Only

| Doc | Status | Current truth |
|-----|--------|---------------|
| 18 — Decay x Knowledge Tier Matrix | REWRITE | the polished 4x4 synthesis reads ahead of implementation |
| 21 — Performance / Numerical Stability | NARROW | useful guidance exists, but the repo does not prove full enforcement |
| 22 — Error Handling / Recovery | KEEP + NARROW | fallback and recovery behavior exists, but the full graceful-degradation ladder reads more like a target contract |

Required wording for this section:

- `current code uses decay plus knowledge tiers`
- `the docs describe a target-state unification of those mechanisms`
- `guidance is ahead of enforcement`

Avoid turning these into implementation-gap claims unless the code actually demonstrates the
promised contract.

## Planned / Deferred

| Doc | Status | Current truth |
|-----|--------|---------------|
| 19 — Compositional Kinds | DEFER | `Kind::Compound` is a plausible extension, not a present-tense parity requirement |

Additional factual correction:

- `Demurrage` remains a documentation concept with zero production code.

That means neither the full demurrage matrix nor compound-kind support should be written as though
batch `00` merely needs to finish them.

This section should read as an implementation-scale correction, not as a promise that the target
mechanics from docs `18-22` are already enforced in code.

## Wording Rules

Prefer:

- `current code uses decay plus knowledge tiers`
- `target-state demurrage model`
- `planned compound kind support`
- `guidance is ahead of enforcement`

Avoid:

- `demurrage is the current memory model`
- `compound kinds are already assumed by the runtime`
- `numerical-stability guarantees` unless code actually enforces them
