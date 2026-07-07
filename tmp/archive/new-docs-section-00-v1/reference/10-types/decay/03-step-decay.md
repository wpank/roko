# Decay — Step Decay

> The Step model: balance drops by a fixed multiplier at the boundary of every epoch.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Used by**: [Tier Matrix](08-tier-matrix.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Step decay divides time into discrete epochs of fixed length. At the end of each epoch the
balance is multiplied by a `step_multiplier` less than 1.0. Within an epoch the weight is
constant — it does not change until the epoch boundary fires. This model suits Engrams
whose relevance has a step-function character: still fully valid during a sprint, then
abruptly less so when the sprint ends.

---

## The Idea

Some knowledge is episodic rather than continuously aging. Consider the minutes from a
planning meeting: they are fully relevant throughout the sprint they apply to, then partially
relevant during the retrospective epoch, then close to zero for most of the future. A
continuous decay model would devalue them during the sprint itself, which is wrong. A step
model holds value flat inside an epoch and applies a sharp discount at each boundary.

The analogy is a fiscal quarter: inside Q3 the balance sheet is current; when Q4 opens it is
already one period stale.

---

## Specification

```rust
<!-- source: crates/roko-core/src/decay.rs -->

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StepDecayParams {
    /// Current balance in [0.0, 1.0].
    pub balance: f64,

    /// Length of one epoch in seconds.
    /// Default: 604_800 (one week).
    pub epoch_secs: u64,

    /// Multiplier applied to balance at each epoch boundary.
    /// Must be in (0.0, 1.0). Default: 0.5 (halve each week).
    pub step_multiplier: f64,
}
```

---

## Weight Function

The balance is **constant within an epoch** and **multiplied by `step_multiplier` at each
epoch boundary**.

```rust
<!-- source: crates/roko-core/src/decay.rs -->

impl StepDecayParams {
    /// Return current weight, applying all full epochs elapsed since `created_at_ms`.
    /// Does NOT mutate self.
    pub fn weight_at(&self, now_ms: i64, created_at_ms: i64) -> f64 {
        let elapsed_secs = ((now_ms - created_at_ms) as f64 / 1_000.0).max(0.0);
        let epochs_elapsed = (elapsed_secs / self.epoch_secs as f64).floor() as u32;
        (self.balance * self.step_multiplier.powi(epochs_elapsed as i32)).max(0.0)
    }

    /// Advance by the number of full epochs elapsed and persist balance.
    pub fn apply_epochs(&mut self, epochs: u32) {
        self.balance *= self.step_multiplier.powi(epochs as i32);
        self.balance = self.balance.max(0.0);
    }
}
```

---

## Concrete Examples

With `epoch_secs = 604_800` (1 week) and `step_multiplier = 0.5`:

| Epochs elapsed | Effective weight |
|---|---|
| 0 (within first week) | 1.000 |
| 1 (after week 1) | 0.500 |
| 2 (after week 2) | 0.250 |
| 3 (after week 3) | 0.125 |
| 7 (after week 7) | 0.0078 |
| 10 (after week 10) | < 0.001 — GC eligible |

With `step_multiplier = 0.8` (gentle step):

| Epochs elapsed | Effective weight |
|---|---|
| 0 | 1.000 |
| 5 | 0.328 |
| 10 | 0.107 |
| 20 | 0.012 |

---

## Default Parameters

```rust
<!-- source: crates/roko-core/src/decay.rs -->

impl Default for StepDecayParams {
    fn default() -> Self {
        StepDecayParams {
            balance: 1.0,
            epoch_secs: 604_800,   // 1 week
            step_multiplier: 0.5,  // halve each week
        }
    }
}
```

---

## Semantics

Unlike Demurrage, Step decay does **not** interact with retrieval. Retrieving a Step-decayed
Engram does not reset or increase its balance. The balance is determined entirely by
elapsed epoch count.

This makes Step decay appropriate for Engrams that are factual records of a past period
rather than knowledge that benefits from reinforcement:

- Sprint backlog items — relevant during their sprint, stale afterward
- Configuration snapshots — valid until the next deploy epoch
- Model checkpoint metadata — current until the next training epoch

If retrieval-based reinforcement is also desired, attach a [Demurrage](01-demurrage.md)
model instead and choose a `idle_tax_per_day` that matches the epoch cadence.

---

## Invariants

1. `balance ∈ [0.0, 1.0]` always.
2. `epoch_secs > 0` — zero-length epochs are undefined.
3. `step_multiplier ∈ (0.0, 1.0)` — 0.0 would zero on first epoch; 1.0 is immortal.
4. `weight_at(t)` is a non-increasing step function with steps at epoch boundaries.
5. `weight_at(t)` is constant within a single epoch.
6. Applying zero epochs is a no-op.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| Balance goes to zero prematurely | `step_multiplier` too small (e.g. 0.1 with a long epoch) | Review epoch × multiplier pairing in the [tier matrix](08-tier-matrix.md) |
| Balance never decays | `step_multiplier` mistakenly set to 1.0 | Validate on construction; reject values ≥ 1.0 |
| GC misses Engram | `weight_at` returns > threshold because epoch has not yet closed | Use GC horizon of `epoch_secs` look-ahead |
| Epoch boundary never fires | Long-running process not calling `apply_epochs` | Substrate compaction job must call this on every tick |

---

## Interactions

- **Substrate compaction**: the compaction job computes `epochs_elapsed` for every
  Step-decayed Engram, calls `apply_epochs`, and marks for GC those with `balance < GC_FLOOR`.
- **Score**: `weight_at` is not the same as `score.confidence`. Score is computed at
  creation; weight modulates how the Engram is ranked at retrieval time.
- **Demurrage vs. Step**: Step decay is epoch-driven and retrieval-independent;
  Demurrage is continuous and retrieval-reinforced. See [overview](00-overview.md)
  for a comparison table.

---

## Open Questions

- Should Step decay support retrieval-based epoch reset (restart epoch counter on access)?
  Not currently planned; use Demurrage if that behaviour is needed.
- Should the epoch boundary be wall-clock aligned (e.g., always fires Monday 00:00 UTC)
  or relative to creation time? Current implementation is relative to creation.

## See Also

- [`00-overview.md`](00-overview.md) — all decay variants compared
- [`01-demurrage.md`](01-demurrage.md) — the primary decay model
- [`08-tier-matrix.md`](08-tier-matrix.md) — which Engram kinds use Step decay by default
