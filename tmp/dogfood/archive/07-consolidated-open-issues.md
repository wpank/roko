# Consolidated Open Issues — 2026-04-26

All unresolved issues from dogfood runs 1 & 2, plus new observations from run 3 (the hung process).

## Critical (blocks basic functionality)

### C1. No executor.json persistence during run
- **Source**: 02 #5, 04 S1, 06 F8
- **Symptom**: `.roko/state/executor.json` never written. Crash = total state loss, can't resume.
- **Root cause**: State only persisted on graceful shutdown (`save_state_to` in `RunExit::Signaled`/`SignalTimedOut`). Normal run completion and phase transitions don't persist.
- **Fix**: Call `save_state_to()` after each phase transition in the main loop.

### C2. Episodes not written during run
- **Source**: 01 #3, 02 #6, 04 S2, 06 F8
- **Symptom**: `episodes.jsonl` stays stale (only 2 entries from prior runs). Running agents produce zero episodes.
- **Root cause**: Episode logger buffers in memory, only flushed at shutdown. Implementation dispatch may not even create episode records.
- **Fix**: Flush episodes to disk after each agent dispatch completes.

### C3. Efficiency events not emitted
- **Source**: 02 #7, 04 S3, 06 F8
- **Symptom**: `.roko/learn/efficiency.jsonl` doesn't exist. `/api/learn/efficiency` returns zeros.
- **Root cause**: Efficiency event emission wired but never triggered during plan execution.
- **Fix**: Emit efficiency events with token counts/cost after each agent dispatch.

### C4. TOML parse failures on markdown-fenced LLM output
- **Source**: 02 #2, screenshot from run 3
- **Symptom**: "invalid table header", "expected newline, `#`" — agent returns TOML wrapped in ```toml fences.
- **Root cause**: No markdown fence stripping before TOML parse in enrichment pipeline.
- **Fix**: Strip ```toml ... ``` fences before parsing.

### C5. force_shutdown() kills self via `kill(0, SIGTERM)`
- **Source**: Run 3 observation, code analysis
- **Symptom**: After drain timeout, roko sends SIGTERM to its own process group. Second SIGTERM has no handler (first was consumed), so process dies mid-cleanup. Terminal appears hung.
- **Root cause**: `force_shutdown()` line 5317 calls `libc::kill(0, libc::SIGTERM)` without masking self.
- **Fix**: Mask SIGTERM before sending group signal, or use `killpg` targeting child pgroup, or set SIG_IGN before the kill.

### C6. Model routing falls back to haiku for everything
- **Source**: 06 F2
- **Symptom**: Requested "claude-sonnet-4-6" but selected "claude-haiku-4-6" with reason "fallback".
- **Root cause**: Cascade router candidate list only contains haiku; sonnet not discovered via claude_cli backend.
- **Fix**: Ensure configured models are included in candidate list regardless of backend discovery.

## High (degrades experience significantly)

### H1. Memory leak — 9.5-11.5GB RSS
- **Source**: 06 F5, run 3 (11.5GB)
- **Symptom**: RSS grows to 9-11GB after ~17 minutes with only a few agent dispatches.
- **Root cause**: Likely enrichment artifacts (full LLM output strings) accumulated in TaskTracker/executor state. No GC or release of completed task artifacts.
- **Fix**: Drop enrichment artifact strings after use; cap retained output size; investigate with DHAT.

### H2. Implementation phase never dispatches
- **Source**: 06 F6
- **Symptom**: Zero implementation dispatches after enrichment. All routing entries show `task_id: "enrich"`.
- **Root cause**: Task tracker not loaded correctly (related to plans_dir resolution bug F1). State machine can't find ready tasks. F1 fix was partially applied — need to verify it works end-to-end.
- **Fix**: Verify task tracker loads from correct plans_dir and transitions enriched tasks to implementing.

### H3. signals.jsonl stays empty
- **Source**: 04 S4, 06 F8
- **Symptom**: 0 lines during run. Prior runs had 255 signals.
- **Root cause**: Signal substrate writes may be buffered or the plan runner path doesn't emit signals.
- **Fix**: Ensure agent results produce signals and flush to disk.

### H4. Learning state not updated during run
- **Source**: 04 S7, 06 F8
- **Symptom**: `learn/` files have timestamps from April 25, current run doesn't update them.
- **Root cause**: Learning persistence (gate thresholds, cascade router, experiments) only happens in `shutdown()`.
- **Fix**: Periodic flush of learning state, or at least after each task completion.

## Medium (UX issues)

### M1. TUI log bar garbled
- **Source**: 06 F9
- **Symptom**: Overlapping/garbled text in bottom log bar.
- **Root cause**: tracing subscriber writes to stderr while TUI has raw mode active.
- **Fix**: Route tracing output through TUI's log tab ring buffer instead of stderr.

### M2. No codex backend support
- **Source**: 02 #11
- **Symptom**: Only claude_cli backend works. Can't dispatch to codex.
- **Root cause**: Agent dispatcher only implements claude_cli path for plan execution.
- **Status**: Lower priority — claude works, codex is secondary.

### M3. Enrichment timeout too short
- **Source**: 02 #3
- **Symptom**: 120s timeout kills enrichment agents mid-work.
- **Root cause**: Hardcoded 120s timeout doesn't account for large plans.
- **Fix**: Make timeout configurable, default higher for enrichment (300s+).

### M4. Agent exit signals not captured
- **Source**: 02 #4
- **Symptom**: Agent killed, no diagnostic info about why.
- **Root cause**: stderr not captured, exit signal not reported.
- **Fix**: Capture and log child stderr and exit status on failure.

### M5. agent-pids.json goes empty while agents running
- **Source**: 06 F7
- **Symptom**: `[]` in pids file even with active child processes.
- **Root cause**: Tracker cleans up dead entries but race condition or premature cleanup.
- **Fix**: Investigate lifecycle; ensure cleanup only after confirmed exit.

## Resolved (7 issues — see archive/)
- TUI crash on plan-approval thread → ws_client.rs guard
- Running plan invisible to TUI → --approval flag + shared StateHub
- Enrichment too aggressive → skip_enrichment flag
- StateHub not exposed via HTTP → /api/statehub/snapshot
- No health endpoint → GET /health
- Config v1 warnings spam → std::sync::Once
- Ctrl+C zombie processes → SHUTDOWN_DRAIN_GRACE_SECS=3 + process group kill
