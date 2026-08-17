# M120 — Stuck detection Lens Cells and aggregation

## Objective
Refactor stuck detection from the monolithic `StuckDetector` into six independent Lens Cells (OutputLoop, NoProgress, GateLoop, CompileLoop, EmptyOutput, ExcessiveRetries) plus a `StuckAggregate` Cell that merges their observations. Each Lens is independently thresholded and testable. Add the `StuckSeverity` classification (Mild/Moderate/Severe) and `MetaCognitionAction` mapping.

## Scope
- Crates: `roko-conductor`
- Files:
  - `crates/roko-conductor/src/stuck_detection.rs` (reference — read but preserve)
  - New: `crates/roko-conductor/src/stuck_lenses.rs`
  - `crates/roko-conductor/src/lib.rs` (add module + re-exports)
- Depth doc: `tmp/unified-depth/07-agent-runtime/16-diagnosis-and-stuck-detection.md`

## Existing types reference

The `StuckDetector` and friends already exist (`crates/roko-conductor/src/stuck_detection.rs`):

```rust
// StuckKind already has 12 variants (more than the 6 core ones):
#[non_exhaustive]
pub enum StuckKind {
    OutputLoop, NoProgress, GateLoop, CompileLoop, EmptyOutput, ExcessiveRetries,
    ReviewLoop, IterationLoop, SilenceTimeout, CompileFailThreshold, TaskStall, ContextPressure,
}

// StuckSignal — a detected stuck condition
pub struct StuckSignal { pub kind: StuckKind, pub confidence: f64, pub duration_ms: Option<i64>, pub description: String }

// ActivityEntry — input record for stuck detection
pub struct ActivityEntry {
    pub timestamp_ms: i64, pub output_hash: String, pub files_changed: u32,
    pub gate_result: Option<String>, pub iteration: u32,
    pub activity: String, pub phase: String, pub task_id: String,
    pub tokens_used: Option<u64>, pub context_window: Option<u64>,
}

// StuckDetector — the monolithic detector, pure function
pub struct StuckDetector { thresholds: StuckThresholds }
impl StuckDetector {
    pub fn check_stuck(&self, entries: &[ActivityEntry]) -> Option<StuckSignal>
}

// MetaCognitionHook — wraps StuckDetector with cooldown and actions
pub struct MetaCognitionHook { ... }
// NOTE: MetaCognitionAction has unit variants (no String payloads, no Pause variant)
pub enum MetaCognitionAction { Continue, AdjustStrategy, Escalate }

// Already re-exported from lib.rs:
// ActivityEntry, CooldownFilter, MetaCognitionAction, MetaCognitionAssessment,
// MetaCognitionHook, StuckDetector, StuckKind, StuckSignal, StuckThresholds
```

## Steps
1. Discover the full existing API:
   ```bash
   grep -rn 'pub fn\|pub struct\|pub enum' crates/roko-conductor/src/stuck_detection.rs | head -30
   grep -rn 'MetaCognitionAction' crates/roko-conductor/src/stuck_detection.rs | head -10
   grep -rn 'stuck_detection' crates/roko-conductor/src/lib.rs
   ```

2. The `StuckKind` enum already has all 6 core variants plus 6 more. No changes needed.

3. Create `crates/roko-conductor/src/stuck_lenses.rs` with 6 individual Lens structs. Each tracks per-agent state:
   ```rust
   use std::collections::{HashMap, VecDeque};
   use serde::{Deserialize, Serialize};
   use super::stuck_detection::StuckKind;

   /// Common input event for stuck lenses.
   #[derive(Debug, Clone)]
   pub enum StuckEvent {
       TurnOutput { content_hash: u64, had_tool_calls: bool, had_file_changes: bool },
       GateResult { passed: bool, error_hash: u64 },
       ToolCall { operation_hash: u64 },
   }

   /// Observation from a single lens.
   #[derive(Debug, Clone)]
   pub struct StuckObservation {
       pub kind: StuckKind,
       pub metric: f64,
       pub description: String,
   }
   ```

   - `OutputLoopLens { threshold: usize, histories: HashMap<String, VecDeque<u64>> }` — default threshold 4
   - `NoProgressLens { threshold_ticks: usize, no_change_counts: HashMap<String, usize> }` — default 5 ticks (use tick-based, not `Instant`, for testability)
   - `GateLoopLens { threshold: usize, failure_histories: HashMap<String, Vec<u64>> }` — default 3
   - `CompileLoopLens { threshold: usize, error_fingerprints: HashMap<String, Vec<u64>> }` — default 3
   - `EmptyOutputLens { threshold: usize, empty_counts: HashMap<String, usize> }` — default 3
   - `ExcessiveRetriesLens { threshold: usize, operation_counts: HashMap<String, HashMap<u64, usize>> }` — default 6

4. Each Lens gets:
   ```rust
   pub trait StuckLens {
       fn observe(&mut self, agent_id: &str, event: &StuckEvent) -> Option<StuckObservation>;
       fn name(&self) -> &str;
   }
   ```

5. Add `StuckSeverity` enum:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
   pub enum StuckSeverity { Mild, Moderate, Severe }
   ```

6. Add `StuckAssessment` and `StuckAggregate`:
   ```rust
   #[derive(Debug, Clone)]
   pub struct StuckAssessment {
       pub severity: StuckSeverity,
       pub observations: Vec<StuckObservation>,
       pub recommended_action: MetaCognitionAction,
   }

   pub struct StuckAggregate {
       lenses: Vec<Box<dyn StuckLens>>,
   }
   impl StuckAggregate {
       pub fn evaluate(&mut self, agent_id: &str, events: &[StuckEvent]) -> Option<StuckAssessment> { ... }
   }
   ```

   Severity rules:
   - 1 detection = Mild (except GateLoop/CompileLoop = Severe)
   - 2+ detections = Moderate
   - GateLoop or CompileLoop = always Severe
   Map: Mild -> `MetaCognitionAction::Continue`, Moderate -> `AdjustStrategy`, Severe -> `Escalate`

7. Add a `LensBundle` struct that wraps all 6 with `LensBundle::default()` constructing all lenses.

8. Add `stuck_lenses` module to `crates/roko-conductor/src/lib.rs` and re-export `StuckLens`, `StuckEvent`, `StuckObservation`, `StuckSeverity`, `StuckAssessment`, `StuckAggregate`, `LensBundle`.

9. Add tests:
   - OutputLoopLens fires after 4 identical hashes
   - EmptyOutputLens fires after 3 turns without tool calls
   - GateLoopLens fires after 3 identical error hashes
   - StuckAggregate classifies Mild for 1 non-gate observation
   - StuckAggregate classifies Severe for any GateLoop/CompileLoop
   - StuckAggregate classifies Moderate for 2+ observations

## Verification
```bash
cargo check -p roko-conductor
cargo clippy -p roko-conductor --no-deps -- -D warnings
cargo test -p roko-conductor -- stuck_lens
cargo test -p roko-conductor -- stuck_aggregate
cargo test -p roko-conductor -- lens_bundle
```

## What NOT to do
- Do NOT delete or modify the existing `StuckDetector` — preserve backward compat
- Do NOT use `DashMap` or async — keep Lens Cells synchronous for testability
- Do NOT use `Instant` in lens state — use tick counts so tests are deterministic
- Do NOT wire into the orchestrator loop — that is integration work
- Do NOT add threshold learning here — that is M122
