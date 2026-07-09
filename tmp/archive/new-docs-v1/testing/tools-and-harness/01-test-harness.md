# Test Harness

> The `roko-test` harness: `TestContext`, `IntegrationContext`, seeded clocks, seeded RNG, and hermetic I/O.

**Status**: Shipping
**Crate**: `roko-test`
**Depends on**: all test tiers
**Last reviewed**: 2026-04-19

---

## TL;DR

`roko-test` is a dedicated test infrastructure crate that provides deterministic, hermetic test environments. Every test that needs filesystem access, clocks, or RNG creates a context object that controls all sources of non-determinism.

---

## TestContext (Unit Tests)

```rust
pub struct TestContext {
    pub dir: TempDir,              // per-test temp directory
    pub clock: FakeClock,          // synthetic clock (does not advance unless told to)
    pub rng: StdRng,               // seeded deterministic RNG
}

impl TestContext {
    pub fn new() -> Self { ... }   // uses seeded defaults
    pub fn new_with_seed(seed: u64) -> Self { ... }

    pub fn file_substrate(&self) -> FileSubstrate { ... }  // JSONL in temp dir
    pub fn engram_fixture(&self) -> Engram { ... }         // deterministic test Engram
    pub fn engram_with_decay_score(&self, score: f32) -> Engram { ... }
    pub fn advance_clock(&mut self, duration: Duration) { ... }
}
```

Usage:
```rust
#[test]
fn example_unit_test() {
    let ctx = TestContext::new();
    let substrate = ctx.file_substrate();
    let e = ctx.engram_fixture();
    substrate.write(&e).unwrap();
    assert!(substrate.read(e.id()).unwrap().is_some());
}
```

---

## IntegrationContext (Integration Tests)

```rust
pub struct IntegrationContext {
    pub ctx: TestContext,
    pub tape: TapeReplayer,            // LLM response replay
    pub orchestrator: OrchestratorHandle,
    pub gate: GatePipelineHandle,
    pub learn: LearningHandle,
}

impl IntegrationContext {
    pub fn builder() -> IntegrationContextBuilder { ... }
}

pub struct IntegrationContextBuilder {
    pub fn with_tape(self, path: &str) -> Self { ... }
    pub fn with_gate_config(self, config: GateConfig) -> Self { ... }
    pub fn with_plan(self, plan: &Plan) -> Self { ... }
    pub async fn build(self) -> IntegrationContext { ... }
}
```

---

## E2EEnvironment (End-to-End Tests)

```rust
pub struct E2EEnvironment {
    ctx: IntegrationContext,
    cli: CliHandle,
}

impl E2EEnvironment {
    pub fn builder() -> E2EEnvironmentBuilder { ... }
    pub async fn run_to_completion(&mut self, plan_id: PlanId) -> Result<()> { ... }
    pub async fn run_to_crash(&mut self, plan_id: PlanId) { ... }
}
```

---

## FakeClock

The `FakeClock` implements the `Clock` trait and provides deterministic time control:

```rust
let ctx = TestContext::new();
// Clock starts at 2026-01-01T00:00:00Z (seeded)

ctx.advance_clock(Duration::from_secs(3600)); // advance 1 hour
// Now: 2026-01-01T01:00:00Z

let now = ctx.clock.now(); // 2026-01-01T01:00:00Z
```

All time-dependent code uses the `Clock` trait (injected via `roko-test` in tests, `SystemClock` in production). Never use `std::time::Instant::now()` in testable code.

---

## Seeded RNG

All RNG in tests must be created via `TestContext::rng` or `StdRng::seed_from_u64(n)`. Never use `rand::thread_rng()` in test code — it is non-deterministic.

---

## Invariants

- `TestContext::new()` always uses the same default seed (42). Tests that need different seeds use `TestContext::new_with_seed(n)`.
- Temp directories are cleaned up when the context is dropped.
- The `FakeClock` never advances automatically; it only advances on explicit `advance_clock` calls.

---

## See also

- [02-mock-llms.md](02-mock-llms.md) — LLM tape replay
- [03-fixture-library.md](03-fixture-library.md) — proptest strategies and fixture factories
