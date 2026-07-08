# Score — Extended Axes

> The 3 extended axes: precision, salience, coherence. Optional; computed by specialized Scorers.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Stable Axes](01-axes-stable.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Extended axes are `Option<f64>` — absent unless a specialized Scorer computes them.
They do not enter the default effective score formula. A Gate or Scorer that needs
one of them checks `score.precision.unwrap_or(0.5)` with a graceful default.

---

## Axis 5: `precision` — Specificity and Accuracy

**What it measures:** How specific and accurate is the claim? Precision distinguishes
vague claims ("something might go wrong") from precise ones ("the timeout is 30 seconds
with a 95% confidence interval [28.1s, 31.8s]").

**Semantic range:**
- 1.0 = Exact, verifiable, quantified claim
- 0.5 = Semi-specific claim with some qualification
- 0.0 = Vague, unverifiable, or vacuous claim

**Typical scorers:** Factual verification gates, code compilation checkers, schema validators.

**When absent:** `precision = None` means "not evaluated." Default in computation: 0.5.

---

## Axis 6: `salience` — Task Relevance

**What it measures:** How relevant is this Engram to the current task or query context?
Salience is context-dependent — the same Engram may be salient for one task and
irrelevant for another.

**Semantic range:**
- 1.0 = Directly answers the current query or is critical to the current task
- 0.5 = Tangentially related
- 0.0 = Completely irrelevant

**Typical scorers:** The retrieval Scorer that runs at context assembly time, using
the current task's HDC fingerprint to compute similarity to the Engram's fingerprint.

**When absent:** `salience = None` means "not evaluated in this context." Never stored
permanently — salience is computed at retrieval time and discarded.

**Important:** Salience should never be stored in the Substrate. It changes with every
query context. Only `confidence`, `novelty`, `utility`, and `reputation` are stored;
salience is computed ephemerally.

<!-- ADDED: clarification that salience is ephemeral — implied by context-dependence but not explicit in source. -->

---

## Axis 7: `coherence` — Internal Consistency

**What it measures:** How internally consistent is the content of the Engram? For a
multi-claim document, coherence measures whether the claims are mutually consistent.
For a code snippet, coherence measures whether the code is self-consistent (imports
match uses, variable names are consistent, etc.).

**Semantic range:**
- 1.0 = Fully internally consistent; no contradictions
- 0.5 = Some inconsistencies, but the main claim is coherent
- 0.0 = Deeply self-contradictory

**Typical scorers:** LLM-based coherence checkers, formal constraint validators.

**When absent:** `coherence = None` means "not evaluated."

---

## Using Extended Axes in a Gate

```rust
<!-- source: crates/roko-core/src/gate.rs -->

fn check_precision_threshold(engram: &Engram, threshold: f64) -> bool {
    engram.score.precision.unwrap_or(0.5) >= threshold
}
```

A Gate that requires a minimum precision can simply call `check_precision_threshold`.
If precision has not been scored, the fallback is 0.5 (neutral).

---

## Invariants

1. Extended axes are `Option<f64>` — `None` means "not scored"
2. When present, extended axis values are in [0.0, 1.0]
3. `salience` should not be stored in the Substrate (ephemeral, context-dependent)
4. Extended axes do not enter the default effective score formula

---

## See Also

- [`01-axes-stable.md`](01-axes-stable.md) — the 4 stable axes
- [`03-arithmetic.md`](03-arithmetic.md) — the effective score formula
- [`08-rationale.md`](08-rationale.md) — why these 3 extended axes
