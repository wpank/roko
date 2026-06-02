# Streaming Output Design for `plan run`

**Date**: 2026-05-04
**Status**: In progress

---

## Problem

When `roko plan run` executes, the user sees no output until the entire plan completes.
Events flow through the internal StateHub (for TUI and HTTP consumers) but nothing is
written to the terminal. For CLI users without the TUI or dashboard, this means minutes
of silence during agent execution.

---

## Current Architecture

```
Agent process (claude/openai)
  │
  ├── stdout/stderr → agent_stream.rs parses JSON events
  │                     │
  │                     ├── AgentEvent::TurnStarted
  │                     ├── AgentEvent::ContentDelta(text)
  │                     ├── AgentEvent::ToolUse { name, input }
  │                     ├── AgentEvent::TurnCompleted { usage }
  │                     └── AgentEvent::Error(msg)
  │
  ├── event_loop.rs receives events via mpsc channel
  │     │
  │     ├── Updates internal RunState (task status, costs, timing)
  │     ├── Sends DashboardEvent to StateHub (watch::Sender)
  │     └── Returns final RunReport when all tasks done
  │
  └── StateHub consumers:
        ├── TUI (ratatui) — polls watch::Receiver
        ├── SSE endpoint (/events) — streams to browser
        └── WebSocket (/ws) — streams to browser
```

The gap: **nothing writes to stderr for direct CLI usage.** The event_loop processes
events silently and only prints a summary at the end.

---

## New Design: stderr Streaming

Add a stderr output layer in `event_loop.rs` that prints task lifecycle events and
optionally agent output as they happen. Controlled by a `stream_to_stderr` flag in
`RunConfig` (defaults to `true` when stdout is a TTY and no TUI is active).

### Output Format

Task lifecycle events use a structured prefix format:

```
[plan-run] Starting task 1/4: "Create roko-foo crate skeleton" (implementer)
[plan-run] Task 1/4 agent running...
[plan-run]   > Creating directory structure for roko-foo
[plan-run]   > Writing src/lib.rs with module stubs
[plan-run]   > Adding roko-foo to workspace Cargo.toml
[plan-run] Task 1/4 gate: rung 0 (compile) — PASS
[plan-run] Task 1/4 gate: rung 1 (clippy) — PASS
[plan-run] Task 1/4 COMPLETED (45s, $0.03)

[plan-run] Starting task 2/4: "Implement Signal trait for FooSignal" (implementer)
[plan-run] Task 2/4 agent running...
[plan-run]   > Reading existing Signal implementations for reference
[plan-run]   > Implementing FooSignal struct with required methods
[plan-run] Task 2/4 gate: rung 0 (compile) — PASS
[plan-run] Task 2/4 gate: rung 1 (clippy) — PASS
[plan-run] Task 2/4 gate: rung 2 (test) — PASS
[plan-run] Task 2/4 COMPLETED (62s, $0.05)

[plan-run] Starting task 3/4: "Research best practices" (researcher)
[plan-run] Task 3/4 agent running...
[plan-run] Task 3/4 COMPLETED (auto-pass gates, read-only role) (30s, $0.02)

[plan-run] Starting task 4/4: "Write integration tests" (implementer)
[plan-run] Task 4/4 agent running...
[plan-run] Task 4/4 gate: rung 0 (compile) — PASS
[plan-run] Task 4/4 gate: rung 1 (clippy) — PASS
[plan-run] Task 4/4 gate: rung 2 (test) — FAIL (3 tests failed)
[plan-run] Task 4/4 FAILED (90s, $0.04)
[plan-run]   error: tests::foo_signal_roundtrip — assertion failed
[plan-run]   (full output in .roko/roko.log)
```

### Agent Content Lines

Agent content deltas are optionally streamed with a `>` prefix. This is controlled
separately from task lifecycle output because agent output can be very verbose.

- **Default (stream_agent_output = false)**: Only task lifecycle lines shown
- **Verbose (stream_agent_output = true, or --verbose)**: Agent content deltas included

### Color

When stderr is a TTY:
- Task start: dim/gray
- PASS: green
- FAIL: red
- Costs: dim/gray
- Agent content: dim cyan with `>` prefix

When stderr is not a TTY (piped): no ANSI codes.

---

## Implementation Details

### 1. Add `stream_to_stderr` field to `RunConfig`

```rust
// runner/mod.rs or wherever RunConfig lives
pub struct RunConfig {
    // ... existing fields ...
    pub stream_to_stderr: bool,
    pub stream_agent_output: bool,
}
```

Default: `stream_to_stderr = atty::is(atty::Stream::Stderr) && !tui_active`,
`stream_agent_output = false` (enable with `--verbose`).

### 2. Add stderr streaming to `handle_agent_event`

In `event_loop.rs`, the match on `AgentEvent` variants already processes each event.
Add stderr writes alongside the existing StateHub sends:

```rust
AgentEvent::TurnStarted { task_id } => {
    if config.stream_to_stderr {
        eprintln!("[plan-run] Starting task {}/{}: {:?} ({})",
            task_num, total, task.title, task.role);
    }
    // ... existing StateHub send ...
}

AgentEvent::ContentDelta(text) => {
    if config.stream_to_stderr && config.stream_agent_output {
        // Print non-empty lines with > prefix
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            eprintln!("[plan-run]   > {}", truncate(line, 120));
        }
    }
}

AgentEvent::TurnCompleted { usage, .. } => {
    if config.stream_to_stderr {
        eprintln!("[plan-run] Task {}/{} COMPLETED ({}s, ${:.2})",
            task_num, total, elapsed.as_secs(), cost);
    }
}
```

### 3. Add task lifecycle printing to `event_loop.rs`

Gate results also need printing. The gate dispatch path already sends
`GateResult` back through the channel — add stderr output there:

```rust
GateResult::Passed { rung, .. } => {
    if config.stream_to_stderr {
        eprintln!("[plan-run] Task {}/{} gate: rung {} ({}) -- PASS",
            task_num, total, rung.index, rung.name);
    }
}

GateResult::Failed { rung, reason, .. } => {
    if config.stream_to_stderr {
        eprintln!("[plan-run] Task {}/{} gate: rung {} ({}) -- FAIL ({})",
            task_num, total, rung.index, rung.name, reason);
    }
}
```

### 4. Wire from `commands/plan.rs`

The `plan run` command constructs the `RunConfig`. Set the streaming fields
based on CLI flags and TTY detection:

```rust
let config = RunConfig {
    stream_to_stderr: !args.quiet && atty::is(atty::Stream::Stderr),
    stream_agent_output: args.verbose,
    // ... other fields ...
};
```

---

## Non-Goals

- **stdout**: All streaming goes to stderr. stdout is reserved for structured output
  (JSON reports, plan validation results) so it can be piped.
- **TUI integration**: The TUI already has its own rendering via StateHub. This design
  is for non-TUI CLI usage only.
- **Buffering**: Lines are written immediately (no buffering beyond what eprintln provides).
  This is intentional — the user should see output as it happens.

---

## Files to Modify

| File | Change |
|------|--------|
| `runner/mod.rs` | Add `stream_to_stderr`, `stream_agent_output` to RunConfig |
| `runner/event_loop.rs` | Add eprintln calls in event handlers |
| `commands/plan.rs` | Set streaming config from CLI args + TTY detection |
| `Cargo.toml` (roko-cli) | Add `atty` dep if not already present |
