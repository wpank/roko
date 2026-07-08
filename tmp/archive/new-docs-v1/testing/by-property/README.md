# By Property — Invariant Catalog

> Every invariant in Roko is a first-class entity. This folder is the authoritative registry of all properties that "must always hold", each with its own page, test reference, and enforcement location.

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## What Belongs Here

A property belongs in this catalog if it:
- Is stated anywhere in the architecture as "must always hold", "is invariant", or "must never".
- Describes a mathematical or semantic relationship (determinism, monotonicity, idempotence, acyclicity, commutativity, bijectivity, boundedness, completeness).
- Is tested by a property-based test, a golden regression test, or an invariant assertion in production code.

Each property gets exactly one page. If the same invariant appears in two subsystems, one page covers the cross-subsystem property and links to both subsystems.

---

## Property Index

### Content Addressing

| Property | File | Tested in |
|---|---|---|
| Content hash determinism | [content-addressing-determinism.md](content-addressing-determinism.md) | `roko-core` (proptest) |
| Content hash collision resistance | [content-hash-collision-resistance.md](content-hash-collision-resistance.md) | `roko-core` (unit) |
| Engram serialization round-trip | [engram-serialization-roundtrip.md](engram-serialization-roundtrip.md) | `roko-core` (proptest) |

### Score and Appraisal

| Property | File | Tested in |
|---|---|---|
| Score axis independence | [score-axis-independence.md](score-axis-independence.md) | `roko-core` (proptest) |
| Score normalization range | [score-normalization-range.md](score-normalization-range.md) | `roko-core` (proptest) |
| Score aggregation monotonicity | [score-aggregation-monotonicity.md](score-aggregation-monotonicity.md) | `roko-core` (unit) |

### Decay

| Property | File | Tested in |
|---|---|---|
| Decay monotone non-increasing | [decay-monotonicity.md](decay-monotonicity.md) | `roko-core` (proptest) |
| Exponential decay approaches zero | [decay-exponential-asymptote.md](decay-exponential-asymptote.md) | `roko-core` (unit) |
| Linear decay reaches zero at lifetime | [decay-linear-terminus.md](decay-linear-terminus.md) | `roko-core` (unit) |

### Lineage and Provenance

| Property | File | Tested in |
|---|---|---|
| Lineage acyclicity | [lineage-acyclicity.md](lineage-acyclicity.md) | `roko-core` (proptest) |
| Provenance chain integrity | [provenance-chain-integrity.md](provenance-chain-integrity.md) | `roko-core` (unit) |

### Gate and Verification

| Property | File | Tested in |
|---|---|---|
| Gate verdict monotonicity | [gate-verdict-monotonicity.md](gate-verdict-monotonicity.md) | `roko-gate` (proptest) |
| Gate verdict idempotence | [gate-verdict-idempotence.md](gate-verdict-idempotence.md) | `roko-gate` (proptest) |
| Pipeline rung ordering | [pipeline-rung-ordering.md](pipeline-rung-ordering.md) | `roko-gate` (unit) |
| Gate adaptive threshold bounds | [gate-adaptive-threshold-bounds.md](gate-adaptive-threshold-bounds.md) | `roko-gate` (unit) |

### Substrate

| Property | File | Tested in |
|---|---|---|
| Substrate write idempotence | [substrate-idempotence.md](substrate-idempotence.md) | `roko-fs` (proptest) |
| Substrate read-after-write consistency | [substrate-read-after-write.md](substrate-read-after-write.md) | `roko-fs` (proptest) |
| GC preserves living Engrams | [substrate-gc-preserves-living.md](substrate-gc-preserves-living.md) | `roko-fs` (unit) |

### HDC and Neuro

| Property | File | Tested in |
|---|---|---|
| HDC bundling commutativity | [hdc-bundling-commutativity.md](hdc-bundling-commutativity.md) | `roko-core` (proptest) |
| HDC binding bijectivity | [hdc-binding-bijectivity.md](hdc-binding-bijectivity.md) | `roko-core` (unit) |
| Neuro knowledge tier monotonicity | [neuro-knowledge-tier-monotonicity.md](neuro-knowledge-tier-monotonicity.md) | `roko-neuro` (unit) |

### Orchestration

| Property | File | Tested in |
|---|---|---|
| Plan DAG acyclicity | [plan-dag-acyclicity.md](plan-dag-acyclicity.md) | `roko-orchestrator` (proptest) |
| Crash recovery consistency | [crash-recovery-consistency.md](crash-recovery-consistency.md) | `roko-orchestrator` (proptest) |
| Event log replay idempotence | [event-log-replay-idempotence.md](event-log-replay-idempotence.md) | `roko-orchestrator` (unit) |

### Agent and Routing

| Property | File | Tested in |
|---|---|---|
| Cascade router fallback ordering | [cascade-router-fallback-ordering.md](cascade-router-fallback-ordering.md) | `roko-agent` (unit) |
| Safety pipeline step ordering | [safety-pipeline-ordering.md](safety-pipeline-ordering.md) | `roko-agent` (unit) |
| Token count determinism | [token-count-determinism.md](token-count-determinism.md) | `roko-agent` (unit) |

### Learning

| Property | File | Tested in |
|---|---|---|
| Bandit score monotonicity | [bandit-score-monotonicity.md](bandit-score-monotonicity.md) | `roko-learn` (unit) |
| C-Factor bounds | [c-factor-bounds.md](c-factor-bounds.md) | `roko-learn` (unit) |

### Daimon

| Property | File | Tested in |
|---|---|---|
| PAD vector bounds | [pad-vector-bounds.md](pad-vector-bounds.md) | `roko-daimon` (unit) |
| Daimon no terminal state | [daimon-no-terminal-state.md](daimon-no-terminal-state.md) | `roko-daimon` (unit) |

### Chain

| Property | File | Tested in |
|---|---|---|
| Soulbound non-transferability | [soulbound-non-transferability.md](soulbound-non-transferability.md) | `roko-chain` (unit) |
| Reputation EMA convergence | [reputation-ema-convergence.md](reputation-ema-convergence.md) | `roko-chain` (unit) |
| Token demurrage rate | [token-demurrage-rate.md](token-demurrage-rate.md) | `roko-chain` (unit) |
| ISFR clearing idempotence | [isfr-clearing-idempotence.md](isfr-clearing-idempotence.md) | `roko-chain` (unit) |

### Prompt and Composition

| Property | File | Tested in |
|---|---|---|
| Prompt layer ordering | [prompt-layer-ordering.md](prompt-layer-ordering.md) | `roko-compose` (unit) |

### Runtime

| Property | File | Tested in |
|---|---|---|
| Cancellation token propagation | [cancellation-token-propagation.md](cancellation-token-propagation.md) | `roko-runtime` (integration) |

---

## Property Count: 33

---

## How to Add a New Property

1. Identify a claim in the architecture docs that uses "must always hold", "invariant", "must never", or a mathematical characterization.
2. Create a new file in this directory named `<property-slug>.md`.
3. Fill in the standard template (see any existing property file).
4. Add a property test in the relevant crate.
5. Add a row to the index table above.

---

## See also

- [../tiers/03-property-tests.md](../tiers/03-property-tests.md) — how to write property tests
- [../by-subsystem/README.md](../by-subsystem/README.md) — subsystem → property mapping
