# 78 — Efficiency Pass Rate Structurally 0% (gate_passed Always None)

**Priority**: P0 — produces permanently wrong data; corrupts cascade router quality signal
**Size**: S (half day to 1 day)
**Crates**: `crates/roko-cli/src/runner/event_loop.rs`,
`crates/roko-learn/src/event_subscriber.rs`,
`crates/roko-learn/src/events.rs`,
`crates/roko-learn/src/efficiency.rs`
**Depends on**: None

---

## Background

The efficiency learning subsystem always reports a 0% gate pass rate regardless of whether
gates actually pass. This is not because gates are failing — it is because `gate_passed` in
every efficiency event is permanently `false`, set from a `None` that is never updated with
the actual gate outcome. The timing of events in the runner-v2 pipeline is the root cause:
the `TurnCompleted` event fires before the gate runs, so the gate result is not available
when the efficiency event is written.

The consequence is that `roko learn efficiency` shows every agent turn as failed, and the
cascade router records every model as having a 0% pass rate. Any model routing or quality
decisions that depend on efficiency data are operating on completely wrong information.

## Current State

1. **`forward_to_learning_bus()` sets `gate_passed: None`.** In
   `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`, the
   function `forward_to_learning_bus()` (starting around line 6698) handles
   `AgentEvent::TurnCompleted` and publishes a `roko_learn::events::AgentEvent::TurnCompleted`
   to the learning bus. Line 6720 unconditionally sets `gate_passed: None`. The gate has not
   run yet at this point.

2. **The subscriber collapses `None` to `false` immediately.** In
   `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/event_subscriber.rs`, the
   `TurnCompleted` handler (around lines 106-210):
   - Line 142: `let success = gate_passed.unwrap_or(false);` — collapses `None` to `false`
   - Line 143: `let _ = router.record_confidence_outcome(&turn_ctx.model, success);` —
     records every model as having failed
   - Line 189: `gate_passed: success,` — writes `false` into the efficiency event
   The efficiency JSONL entry is written immediately at line 202.

3. **The `GateResult` event arrives later but doesn't patch the efficiency record.** In
   `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/event_subscriber.rs`, the
   `GateResult` handler (around lines 242-257) only calls
   `verdict_history.record(VerdictRecord { ... })`. It does not update the already-written
   efficiency event or correct the cascade router's confidence outcome.

4. **`GateCompletion` carries `task_id` but `GateResult` does not.** The
   `GateCompletion` struct at
   `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs` lines 144-164
   has `pub task_id: String` (line 153). At the publication site in `event_loop.rs` around
   line 3833, `completion.task_id` is available. However, the `AgentEvent::GateResult`
   variant in `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/events.rs` lines
   35-40 has no `task_id` field — only `gate_name: String`, `passed: bool`, `score: f32`,
   and `duration_ms: u64`. Without `task_id`, the subscriber cannot correlate a `GateResult`
   with the buffered `TurnCompleted` event from the same task.

5. **`AgentEfficiencyEvent.gate_passed` is `bool`, not `Option<bool>`.** The struct in
   `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/efficiency.rs` line 151 defines
   `pub gate_passed: bool`. The `Default` impl (line 238) sets it to `false`. There is no
   representation for "gate not yet known" or "task skipped gating".

6. **Timeline of events in the runner-v2 pipeline:**
   - Agent completes turn
   - `AgentEvent::TurnCompleted` fires in `forward_to_learning_bus()` with `gate_passed: None`
   - Learning bus subscriber receives it, collapses `None` to `false`, writes efficiency
     event with `gate_passed: false`, records model failure in cascade router
   - Gate dispatch begins (in a separate tokio task via `gate_rx` channel at line 3236)
   - Gate runs (potentially seconds later)
   - `GateCompletion` arrives via `gate_rx`
   - `learning_event_bus.publish(AgentEvent::GateResult { ... })` fires at line 3833
   - Subscriber records `VerdictHistory` only — efficiency event is already written

## Implementation Plan

The cleanest fix adds `task_id` to `GateResult` and defers efficiency event emission in
the subscriber until the gate result is known.

### Step 1: Add `task_id` to `AgentEvent::GateResult`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/events.rs`, update the
`GateResult` variant (lines 35-40):

```rust
GateResult {
    gate_name: String,
    passed: bool,
    score: f32,
    duration_ms: u64,
    // NEW:
    task_id: String,
},
```

Update the publication site in `event_loop.rs` at line 3833 to populate the new field:

```rust
learning_event_bus.publish(
    roko_learn::events::AgentEvent::GateResult {
        gate_name: format!("rung-{}", completion.rung),
        passed: completion.passed,
        score: if completion.passed { 1.0 } else { 0.0 },
        duration_ms: completion.duration_ms,
        task_id: completion.task_id.clone(),  // NEW
    },
);
```

`completion.task_id` is already available at this site (it is a field of `GateCompletion`
in `types.rs` line 153).

### Step 2: Change `AgentEfficiencyEvent.gate_passed` to `Option<bool>`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/efficiency.rs`:

1. Change line 151 from `pub gate_passed: bool` to `pub gate_passed: Option<bool>`.

2. Update the `Default` impl at line 238: change `gate_passed: false` to
   `gate_passed: None`.

3. Update the composite score calculation (around line 337):
   ```rust
   // Before:
   let outcome = if self.gate_passed { 1.0 } else { 0.0 };
   // After:
   let outcome = match self.gate_passed {
       Some(true) => 1.0,
       Some(false) => 0.0,
       None => 0.5,  // neutral when gate result unknown
   };
   ```

4. Update any pass-rate calculations that use `gate_passed` as a `bool`. Search for uses of
   `gate_passed` in `efficiency.rs` and update any `if event.gate_passed` patterns to
   `if event.gate_passed == Some(true)`.

5. Add `#[serde(skip_serializing_if = "Option::is_none")]` or keep as required field for
   backward compatibility — since existing JSONL files have `"gate_passed": false`, the
   deserialization default should remain `None` (unknown), not `false`. Add
   `#[serde(default)]` to the field.

### Step 3: Buffer efficiency events in the subscriber until gate result arrives

In `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/event_subscriber.rs`:

1. Add a `pending_efficiency: HashMap<String, (AgentEfficiencyEvent, ActiveTurn)>` field
   (or local variable in the subscriber loop) keyed by `task_id`.

2. In the `TurnCompleted` handler (around line 106): instead of immediately writing the
   efficiency event when `gate_passed` is `None`, build the partial event and insert it into
   `pending_efficiency` keyed by `turn_ctx.task_id`:

   ```rust
   AgentEvent::TurnCompleted { turn, usage, tool_call_count, gate_passed, ref finish_reason } => {
       // ... existing calibration policy code ...
       let Some(turn_ctx) = active_turn.take() else { continue; };

       if gate_passed.is_none() {
           // Gate result not yet known — defer efficiency event emission.
           let partial_event = build_efficiency_event(&turn_ctx, &usage, ...);
           // Do NOT record_confidence_outcome yet.
           pending_efficiency.insert(turn_ctx.task_id.clone(), (partial_event, turn_ctx));
           continue;
       }

       // gate_passed is Some (e.g., ACP path where gate is inline) — emit immediately.
       let success = gate_passed.unwrap_or(false);
       let _ = router.record_confidence_outcome(&turn_ctx.model, success);
       // ... write efficiency event as before ...
   }
   ```

3. In the `GateResult` handler (around line 242): after updating `VerdictHistory`, look up
   and flush any pending efficiency event for the same `task_id`:

   ```rust
   AgentEvent::GateResult { ref gate_name, passed, task_id, .. } => {
       // Existing VerdictHistory code:
       if let Some(turn_ctx) = &active_turn {
           verdict_history.record(VerdictRecord { ... });
       }

       // NEW: flush pending efficiency event for this task.
       if let Some((mut event, ctx)) = pending_efficiency.remove(&task_id) {
           event.gate_passed = Some(passed);
           event.outcome = if passed {
               "success".to_string()
           } else {
               "gate_failed".to_string()
           };
           let _ = router.record_confidence_outcome(&ctx.model, passed);
           if let Err(err) = append_efficiency_event(&efficiency_path, &event).await {
               tracing::warn!(
                   path = %efficiency_path.display(),
                   error = %err,
                   task_id = %task_id,
                   "failed to write deferred efficiency event"
               );
           }
       }
   }
   ```

### Step 4: Handle tasks that skip gating or error before gating

At subscriber shutdown (the end of the `while let` loop or a drop impl), flush all
remaining buffered events with `gate_passed: None` and `outcome: "no_gate"` so data is
never silently lost:

```rust
// On subscriber loop exit:
for (task_id, (mut event, ctx)) in pending_efficiency.drain() {
    event.gate_passed = None;  // unknown / skipped
    event.outcome = "no_gate".to_string();
    tracing::debug!(task_id = %task_id, "flushing ungated efficiency event on shutdown");
    let _ = append_efficiency_event(&efficiency_path, &event).await;
}
```

### Step 5: Update call sites that read `gate_passed` as `bool`

Search the workspace for uses of `.gate_passed` on `AgentEfficiencyEvent`:
```
rg 'gate_passed' crates/roko-learn/src/ crates/roko-cli/src/
```

Update any `event.gate_passed` usage in the CLI `learn` command
(`crates/roko-cli/src/commands/learn.rs`) that treats it as `bool` to treat it as
`Option<bool>`. For example, `if event.gate_passed { "pass" } else { "fail" }` becomes
`match event.gate_passed { Some(true) => "pass", Some(false) => "fail", None => "?" }`.

### Not required (explicitly out of scope for this fix)

The `GateResult` event currently does not carry the `active_turn` model context. The
subscriber already has `active_turn` populated when processing sequential events. For
concurrent tasks, the `task_id`-keyed pending map from Step 3 provides the correlation;
`active_turn` is not needed for the flush path.

## Acceptance Criteria

1. After running `roko plan run plans/demo-hello --fresh` on a plan with passing gates,
   `roko learn efficiency` reports a non-zero pass rate.
2. Each efficiency event in `.roko/learn/efficiency.jsonl` for a passing gate has
   `"gate_passed": true` (not `false` or `null`).
3. After running a plan with a known failing gate, efficiency events for failed gates have
   `"gate_passed": false`.
4. Tasks that error before gating write efficiency events with `"gate_passed": null`
   (not `false`).
5. After subscriber shutdown, no buffered efficiency events are silently dropped (logged at
   debug level if flushed without a gate result).
6. `roko learn efficiency` JSON output reflects actual gate pass/fail, not uniform 0%.
7. Cascade router per-model confidence stats show differentiated values (not uniform 0
   successes) after a plan run with mixed pass/fail gates.
8. `cargo test --workspace` passes.
9. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

## Verification Checklist

- [ ] Run `roko plan run plans/demo-hello --fresh` on a plan with gates that pass
- [ ] Check `cat .roko/learn/efficiency.jsonl | tail -5 | jq '.gate_passed'` — should show
  `true` for passing tasks, not `false`
- [ ] Run `roko learn efficiency` — pass rate should be > 0%
- [ ] Run a plan where the compile gate is known to fail; check efficiency JSONL shows
  `"gate_passed": false` for that task
- [ ] Run a plan where a task errors before gating; check efficiency JSONL shows
  `"gate_passed": null`
- [ ] Run `roko learn router` — model success counts should be > 0 after a plan run with
  passing gates
- [ ] `cargo test -p roko-learn` passes (unit tests for `efficiency.rs` updated to use
  `Option<bool>`)

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/events.rs` | Add `task_id: String` field to `AgentEvent::GateResult` variant (lines 35-40) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | Add `task_id: completion.task_id.clone()` to `GateResult` publication at line 3834 |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/efficiency.rs` | Change `gate_passed: bool` to `gate_passed: Option<bool>` (line 151); update `Default` impl (line 238); update composite score calculation and pass-rate calculations |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/event_subscriber.rs` | Buffer `TurnCompleted` events with `gate_passed: None` in `pending_efficiency` map; flush in `GateResult` handler with actual gate result; flush remaining on shutdown |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs` | Update any `gate_passed` field reads to handle `Option<bool>` instead of `bool` |
