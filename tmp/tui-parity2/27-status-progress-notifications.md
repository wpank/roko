# 27 - Status Indicators, Progress, and Notification System Audit

**Audited**: 2026-09-01
**Files examined**:
- `crates/roko-cli/src/tui/widgets/header_bar.rs`
- `crates/roko-cli/src/tui/widgets/status_bar.rs`
- `crates/roko-cli/src/tui/widgets/task_progress.rs`
- `crates/roko-cli/src/tui/widgets/phase_compact.rs`
- `crates/roko-cli/src/tui/modals/notification.rs`
- `crates/roko-cli/src/tui/modals/mod.rs`
- `crates/roko-cli/src/tui/app.rs` (notification lifecycle, snapshot integration)
- `crates/roko-cli/src/tui/state.rs` (active_warnings, tab_badge, critical_path_eta)
- `crates/roko-cli/src/tui/input.rs` (DismissNotification action)
- `crates/roko-cli/src/tui/atmosphere.rs` (animation primitives)

---

## 1. Header Bar: What status is always visible? Sufficient?

**File**: `crates/roko-cli/src/tui/widgets/header_bar.rs`

The header bar is a dense, always-visible information strip with 9 sections:

| Section | Content | Notes |
|---|---|---|
| 1. Health dot | Pulsing circle (filled/hollow), color-coded: SAGE=healthy, WARNING=gating, EMBER=error, GHOST=idle | Animates via `Atmosphere::heartbeat()` brightness modulation |
| 1b. Queue/plan name | Active or first plan name, truncated to 24 chars | Only shown when plans exist |
| 2. Wave indicator | `Wave N/M` | Only shown when execution_waves is non-empty |
| 3. Progress bar | 15-char gradient bar using `gradient_fire()` | Only shown when total tasks > 0 |
| 4. Plan count | `done/total`, percentage, in-flight agent count, or `COMPLETE`/`ERR:done/total` | Semantic coloring: SAGE for done, EMBER for errors, gradient for in-progress |
| 5. ETA/elapsed/cost/tokens | Critical-path ETA or proportional ETA, elapsed time, `$X.XX/$Y.YY (Z%)` cost/budget, token count | CP-ETA preferred over proportional; cost shows budget utilization % |
| 6. System metrics | CPU%, MEM (bytes), agents online, gate pass rate, MCP connections, NET rate, disk free, FPS | CPU/MEM color-coded at 50%/80% thresholds; MCP/NET/DSK/FPS hidden in compact mode (<120 cols) |
| 7. Active agent | Spinner + role + model name | Only shown when an agent is active |
| 8. F-key strip | F1-F9 tab labels with active highlighting and badge counts | Right-aligned; badges show semantic counts per tab |

**Assessment**: Highly comprehensive. The header bar packs substantial operational telemetry into a single line. The health dot provides at-a-glance system status (4 states), the progress bar gives visual completion, and the metrics section covers CPU/MEM/disk/network/gates/agents/MCP. Cost and budget utilization are always visible when non-zero.

**Sufficiency**: Strong. The compact mode (<120 columns) correctly hides less critical items (MCP, NET, DSK, FPS) while preserving the essentials. The badge system on F-keys provides attention-routing cues. One weakness: the health dot derives status purely from plan failure/gating flags and does not reflect provider health or system resource pressure (high CPU/low disk do not turn the dot amber/red).

---

## 2. Status Bar: What info? Accurate key hints?

**File**: `crates/roko-cli/src/tui/widgets/status_bar.rs`

The bottom status bar has 4 sections:

| Section | Content |
|---|---|
| 1. Git info | Branch name, short commit hash, last commit age |
| 2. Heartbeat + pause | 4-frame cycling heartbeat, `PAUSED` badge when paused (bold WARNING bg) |
| 3. Plan progress + health | `COMPLETE`/`ERR:N`/`done/total`, active plans count, live agents, flailing count (>=3 failures), total failures, cost/budget |
| 4. Key hints | Context-sensitive, max 5 hints, tab-specific |

**Key hints by tab**:

| Tab | Hints |
|---|---|
| Dashboard | arrows:nav, a/o/d/e/g:sub-tab, [R:retry, D:diag if failures], Tab:panel |
| Plans | arrows:nav, Enter:expand, [r:retry/s:skip/d:details if failed task], h/l:drill, /:filter |
| Agents | arrows:nav, [x:stop/c:chat/d:details if active], [S:start/d:details if failed/idle], backtick:cycle, Ctrl+T:topology, i:inject |
| Git | arrows:nav, h/l:drill, Enter:expand |
| Logs | arrows/PgUp/PgDn:scroll, 1-4:levels, a:all, /:search |
| Config | j/k:nav, Enter:toggle, r:reload |
| Inspect | arrows:nav, Tab:panel, Enter:details |
| Marketplace | j/k:nav, Enter:detail, n:new, r:refresh |
| Atelier | j/k:nav, Enter:detail, p:publish, g:gen plan |
| Learning | arrows:nav, Enter:details |

**Assessment**: The key hints are context-sensitive and adapt to the selected item state (e.g., different hints when a failed task is selected vs. an active task). The `?:help` hint is appended when there are fewer than 5 hints, which is correct. The 5-hint cap prevents visual clutter.

**Accuracy concerns**:
- The Plans tab shows `r:retry` and `s:skip` hints when a failed task exists in the selected plan, but these are derived from `TaskStatus::Failed` -- the hint system checks only whether any task in the plan is failed/active, not whether the *selected* task is the failed one. This could show misleading hints if the user has scrolled to a different task.
- The Agents tab hints for `S:start` appear for both Failed and Idle agents, which is correct, but `c:chat` only appears for active agents -- no way to chat with an idle agent from hints.
- Cost/budget is duplicated between header and status bar. This is intentional redundancy for runs where the header might be clipped, but on wide terminals the same `$2.50 / $10.00 (25%)` appears twice.

---

## 3. Task Progress: How is progress communicated?

**File**: `crates/roko-cli/src/tui/widgets/task_progress.rs`

Progress is communicated through **four simultaneous mechanisms**:

1. **Gradient progress bar**: Full-width bar with semantic coloring via `Theme::semantic_color(pct)`. The leading edge pulses via heartbeat modulation. Filled portion uses solid blocks, empty uses light shading.

2. **Numeric count**: `done/total` displayed after the bar.

3. **Summary badge line**: A colored status tag (`DONE`/`FAIL`/`RUN`/`WAIT`) followed by breakdown counts (`3 active`, `5 queued`, `2 blocked`, `1 failed`), joined by middle-dot separators.

4. **Per-task row list**: Scrollable checklist with status icons:
   - Done: checkmark (green)
   - Active: play arrow with pulsing rose color + elapsed time tag
   - Blocked: cross mark (red)
   - Failed: cross mark (ember)
   - Pending: middle dot (dim)

**Scrolling**: Scroll position tracked via `state.task_scroll`, with `up/down more` indicators when the list overflows. A ratatui `Scrollbar` widget is rendered on the right edge with custom styled thumb/track.

**Title**: Shows `Tasks (done/total)` with scroll range `[start-end of total]` when the list overflows.

**Assessment**: Thorough multi-layered progress communication. The combination of bar + count + status badge + per-task detail provides information at three zoom levels (glance, summary, detail). The elapsed time on active tasks is a nice operational touch. One gap: there is no ETA shown at the task-progress level (ETA is only in the header bar).

---

## 4. Phase Compact: What phases are shown?

**File**: `crates/roko-cli/src/tui/widgets/phase_compact.rs`

The phase widget is a compact 2-line display:

**Line 1 -- Segmented phase bar**: Each phase gets an equal-width segment. Colors by status:
- Done: SAGE (green solid blocks)
- Active: WARNING (amber solid blocks + ethereal spinner character at trailing edge)
- Failed: EMBER (red solid blocks)
- Pending: TEXT_GHOST (horizontal dashes)

**Line 2 -- Active phase detail**: Shows the active phase with:
- Pulsing spinner icon
- Phase name (bold ROSE)
- Completion percentage (DREAM color, capped at 99%)
- Elapsed time (`Xm XXs`)

Special states:
- `HALTED at <phase-name>` when any phase has Failed status (takes priority over active display)
- `all phases complete` when all phases are Done
- `waiting...` when all phases are Pending

**Phases shown**: The widget renders whatever is in `state.phase_pipeline`, which is a `Vec<PhaseStep>` populated from the runner state. Common phases would be: preflight, implementer, reviewing, gating, etc. The pipeline is dynamic -- no hardcoded phase names in the widget itself.

**Assessment**: Clean, minimal design. The segmented bar gives instant visual progress across the pipeline. The failed state is prominent with HALTED + ember coloring. The ethereal spinner on the active segment's trailing edge is a subtle but effective animation touch. One issue: no ETA is shown for the active phase despite the `PhaseStep` having enough data to estimate one (elapsed + pct could give proportional ETA).

---

## 5. Notification Toasts: How do they appear/disappear? Timing?

**File**: `crates/roko-cli/src/tui/modals/notification.rs`

**Appearance**:
- Toasts render as bordered rectangles in the **bottom-right corner** of the screen.
- They stack upward: the most recent toast is at the bottom, older ones above it.
- Each toast is 3 rows tall (top border + message line + bottom border).
- Toast width is computed from the longest message, clamped to `[30, 80% of screen width]`.
- At most 5 toasts are visible simultaneously (limited by `area.height / toast_height`).
- The `Clear` widget is rendered first to erase underlying content before drawing the toast.

**Styling by level**:
- Info: `theme.info()` border
- Warn: `theme.warning()` border
- Error: `theme.danger()` border
- Debug: `theme.muted()` border

Each toast has a `[TAG] message` format where TAG is `INFO`/`WARN`/`ERR `/`DBG ` (padded to 4 chars).

**Disappearance (TTL)**:
- Info: 5 seconds
- Warn: 8 seconds
- Error: 10 seconds
- Debug: implicit via constructor (no default helper, must be created with `Notification::new`)
- Custom TTLs are supported via `Notification::new(msg, level, ttl_secs)`.

**Expiration lifecycle** (in `app.rs`):
- `expire_notifications()` is called every tick, retaining only non-expired notifications.
- Hard cap of 20 notifications maximum; oldest are removed first when exceeded.
- Notifications are also manually dismissible via `n` key (`TuiAction::DismissNotification`), which removes the oldest (index 0).

**Deduplication** (in `app.rs`):
- `push_deduped_notification()` suppresses duplicates: if a notification with the same message exists and was created within the last 2 seconds, the new one is skipped. This prevents toast spam from rapidly repeating events.

**Assessment**: The toast system is solid. The TTL-based auto-dismiss with level-appropriate durations, deduplication, hard cap, and manual dismiss all work together well. The bottom-right stacking is standard and non-intrusive.

**Weaknesses**:
- No fade-in or fade-out animation. Toasts appear/disappear instantly between frames.
- The `n` key dismiss removes the *oldest* notification (index 0), not the most visible one (which is the most recent, at the bottom of the stack). This could be counterintuitive.
- No way to dismiss a specific notification; it is FIFO only.
- The `n` key also simultaneously dismisses the persistent warning bar (`warnings_dismissed = true`), which conflates two unrelated actions.

---

## 6. Is there a notification queue/history?

**No persistent notification history exists.**

The notification system is a simple `Vec<Notification>` in `App`. Once a notification expires (TTL elapses) or is manually dismissed, it is permanently removed with no trace. There is no:
- Notification log file
- Scrollable notification history view
- Notification count/badge for missed notifications
- Way to recall dismissed notifications

Notifications that arrive while the TUI is not being watched (e.g., during a long unattended run) will simply expire and be lost. The only durable record of events that generated notifications is in the underlying snapshot data (gate results, plan phases, error logs), but the notifications themselves have no history.

**Assessment**: This is a significant gap for long-running operations. An operator returning after a break has no way to see what happened while they were away, at least not via the notification system. The Logs tab partially compensates, but gate failures, plan completions, agent stalls, and other events that generated toasts are not aggregated into a reviewable timeline.

---

## 7. Warning Bar: When does it show?

**File**: `crates/roko-cli/src/tui/widgets/header_bar.rs` (render_warning_bar) + `crates/roko-cli/src/tui/state.rs` (active_warnings)

The warning bar is a **1-line persistent bar** rendered immediately below the header. It appears when any of these conditions are true:

| Condition | Warning text |
|---|---|
| Disk free < 1 GiB (and disk data is available) | `DSK LOW: Xg free` |
| Any agent has `AgentStatus::Failed` | `N provider(s) unhealthy` |
| No run is active (`run_started.is_none()`) but plans have `active` flag | `Stale snapshot: plans active but no run` |

**Rendering**: Amber warning icon, joined warnings separated by ` | `, and a `[n] dismiss` hint at the right.

**Dismissal**: Pressing `n` sets `warnings_dismissed = true`, which causes `active_warnings()` to return an empty vec. The warnings are *suppressed*, not resolved. The `warning_bar_height()` function returns 0 or 1, and the layout allocates space accordingly -- so the content area grows by 1 row when warnings are dismissed.

**Assessment**: The warning conditions are well-chosen for operational awareness. Low disk is critical for JSONL-heavy workloads. Provider health failures are operationally important. Stale snapshot detection prevents confusion from leftover state.

**Weaknesses**:
- Warnings cannot return after dismissal within the same session. If disk free drops further or another provider fails after the user presses `n`, the warning bar stays hidden because `warnings_dismissed` is a boolean, not per-warning.
- The warning bar has no severity levels -- all warnings render identically with the amber icon.
- Only 3 warning conditions are checked. Missing: high CPU sustained, high memory pressure, git working tree dirty during plan execution, network connectivity loss, budget threshold approaching.

---

## 8. Are there sound/bell alerts for important events?

**No.** There are no terminal bell (`\x07`), system notification, or sound alerts anywhere in the TUI codebase. Searched for `bell`, `sound`, `audio`, `beep`, `\x07`, and `\a` -- none found.

All alerting is purely visual: the pulsing health dot, color changes, toast notifications, and the warning bar. This means that if the operator's terminal is not visible (e.g., minimized, on another desktop, or in a tmux pane), they will miss all alerts.

---

## 9. Is the overall system status (healthy/degraded/error) always clear?

The `HealthStatus` enum in the header bar provides four states:

```rust
enum HealthStatus {
    Healthy,   // SAGE dot   -- active plans, no failures
    Gating,    // WARNING dot -- current_phase contains "gat" or "verif"
    Error,     // EMBER dot  -- any plan has tasks_failed > 0
    Idle,      // GHOST dot  -- no active plans
}
```

**Always visible**: Yes, the pulsing dot is always in the header bar at position 0.

**Assessment**: The health dot is a good at-a-glance indicator, but the derivation logic has gaps:

1. **Provider health not reflected**: If all providers are unhealthy (`AgentStatus::Failed`), the health dot can still show Healthy as long as no *tasks* have failed yet.
2. **Resource pressure invisible**: High CPU (90%+), low memory, or low disk do not affect the health dot color, even though these conditions are operationally significant.
3. **Gating detection is fragile**: Phase matching uses string contains on "gat" and "verif", which could match unrelated phase names or miss renamed phases.
4. **No "degraded" state**: The dot jumps directly from Healthy to Error. There is no intermediate degraded state for situations like "1 task failed but the plan is continuing" vs. "the plan is halted."
5. **The PAUSED state** (shown in the status bar) is not reflected in the health dot -- a paused system still shows Healthy/Idle depending on plan state.

---

## 10. Proposals

### A. Unified Status Strip

Replace the current split between header dot + status bar progress with a dedicated 1-line status strip that consolidates the system state into a canonical reading order:

```
[state-dot] STATE | phase:NAME pct% | done/total tasks (N active, M queued) | ETA:Xm | $cost/$budget | CPU:X% MEM:Xg DSK:Xg
```

The state would be derived from a richer set of inputs:
- `RUNNING` -- active tasks, no failures, resources healthy
- `DEGRADED` -- active but with provider unhealthy, high resource pressure, or partial failures
- `GATING` -- in a gate/verify phase
- `PAUSED` -- user-paused
- `ERROR` -- plan halted due to failures
- `IDLE` -- no active plans
- `STALLED` -- active but no progress in N minutes

This avoids the current problem where health status, progress, and phase are scattered across header + status bar + phase widget with overlapping but inconsistent information.

### B. Progress Ring / Circular Gauge

For the Dashboard tab's main content area, replace or supplement the linear progress bar with a Unicode circular gauge showing plan completion:

```
    ╭───╮
   │ 42% │
    ╰───╯
  5/12 tasks
  ETA: ~8m
```

This would serve as the hero widget on the Dashboard tab, providing immediate visual impact. The header bar's linear bar would remain for persistent visibility, while the circular gauge would be the detailed view.

### C. Event Timeline

Add a scrollable event timeline widget (suitable for the Dashboard or Logs tab) that records all state transitions:

```
 12:34:05  TASK  t-003 started (implementer, claude-opus-4-6)
 12:35:12  GATE  t-003 compile PASS (1.2s)
 12:35:14  GATE  t-003 clippy FAIL: 3 warnings
 12:35:14  TASK  t-003 failed (gate)
 12:35:15  PLAN  Replan triggered for t-003
 12:36:01  TASK  t-003 retry started
```

Implementation:
- Add a `Vec<TimelineEvent>` to `TuiState` (bounded to e.g. 500 entries).
- Populate from snapshot deltas: gate results, plan phase transitions, agent start/stop, errors.
- Render as a scrollable list with timestamp, category icon, and description.
- This would solve the notification history gap (item 6) by providing a durable, scrollable record.

### D. Activity Pulse

Add a subtle background pulse to the main content area that indicates liveness:

1. **Working pulse**: When agents are active, a very subtle background brightness modulation (similar to the existing `Atmosphere::breathing_brightness()`) applied to the content area border. This provides subconscious "the system is alive" feedback without being distracting.

2. **Stall detection**: If no state change has occurred for a configurable duration (e.g., 2 minutes while agents are supposedly active), change the pulse to a slow amber throb and generate a warning notification. The current system has no stall detection at the TUI level.

3. **Terminal bell on critical events**: When a plan completes, fails, or stalls, emit `\x07` to trigger the terminal bell. Most modern terminals convert this to a visual bell (tab flash, dock bounce on macOS). This is zero-cost to implement and solves the "operator looking away" problem (item 8).

```rust
// In apply_dashboard_snapshot, after plan completion/failure toast:
if matches!(cur_phase, "completed" | "failed") {
    // Terminal bell for attention
    eprint!("\x07");
}
```

### E. Additional Warning Conditions

Expand the warning bar to cover more operational scenarios:

| Condition | Priority | Warning text |
|---|---|---|
| CPU > 90% sustained (3+ samples) | Medium | `CPU HIGH: X% sustained` |
| Memory > 85% of total | Medium | `MEM HIGH: Xg/Yg (Z%)` |
| Budget > 80% consumed | High | `BUDGET: $X/$Y (Z%) -- approaching limit` |
| No gate pass in last N attempts | High | `GATE STREAK: N consecutive failures` |
| Git working tree dirty during execution | Low | `GIT: uncommitted changes in worktree` |
| Agent stall (active > 5 min, no progress) | High | `STALL: agent X idle for Nm` |

### F. Per-Warning Dismissal

Replace the current boolean `warnings_dismissed` with a `HashSet<String>` of dismissed warning keys. This allows:
- Dismissing individual warnings while keeping others visible.
- New warnings appearing even after a previous dismiss.
- Re-showing warnings if their condition resolves and then recurs.

---

## Summary

| Aspect | Status | Rating |
|---|---|---|
| Header bar information density | 9 sections, comprehensive metrics | Strong |
| Status bar key hints | Context-sensitive, tab + item-state aware | Strong |
| Task progress communication | 4-layer (bar + count + badge + list) | Strong |
| Phase pipeline visualization | Segmented bar + active detail | Strong |
| Notification toasts | TTL-based, deduped, level-colored, dismissible | Good |
| Notification history | None -- ephemeral only | Gap |
| Warning bar conditions | 3 conditions (disk, provider, stale) | Adequate |
| Sound/bell alerts | None | Gap |
| System health derivation | 4 states but ignores resource pressure and provider health | Adequate |
| Health dot - degraded state | Missing intermediate between Healthy and Error | Gap |
| Warning dismissal model | All-or-nothing boolean | Weak |

**Top 3 actionable items**:
1. Add a terminal bell (`\x07`) on plan completion/failure -- near zero implementation cost, solves the attention problem.
2. Add an event timeline to the Dashboard or Logs tab -- solves the notification history gap and provides the post-mortem review capability that long-running operations need.
3. Expand `HealthStatus` to include `Degraded` (provider unhealthy or resource pressure) and `Stalled` (active but no progress) -- makes the always-visible health dot more accurate.
