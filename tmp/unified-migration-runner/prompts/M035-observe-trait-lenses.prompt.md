# M035 — Define Observe trait and implement builtin Lenses

## Objective
Define the `Observe` trait in roko-core as a formal protocol for telemetry observation. Then implement the 10 builtin Lenses (AgentLens, PlanLens, VerifyLens, RouteLens, MemoryLens, CostLens, HealthLens, ErrorLens, ThroughputLens, DreamLens) in roko-conductor, each producing structured observation Signals.

## Scope
- Crates: `roko-core`, `roko-conductor`
- Files:
  - `crates/roko-core/src/traits.rs` (Observe trait definition)
  - New: `crates/roko-conductor/src/lenses/` (directory with per-lens modules)
  - `crates/roko-conductor/src/lib.rs` (module declaration)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.10
- Spec ref: `tmp/unified/09-TELEMETRY.md` §2 (Observe protocol), §5 (Builtin Lenses)
- Architecture ref: `tmp/architecture/16-observability.md`

## Steps
1. Check if Observe-related types already exist:
   ```bash
   grep -rn 'Observe\|Lens\|LensScope\|ObservableEvent' crates/roko-core/src/ --include='*.rs' | head -10
   grep -rn 'Observe\|Lens' crates/roko-conductor/src/ --include='*.rs' | head -10
   ```

2. Define the Observe trait in `crates/roko-core/src/traits.rs`:
   ```rust
   /// Scope at which a Lens observes the system.
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
   pub enum LensScope {
       /// Observes a single Cell's execution.
       Cell,
       /// Observes a Graph (plan) execution.
       Graph,
       /// Observes an Agent's lifecycle.
       Agent,
       /// Observes a Space (workspace).
       Space,
       /// Observes the entire system.
       Global,
   }

   /// Events a Lens can observe.
   #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
   #[non_exhaustive]
   pub enum ObservableEventKind {
       SignalCreated,
       SignalDecayed,
       CellStarted,
       CellCompleted,
       CellFailed,
       GraphStarted,
       GraphCompleted,
       AgentStarted,
       AgentStopped,
       AgentHeartbeat,
       VerifyPassed,
       VerifyFailed,
       RouteSelected,
       DreamCycleCompleted,
   }

   /// Protocol for observing system state.
   ///
   /// Lenses implement this trait to produce structured observation Signals
   /// from the runtime. See: tmp/unified/09-TELEMETRY.md §2.
   pub trait Observe: Send + Sync {
       /// Collect current observations as Signals.
       fn observe(&self, ctx: &Context) -> Vec<Engram>;

       /// What kinds of events this Lens subscribes to.
       fn observes(&self) -> &[ObservableEventKind];

       /// The scope at which this Lens operates.
       fn scope(&self) -> LensScope;

       /// Human-readable name for this Lens.
       fn name(&self) -> &str;
   }
   ```

3. Create `crates/roko-conductor/src/lenses/mod.rs` and individual lens files:
   ```bash
   mkdir -p crates/roko-conductor/src/lenses/
   ```

4. Implement each Lens (start with stubs that return empty/mock data, then fill in as data sources are available):

   **AgentLens** — turns, tokens, cost, latency per active agent
   **PlanLens** — tasks completed/failed/pending for active plans
   **VerifyLens** — pass rates, threshold drift across gate pipeline
   **RouteLens** — model distribution, cost breakdown per model
   **MemoryLens** — Signal counts by kind, tier distribution, decay stats
   **CostLens** — real-time USD cost across all providers
   **HealthLens** — provider health, error rates, circuit breaker state
   **ErrorLens** — recent errors by category, frequency, trend
   **ThroughputLens** — requests/sec, latency percentiles
   **DreamLens** — last dream cycle stats, insights generated

5. Each Lens should:
   - Implement `Observe` trait
   - Return observations as Vec<Engram> with Kind::Metric
   - Be constructible from shared state (Arc<AppState> or similar)

6. Export all Lenses from `crates/roko-conductor/src/lenses/mod.rs`.

7. Add basic tests for each Lens:
   ```rust
   #[test]
   fn agent_lens_produces_observations() {
       let lens = AgentLens::new(mock_state());
       let obs = lens.observe(&Context::default());
       assert!(!obs.is_empty());
   }
   ```

## Verification
```bash
cargo check -p roko-core
cargo check -p roko-conductor
cargo clippy -p roko-conductor --no-deps -- -D warnings
cargo test -p roko-conductor -- lens
# Verify trait is exported:
grep 'Observe\|LensScope' crates/roko-core/src/lib.rs
```

## What NOT to do
- Do NOT wire Lenses into TUI/HTTP yet — that's a follow-up
- Do NOT make Lenses depend on concrete implementations — accept trait objects or shared state
- Do NOT block on missing data sources — return empty observations when data is unavailable
- Do NOT add external monitoring dependencies (prometheus, etc.) — Lenses are internal
