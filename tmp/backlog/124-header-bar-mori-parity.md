# 124 — Header Bar Mori Parity (Dense Single-Row Layout)

**Priority**: P2 — The current TUI header shows stale or zero data (documented in backlog #108), and its layout is less information-dense than Mori's production header, which operators rely on for at-a-glance status.
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/tui/tabs.rs`, `crates/roko-cli/src/tui/app.rs`
**Depends on**: #117 (wave computation — the wave indicator requires it), #123 (ROSEDUST palette for gradient progress bar)
**Sources**: `tmp/backlog/_checklist-gaps.md` §2.4, `tmp/backlog/_mori-old-gaps.md` MO-09

---

## Background

Mori's header bar packed the following into a single row, balanced for 120-character terminals:

```
● ROKO  Wave 2/5  queue-name  ████████████░░░ 73%  ETA 1h23m  Elapsed 2h41m  MCP:3  CPU:28% MEM:1.2G NET:↑12k  [F1]Dash [F2]Plans [F3]Agents...
```

Elements:
- **Pulsing dot** (● or ○, toggling each tick) — visual heartbeat showing the process is alive.
- **Wave indicator** (`Wave N/M`) — current and total waves (requires #117).
- **Queue/milestone name** — active milestone name from queue.toml (requires #116) or default `"plans/"`.
- **15-char gradient progress bar** — proportional completion, HSV green-to-red.
- **ETA** — computed from critical path remaining minutes.
- **Elapsed** — time since `roko plan run` started.
- **MCP count** — number of active MCP tool calls in the last 60 seconds.
- **System metrics** — compact CPU%, memory used, network I/O, disk writes.
- **F-key tab strip** — abbreviated tab names with active tab highlighted.

Roko's current header (in `tabs.rs`) shows the app name and tab strip but none of the operational metrics. Backlog #108 (TUI Live Feedback Gaps) documents that all header metrics show zero.

## Current State

- `crates/roko-cli/src/tui/tabs.rs` — header rendered here; shows app name and F-key strip.
- `crates/roko-cli/src/tui/app.rs` — `TuiState` contains some metrics but they are not reliably populated.
- `DashboardSnapshot` — carries `elapsed_secs`, `plans_total`, `plans_completed`, but ETA and wave are absent.
- System metrics (CPU, memory): `sysinfo` crate is a dependency; the TUI module may already call it (check `tui/app.rs`). If not, it needs wiring.
- Pulsing dot: not implemented.
- Gradient progress bar: not implemented (requires #123).

## Implementation Plan

1. **Pulsing heartbeat**: Add a `tick_count: u64` to `TuiModel` that increments each render frame. Use `tick_count % 2 == 0` to toggle between `●` and `○`.

2. **Wave indicator**: Source from `PlanDag` wave data (requires #117). Until #117 is done, show `Wave ?/?` as a placeholder.

3. **Gradient progress bar**: Use `theme::progress_color(completed_fraction)` (requires #123). The bar is 15 chars wide: `n` filled chars + `15-n` empty chars, colored by fraction.

4. **ETA calculation**: `(total_tasks - completed_tasks) * avg_task_minutes`. Expose `avg_task_minutes` from efficiency events in `TuiModel`. Show `ETA -` if no data available.

5. **Elapsed**: `started_at.elapsed()` already available in `DashboardSnapshot`; format as `Xh Ym`.

6. **MCP count**: Count active tool call events in the last 60 seconds from the TUI event buffer. Show `MCP:N`.

7. **System metrics**: Use the `sysinfo` crate to read CPU%, memory in GB, and network I/O counters. Sample once per 2 seconds (not every frame) to avoid CPU overhead. Cache in `TuiModel.system_metrics`.

8. **Layout**: Use a ratatui `Layout::horizontal` split to allocate space per element. Priority order for truncation when terminal is narrow: drop ETA first, then system metrics, then MCP count, then wave indicator.

9. **Queue/milestone name**: Source from `TuiModel.active_milestone_name` (populated from queue manifest if loaded; otherwise show the plans directory name).

## Acceptance Criteria

1. Header displays a pulsing dot that alternates each frame.
2. Elapsed time increments correctly and is visible in the header.
3. Gradient progress bar fills proportionally to task completion.
4. CPU% and memory used are non-zero when a plan is running.
5. Wave indicator shows `Wave 1/3` when wave computation is available.
6. Header renders without layout overflow on an 80-column terminal (elements truncate gracefully).
7. All header metrics update on each render tick.

## Verification Checklist

- [ ] `roko dashboard` shows non-zero elapsed time while a plan runs.
- [ ] After 50% task completion, the progress bar is approximately half-filled.
- [ ] CPU% changes as builds run (not stuck at 0).
- [ ] On a 80-column terminal, the header does not overflow or wrap.
- [ ] The dot alternates between ● and ○ on consecutive frames.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/tabs.rs` | Rebuild header layout with all elements |
| `crates/roko-cli/src/tui/app.rs` | Add `tick_count`, `system_metrics` to `TuiModel` |
| `crates/roko-cli/src/tui/mod.rs` | Wire sysinfo sampling to `TuiModel` update |
| `crates/roko-cli/src/tui/theme.rs` | Provide `progress_color()` used by header bar |
