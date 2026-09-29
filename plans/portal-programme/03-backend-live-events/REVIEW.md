# 03-backend-live-events — Live Acceptance Review

Date: 2026-09-28

---

## Full check output

First run (before fix) produced one failure:

```
PASS execute on a directory plan returns 202
PASS the run finishes within 180s
PASS both tasks wrote their artifacts
PASS task events reach the server stream
    before: task_completed plan_id=live-a task_id=T01 at 24, task_started plan_id=live-a task_id=T02 at 23
FAIL status is live: T01 completes before T02 starts
PASS no task is reported complete twice
PASS the agent is visible while it works (heartbeat between spawn and completion)
PASS input tokens are attributed to the task
PASS cost is attributed to the task
PASS snapshot token total is non-zero
PASS snapshot cost total is non-zero
PASS run_completed reports success with elapsed time
LIVE-EVENTS-CHECK: FAIL (1 of 12 checks failed; workspace kept at /private/tmp/roko-live-Dli2KT)
```

### Root cause and fix

`poll_status_changes` (`crates/roko-cli/src/runner/graph_tui_bridge.rs`) iterates a
`HashMap`, whose key ordering is non-deterministic in Rust. When T01 completed and T02
started in the same 100 ms polling tick, the iteration could emit T02's `task_started`
before T01's `task_completed`, reversing the causal order.

**Fix** (`crates/roko-cli/src/runner/graph_tui_bridge.rs`): collect completions and
starts into separate buckets, then emit all completions first and all starts second. In a
serial DAG a predecessor completing is always the cause of any successor starting, so
completions must precede starts when they co-occur in the same tick.

After the fix, the second run produced:

```
PASS execute on a directory plan returns 202
PASS the run finishes within 180s
PASS both tasks wrote their artifacts
PASS task events reach the server stream
PASS status is live: T01 completes before T02 starts
PASS no task is reported complete twice
PASS the agent is visible while it works (heartbeat between spawn and completion)
PASS input tokens are attributed to the task
PASS cost is attributed to the task
PASS snapshot token total is non-zero
PASS snapshot cost total is non-zero
PASS run_completed reports success with elapsed time
LIVE-EVENTS-CHECK: PASS (12 checks)
```

---

## Event evidence (from `KEEP_WS=1` run at `/private/tmp/roko-live-Yo4kLf/events.sse`)

### 1. Did task status arrive as deltas?

Yes. Each task produced exactly one `task_started` and one `task_completed`, published as
the task actually started and finished — not pre-populated at plan start and not batched at
plan end.

T01 started before its first heartbeat:
```
data: {"type":"agent_spawned","agent_id":"live-a/T01","plan_id":"live-a","task_id":"T01","attempt":0,"role":"implementer","model":"claude-sonnet-4-6","provider":"claude_cli"}
data: {"type":"task_started","plan_id":"live-a","task_id":"T01","title":"Fixture task T01","phase":"graph-executing"}
data: {"type":"agent_heartbeat","agent_id":"live-a/T01","plan_id":"live-a","task_id":"T01","elapsed_ms":5002}
```

T01 completed before T02 started:
```
data: {"type":"task_completed","plan_id":"live-a","task_id":"T01","outcome":"passed"}
data: {"type":"critical_path_eta_updated","plan_id":"live-a","eta_minutes":1}
data: {"type":"task_started","plan_id":"live-a","task_id":"T02","title":"Fixture task T02","phase":"graph-executing"}
```

T02 completed before the plan closed:
```
data: {"type":"task_completed","plan_id":"live-a","task_id":"T02","outcome":"passed"}
data: {"type":"plan_completed","plan_id":"live-a","success":true}
```

### 2. Was the agent visible while it worked?

Yes. For T01 (which sleeps 12 seconds via the `SLOW 12` marker), two heartbeats arrive
between `agent_spawned` and `agent_completed`:

```
data: {"type":"agent_spawned","agent_id":"live-a/T01","plan_id":"live-a","task_id":"T01","attempt":0,"role":"implementer","model":"claude-sonnet-4-6","provider":"claude_cli"}
data: {"type":"agent_heartbeat","agent_id":"live-a/T01","plan_id":"live-a","task_id":"T01","elapsed_ms":5002}
data: {"type":"agent_heartbeat","agent_id":"live-a/T01","plan_id":"live-a","task_id":"T01","elapsed_ms":10001}
data: {"type":"agent_completed","agent_id":"live-a/T01","plan_id":"live-a","task_id":"T01","attempt":0}
```

The assertion `sse between agent_heartbeat task_id=T01 -- agent_spawned task_id=T01 --
agent_completed task_id=T01` passed: at least one heartbeat lies between the spawn and
completion events.

### 3. Did the token and cost totals become non-zero?

Yes. Per-task efficiency events were published for both tasks:

```
data: {"type":"efficiency_event","plan_id":"live-a","task_id":"T01","metric":"input_tokens","value":120.0}
data: {"type":"efficiency_event","plan_id":"live-a","task_id":"T01","metric":"output_tokens","value":30.0}
data: {"type":"efficiency_event","plan_id":"live-a","task_id":"T01","metric":"cost_usd","value":0.0012000000569969416}
data: {"type":"efficiency_event","plan_id":"live-a","task_id":"T02","metric":"input_tokens","value":120.0}
data: {"type":"efficiency_event","plan_id":"live-a","task_id":"T02","metric":"cost_usd","value":0.0012000000569969416}
```

The `GET /api/statehub/snapshot` taken after the run confirmed the totals were folded into
the state:

```json
"stats": { "total_input_tokens": 240, "cost_usd_total": 0.002400000113993883 }
```

The terminal event confirmed a non-zero elapsed time:

```
data: {"type":"run_completed","outcome":"succeeded","duration_ms":12666,"cleanup_degraded":false,"surviving_agent_ids":[],"surviving_agent_pids":[]}
```

---

## What remains out of scope: transcript streaming

The agent_output events carry the final screened result only — not a live stream of tool
calls or intermediate tokens. This is a safety decision, not a wiring gap.

Every provider agent is wrapped by the immune boundary
(`crates/roko-agent/src/immune_boundary.rs`), which lets only the screened final body
cross. Its `run_streaming` implementation explicitly states "Provider events are not
replayed". The Claude CLI adapter has no `run_streaming` override. Streaming the raw
transcript before the immune boundary screens it requires a product decision about trust
boundaries (see T07 for the decision item to be created).

---

## Verdict

```
LIVE-EVENTS-CHECK: PASS (12 checks)
```
