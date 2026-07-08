# loop_tick() — The Canonical Reference Implementation

> The complete Rust implementation of one cognitive tick.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [All eight stages](00-overview.md), [TickContext](../08-layers/03-L3-harness.md)
**Used by**: [Three Cognitive Speeds](../07-speeds/README.md), all agent implementations
**Last reviewed**: 2026-04-19

---

## TL;DR

`loop_tick()` is a single `async fn` in `roko-agent` that orchestrates the eight
stages in sequence. It is the canonical entry point for one unit of agent cognition.
Everything about the loop's behavior is controlled by the trait objects injected into
`TickContext`. The function itself contains no business logic — it is pure
orchestration.

---

## The Full Implementation

```rust
// source: crates/roko-agent/src/loop/tick.rs

/// Execute one full cognitive tick.
///
/// Returns a `TickResult` describing the outcome of all eight stages.
/// Never panics in release mode. All errors are reported via `TickResult`.
pub async fn loop_tick(ctx: &TickContext) -> TickResult {
    let tick_id  = TickId::new();
    let started  = Instant::now();

    // ── QUERY ───────────────────────────────────────────────────────────────
    let query_spec = ctx.query_builder.build(&ctx.stimulus, &ctx.speed_tier);
    let candidates = match ctx.query_stage.query(&ctx.substrate, &query_spec) {
        Ok(c)  => c,
        Err(e) => {
            ctx.metrics.record_stage_error("query", &e);
            vec![]   // empty; tick continues
        }
    };

    // ── SCORE ───────────────────────────────────────────────────────────────
    let scored = ctx.scorer.score(candidates, &ctx.stimulus, &ctx.scorer_ctx);

    // ── ROUTE ───────────────────────────────────────────────────────────────
    let route = match ctx.router.route(&scored, &ctx.stimulus, &ctx.router_ctx) {
        Ok(r)  => r,
        Err(e) => {
            ctx.metrics.record_stage_error("route", &e);
            return TickResult::aborted(tick_id, started, "route failed", e.into());
        }
    };

    // Check for deferral
    if let RouteTarget::Defer(reason) = &route.target {
        ctx.bus.publish(Pulse::route_uncertain(tick_id, reason.clone())).await;
        return TickResult::deferred(tick_id, started, route);
    }

    // ── COMPOSE ─────────────────────────────────────────────────────────────
    let composed = match ctx.composer.compose(&route, &scored, &ctx.stimulus, &ctx.composer_ctx) {
        Ok(c)  => c,
        Err(e) => {
            ctx.metrics.record_stage_error("compose", &e);
            return TickResult::aborted(tick_id, started, "compose failed", e.into());
        }
    };

    // ── ACT ─────────────────────────────────────────────────────────────────
    let act_output = ctx.act_stage.act(&composed, &ctx.policy, &ctx.budget).await;

    // ── VERIFY ──────────────────────────────────────────────────────────────
    let verify_result = match &act_output {
        Ok(output) => ctx.gate_pipeline.verify(output, &ctx.gate_ctx),
        Err(e) => VerifyResult::skipped_due_to_act_error(e),
    };

    // ── PERSIST ─────────────────────────────────────────────────────────────
    let persist_result = match ctx.persist_stage.persist(
        act_output.as_ref().ok(),
        &verify_result,
        &ctx.persist_ctx,
        &ctx.substrate,
    ) {
        Ok(p)  => p,
        Err(e) => {
            // Persist failure is a critical error — log loudly
            ctx.metrics.record_critical("persist", &e);
            return TickResult::persist_failed(tick_id, started, e.into());
        }
    };

    // ── REACT ────────────────────────────────────────────────────────────────
    let react_result = ctx.react_stage.react(
        &persist_result,
        &verify_result,
        &route,
        &ctx.bus,
        &ctx.scheduler,
    ).await;

    // ── FINALIZE ─────────────────────────────────────────────────────────────
    let elapsed = started.elapsed();
    ctx.metrics.record_tick(tick_id, elapsed, &persist_result, &verify_result);

    TickResult {
        tick_id,
        outcome_id:   persist_result.outcome_id,
        provenance_id: persist_result.provenance_id,
        verify:       verify_result.verdict,
        next_tick_at: react_result.map(|r| r.next_tick_at).unwrap_or_default(),
        elapsed,
    }
}
```

---

## TickContext

`TickContext` is the dependency injection container for the loop. It is built by the
Harness layer (Layer 3) and passed to `loop_tick()`. Every trait object in the context
is swappable at agent-configuration time.

```rust
// source: crates/roko-agent/src/loop/context.rs
pub struct TickContext {
    // Stimulus and speed
    pub stimulus:     Pulse,
    pub speed_tier:   SpeedTier,

    // Trait objects — the pluggable parts
    pub substrate:    Arc<dyn Substrate>,
    pub query_stage:  Arc<dyn QueryStage>,
    pub query_builder: Arc<dyn QueryBuilder>,
    pub scorer:       Arc<dyn Scorer>,
    pub router:       Arc<dyn Router>,
    pub composer:     Arc<dyn Composer>,
    pub act_stage:    Arc<dyn ActStage>,
    pub gate_pipeline: Arc<GatePipeline>,
    pub persist_stage: Arc<dyn PersistStage>,
    pub react_stage:  Arc<dyn ReactStage>,
    pub policy:       Arc<dyn Policy>,
    pub bus:          Arc<dyn Bus>,
    pub scheduler:    Arc<dyn Scheduler>,

    // Contextualized sub-contexts
    pub scorer_ctx:   ScorerContext,
    pub router_ctx:   RouterContext,
    pub composer_ctx: ComposerContext,
    pub gate_ctx:     GateContext,
    pub persist_ctx:  PersistContext,

    // Budget and observability
    pub budget:       TickBudget,
    pub metrics:      Arc<dyn TickMetrics>,
}
```

---

## TickResult

```rust
// source: crates/roko-agent/src/loop/result.rs
pub struct TickResult {
    pub tick_id:      TickId,
    pub outcome_id:   Option<EngramId>,
    pub provenance_id: EngramId,
    pub verify:       Verdict,
    pub next_tick_at: Timestamp,
    pub elapsed:      Duration,
}

impl TickResult {
    pub fn succeeded(&self) -> bool {
        self.outcome_id.is_some() && matches!(self.verify, Verdict::Pass)
    }
    pub fn aborted(tick_id: TickId, started: Instant, reason: &str, err: TickError) -> Self { … }
    pub fn deferred(tick_id: TickId, started: Instant, route: RouteDecision) -> Self { … }
    pub fn persist_failed(tick_id: TickId, started: Instant, err: TickError) -> Self { … }
}
```

---

## Error Handling Philosophy

`loop_tick()` never panics in release mode. Its error contract:

- **QUERY error** → empty candidate set; tick continues.
- **SCORE** → always succeeds (transforms a vec; no I/O).
- **ROUTE error** → abort tick; return `TickResult::aborted`.
- **COMPOSE error** → abort tick; return `TickResult::aborted`.
- **ACT error** → continue to VERIFY with null output.
- **VERIFY** → always produces a result (even for null input).
- **PERSIST error** → `TickResult::persist_failed`; this is critical.
- **REACT error** → logged but not returned; tick outcome is already durable.

The only critical failure is a PERSIST error — if we cannot write to the Substrate,
we cannot guarantee the agent's memory. All other errors are recoverable or
acceptable.

---

## Testing

The canonical test suite for `loop_tick()` is in
`crates/roko-agent/tests/loop_integration.rs`. Each test injects mock trait objects
and verifies the `TickResult`:

```rust
#[tokio::test]
async fn test_query_timeout_continues_tick() {
    let ctx = TickContextBuilder::new()
        .with_query_stage(TimeoutQueryStage::new(Duration::from_millis(100)))
        .build();
    let result = loop_tick(&ctx).await;
    // Tick should complete despite query timeout
    assert!(result.provenance_id.is_valid());
    assert_eq!(result.verify, Verdict::Pass); // no output to fail
}

#[tokio::test]
async fn test_hardFail_does_not_persist_outcome() {
    let ctx = TickContextBuilder::new()
        .with_gate(AlwaysFailGate::new())
        .build();
    let result = loop_tick(&ctx).await;
    assert!(result.outcome_id.is_none());
    assert!(result.provenance_id.is_valid());
}
```

---

## See also

- [Overview](00-overview.md) — the conceptual shape of the loop
- [All eight stage pages](README.md) — per-stage specifications
- [Invariants](12-invariants.md) — postconditions guaranteed by this function
- [Failure Modes](13-failure-modes.md) — what to do when this function returns error variants
- [Performance](14-performance.md) — end-to-end budget breakdown
