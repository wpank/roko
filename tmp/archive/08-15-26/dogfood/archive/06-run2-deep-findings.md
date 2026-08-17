# Dogfood: Run 2 Deep Findings — 2026-04-26

Observations from monitoring the second run of `roko plan run --approval` after
TUI crash fix, skip_enrichment, and health endpoint patches.

## F1. skip_enrichment did NOT work — wrong plans_dir resolution

**Root cause found**: `ensure_task_tracker()` uses `plans_dir()` which resolves to
`plans/` (top-level) when that directory exists. The actual tasks.toml is at
`.roko/plans/unified-migration-phase0/tasks.toml`. The tracker was never loaded,
so `skip_enrichment = true` was never read.

Same bug in `dispatch_agent_with()` at line 13879 — task definitions also not found.

**Fix applied**: Both `ensure_task_tracker()` and `dispatch_agent_with()` now check
both `plans_dir()` and `.roko/plans/` as candidates.

## F2. Model routing falls back to haiku

Routing log shows:
```
requested_model: "claude-sonnet-4-6"
selected_model: "claude-haiku-4-6"
routing_reason: "fallback"
candidates: [{ model: "claude-haiku-4-6", provider: "claude_cli" }]
```

The CascadeRouter only has haiku as a candidate. Sonnet is requested but not in
the candidate list. This means the provider/model discovery doesn't find sonnet
via the claude_cli backend.

**Impact**: All agents run on haiku instead of sonnet, producing lower quality
output and more failures.

## F3. AgentOutput never emitted — TUI shows nothing

`ServerEvent::AgentOutput` type exists with all conversion plumbing, but was never
called anywhere. The TUI's agent output pane stays empty.

**Fix applied**: Added `emit_server_event(ServerEvent::AgentOutput {...})` at the
end of `dispatch_agent_with()` after the agent returns. Emits final output text
with model, tokens, and cost metadata.

## F4. TaskState lacks title — TUI shows "plan plan"

`DashboardEvent::TaskStarted` and `TaskState` only had `task_id`, no human-readable
title. The TUI fell back to showing `task_id` (which is often just "plan" or similar).

**Fix applied**: Added `title: String` field to both `TaskState` and
`DashboardEvent::TaskStarted`. TUI now prefers `title` when non-empty.

## F5. Memory leak — 9.5GB RSS after 17 minutes

Roko process at 9.5GB (14.1% of 64GB) after 17 minutes of running with only 3
agent dispatches (all enrichment). No active child processes, no agents running.
The process seems stalled — enrichment is done but implementation hasn't started.

Possible causes:
- Enrichment pipeline accumulates large strings/artifacts in memory
- TaskTracker/executor state never garbage collected
- The full enrichment artifacts (rubric.md at 119KB, etc.) held in memory
- Debug build with optimized profile may have different memory characteristics

## F6. Implementation phase never dispatches

Routing log shows 3 entries, ALL for `task_id: "enrich"`. Zero implementation
dispatches. After enrichment completes, the state machine should transition to
implementing and start dispatching tasks. This isn't happening.

Likely related to F1 — without a task tracker loaded, the state machine can't
find ready tasks and the implementation phase has nothing to dispatch.

## F7. agent-pids.json goes empty during run

`agent-pids.json` shows `[]` — no tracked agents — even while the process is
running. PIDs were tracked earlier (saw `[90805]`, `[10949]`, `[15695]`) but
are now empty. The PID tracker cleans up dead agents but doesn't show what
happened.

## F8. No persistence during run (confirmed)

After 17 minutes:
- `episodes.jsonl`: 2 lines (stale from April 24-25)
- `signals.jsonl`: 0 lines
- `efficiency.jsonl`: does not exist
- `executor.json`: does not exist
- `routing.jsonl`: 3 entries (only enrichment dispatches)

The only file actively written is `mirage-snapshot.json` (TUI state) and
`routing.jsonl` (routing decisions).

## F9. TUI log bar garbled

The bottom log bar shows overlapping/garbled text. Likely cause: tracing
subscriber writes directly to stderr while the TUI has terminal raw mode
active. The tracing output and ratatui rendering fight over the terminal.

Mori solved this by routing all logs through a ring buffer displayed in the
TUI's log tab, not to stderr.

## Fixes Applied This Session

| Fix | File(s) | Issue |
|-----|---------|-------|
| plans_dir fallback in ensure_task_tracker | orchestrate.rs | F1 — skip_enrichment not working |
| plans_dir fallback in dispatch_agent_with | orchestrate.rs | F1 — task defs not found |
| Emit AgentOutput after dispatch | orchestrate.rs | F3 — TUI output pane empty |
| TaskState.title field | dashboard_snapshot.rs, state.rs | F4 — "plan plan" display |
| TaskStarted.title field | dashboard_snapshot.rs + 9 files | F4 — title propagation |
