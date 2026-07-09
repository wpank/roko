# Dogfood Run 2 — 2026-04-26 09:32

Second run after fixing TUI crash, skip_enrichment, health endpoint, config v1 dedup,
and process group kill.

## What worked

1. **skip_enrichment = true** — No enrichment artifacts written. Agent went straight to
   implementation. Saved ~8 min and ~$0.30.
2. **TUI no longer crashes** — The plan-approval TUI thread runs without panicking. The
   `ws_client` gracefully skips websocket connections when no tokio runtime is available.
3. **Config v1 warning** — Only emitted once at startup, not per-agent.

## Still broken

### S1. No executor.json written during run
`.roko/state/executor.json` still does not exist. State is only in-memory. If the process
crashes mid-run, all progress is lost. The executor snapshot should be written:
- After each phase transition (enrich → implement)
- After each task completion
- On graceful shutdown

### S2. No episodes written during implementation
`episodes.jsonl` still has only 2 entries from April 24-25. The running implementation
agent (PID 90805, alive 1+ min) has not produced any episode records. Token usage from
the current run is invisible.

### S3. No efficiency events
`.roko/learn/efficiency.jsonl` does not exist. Per-turn cost tracking is not happening.

### S4. signals.jsonl stays at 0 lines
The signal log is empty. The plan runner emits conductor signals in-memory
(`emit_tagged_conductor_signal`) but they're not being persisted to disk.

### S5. TUI log is useless
`.roko/tui.log` only contains repeated "TUI file logging enabled" lines (18 entries
across many sessions). No actual TUI events, errors, or state changes are logged.

### S6. mirage-snapshot.json is 344KB and updating
The TUI snapshot file is being written (344KB, updated at 09:33). This is the TUI's
internal state, not the executor's. The data flows through StateHub → TUI but not to
any persisted executor state.

### S7. learn/ files are stale
All files in `.roko/learn/` have timestamps from April 25 or earlier (episodes.jsonl
at 540KB, costs.jsonl at 18KB, etc.). The current run is not writing to any of them.
The learning runtime may not be connected to the plan runner's execution path.

## Process observations

- roko process: PID 90558, 51MB RSS, running 1m+
- claude agent: PID 90805, 238MB RSS, running 42s+ at check time
- Only 1 agent tracked in `agent-pids.json`: `[90805]`
- `experiment-winners.json` updated at 09:33 (content: `{}` — empty)

## Live observations (during run)

- **Agent cycling**: The plan runner spawns agents sequentially. Observed PIDs:
  90805 (42s), 10949 (short-lived), 15695 (running). Each dies and is replaced.
  `agent-pids.json` tracks only one PID at a time.
- **Still 2 episodes** after 3+ minutes of agent work. Confirms episodes are not
  being written during the run.
- **Enrichment artifacts touched** at 09:35 despite `skip_enrichment = true`. Content
  unchanged — likely `ensure_task_tracker` reading them causes mtime update on APFS.
  Not a real re-enrichment.

## Hypothesis

The plan runner's learning runtime (`self.learning`) and episode logger may not be
flushing to disk during the run. The `enrich_completed_run()` and `record_and_check_learning()`
calls might only buffer in memory, with disk writes happening at graceful shutdown (which
never happened in run 1 due to the TUI crash). Worth checking if run 2 completes and
whether episodes/efficiency appear at that point.
