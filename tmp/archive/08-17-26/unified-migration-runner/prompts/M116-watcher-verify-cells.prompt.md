# M116 — Refactor conductor watchers as Verify Cells

## Objective
Refactor the 10 conductor watchers from `React` impls into Verify Cells that conform to the kernel's Verify protocol. Each watcher keeps its detection logic unchanged but gains a typed `WatcherVerdict` return (separate from the gate `Verdict` in roko-core), numeric metric field for threshold learning, and a Remediation hint for the downstream Route Cell.

## Scope
- Crates: `roko-conductor`
- Files:
  - `crates/roko-conductor/src/watchers/*.rs` (all 10 watcher files)
  - `crates/roko-conductor/src/watchers/mod.rs` (re-exports)
  - `crates/roko-conductor/src/conductor.rs` (composite conductor, uses `Conductor` struct with `Vec<Box<dyn React>>` watchers)
  - `crates/roko-conductor/src/interventions.rs` (existing: `Severity`, `WatcherOutput`, `InterventionPolicy` trait)
  - `crates/roko-conductor/src/lib.rs` (re-exports)
- Depth doc: `tmp/unified-depth/07-agent-runtime/14-conductor-as-verify-pipeline.md`

## Existing types reference

The kernel `Verdict` already exists at `crates/roko-core/src/verdict.rs`:
```rust
pub struct Verdict {
    pub passed: bool,
    pub reason: String,
    pub gate: String,
    pub score: f32,
    pub detail: Option<String>,
    pub test_count: Option<TestCount>,
    pub error_digest: Option<String>,
    pub duration_ms: u64,
}
```
This is a **gate** verdict. Do NOT modify it for conductor watchers. Instead, create a separate `WatcherVerdict` in roko-conductor.

The `Verify` trait is async (`crates/roko-core/src/traits.rs`):
```rust
#[async_trait]
pub trait Verify: Send + Sync {
    async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict;
    fn name(&self) -> &str;
}
```

The watchers currently implement `React` (synchronous):
```rust
pub trait React: Send + Sync {
    fn decide(&self, stream: &[Engram], ctx: &Context) -> Vec<Engram>;
    fn name(&self) -> &str;
}
```

The existing `WatcherOutput` already has a metric field:
```rust
pub struct WatcherOutput {
    pub watcher: String,
    pub severity: Severity,
    pub description: String,
    pub metric: Option<f64>,
}
```

## Steps
1. Discover current watcher implementations:
   ```bash
   # All watchers implement React::decide(&self, &[Engram], &Context) -> Vec<Engram>
   grep -rn 'impl React for' crates/roko-conductor/src/watchers/ --include='*.rs'
   # Current WatcherOutput struct (already has metric: Option<f64>)
   grep -rn 'pub struct WatcherOutput' crates/roko-conductor/src/interventions.rs
   # Existing Severity enum (Info, Warning, Critical)
   grep -rn 'pub enum Severity' crates/roko-conductor/src/interventions.rs
   ```

2. Add a `WatcherVerdict` struct and `Remediation` enum to `crates/roko-conductor/src/interventions.rs`:
   ```rust
   /// Conductor-specific verdict from a watcher evaluation.
   /// Distinct from roko_core::Verdict which is for gate verdicts.
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub struct WatcherVerdict {
       /// Name of the watcher that produced this verdict.
       pub watcher: String,
       /// Whether the check passed (no anomaly detected).
       pub passed: bool,
       /// Severity if an anomaly was detected.
       pub severity: Severity,
       /// Human-readable description.
       pub description: String,
       /// Numeric metric for threshold learning (e.g. count, ratio).
       pub metric: Option<f64>,
       /// Suggested remediation for the downstream route cell.
       pub remediation: Option<Remediation>,
   }

   /// Remediation hint from a watcher to the conductor pipeline.
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub enum Remediation {
       LogOnly,
       Restart { context: String },
       Abort { reason: String },
       Escalate { to_tier: u32 },
       Cooldown { factor: f64 },
       Explore { budget_multiplier: f64 },
   }
   ```

3. Add a `WatcherCell` trait to `crates/roko-conductor/src/interventions.rs`:
   ```rust
   /// Synchronous verification trait for conductor watchers.
   /// Parallel to React but returns structured WatcherVerdict instead of Vec<Engram>.
   pub trait WatcherCell: Send + Sync {
       fn evaluate(&self, stream: &[Engram], ctx: &Context) -> WatcherVerdict;
       fn name(&self) -> &str;
   }
   ```

4. For each of the 10 watchers in `crates/roko-conductor/src/watchers/`, implement `WatcherCell` alongside the existing `React` impl. Reuse the existing detection logic, converting the output. Map severity levels:
   - `GhostTurnWatcher`: metric = consecutive ghost turn count, remediation = `Restart`
   - `CompileFailRepeatWatcher`: metric = consecutive compile failures, remediation = `Escalate { to_tier: 2 }`
   - `CostOverrunWatcher`: metric = cost/budget ratio, remediation = `Abort`
   - `IterationLoopWatcher`: metric = loop cycle count, remediation = `Restart`
   - `ReviewLoopWatcher`: metric = review cycle count, remediation = `Abort`
   - `SpecDriftWatcher`: metric = drift ratio, remediation = `Restart`
   - `StuckPatternWatcher`: metric = repeat count, remediation = `Restart`
   - `TestFailureBudgetWatcher`: metric = failure count increase, remediation = `Restart`
   - `ContextWindowPressureWatcher`: metric = context usage fraction, remediation = `Cooldown { factor: 0.5 }`
   - `TimeOverrunWatcher`: metric = time/budget ratio, remediation = `Abort`

5. Update `conductor.rs` to collect `WatcherVerdict` values in addition to existing `WatcherOutput` collection.

6. Add re-exports to `crates/roko-conductor/src/lib.rs`: `WatcherVerdict`, `Remediation`, `WatcherCell`.

7. Add tests verifying each watcher produces a correctly-typed `WatcherVerdict`.

## Verification
```bash
cargo check -p roko-conductor
cargo clippy -p roko-conductor --no-deps -- -D warnings
cargo test -p roko-conductor -- watcher
cargo test -p roko-conductor -- verdict
```

## What NOT to do
- Do NOT modify the `Verdict` struct in `roko-core/src/verdict.rs` — that is for gates, not conductor watchers
- Do NOT make watchers async — they are synchronous by design
- Do NOT delete the existing `React`-based watcher code — add the `WatcherCell` path alongside
- Do NOT wire the pipeline into the CLI orchestrator yet — that is a separate batch
- Do NOT change watcher detection thresholds — preserve existing constants
- Do NOT add Bus Pulse emission yet — that depends on Bus wiring (M024)
