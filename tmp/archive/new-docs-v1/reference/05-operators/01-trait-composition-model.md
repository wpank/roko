# Trait Composition Model

> How operators compose in Roko: the loop tick, stacking rules, and what "composable"
> actually means in Rust.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Overview](./00-overview.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

A loop tick calls each operator trait once. Operators of the same type can be stacked
(called sequentially on the same data) using wrapper types. The runtime provides
`ScorerStack`, `GatePipeline`, and similar composites for the common case.

---

## The Loop Tick

`loop_tick` is the core of the cognitive loop. A simplified view:

```rust
// source: crates/roko-runtime/src/loop.rs
pub fn loop_tick(
    ctx: &mut LoopContext,          // mutable loop state
    scorers: &[Box<dyn Scorer>],    // stacked scorers (applied in order)
    gates: &[Box<dyn Gate>],        // stacked gates (all must pass)
    router: &dyn Router,            // single router
    composer: &dyn Composer,        // single composer
    policy: &dyn Policy,            // single policy
    substrate: &mut dyn Substrate,  // single substrate
) -> Result<LoopOutcome, LoopError> {
    // SENSE: receive input
    // RECALL: query substrate
    // SCORE: apply all scorers
    let score = scorers.iter()
        .try_fold(Score::default(), |acc, s| s.score(&ctx.engram, acc))?;
    // GATE: all gates must pass
    for gate in gates {
        if gate.evaluate(&ctx.engram, &score)?.is_reject() {
            return Ok(LoopOutcome::Rejected);
        }
    }
    // ROUTE: select action
    let action = router.route(&ctx.engram, &score)?;
    // COMPOSE: build prompt
    let prompt = composer.compose(&ctx, &action)?;
    // ACT: call LLM (external)
    // OBSERVE + STORE + LEARN: outcome → substrate
    Ok(LoopOutcome::Completed(prompt))
}
```
<!-- source: crates/roko-runtime/src/loop.rs -->

---

## Stacking Operators

### Stacking Scorers

Multiple `Scorer` implementations are called in sequence. Each receives the `Engram` and the
accumulated `Score` from the previous scorer:

```rust
// source: crates/roko-core/src/scorer.rs
// Signature that enables stacking:
fn score(&self, engram: &Engram, prior: Score) -> Result<Score, ScorerError>;
```
<!-- source: crates/roko-core/src/scorer.rs -->

The `prior` parameter means each scorer in the chain sees the combined score of all
earlier scorers. The last scorer's output is the final `Score` for the loop tick.

### Stacking Gates

Multiple `Gate` implementations form a pipeline. The loop calls each gate in order; if any
returns `Reject`, the engram is rejected without calling the remaining gates:

```rust
// source: crates/roko-gate/src/lib.rs
// Short-circuit: first Reject ends the pipeline.
for gate in gates {
    match gate.evaluate(engram, score)? {
        Verdict::Pass => continue,
        Verdict::Reject(reason) => return Ok(Verdict::Reject(reason)),
        Verdict::Abstain => continue, // abstaining gates are skipped
    }
}
return Ok(Verdict::Pass);
```
<!-- source: crates/roko-gate/src/lib.rs -->

### Router, Composer, Policy: Single Instance

`Router`, `Composer`, and `Policy` are called once per tick, not stacked. If you need
multiple routing strategies, use `CascadeRouter` (which is itself a single `Router`
implementation that internally cascades strategies). See [Router Semantics](./03-router/02-semantics.md).

---

## Stacking Rules

1. **Scorer stacking**: always additive. Every scorer in the chain runs (no short-circuit).
   The final score is the last scorer's output.
2. **Gate stacking**: short-circuits on first `Reject`. Order matters — cheap gates should
   come first.
3. **Router**: single instance per tick. Use `CascadeRouter` for multi-strategy fallback.
4. **Composer**: single instance per tick. Composition layers are a `Composer`-internal
   concern, not an inter-trait stack.
5. **Policy**: single instance per tick. If multiple policies are needed, wrap in a
   `PolicyComposite`.

---

## Object Safety and Dynamic Dispatch

All operator traits are object-safe (`dyn Trait` works). The runtime holds operators as
`Box<dyn Trait>` and calls them via vtable dispatch. The per-call overhead is ~1 ns
(indirection + branch prediction miss on first call). For scoring, gating, and routing,
this overhead is negligible relative to the actual computation.

---

## See Also

- [Overview](./00-overview.md)
- [Trait × Layer Map](./02-trait-layer-map.md)
- [Scorer — Composition Patterns](./01-scorer/09-composition-patterns.md)
- [Gate — Composition](./02-gate/09-gate-composition.md)

## Open Questions

- Should `loop_tick` be made configurable (plugins for custom steps) or is the fixed 7-step
  structure a hard constraint?
