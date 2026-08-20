# 90 — UX34: Force-Backend Overrides Corrupt Cascade Router Learning

**Priority**: P1 — correctness (bandit statistics are poisoned by outcomes the router did not choose)
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli` (`src/runner/event_loop.rs`, `src/runner/types.rs`, `src/runtime_feedback/routing.rs`)
**Depends on**: None

---

## Background

The cascade router is a bandit-based model selection system that learns from outcomes: when a task succeeds or fails, it records which model was used and updates its probability weights accordingly. This is how roko learns that "Opus is better for architectural tasks" or "Sonnet is more cost-effective for mechanical tasks."

When a user or config specifies `force_backend` (a CLI flag or config option that overrides the router's model selection), the task runs on the forced model. But crucially, the router did NOT choose that model — the human did. Recording this outcome as a bandit observation teaches the router that its selection (which was bypassed) produced this result, which is false. Over time, the router's statistics are distorted by observations it was not responsible for.

The infrastructure to fix this already exists. The `ModelChoiceSource` enum in `dispatch/model_routing.rs` has an `Override` variant that is correctly set when `force_backend` is used. The `RunnerDispatchPlan.forced` boolean (set to `true` when `force_backend` is active) is correctly computed. The `RoutingObservationSink` already receives `model_source` in the `FeedbackEvent::TaskCompleted` event — it just immediately discards it with `let _ = model_source`.

The problem is a pipeline break in two places:
1. When converting a `RunnerEvent::TaskAttemptCompleted` into a `FeedbackEvent::TaskCompleted`, the code hardcodes `model_source: ModelChoiceSource::Default` instead of reading the actual source from the dispatch plan.
2. Even if the correct source arrived, the `RoutingObservationSink` ignores it.

## Current State

### Disconnect 1: Hardcoded `ModelChoiceSource::Default` in feedback conversion

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

The `runner_event_to_feedback` function at line 7794 converts `RunnerEvent::TaskAttemptCompleted` into `FeedbackEvent::TaskCompleted`. At line 7832, `model_source` is hardcoded:

```rust
Some(FeedbackEvent::TaskCompleted {
    plan_id: attempt.plan_id.clone(),
    task_id: attempt.task_id.clone(),
    outcome: agent_outcome,
    model_source: ModelChoiceSource::Default,  // ← line 7832, always wrong
    succeeded,
    routing_context: routing_ctx.clone(),
    prompt_text: usage.prompt_text.clone(),
})
```

The `RunnerEvent::TaskAttemptCompleted` struct (defined in `src/runner/types.rs` lines 1010-1031) does not carry a `model_source` field — it only carries `model: String` and `provider: String`. There is no field to read the actual source from.

The dispatch plan (which has the source via `dispatch_plan.forced`) is resolved at event_loop.rs line 9630. The plan's `.forced` field is used at line 9657 to determine `allow_learned_model_modulation` and at line 9992 for error handling, but it is never stored anywhere the `runner_event_to_feedback` function can access it later.

### Disconnect 2: Routing sink ignores the source

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runtime_feedback/routing.rs`

The `RoutingObservationSink::on_event` method at line 67 receives `model_source` but discards it at line 82:

```rust
// The model_source tag still flows through the per-sink event
// log so override-vs-router observations can be dampened
// downstream. See `.roko/GAPS.md`.
let _ = model_source;
```

The comment acknowledges the gap. The code then proceeds to update bandit statistics unconditionally for both router-chosen and manually-overridden models.

### What works correctly

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/model_routing.rs`

- Lines 95-104: `ModelChoiceSource` enum with `Override`, `TaskHint`, `Router`, `Default` variants.
- Lines 276-280: When `force_backend` is set, the dispatcher returns `ModelChoiceSource::Override`.
- Line 118-120: `ModelChoice::forced()` checks for `Override`.

The tagging layer correctly identifies override decisions. Only the feedback pipeline is broken.

## Implementation Plan

### Step B1: Add `model_source` to `RunnerEvent::TaskAttemptCompleted`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs`, the `TaskAttemptCompleted` variant at line 1011 needs a new field:

```rust
#[serde(rename = "task.attempt.completed")]
TaskAttemptCompleted {
    timestamp: String,
    timestamp_ms: u64,
    run_id: String,
    #[serde(flatten)]
    attempt: TaskAttemptRef,
    outcome: TaskAttemptOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    failure_kind: Option<RunnerFailureKind>,
    duration_ms: u64,
    #[serde(default)]
    phase_durations: TaskPhaseDurations,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    model: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    provider: String,
    #[serde(default)]
    prompt_experiment_observation_eligible: bool,
    // NEW:
    #[serde(default)]
    model_source: crate::dispatch::ModelChoiceSource,
},
```

`ModelChoiceSource` must implement `Default` (add `#[derive(Default)]` with `Default = Default` on the enum, or implement `Default` manually to return `ModelChoiceSource::Default`).

The `task_attempt_completed` constructor function at line 1341 must be updated to accept a `model_source` parameter:

```rust
pub fn task_attempt_completed(
    run_id: &str,
    attempt: TaskAttemptRef,
    outcome: TaskAttemptOutcome,
    failure_kind: Option<RunnerFailureKind>,
    duration_ms: u64,
    model: impl Into<String>,
    provider: impl Into<String>,
    model_source: crate::dispatch::ModelChoiceSource,  // NEW
) -> Self {
```

And `task_attempt_completed_with_timing` at line 1366 similarly.

### Step B2: Thread `model_source` through the dispatch path

The dispatch path resolves `dispatch_plan` at `event_loop.rs:9630`. After the model selection/override logic completes and the attempt is submitted, the `model_source` is derivable from `dispatch_plan.forced`:

```rust
let model_source = if dispatch_plan.forced {
    ModelChoiceSource::Override
} else {
    // Check if it was a task hint (model_hint was set) or router selection.
    if ctx.state.task_model_hint.is_some() {
        ModelChoiceSource::TaskHint
    } else {
        ModelChoiceSource::Router
    }
};
```

This `model_source` value must be stored on the run state or threaded to the point where `RunnerEvent::task_attempt_completed` is called. The simplest approach: add a `model_source: ModelChoiceSource` field to whatever state struct holds per-attempt metadata (look for where `ctx.state.task_model_hint` is set at line 9623 — the same location can set `ctx.state.task_model_source = model_source`).

Then when calling `RunnerEvent::task_attempt_completed` at its various call sites, pass the stored `model_source`.

### Step B3: Read `model_source` in `runner_event_to_feedback`

In `event_loop.rs` at line 7803-7837, the `RunnerEvent::TaskAttemptCompleted` match arm can now destructure `model_source`:

```rust
RunnerEvent::TaskAttemptCompleted {
    attempt,
    outcome,
    model,
    provider,
    model_source,   // NEW: destructure the field
    ..
} => {
    // ...
    Some(FeedbackEvent::TaskCompleted {
        // ...
        model_source: *model_source,  // Use actual source instead of hardcoded Default
        // ...
    })
}
```

### Step B4: Implement dampening in `RoutingObservationSink`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runtime_feedback/routing.rs`, replace the `let _ = model_source` at line 82 with:

```rust
match model_source {
    ModelChoiceSource::Override => {
        // The user manually specified the model. Do not update bandit stats —
        // the router did not choose this model and should not learn from its outcome.
        tracing::debug!(
            model = %outcome.model,
            "skipping cascade router update for force_backend override"
        );
        return Ok(());
    }
    ModelChoiceSource::TaskHint => {
        // Author-specified model hint. Treat as an observation but consider
        // whether to dampen. Current decision: let task hints feed the router
        // since they represent informed author intent.
        // (Fall through to normal bandit update below.)
    }
    ModelChoiceSource::Router | ModelChoiceSource::Default => {
        // Router chose the model — normal observation.
    }
}
```

Remove the `#[cfg(test)]` guard on the `ModelChoiceSource` import at line 32 of `routing.rs` since it is now used in production code.

### Step B5: Add `#[derive(Default, Serialize, Deserialize)]` to `ModelChoiceSource`

`ModelChoiceSource` is used in serde-tagged structs. For `#[serde(default)]` to work on the new field, the type needs `Deserialize` (already has it for use in events) and `Default`. Add `Default` that returns `ModelChoiceSource::Default`.

## Acceptance Criteria

1. `RunnerEvent::TaskAttemptCompleted` carries a `model_source: ModelChoiceSource` field.
2. The `task_attempt_completed` and `task_attempt_completed_with_timing` constructors in `types.rs` accept and store `model_source`.
3. All call sites of `task_attempt_completed` compile with the new parameter.
4. `runner_event_to_feedback` at event_loop.rs:7832 uses the actual `model_source` from the event instead of `ModelChoiceSource::Default`.
5. `RoutingObservationSink::on_event` returns early (without updating bandit stats) when `model_source` is `ModelChoiceSource::Override`.
6. `let _ = model_source` on line 82 of `routing.rs` is removed.
7. The `#[cfg(test)]` guard on the `ModelChoiceSource` import in `routing.rs` is removed.
8. A new test: dispatch a task with `force_backend` set, verify the cascade router's `confidence_snapshot()` does not change after the task completes.
9. Existing routing sink tests in `routing.rs` pass (they use `ModelChoiceSource::Router` which should still update bandit stats normally).
10. `cargo test -p roko-cli` passes.
11. `cargo clippy -p roko-cli -- -D warnings` passes.

## Verification Checklist

- [ ] Read `dispatch/model_routing.rs` lines 95-130 to understand `ModelChoiceSource` and `ModelChoice` before editing
- [ ] Add `model_source` field to `TaskAttemptCompleted` struct in `types.rs` line 1011
- [ ] Add `#[derive(Default)]` (or `impl Default`) to `ModelChoiceSource` with `Default` = `ModelChoiceSource::Default`
- [ ] Update `task_attempt_completed` and `task_attempt_completed_with_timing` constructors in `types.rs`
- [ ] Find all call sites of those constructors and add the `model_source` argument (search: `RunnerEvent::task_attempt_completed(` in the repo)
- [ ] Store `model_source` on `RunState` near where `task_model_hint` is stored (event_loop.rs ~line 9623)
- [ ] Update `runner_event_to_feedback` at event_loop.rs:7832 to use the field
- [ ] Remove `#[cfg(test)]` from the `ModelChoiceSource` import in `routing.rs`
- [ ] Implement Override early-return in `RoutingObservationSink::on_event`
- [ ] Run `cargo build -p roko-cli` to confirm all call sites compile
- [ ] Run `cargo test -p roko-cli`
- [ ] Run `cargo clippy -p roko-cli -- -D warnings`

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs` | Add `model_source: ModelChoiceSource` field to `TaskAttemptCompleted` variant (line 1011); update `task_attempt_completed` and `task_attempt_completed_with_timing` constructors (lines 1341, 1366) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/model_routing.rs` | Add `#[derive(Default)]` to `ModelChoiceSource` with `Default = ModelChoiceSource::Default` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | Compute and store `model_source` at dispatch time (~line 9623); destructure `model_source` in `runner_event_to_feedback` (~line 7803); replace hardcoded `ModelChoiceSource::Default` at line 7832 |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runtime_feedback/routing.rs` | Remove `#[cfg(test)]` guard on `ModelChoiceSource` import (line 32); replace `let _ = model_source` (line 82) with Override early-return; add test |
