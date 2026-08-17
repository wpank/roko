# ACP Stability Hardening

**Priority**: P0 — crashes in production, concurrency bugs, silent failures
**Size**: L (5–7 days)
**Crate**: `crates/roko-acp/` (19,915 LOC, 15 modules)

## Source Analysis

This document consolidates findings from the 2026-08-15 audit of `roko-acp`.
The detailed per-line findings live in:

- `tmp/archive/08-15-26/acp-todos/01-PANIC-AND-ERROR-FIXES.md` — 35 findings, 7 P0 crashes in `bridge_events.rs`
- `tmp/archive/08-15-26/acp-todos/02-OTHER-MODULE-ERROR-FIXES.md` — 14 findings across 12 non-bridge modules
- `tmp/archive/08-15-26/acp-todos/07-CONCURRENCY-ISSUES.md` — 12 issues (race conditions, lock contention, channel problems)
- `tmp/archive/08-15-26/acp-todos/08-CODE-QUALITY.md` — clippy blockers, oversized functions, suppression audit

## What Exists

- Full `roko-acp` crate compiles on stable toolchain and passes 180 ACP tests
- Error types use `thiserror` consistently; no raw `unwrap()` calls anywhere in production code
- Permission flow is correctly fail-closed on all timeout/cancel/disconnect paths (P3, no action needed)
- MCP server discovery correctly degrades gracefully with `McpServerStatus::failed(...)` forwarded to editor
- `BridgeEventsError` propagates from all public entry points back to the JSON-RPC error layer
- Existing test harness: 164 inline tests + 16 external tests across 3 test files

## What Is Missing / Broken

### Section A: P0 — Server Process Crashes

Seven `.expect()` and `unreachable!()` calls in production paths in `bridge_events.rs` will
terminate the ACP server process (not just the current request). All must be replaced with
`BridgeEventsError` or `anyhow::Error` returns before the next Zed/Cursor test session.

**A1. `provider_health_registry` panic** (`bridge_events.rs:1675`)

```rust
session
    .provider_health_registry
    .as_ref()
    .expect("ACP provider health initialized before prompt")
```

Called from `handle_session_prompt_inner`. If initialization races or fails silently, the ACP
process dies mid-prompt. Fix: return `BridgeEventsError::Pipeline(anyhow!("..."))`.

**A2. `provider_rate_limiter` panic** (`bridge_events.rs:1681`)

Same function as A1. Same pattern. Same fix.

**A3. Slash-command stdout not piped** (`bridge_events.rs:4923`)

```rust
let stdout = child.stdout.take().expect("stdout was piped");
let stderr = child.stderr.take().expect("stderr was piped");
```

Production code in the slash-command dispatch path. Any refactor of the `Command` construction
that omits `Stdio::piped()` crashes the server mid-prompt. Fix: return `Err(anyhow!(...))` on
`None`.

**A4. Shell-command stdout not piped** (`bridge_events.rs:5241`)

Identical pattern to A3 in a second shell-command dispatch path. Same fix.

**A5. At-mention char boundary** (`bridge_events.rs:5753`)

```rust
let ch = text[end..].chars().next().expect("valid char boundary");
```

Parsing user-provided prompt text. The loop invariant makes this theoretically safe, but
user-input parsers must never panic. Fix: `unwrap_or(' ')` or `let Some(ch) = ... else { break; }`.

**A6. Unique tool name exhaustion** (`bridge_events.rs:3694`)

```rust
unreachable!("suffix search should always find a unique tool name")
```

A malicious or buggy MCP server registering 1000+ identically-named tools hits this. Fix:
return an `Err(...)` instead of `unreachable!`.

**A7. Unhandled `CognitiveEvent` variant** (`bridge_events.rs:5369`)

```rust
CognitiveEvent::Complete { .. } | CognitiveEvent::Failure { .. } | ... => {
    unreachable!("terminal/async cognitive events are handled before update mapping")
}
```

If a new `CognitiveEvent` variant is added without updating the stream handler, this panics
at runtime. Fix: replace with `warn!` + return a no-op `SessionUpdate` (empty `AgentMessageChunk`),
or return `Option<SessionUpdate>` and skip sending.

**Estimated effort for A1–A7**: ~1 hour total. All are 3–10 minute mechanical substitutions.

---

### Section B: P1 — Silent Failures (Data Loss / Incorrect Behavior)

These do not crash the process but silently discard data, produce wrong results, or hide errors
from the user and the system. They should be fixed in the same batch as the P0 items.

**B1. Reviewer failure treated as approved** (`runner.rs:1839`)

```rust
Err(e) => {
    warn!(error = %e, "reviewer failed, treating as approved");
    run.pipeline.step(PipelineEvent::ReviewApproved { ... })
}
```

A code-review agent crash silently auto-approves the submission. Fix: emit a visible
`CognitiveEvent::TokenChunk` warning, and treat as `ReviewRevise` with a "could not complete"
finding so the pipeline retries or the user is explicitly notified.

**B2. Architect reviewer failure not tracked** (`runner.rs:1909`) and
**B3. Auditor reviewer failure not tracked** (`runner.rs:1936`)

In `run_thorough_review`, both reviewer legs can fail while `all_approved` stays `true`
(it only flips to `false` on non-approved output, not on errors). Fix: initialize
`all_approved = false` when any reviewer errors, or accumulate a `reviewer_error` finding.

**B4. Adaptive gate threshold save failure** (`runner.rs:2143`)

Threshold learning data is silently lost on disk write failure. Gate thresholds regress to
defaults over time. Fix: retry once, then emit a structured warn so the user knows learning
data is not persisting.

**B5. Session persistence failure** (`session.rs:1317–1330`)

Three separate silent failure paths (directory creation, serialization, file write) when
persisting session state. Session history, config, and cost tracking are lost. Fix: return
`Result` from `persist_session` and let `handler.rs` surface the error.

**B6. Workspace trust deserialization swallowed** (`session.rs:519–521`)

```rust
std::fs::read_to_string(&path)
    .ok()
    .and_then(|data| serde_json::from_str(&data).ok())
    .unwrap_or_default()
```

If `permissions.json` is corrupt, the user re-approves every tool action with no explanation.
Fix: log a `warn!` when the file exists but parse fails.

**B7. Config option serialization failure** (`handler.rs:209`, `447`)

```rust
serde_json::to_value(&options).unwrap_or_else(|_| serde_json::json!([]))
```

IDE config dropdowns silently disappear if serialization fails. Fix: log in the
`unwrap_or_else` closure.

**B8. Invalid pipeline state transition returns Done** (`pipeline.rs:371–378`)

```rust
(phase, event) => {
    tracing::warn!(..., "unexpected pipeline event");
    PipelineAction::Done
}
```

A logic bug causes the pipeline to silently terminate with a false-success result. Fix: return
`PipelineAction::Halt { reason }` so the pipeline reports a halted (not completed) state.

**B9. ACP event forwarding drops silently** (`acp_adapter.rs:164`)

```rust
let _ = self.sender.try_send(cognitive_event);
```

Agent output, gate results, and completion signals are dropped when the channel is full,
leaving the IDE showing stale/incomplete progress. Fix: log on drop, consider increasing
buffer or using bounded backpressure.

**B10–B12. Cost, efficiency, and experiment recording failures** (multiple locations
in `bridge_events.rs`: lines 631–651, 1148–1154, 1752–1759, 2306–2314)

All are correctly warn-logged but no counter is incremented. Over time, cost budgets and
experiment statistics drift silently from reality. Fix: increment a metric counter at minimum.

**Estimated effort for B1–B12**: ~3 hours total.

---

### Section C: Race Conditions

**C1. Session busy check-then-act TOCTOU** (`bridge_events.rs:1638–1643`)

```rust
if session.is_busy() { ... }             // LOAD
session.ensure_provider_runtime(...);    // gap
session.begin_prompt();                  // STORE
```

The check and set are not atomic. Currently mitigated by the sequential handler loop,
but `session.cancel()` (which sets `busy = false`) can be called from the notification
handler path concurrently. Fix: replace with an atomic `compare_exchange`:

```rust
pub fn try_begin_prompt(&mut self) -> bool {
    let was_idle = self.busy.compare_exchange(
        false, true, Ordering::AcqRel, Ordering::Acquire,
    ).is_ok();
    if was_idle { self.cancel_token = CancelToken::new(); }
    was_idle
}
```

See `tmp/archive/08-15-26/acp-todos/07-CONCURRENCY-ISSUES.md` issue #3 for full diff.

**C2. CancelToken replacement race** (`session.rs:647–655`)

If `cancel()` fires between `begin_prompt()` creating the new token and the cognitive
task capturing it, the new prompt is immediately cancelled or the old cancellation is
lost. Fix: monotonic generation counter — `cancel()` only fires if the generation
matches the in-flight prompt's generation.

See `07-CONCURRENCY-ISSUES.md` issue #4 for the full interleaving analysis.

**C3. Cascade router read/write race** (`bridge_events.rs:1031` vs `1128–1156`)

The read path (`cascade_select_model`) calls `CascadeRouter::load_or_new()` without
holding `CASCADE_ROUTER_IO_LOCK`. A concurrent writer in `record_cascade_observation`
may be mid-`save()`, yielding a truncated JSON read and a silently-reset router. Fix:
use atomic file writes (`write-to-temp + rename`) in `CascadeRouter::save()` so readers
always see a complete file, OR guard the read under the same lock as `RwLock` read guard.

---

### Section D: Concurrency Bottlenecks (Async Safety)

**D1. `std::sync::Mutex` held during file I/O on tokio thread** (`bridge_events.rs:782–787`)

`assign_acp_experiment()` acquires `EXPERIMENT_STORE_IO_LOCK` (a `std::sync::Mutex`) and
then reads a JSON file from disk — while running on a tokio worker thread. This blocks the
worker thread and violates tokio's cooperative scheduling contract. Fix: wrap in
`tokio::task::spawn_blocking()` (consistent with `record_cascade_observation`).

**D2. Global `Mutex<()>` serializes all cascade router I/O** (`bridge_events.rs:152`)

`CASCADE_ROUTER_IO_LOCK` is a process-wide `std::sync::Mutex` that serializes every cascade
router disk write across all concurrent ACP sessions. Fix: move to a dedicated actor (channel
+ `spawn` task) that owns the file, or switch to `tokio::sync::RwLock<()>` with the read
path also guarded.

**D3. Cognitive event channel capacity causes backpressure stalls** (`bridge_events.rs:1864`)

```rust
let (event_sender, event_receiver) = mpsc::channel(64);
```

Capacity of 64 events. If the editor reads stdout slowly, the cognitive task blocks on
`event_sender.send()`, which can trigger provider-side timeouts. Meanwhile
`AcpWorkflowEventConsumer::publish()` uses `try_send` which silently drops events on full
buffer. Fix: increase channel capacity to 256+, add `warn!` on drop in `publish()`, and
consider unbounded channel for terminal events (Complete, Failure).

**D4. Permission reply channel spin-polls a `std::sync::Mutex`** (`bridge_events.rs:222–269`)

`receiver_is_closed()` acquires `std::sync::Mutex` from an async context every 25ms.
Fix: replace with `tokio::sync::watch` channel so the async poller can `.changed().await`
without the sleep-poll.

**D5. `workflow_cost_sink` uses `std::sync::Mutex` across spawn boundary** (`bridge_events.rs:1961`)

Written from a `tokio::spawn` task and read in the parent context. Fix: replace with
`tokio::sync::oneshot` — the spawned task sends the cost once, the parent `.await`s it.

---

### Section E: Upstream Compile Blockers (must fix before clippy runs on roko-acp)

Clippy cannot currently run against `roko-acp` because two errors in `roko-agent` block
the entire dependency graph:

```
error[E0425]: cannot find type `Command` in this scope
 --> crates/roko-agent/src/harness/child_process_runner.rs:35:35
     pub fn apply(&self, cmd: &mut Command) {

error[E0015]: cannot call non-const operator in constant functions
 --> crates/roko-agent/src/process/limits.rs:48:16
     || self.network == ProviderNetworkPolicy::Deny
```

Plus 3 unused-import warnings in `cursor_cli_agent.rs`, `exec.rs`, `openclaw/probe.rs`.

These must be fixed first. After fixing, run:

```bash
cargo clippy -p roko-acp --no-deps -- -D warnings
```

---

## Acceptance Criteria

- [ ] All 7 P0 `.expect()` / `unreachable!()` calls in production paths replaced with error returns; no production code panics on malformed input
- [ ] `run_thorough_review` does not auto-approve when any reviewer errors; a visible finding is emitted
- [ ] `try_begin_prompt()` uses `compare_exchange` — no TOCTOU window between busy check and set
- [ ] `CancelToken` replacement is generation-gated — stale `cancel()` calls cannot affect a new prompt
- [ ] `CascadeRouter::save()` uses atomic write (temp + rename) — readers never see a partial file
- [ ] `assign_acp_experiment()` runs inside `spawn_blocking` — no blocking I/O on tokio worker threads
- [ ] `EXPERIMENT_STORE_IO_LOCK` and `CASCADE_ROUTER_IO_LOCK` do not serialize reads and writes together; read path is guarded or file writes are atomic
- [ ] Cognitive event channel capacity raised; `publish()` logs on drop instead of silently discarding
- [ ] Upstream `roko-agent` compile errors resolved; `cargo clippy -p roko-acp --no-deps -- -D warnings` passes clean
- [ ] All existing 180 ACP tests continue to pass after changes
- [ ] `cargo test -p roko-acp` passes; no new test regressions

## Prerequisites

- Rust 1.91+ (alloy dependency floor; the 2026-08-16 release used 1.96.1)
- `roko-agent` compile errors must be fixed before running clippy on `roko-acp`
- Read `tmp/archive/08-15-26/acp-todos/07-CONCURRENCY-ISSUES.md` before touching `session.rs:647–666` — the atomics ordering analysis there documents why the current Acquire/Release pairing on `busy` does not establish happens-before for `cancel_token`

## Not in Scope

- The `bridge_events.rs` module split (covered in backlog/18)
- Spec version bump to v0.13.6 (covered in backlog/18)
- Integration gaps (TUI visibility, roko-serve routes, force_backend learning) — covered in backlog/18
- Editor-specific config generation helpers (`roko acp --emit-config jetbrains`) — covered in backlog/18
- P3 / no-fix-needed findings from `01-PANIC-AND-ERROR-FIXES.md`: permission flow warn-and-Reject patterns (issues 27, 29), MCP server discovery warn patterns (issue 26), slash command stdout/stderr read errors (issue 33), file context resolution warnings (issue 34), and event-stream-closed-without-completion (issue 28) are all correct defensive behavior and require no changes
