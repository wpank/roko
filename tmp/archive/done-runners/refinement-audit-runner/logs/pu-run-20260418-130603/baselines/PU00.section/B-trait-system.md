# B — Trait System (Docs 06-08)

Audit-aligned parity read of `docs/00-architecture/06-synapse-traits.md` through
`08-scorer-gate-router-composer-policy.md`.

The stable correction here is factual: the runtime still has six shipped architecture traits.
Later Bus/Pulse/Datum material is proposal posture, not a hidden seventh-trait backlog.

---

## Shipped Today

| Item | Status | Current truth |
|------|--------|---------------|
| 06 — Six Synapse Traits | SHIPPED | the architecture spine is still `Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, and `Policy` |
| Gate pipeline surface | MOSTLY SHIPPED | the workspace has a broad, real gate surface beyond the original doc baseline |
| Policy behavior | SHIPPED with naming drift | policy behavior exists, but code names and doc names do not always line up cleanly |

These sections should read as present-tense runtime facts. None of them should imply that later
Bus-era rewrite material is already part of the trait contract.

## Partial / Live Parity Debt

| Item | Status | What is true now | What to avoid |
|------|--------|------------------|---------------|
| 07 — Substrate trait deep dive | PARTIAL | memory and file-backed implementations are real | treating HDC or chain substrates as nearly shipped |
| Scoring inventory | PARTIAL | scoring ships, but the doc inventory does not match the code inventory | inflating this into a major missing runtime system |
| Router boundary | PARTIAL / architecture violation | sophisticated routers exist, but the trait boundary is bypassed in places | burying the live trait-bypass issue under future Bus ideas |
| Composer boundary | PARTIAL / architecture violation | composition is shipped, but some prompt-building logic still sits awkwardly relative to the trait story | treating future `Datum` operators as the relevant fix |

Router and composer notes are real current-state findings. They should stay visible as current
architecture debt rather than being inflated into "missing future systems."

## REF04 Audit Verdict

`REF04` should be treated as **deferred**.

- Operator generalization around `Datum` is not the current runtime shape.
- `Datum` has 0 production LOC.
- The audit recommendation is to avoid doubling every trait surface for a speculative abstraction.
- If code work happens later, the smallest plausible change is a focused signature adjustment where
  it is actually needed, not a full two-medium trait rewrite.
- In parity terms, keep the six live traits in present tense and treat any future `Bus<E>` trait
  as a planned generic cleanup rather than a missing seventh live trait.

## Planned / Deferred

- `Bus` is not a seventh shipped trait.
- `Pulse` and `Datum` are not live runtime primitives in this section.
- The live runtime transport is still a narrow utility with exactly two live RokoEvent variants.
- Operator generalization is deferred.
- `HdcSubstrate` and `ChainSubstrate` should remain explicitly deferred until code exists.

This separation matters because otherwise the docs read like batch `00` is supposed to implement a
rewrite, when the actual parity work is mostly naming, inventory, and boundary accuracy.

## Doc File Coverage

| Doc file | Post-audit read |
|----------|-----------------|
| `06-synapse-traits.md` | Six traits overview; all six are live and can stay in present tense |
| `07-substrate-trait.md` | Durable storage fabric is real; future substrate families stay planned |
| `07b-bus-transport-fabric.md` | Useful design note for a future kernel bus, not proof of a shipped architecture trait |
| `08-scorer-gate-router-composer-policy.md` | Real parity debt is naming drift and boundary bypass, not missing future types |

This arc should stay centered on the shipped six-trait spine even when later docs speculate about
Bus, Pulse, or Datum.

## Editing Bias For This Arc

Prefer:

- `six live traits`
- `current trait boundary debt`
- `planned generic bus trait`
- `deferred operator generalization`

Avoid:

- `Bus is already the seventh trait`
- `Datum-aware operators are the current contract`
- `operator generalization is the batch-00 blocker`
