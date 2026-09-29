# Depth: Filter and Admission Pipeline

> Parent: [15-TRIGGERS](../../15-TRIGGERS.md) -- Section 6

---

## Overview

Every submitted `TriggerEvent` passes through a three-stage admission
pipeline before the concurrency policy is evaluated. Each stage can
suppress the event (returning `TriggerSubmitStatus::Suppressed` or
`Queued`). The stages are applied in order from cheapest to most expensive.

## Pipeline flow

```
submit_event(event)
    |
    +-> 1. Payload matching (admit_event)
    |       if matches fail -> Filtered -> Suppressed
    |
    +-> 2. Debounce (admit_event)
    |       if debounce active -> retain latest, start timer -> Queued
    |
    +-> 3. Rate limiting (admit_event)
    |       if window exceeded -> Drop/Queue/Warn per action
    |
    +-> 4. Concurrency policy (apply_concurrency)
    |       Queue/Skip/CancelRunning/Parallel
    |
    +-> 5. Launch Flow (launch_flow)
            trace dedup -> scope resolution -> input mapping ->
            persist event -> emit lifecycle -> spawn task
```

## Stage 1: Payload matching

The `payload_matches` function checks the `filter.matches` map against the
event payload. Each entry is a key-value pair; the payload must contain
every specified key with a matching value. This is a shallow structural
comparison, not a deep JSONPath query.

If matching fails, the coordinator emits a `TriggerEventKind::Filtered`
lifecycle event with `reason: "payload did not match"` and returns
`Suppressed`.

## Stage 2: Debounce

When `filter.debounce_ms` is set and greater than zero:

1. The coordinator stores the event in `active.debounce_event`, replacing
   any previously retained event.
2. Increments the `debounce_generation` counter.
3. Spawns a timer task that sends `Command::DebounceReady { name, generation }`
   after the debounce window expires.
4. Returns `Queued`.

When `debounce_ready` fires:

1. Checks that the `debounce_generation` still matches (i.e. no newer event
   arrived during the window).
2. If matched, takes the retained event and re-enters `admit_event` with
   `bypass_debounce = true` to skip the debounce stage on the second pass.
3. If the generation is stale, the event was superseded and nothing happens.

This ensures that rapid bursts of events produce only a single Flow
execution from the most recent event.

## Stage 3: Rate limiting

When `filter.rate_limit` is set:

1. Prune `rate_history` entries older than `window_ms`.
2. If `rate_history.len() >= max_fires`, the action determines behavior:

| Action | Behavior |
|---|---|
| `Drop` | Return `Suppressed` immediately |
| `Queue` | Push event to `rate_queue` (bounded by queue depth), schedule wakeup |
| `Warn` | Log warning, continue to concurrency stage |

3. If under the limit, record the current timestamp in `rate_history` and
   continue.

### Rate queue drain

When a `RateReady` command arrives:

1. Check that the `rate_generation` matches.
2. Pop one event from `rate_queue`.
3. Re-enter `admit_event` with `bypass_debounce = true`.
4. If the queue is not empty, schedule another wakeup after the remaining
   window duration.

The wakeup delay is computed as `window_ms - elapsed_since_oldest_fire`,
clamped to a minimum of 1ms.

## Stage 4: Concurrency policy

After admission, `apply_concurrency` evaluates the binding's
`ConcurrencyPolicy`:

| Policy | Condition | Action |
|---|---|---|
| `Queue` | Flow running, queue not full | Push to `concurrency_queue`, return `Queued` |
| `Queue` | Flow running, queue full | Emit `Skipped`, return `Suppressed` |
| `Skip` | Flow running | Emit `Skipped`, return `Suppressed` |
| `CancelRunning` | Flow running | Cancel all running Flow tokens, launch new |
| `Parallel` | Under limit | Launch alongside existing Flows |
| `Parallel` | At limit | Emit `Skipped`, return `Suppressed` |
| Any | No Flow running | Launch directly |

Queue limits:

| Setting | Default | Hard maximum |
|---|---|---|
| `Queue.max_depth` | 10 (`DEFAULT_QUEUE_DEPTH`) | 1,024 (`MAX_QUEUE_DEPTH`) |
| `Parallel.max_concurrent` | 16 (`DEFAULT_PARALLEL_LIMIT`) | 64 (`MAX_PARALLEL_LIMIT`) |

### Queue drain

When a `FlowDone` command arrives and `running` becomes empty, the
coordinator pops one event from `concurrency_queue` and calls `launch_flow`
to process the next queued event.

## Stage 5: Flow launch

`launch_flow` performs the final steps:

1. **Trace dedup**: Check `(binding_name, trace_id)` against `seen_traces`.
   Duplicate traces are suppressed.
2. **Scope resolution**: Call `resolve_trigger_execution_scope` to compute
   the capability boundary.
3. **Input mapping**: Apply `map_event_input` to transform the event payload
   according to the binding's `input_mapping`.
4. **Persist**: Write the fired event as a JSON file in
   `.roko/triggers/events/`.
5. **Lifecycle**: Emit `Fired` and `FlowStarted` lifecycle events.
6. **Spawn**: Launch a tokio task that calls
   `runtime.run_trigger_graph_scoped` and sends `FlowDone` on completion.

**Source:** `crates/roko-serve/src/trigger_runtime.rs` (methods `admit_event`,
`apply_concurrency`, `launch_flow`, `debounce_ready`, `rate_ready`, `flow_done`)
