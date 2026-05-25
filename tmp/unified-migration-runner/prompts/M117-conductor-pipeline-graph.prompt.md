# M117 — Conductor Pipeline Graph and Route Cell

## Objective
Compose the 10 watcher Verify Cells into a Pipeline Graph with a terminal Route Cell that selects the intervention. The Route Cell maps worst-severity `WatcherVerdict` (from M116) to a `ConductorDecision` (Continue/Restart/Fail) and carries `Remediation` hints. This adds a declarative pipeline alongside the existing imperative Conductor loop.

## Scope
- Crates: `roko-conductor`
- Files:
  - New: `crates/roko-conductor/src/pipeline.rs`
  - `crates/roko-conductor/src/conductor.rs` (existing `Conductor` struct with `watchers: Vec<Box<dyn React>>`, `policy: Box<dyn InterventionPolicy>`)
  - `crates/roko-conductor/src/interventions.rs` (existing: `InterventionPolicy` trait, `WatcherOutput`, `Severity`, `WorstSeverityPolicy`; plus M116 additions: `WatcherVerdict`, `WatcherCell`, `Remediation`)
  - `crates/roko-conductor/src/lib.rs` (re-exports)
- Depth doc: `tmp/unified-depth/07-agent-runtime/14-conductor-as-verify-pipeline.md`

## Existing types reference

The `ConductorDecision` type lives in `roko-core`:
```rust
// roko_core::ConductorDecision — has ::cont(), ::restart(watcher, reason), ::fail(watcher, kind)
```

The `Conductor` struct (from `conductor.rs`):
```rust
pub struct Conductor {
    watchers: Vec<Box<dyn React>>,
    policy: Box<dyn InterventionPolicy>,
    circuit_breaker: CircuitBreaker,
    routing_bias: Mutex<RoutingBias>,
    provider_health: Option<Arc<ProviderHealthTracker>>,
    threshold_learner: Mutex<ThresholdLearner>,
    pattern_detector: Mutex<PatternDetector>,
    last_compound_patterns: Mutex<Vec<CompoundPattern>>,
}
```

The `InterventionPolicy` trait:
```rust
pub trait InterventionPolicy: Send + Sync {
    fn evaluate(&self, outputs: &[WatcherOutput], ctx: &Context) -> ConductorDecision;
    fn name(&self) -> &str;
}
```

## Steps
1. Discover existing types and patterns:
   ```bash
   grep -rn 'Pipeline\|ConductorPipeline' crates/roko-conductor/src/ --include='*.rs' | head -10
   grep -rn 'pub.*ConductorDecision' crates/roko-core/src/ --include='*.rs' | head -5
   grep -rn 'WatcherCell\|WatcherVerdict' crates/roko-conductor/src/ --include='*.rs' | head -10
   grep -rn 'fn evaluate.*WatcherOutput' crates/roko-conductor/src/interventions.rs | head -5
   ```

2. Create `crates/roko-conductor/src/pipeline.rs` with:
   - `ConductorPipeline` struct that holds a `Vec<Box<dyn WatcherCell>>` (the 10 watchers from M116)
   - An `evaluate(&self, stream: &[Engram], ctx: &Context) -> PipelineResult` method that:
     a. Runs each watcher sequentially via `WatcherCell::evaluate()`, collecting `WatcherVerdict` values
     b. Selects the worst-severity `WatcherVerdict`
     c. Passes it through a `DecisionRouter` that maps to a `ConductorDecision`
   - A `PipelineResult` struct:
     ```rust
     #[derive(Debug, Clone)]
     pub struct PipelineResult {
         pub decision: ConductorDecision,
         pub verdicts: Vec<WatcherVerdict>,
         pub worst_severity: Severity,
         pub remediation: Option<Remediation>,
     }
     ```

3. Implement the Route Cell logic as a `DecisionRouter`:
   ```rust
   pub struct DecisionRouter {
       policy: Box<dyn InterventionPolicy>,
   }
   impl DecisionRouter {
       pub fn route(&self, verdicts: &[WatcherVerdict], ctx: &Context) -> ConductorDecision {
           // Convert WatcherVerdicts to WatcherOutputs for policy compatibility
           let outputs: Vec<WatcherOutput> = verdicts.iter()
               .filter(|v| !v.passed)
               .map(|v| {
                   let mut out = WatcherOutput::new(&v.watcher, v.severity, &v.description);
                   if let Some(m) = v.metric { out = out.with_metric(m); }
                   out
               })
               .collect();
           self.policy.evaluate(&outputs, ctx)
       }
   }
   ```

4. Add a `ConductorPipeline::default()` constructor that includes all 10 watchers in the standard order (same order as `default_watchers()` in conductor.rs).

5. Update `conductor.rs` to optionally use the pipeline path alongside the existing direct-evaluation path. Add a `pipeline: Option<ConductorPipeline>` field.

6. Add `pipeline` module declaration to `crates/roko-conductor/src/lib.rs` and re-export `ConductorPipeline`, `PipelineResult`, `DecisionRouter`.

7. Add tests:
   - Pipeline with all-passing watchers returns `ConductorDecision::cont()`
   - Pipeline with one Critical watcher returns `ConductorDecision::fail(..)`
   - Pipeline with mixed severities returns the worst-severity decision

## Verification
```bash
cargo check -p roko-conductor
cargo clippy -p roko-conductor --no-deps -- -D warnings
cargo test -p roko-conductor -- pipeline
```

## What NOT to do
- Do NOT remove the existing Conductor `React` evaluation path — keep both paths active
- Do NOT add TOML-based Graph configuration yet — that depends on M038
- Do NOT wire Loop feedback edges — that depends on M035 (Observe protocol)
- Do NOT add Bus Pulse publication from the pipeline
- Do NOT confuse `WatcherVerdict` (conductor) with `Verdict` (gate in roko-core) — they are separate types
