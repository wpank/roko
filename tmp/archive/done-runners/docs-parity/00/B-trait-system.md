# B — Trait System (Docs 06-08)

Post-audit parity notes for `docs/00-architecture/06-synapse-traits.md` through
`08-scorer-gate-router-composer-policy.md`.

The core correction here is restraint. The six live traits stay central. The audit rejected the
idea that topic `00` should imply a second medium, a seventh trait, and a generalized operator
surface that does not exist in code.

---

## Current Runtime Truth

| Item | Status | Current truth |
|------|--------|---------------|
| `06-synapse-traits.md` | `keep` | the live trait spine is still `Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, and `Policy` |
| `07-substrate-trait.md` | `keep` + `narrow` | the durable storage seam is real and important |
| `07b-bus-transport-fabric.md` | `planned` | useful transport design note, not proof of a shipped kernel trait |
| `08-scorer-gate-router-composer-policy.md` | `narrow` | real runtime debt is naming drift and boundary bypass, not missing future abstractions |

## REF04 Posture

`REF04` is **narrowed, with operator generalization deferred**.

What parity should say:

- `Datum` has 0 production LOC.
- operator-wide signature generalization is not the current runtime contract.
- a generic `Bus<E>` trait may still be worth documenting as future cleanup.
- the smallest believable future change is targeted surface cleanup where the code actually needs
  it, not a two-medium rewrite of every trait.

## What Stays Present Tense

- the six live traits
- the fact that routing, composition, and gating exist in code today
- the fact that some higher-level implementations bypass or blur the ideal trait boundary

Those are useful parity findings because they describe current architecture debt without inventing
new kernel nouns to explain it.

## What Must Stay Planned Or Deferred

- `Bus` is not a shipped seventh trait.
- `Pulse` is not a live payload type for the trait system.
- `Datum` is not a live operator input surface.
- operator generalization is deferred.
- `HdcSubstrate` and `ChainSubstrate` remain target-state language until code exists.

## Rewrite Bias For Docs 06-08

Prefer:

- `six live traits`
- `current trait-boundary debt`
- `planned generic bus trait`
- `deferred operator generalization`

Avoid:

- `Bus is already a core trait`
- `Datum-aware operators are the current contract`
- `topic 00 owes a trait rewrite`

## Batch-00 Boundary

For docs `06-08`, parity work is limited to:

1. keeping the six-trait story accurate,
2. naming current boundary debt plainly,
3. labeling Bus/Pulse/Datum material as planned rather than implied runtime fact.
