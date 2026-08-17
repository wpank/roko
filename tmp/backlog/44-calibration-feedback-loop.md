# Calibration Feedback Loop Closure

**Priority**: P2
**Size**: M (2–3 days)

---

## Problem

The learning pipeline has three subsystems that compute useful signal — prediction
residuals, section effectiveness, and per-task efficiency grades — but two of those
signals never flow back into routing or prompt decisions. The result is that the
`CascadeRouter` and the `SystemPromptBuilder` learn from cost history but are blind to
the two other available feedback channels.

### Loop 1 — calibration residuals do not bias model scoring

`CalibrationTracker` (in `roko-learn`) records how wrong cost/quality predictions were
for each model. When a task runs, the actual cost is compared to the predicted cost;
the residual is published via a `LearningEvent::CalibrationCorrection` bus event.

`CascadeRouter::apply_calibration_correction` exists
(`crates/roko-learn/src/cascade_router.rs:1326`) and is called from
`event_subscriber.rs:124`. However, `apply_calibration_correction` only adjusts an
internal correction factor that is stored separately from the router's scoring weights.
The main `score_candidate` path in the router does not incorporate the correction factor
when ranking models for dispatch. A model that consistently underdelivers relative to its
predicted quality will still score identically to one that overdelivers.

### Loop 2 — section effectiveness is written but not read by the runner

`SectionEffectivenessRegistry` (`crates/roko-learn/src/section_effect.rs`) tracks
which prompt sections correlated with gate passes and which correlated with gate
failures. The registry is persisted to `.roko/learn/section-effects.json` via
`FeedbackService::persist_score_snapshots`.

`PromptCache` (`crates/roko-cli/src/dispatch/prompt_cache.rs:39`) loads the registry
at startup from disk and stores it in `PromptCache::effectiveness`. The runner
(`event_loop.rs`) creates and refreshes a `PromptCache`, and the `DispatchPromptBuilder`
reads `PromptCache::effectiveness` into its `section_effectiveness` field
(`prompt_builder.rs:1115`).

The chain from disk → cache → builder exists and is wired. The gap is on the write side:
nothing in the runner or event subscriber updates `section-effects.json` during a live
run. The `FeedbackService` that writes the file is used by `roko-serve` routes
(`service_factory.rs:229`) but is not integrated into the runner's learning subscriber
(`run_learning_subscriber` in `event_loop.rs`). Gate outcomes observed during plan
execution are not fed into the `SectionEffectivenessRegistry`, so the file on disk never
reflects current-run data unless `roko serve` is also running.

### Loop 3 — efficiency grades do not bias the cascade router

Per-task efficiency grades are computed by `roko-learn/src/efficiency.rs` and appended
to `.roko/learn/efficiency.jsonl` by the event subscriber
(`event_subscriber.rs:163`). The `EfficiencyGrade` type (A/B/C/D) captures the ratio
of useful signal produced per token consumed. A model that consistently earns grade D
(low signal, high cost) should be deprioritized by the router.

`CascadeRouter` (`crates/roko-learn/src/cascade_router.rs`) has no field or method
that accepts efficiency grades. The router's scoring uses cost history and calibration
corrections but not the efficiency dimension. The `AgentEfficiencyEvent` written to disk
is never read back into routing decisions.

### What already exists

| Component | Location | Status |
|---|---|---|
| `CalibrationTracker` | `roko-learn/src/oracles/witness.rs` | EXISTS |
| `CascadeRouter::apply_calibration_correction` | `roko-learn/src/cascade_router.rs:1326` | EXISTS (called but not used in scoring) |
| `CalibrationCorrection` event → router path | `event_subscriber.rs:124` | EXISTS (correction stored, not applied to scores) |
| `SectionEffectivenessRegistry` | `roko-learn/src/section_effect.rs` | EXISTS |
| `PromptCache::effectiveness` → `DispatchPromptBuilder` | `prompt_builder.rs:1115` | EXISTS (read path wired) |
| `FeedbackService` (writes section-effects.json) | `roko-learn/src/feedback_service.rs` | EXISTS (not wired into runner) |
| `AgentEfficiencyEvent` | `roko-learn/src/efficiency.rs` | EXISTS |
| Efficiency event append | `event_subscriber.rs:163` | EXISTS (writes JSONL, not read back) |
| `EfficiencyGrade` enum (A/B/C/D) | `roko-learn/src/efficiency.rs` | EXISTS |

### What is missing

1. **Calibration correction applied during scoring.** `score_candidate` inside
   `CascadeRouter` must read the stored correction factor and incorporate it as a
   multiplicative bias on the quality score.

2. **Gate-outcome → section-effectiveness write path in the runner.** The runner's
   learning subscriber must call into a `SectionEffectivenessRegistry` (or a
   `FeedbackService` instance) when a gate result is observed, then persist the updated
   registry to `section-effects.json`. This closes the loop so the next-task prompt
   assembly reflects the current run's data.

3. **Efficiency grade → router bias.** `CascadeRouter` needs an
   `update_efficiency_grade(model: &str, grade: EfficiencyGrade)` method and must apply
   a per-model efficiency multiplier when scoring candidates. The event subscriber that
   already records the `AgentEfficiencyEvent` to JSONL should also call this method.

---

## Proposed changes

### Change A: apply calibration in `score_candidate`

File: `crates/roko-learn/src/cascade_router.rs`

`apply_calibration_correction` already stores a correction multiplier per model. Locate
`score_candidate` (or equivalent ranking function) and multiply the quality score by
`correction_factor.get(model_id).unwrap_or(1.0)`. Add a unit test that verifies a model
with a negative correction ranks lower than one without.

Estimated: ~30 lines (read stored correction, one multiply, one test).

### Change B: wire gate outcomes into section effectiveness during plan execution

File: `crates/roko-learn/src/event_subscriber.rs`

When the subscriber observes a `LearningEvent::GateResult` (or equivalent gate-outcome
event), extract the prompt section metadata that was recorded in the dispatch context
and call `SectionEffectivenessRegistry::record_gate_outcome`. Persist the registry to
`section-effects.json` using the same path as `FeedbackService`. Reload `PromptCache`
on the next stale-check cycle (already handled by `event_loop.rs:4858`).

Estimated: ~60 lines (event match arm, registry update, persist call, tests).

### Change C: efficiency grade → router multiplier

File: `crates/roko-learn/src/cascade_router.rs` and
`crates/roko-learn/src/event_subscriber.rs`

Add a `HashMap<String, f64>` efficiency weight map to `CascadeRouter` (persisted in
`cascade-router.json`). Add `update_efficiency(model: &str, grade: EfficiencyGrade)`.
Grade A → multiplier stays at 1.0; grades B/C/D apply a small downward nudge
(e.g., 0.95 / 0.85 / 0.70). In `event_subscriber.rs`, after appending the
`AgentEfficiencyEvent` to JSONL, also call `router.update_efficiency(...)`.

Estimated: ~80 lines (field, method, scoring integration, test, subscriber call).

---

## Acceptance criteria

1. After N plan tasks, `cascade-router.json` scores differ between models with
   different calibration correction histories (A: correction factor visible in JSON,
   B: scores reflect it).
2. After a gate failure on a task, the next task's system prompt has section priorities
   adjusted — verifiable by comparing prompt bytes before and after gate failure with
   the same workdir.
3. After a task that earns `EfficiencyGrade::D`, the model's routing weight in
   `cascade-router.json` is lower than before the task ran.
4. `cargo test -p roko-learn` passes with zero failures.
5. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

---

## References

- `crates/roko-learn/src/cascade_router.rs:1326` — `apply_calibration_correction`
- `crates/roko-learn/src/event_subscriber.rs:124` — calibration correction call site
- `crates/roko-learn/src/event_subscriber.rs:163` — efficiency event append call site
- `crates/roko-learn/src/section_effect.rs` — `SectionEffectivenessRegistry`
- `crates/roko-learn/src/feedback_service.rs` — `FeedbackService` (writes section-effects.json)
- `crates/roko-cli/src/dispatch/prompt_cache.rs:39` — `PromptCache::effectiveness`
- `crates/roko-cli/src/dispatch/prompt_builder.rs:1115` — section effectiveness read path
- `crates/roko-cli/src/runner/event_loop.rs:4858` — stale `PromptCache` refresh point
- `crates/roko-learn/src/efficiency.rs` — `EfficiencyGrade` / `AgentEfficiencyEvent`
- `.roko/learn/cascade-router.json` — router persistence (runtime artifact)
- `.roko/learn/section-effects.json` — section effectiveness persistence (runtime artifact)
- `tmp/backlog/34-prd-cascade-learning.md` — related cascade router learning backlog
