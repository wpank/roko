# M170 — Wire Test Strategy as Verify Pipeline

## Objective
Wire the test strategy as a Verify Pipeline in `roko-gate`. The `GatePipeline` struct already composes multiple gates, but there is no stage-aware test pipeline that maps test categories (unit, integration, property, eval, red-team) to gate rungs with escalating cost. Create `TestPipelineGraph` with 5 stages and wire stage selection into the existing gate rung system so that lower rungs run cheap tests and higher rungs progressively add expensive verification.

## Scope
- Crates: `roko-gate`, `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/test_pipeline.rs` (new — stage-aware pipeline)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_pipeline.rs` (existing pipeline, read for interface)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/lib.rs` (re-export)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (wire rung→stage mapping)
- Depth doc: `tmp/unified-depth/21-roadmap/09-test-strategy-and-verification.md`

## Steps
1. Read existing gate pipeline to understand composition interface:
   ```bash
   grep -n 'pub struct\|pub fn\|pub trait\|Verify\|GatePipeline\|rung' /Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_pipeline.rs | head -25
   grep -n 'rung\|Rung\|level\|GateLevel' /Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/lib.rs | head -15
   ```

2. Read orchestrate.rs gate rung configuration:
   ```bash
   grep -n 'rung\|enrich_rung\|gate.*config\|GateConfig' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -15
   ```

3. Check for existing test categorization:
   ```bash
   grep -rn 'unit.*test\|integration.*test\|property.*test\|eval.*test\|red.?team' /Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/ --include='*.rs' | head -15
   ```

4. Define the 5 test stages with cost tiers:
   ```rust
   /// Test stage with associated cost tier and gate rung mapping.
   #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
   pub enum TestStage {
       /// Fast unit tests. Cost tier: T0. Rungs: 1-7 (always run).
       Unit,
       /// Integration tests (may need services). Cost tier: T0. Rungs: 1-7.
       Integration,
       /// Property-based tests (randomized). Cost tier: T1. Rungs: 3-7.
       Property,
       /// LLM-as-judge evaluation. Cost tier: T2. Rungs: 5-7.
       Eval,
       /// Adversarial red-team probes. Cost tier: Delta. Rung: 7 only.
       RedTeam,
   }

   impl TestStage {
       /// Minimum gate rung required to activate this stage.
       pub fn min_rung(&self) -> u8 {
           match self {
               Self::Unit => 1,
               Self::Integration => 1,
               Self::Property => 3,
               Self::Eval => 5,
               Self::RedTeam => 7,
           }
       }

       /// Cost tier label for budget accounting.
       pub fn cost_tier(&self) -> &'static str {
           match self {
               Self::Unit => "T0",
               Self::Integration => "T0",
               Self::Property => "T1",
               Self::Eval => "T2",
               Self::RedTeam => "Delta",
           }
       }
   }
   ```

5. Create `TestPipelineGraph`:
   ```rust
   /// Stage-aware test pipeline that activates stages based on gate rung.
   ///
   /// Rung 1-2: Unit + Integration (fast, cheap)
   /// Rung 3-4: + Property (randomized, medium)
   /// Rung 5-6: + Eval (LLM-judge, expensive)
   /// Rung 7:   + RedTeam (adversarial, highest cost)
   pub struct TestPipelineGraph {
       stages: Vec<TestStageConfig>,
       current_rung: u8,
   }

   #[derive(Debug, Clone)]
   pub struct TestStageConfig {
       pub stage: TestStage,
       pub gate: Box<dyn Verify>,
       pub timeout: Duration,
       pub required: bool,   // Must pass, or advisory-only?
   }

   impl TestPipelineGraph {
       pub fn new() -> Self { ... }
       pub fn with_rung(mut self, rung: u8) -> Self { ... }
       pub fn add_stage(mut self, config: TestStageConfig) -> Self { ... }

       /// Get stages active at the current rung level.
       pub fn active_stages(&self) -> Vec<&TestStageConfig> {
           self.stages.iter()
               .filter(|s| s.stage.min_rung() <= self.current_rung)
               .collect()
       }

       /// Run all active stages, returning results per stage.
       pub async fn run(&self, context: &GateContext) -> TestPipelineResult { ... }
   }
   ```

6. Define result type:
   ```rust
   #[derive(Debug)]
   pub struct TestPipelineResult {
       pub passed: Vec<TestStageResult>,
       pub failed: Vec<TestStageResult>,
       pub skipped: Vec<TestStage>,    // Stages above current rung
       pub total_duration: Duration,
       pub total_cost_tier: String,    // Highest cost tier used
   }

   #[derive(Debug)]
   pub struct TestStageResult {
       pub stage: TestStage,
       pub passed: bool,
       pub duration: Duration,
       pub details: String,
   }
   ```

7. Wire into orchestrate.rs gate execution — replace flat gate list with pipeline graph:
   ```bash
   grep -n 'gate.*run\|run.*gates\|gate_pipeline\|verify' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -10
   ```
   At gate execution time, construct `TestPipelineGraph` with the task's rung level and run only the appropriate stages.

8. Re-export from `lib.rs`:
   ```rust
   pub mod test_pipeline;
   pub use test_pipeline::{TestPipelineGraph, TestStage, TestPipelineResult};
   ```

9. Write unit tests:
   - Rung 1 activates only Unit + Integration
   - Rung 4 adds Property
   - Rung 6 adds Eval
   - Rung 7 adds RedTeam (all stages active)
   - Failed required stage fails the pipeline
   - Failed advisory stage does not fail pipeline
   - Skipped stages are reported correctly

## Verification
```bash
cargo check -p roko-gate
cargo clippy -p roko-gate --no-deps -- -D warnings
cargo test -p roko-gate -- test_pipeline
cargo check -p roko-cli
```

## What NOT to do
- Do NOT modify existing GatePipeline — TestPipelineGraph is a higher-level orchestrator that uses it
- Do NOT implement actual test runners (cargo test, proptest, etc.) — use trait objects for stage executors
- Do NOT hardcode test commands — stage configs should be data-driven from roko.toml
- Do NOT remove existing gate rung logic in orchestrate.rs — augment it with stage selection
- Do NOT make red-team mandatory at rung 7 — it should be opt-in via config
