# 112 — Plan Run Continuous Screenshots (`--screenshots` on `plan run`)

**Priority**: P2 — Enables post-hoc visual review of how the TUI evolved during a run, critical for dogfood verification and debugging without requiring an operator to watch live.
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/commands/do_cmd.rs`, `crates/roko-cli/src/tui/snapshot.rs`
**Depends on**: #111 (screenshot command completion — snapshot engine must be stable)
**Sources**: `tmp/backlog/_checklist-gaps.md` §0.3, `tmp/backlog/_mori-old-gaps.md` MO-02

---

## Background

When `roko plan run` executes unattended, there is currently no record of what the TUI displayed at key moments. An operator returning after a multi-hour run sees only the final terminal state and the log files — the visual evolution is lost. For dogfood verification (proving that roko can develop itself), a visual timeline is valuable evidence.

Mori addressed this by capturing screenshots at configurable intervals and on significant events, writing them to `.mori/screenshots/run-<timestamp>/` with a `manifest.json` timeline linking each capture to the event that triggered it. The roko equivalent should do the same through the existing runner event loop.

The approach is event-driven rather than timer-only: gate completions, wave transitions, agent spawns, and error events each trigger a targeted capture of the relevant tabs. This keeps disk usage bounded while capturing the moments that matter.

## Current State

- `crates/roko-cli/src/tui/snapshot.rs` — text rendering engine exists (see #111).
- `crates/roko-cli/src/runner/event_loop.rs` — the runner event loop handles all runner events but does not invoke the snapshot engine.
- `crates/roko-cli/src/runner/structured_log.rs` — `StructuredLogger` exists and hooks into events; the screenshot capture can follow the same pattern.
- `crates/roko-cli/src/commands/do_cmd.rs` — `plan run` CLI args are assembled here; `--screenshots` flag does not exist.
- No `--screenshots` flag on `roko plan run` today.

## Implementation Plan

1. **Add `--screenshots` flag** to the `plan run` subcommand in `crates/roko-cli/src/commands/do_cmd.rs`. Also add `--screenshot-interval <secs>` (default 60) and `--screenshot-dir <path>` (default `.roko/screenshots/run-<timestamp>`).

2. **Create `ScreenshotCollector` struct** in a new file `crates/roko-cli/src/runner/screenshot_collector.rs`:
   - Holds: output directory, interval, snapshot engine handle, event manifest.
   - Two capture modes: `capture_event(event_label, tabs)` for event-triggered captures, `capture_interval(tabs)` for timer-triggered captures.
   - Writes each capture to a subdirectory `<dir>/<n>-<event-label>/` and appends an entry to `manifest.json`.

3. **Wire into event loop**: In `event_loop.rs`, when `--screenshots` is enabled, instantiate `ScreenshotCollector`. Add capture calls at:
   - Runner startup: capture all tabs.
   - `RunnerEvent::TaskCompleted` / `TaskFailed`: capture F1 (dashboard) + F2 (plans).
   - `RunnerEvent::GateCompleted`: capture F1 + F2 + F10 (gates/learn).
   - `RunnerEvent::AgentSpawned` / `AgentExited`: capture F1 + F3 (agents).
   - Plan wave transition (when wave index increments): capture all tabs.
   - Runner shutdown: capture all tabs.

4. **Interval timer**: In the event loop's `tokio::select!`, add a periodic tick (default 60s). On tick, capture all tabs and label as `interval-<n>`.

5. **Smart tab selection**: Each event type maps to a relevant tab subset. Full-tab captures only on startup, wave transitions, and completion. This keeps per-event captures lightweight.

6. **Manifest format**:
   ```json
   {
     "run_id": "...",
     "started_at": "...",
     "captures": [
       {"n": 0, "label": "startup", "tabs": ["dashboard", "plans"], "path": "0-startup/", "timestamp": "..."},
       {"n": 1, "label": "gate-pass-task-01", "tabs": ["dashboard", "plans", "learn"], "path": "1-gate-pass-task-01/", "event": {...}}
     ]
   }
   ```

7. **Disk guard**: Skip capture if free disk space drops below 500 MB (reuse the disk admission check already wired in runner-v2).

## Acceptance Criteria

1. `roko plan run plans/ --screenshots` runs without error and creates `.roko/screenshots/run-<timestamp>/`.
2. At least one capture exists for: startup, each gate completion, and shutdown.
3. `manifest.json` in the output directory is valid JSON with a `captures` array.
4. Captures are skipped gracefully when disk space is low (log a warning, do not crash).
5. `--screenshot-interval 30` triggers interval captures every 30 seconds.
6. The runner's performance is not measurably degraded by screenshot capture (captures must be async / non-blocking).

## Verification Checklist

- [ ] Run a minimal plan with `--screenshots` and verify at least one capture directory exists.
- [ ] Verify `manifest.json` has entries for startup and shutdown.
- [ ] Verify gate pass/fail events produce captures labeled with the gate rung.
- [ ] Simulate low disk (mock the check) and verify captures are skipped with a warning logged.
- [ ] Run with `--screenshot-interval 10` on a 30-second plan and verify three or more interval captures.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/commands/do_cmd.rs` | Add `--screenshots`, `--screenshot-interval`, `--screenshot-dir` flags |
| `crates/roko-cli/src/runner/event_loop.rs` | Instantiate `ScreenshotCollector`; add capture call sites |
| `crates/roko-cli/src/runner/screenshot_collector.rs` | New file: `ScreenshotCollector` struct |
| `crates/roko-cli/src/runner/mod.rs` | Export `screenshot_collector` module |
