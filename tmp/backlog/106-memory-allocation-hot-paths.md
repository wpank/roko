# 106 — Excessive Clone/Allocation in Event Loop and Dispatcher Hot Paths

**Priority**: P2 — performance; clone accumulation in gate-failure retry loops wastes heap and slows plan execution
**Size**: M (2-3 days)
**Crates**: `roko-cli` (`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/`), `roko-agent` (`/Users/will/dev/nunchi/roko/roko/crates/roko-agent/`)
**Depends on**: None

---

## Background

The roko event loop (`crates/roko-cli/src/runner/event_loop.rs`, 23,154 lines) is the hot path
for all plan execution. It orchestrates dispatch, gate evaluation, retry, and persistence for
every task in every plan. The tool dispatcher (`crates/roko-agent/src/dispatcher/mod.rs`) runs
inside each agent turn to fan out parallel tool calls.

Both files contain `.clone()` calls on heap-allocated data in tight inner loops. Each individual
clone is cheap, but they compound during retry-heavy execution where gate failures trigger
repeated preflight→dispatch→gate→replan cycles per task. The five worst hotspots are: the event
forwarder task that clones `TaskAttemptRef` (two `String` fields) for every agent event; the
parallel tool dispatcher that clones the entire `ToolCall` struct (including a potentially large
`serde_json::Value` arguments field) to preserve the name after dispatch; the hook chain
evaluator that clones the full arguments `Value` again; a `HashMap` insertion that clones
`TaskAttemptRef` per failed preflight attempt; and the telemetry emission path that calls
`.to_string()` on borrowed `&str` slices to construct `LensScope` variants.

The goal is to eliminate unnecessary heap allocations from these five hotspots without changing
any observable behavior.

## Current State

### Hotspot 1: Event forwarder loop — `TaskAttemptRef` cloned per agent event

**File:** `crates/roko-cli/src/runner/event_loop.rs`, lines 588-601

```rust
async fn forward_agent_events(
    attempt: TaskAttemptRef,       // ← owns two Strings: plan_id, task_id
    effect: EffectRef,
    agent_id: String,
    mut raw_rx: mpsc::Receiver<AgentEvent>,
    routed_tx: mpsc::Sender<RoutedAgentEvent>,
) {
    while let Some(event) = raw_rx.recv().await {
        let routed = RoutedAgentEvent::for_attempt(
            attempt.clone(),       // ← allocates two Strings per event
            effect,
            agent_id.clone(),      // ← allocates one String per event
            event,
        );
        if routed_tx.send(routed).await.is_err() { break; }
    }
}
```

`TaskAttemptRef` is defined at `crates/roko-cli/src/runner/types.rs` line 408:
```rust
pub struct TaskAttemptRef {
    pub plan_id: String,
    pub task_id: String,
    pub attempt: u32,
}
```
This is immutable after creation. Every agent event (tool calls, results, streaming text chunks)
triggers one `TaskAttemptRef::clone()` = two new `String` heap allocations.

### Hotspot 2: Parallel tool dispatcher clones full `ToolCall`

**File:** `crates/roko-agent/src/dispatcher/mod.rs`, lines 765-773

```rust
let par_stream = futures::stream::iter(parallel.into_iter().map(|call| async {
    let name = call.clone();               // ← clones id+name+arguments+timestamp
    let res = self.dispatch_with_result_limit(call, ctx, result_limit).await;
    (name, res)
}))
.buffer_unordered(DEFAULT_MAX_CONCURRENT_TOOLS);
let mut out: Vec<(ToolCall, ToolResult)> = par_stream.collect().await;
```

`ToolCall` is defined at `crates/roko-core/src/tool/call.rs` line 35:
```rust
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,  // ← can be 10KB+ for file-content calls
    pub request_ts_ms: i64,
}
```
The full struct including `arguments` is cloned only to extract `name` after dispatch completes.

### Hotspot 3: Hook chain evaluator clones arguments `Value`

**File:** `crates/roko-agent/src/dispatcher/mod.rs`, line 794

```rust
let evaluated = chain.evaluate(def, call.arguments.clone(), ctx).await;
```

The `arguments` field is cloned again for hook evaluation, even though `evaluate` takes
ownership. If `arguments` is large (API payload, file content), this is another big allocation.

### Hotspot 4: Baseline failure map accumulates `TaskAttemptRef` clones

**File:** `crates/roko-cli/src/runner/event_loop.rs`, line 3536

```rust
baseline_gate_failures.insert(completion_attempt.clone(), failures);
```

`baseline_gate_failures` is a `HashMap<TaskAttemptRef, Vec<String>>` (line 1532). For a plan
where 30 tasks fail preflight, 30 full `TaskAttemptRef` structs (two `String` allocations each)
are cloned into the map. The map owner already holds the original; the `HashMap` could use a
hashable key instead of the full struct.

### Hotspot 5: Telemetry `.to_string()` on borrowed `&str`

**File:** `crates/roko-cli/src/runner/event_loop.rs`, lines 150-151 (inside `emit_signal_scores`)

```rust
LensScope::Cell(task_id.to_string()),
LensScope::Graph(plan_id.to_string()),
```

This function is called at line 9832 for every scored signal. Both `task_id` and `plan_id` are
available as `&str` at the call site, but `LensScope` variants hold `String`, so `.to_string()`
allocates a new `String` every call. The same pattern repeats at lines 202-203.

**Bonus hotspot confirmed in `gate_dispatch.rs` line 663:**
```rust
let verified = ObservableEvent::SignalVerified(signal.id.to_hex(), (*verdict).clone());
```
`GateVerdictSummary` (cloned here) contains multiple `String` fields (`gate_name`, `summary`,
`error_digest` items). With a 7-rung pipeline, this is 7 verdict clones per task.

## Implementation Plan

### Fix 1: Wrap `TaskAttemptRef` in `Arc` for the event forwarder

The forwarder task owns `attempt` for its entire lifetime and only uses it to stamp outgoing
events. Change the function signature to accept `Arc<TaskAttemptRef>`:

```rust
// In crates/roko-cli/src/runner/event_loop.rs, line 588:
async fn forward_agent_events(
    attempt: Arc<TaskAttemptRef>,      // was: TaskAttemptRef
    effect: EffectRef,
    agent_id: String,
    mut raw_rx: mpsc::Receiver<AgentEvent>,
    routed_tx: mpsc::Sender<RoutedAgentEvent>,
) {
    while let Some(event) = raw_rx.recv().await {
        let routed = RoutedAgentEvent::for_attempt(
            (*attempt).clone(),       // still clones once to satisfy for_attempt signature,
            // OR change for_attempt to accept Arc<TaskAttemptRef> too
```

The deeper fix is to make `RoutedAgentEvent::for_attempt` accept `Arc<TaskAttemptRef>` and store
it as `Arc<TaskAttemptRef>` in the event. This eliminates ALL clones in the loop body — just an
`Arc::clone` (atomic refcount bump). Check all call sites of `RoutedAgentEvent::for_attempt` in
`event_loop.rs` and update them.

If `RoutedAgentEvent` is used widely, a smaller first step is to just clone `.plan_id` and
`.task_id` strings once before the loop and reconstruct the ref inline:

```rust
let plan_id = attempt.plan_id.clone();
let task_id = attempt.task_id.clone();
let attempt_n = attempt.attempt;
// loop:
let routed = RoutedAgentEvent::for_attempt(
    TaskAttemptRef { plan_id: plan_id.clone(), task_id: task_id.clone(), attempt: attempt_n },
    // ...
```

This is the same number of allocations but avoids the `agent_id.clone()` since you can store it
separately too. The full `Arc` approach is preferred.

### Fix 2: Clone only `id` and `name` from `ToolCall` before dispatch

In `crates/roko-agent/src/dispatcher/mod.rs`, line 765, replace:

```rust
// Before:
let name = call.clone();   // clones full struct including arguments
let res = self.dispatch_with_result_limit(call, ctx, result_limit).await;
(name, res)

// After:
let id = call.id.clone();
let name = call.name.clone();
let res = self.dispatch_with_result_limit(call, ctx, result_limit).await;
// Return a lightweight identifier, not the full ToolCall:
((id, name), res)
```

Update the `out` type to `Vec<((String, String), ToolResult)>` and the serial loop at line 777
similarly. Update all call sites that destructure the output tuple. The serial branch at lines
776-782 has the same pattern (`call_copy = call.clone()`); apply the same fix.

### Fix 3: Pass `arguments` by reference to hook chain

In `crates/roko-agent/src/dispatcher/mod.rs`, line 794:

```rust
// Before:
let evaluated = chain.evaluate(def, call.arguments.clone(), ctx).await;

// After — if evaluate can accept &Value:
let evaluated = chain.evaluate(def, &call.arguments, ctx).await;
```

Check the `evaluate` signature in the `SafetyHookChain` type and update it to accept
`&serde_json::Value` if it currently takes ownership. If it must take ownership (e.g., WASM
boundary), document why and leave a `// TODO: evaluate takes ownership; pass ref when possible`
comment.

### Fix 4: Store only the key string in `baseline_gate_failures`

In `crates/roko-cli/src/runner/event_loop.rs`, change the map type at line 1532:

```rust
// Before:
baseline_gate_failures: &'a mut HashMap<TaskAttemptRef, Vec<String>>,

// After:
baseline_gate_failures: &'a mut HashMap<String, Vec<String>>,
```

Use `completion_attempt.key()` as the map key (the `key()` method at `types.rs` line 427
returns a cheap `String` composite of plan_id + task_id + attempt). Update the insertion at line
3536:

```rust
baseline_gate_failures.insert(completion_attempt.key(), failures);
```

And update all lookups in the same function to use `.key()`.

### Fix 5: Address telemetry `.to_string()` allocations

In `crates/roko-cli/src/runner/event_loop.rs`, lines 150-151 and 202-203:

The simplest fix is to verify whether `LensScope` could hold `Cow<'_, str>` instead of `String`.
Check the `LensScope` definition in `roko-core`. If `LensScope::Cell` holds `String`, changing
it to `Cow<'static, str>` or keeping it as `String` but pre-computing the scope once outside the
emit loop is the practical approach:

```rust
// Before: called in loop, allocates per-iteration
emit_signal_scores(scores, task_id, plan_id, ...).await;
// Inside emit_signal_scores:
LensScope::Cell(task_id.to_string()),   // allocates

// After: compute once before the loop if task_id and plan_id don't change
let cell_scope = LensScope::Cell(task_id.to_string());
let graph_scope = LensScope::Graph(plan_id.to_string());
// Pass pre-built scopes into the function
```

This converts per-call allocations to one-per-task.

## Acceptance Criteria

1. `forward_agent_events` does not call `attempt.clone()` inside the event loop body — uses `Arc::clone` or pre-computed fields.
2. The parallel tool dispatcher at `dispatcher/mod.rs` line 765 does not clone `call.arguments` to preserve the tool name — only `id` and `name` are extracted before dispatch.
3. `baseline_gate_failures` is keyed by `String` (the attempt key), not by `TaskAttemptRef` (which owns two Strings).
4. `LensScope::Cell(task_id.to_string())` inside `emit_signal_scores` is computed at most once per task, not per signal scored.
5. No functional change to plan execution output, episode logging, or gate verdicts.
6. All existing tests in `roko-cli` and `roko-agent` pass: `cargo test -p roko-cli -p roko-agent`.

## Verification Checklist

- [ ] `grep -n 'attempt\.clone()' crates/roko-cli/src/runner/event_loop.rs` — confirm no `attempt.clone()` inside a `while let Some(event)` loop
- [ ] `grep -n 'call\.clone()' crates/roko-agent/src/dispatcher/mod.rs` — confirm line 766 no longer clones the full `ToolCall`
- [ ] `cargo test -p roko-cli -p roko-agent 2>&1 | tail -5` — all tests pass
- [ ] `cargo clippy -p roko-cli -p roko-agent --no-deps -- -D warnings` — no warnings
- [ ] Manual: run `roko plan run plans/ --engine runner-v2` on a plan with 3+ tasks and confirm plan completes correctly

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | Fix `forward_agent_events` (line 588) to use `Arc<TaskAttemptRef>` or pre-cloned fields; change `baseline_gate_failures` type (line 1532) from `HashMap<TaskAttemptRef, _>` to `HashMap<String, _>`; pre-compute `LensScope` values in `emit_signal_scores` (lines 150-151, 202-203) |
| `crates/roko-agent/src/dispatcher/mod.rs` | Fix parallel dispatch (line 765) to extract only `id`/`name` before dispatch instead of cloning full `ToolCall`; fix serial branch (line 777) identically; fix hook chain evaluation (line 794) to pass `&call.arguments` if `evaluate` can accept a reference |
| `crates/roko-cli/src/runner/types.rs` | Optionally add `pub fn key(&self) -> String` to `TaskAttemptRef` if not already present (check line 427); update `RoutedAgentEvent::for_attempt` to accept `Arc<TaskAttemptRef>` if pursuing the full Arc approach |
