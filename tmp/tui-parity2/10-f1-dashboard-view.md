# F1 Dashboard View -- Deep Audit

**File**: `crates/roko-cli/src/tui/views/dashboard_view.rs` (~2467 lines)
**Related widgets**: `header_bar.rs` (779 LOC), `status_bar.rs` (397 LOC),
`phase_compact.rs` (359 LOC), `task_progress.rs` (506 LOC),
`token_sparkline.rs` (315 LOC), `cost_by_model.rs` (428 LOC),
`wave_progress.rs` (119 LOC), `plan_tree.rs`, `parallel_pool.rs`,
`sys_metrics.rs`, `gate_output.rs`, `diff_panel.rs`

---

## 1. Layout: How Is Screen Space Divided?

### Current structure

```
+----------------------------------------------------------------------+
|  Header bar  (1 line)                                                |
+----------------------------------------------------------------------+
|  Warning bar (0 or 1 lines, only when warnings active)               |
+----------------------------------------------------------------------+
|  LEFT PANEL (38%)     | 1px | RIGHT PANEL (62%)                     |
|  +-----------------+  | gap | +----------------------------------+  |
|  | Plan Tree       |  |     | | Sub-tab bar (1 line)             |  |
|  | (content-aware) |  |     | +----------------------------------+  |
|  +-----------------+  |     | |                                  |  |
|  | Phase Compact   |  |     | |  Sub-tab content                 |  |
|  | (4 lines fixed) |  |     | |  (Agents/Output/Diff/Verify/     |  |
|  +-----------------+  |     | |   Git/MCP/Learning/Procs)        |  |
|  | Task Progress   |  |     | |                                  |  |
|  | (content-aware) |  |     | |                                  |  |
|  +-----------------+  |     | +----------------------------------+  |
|                       |     | | Diagnosis panel (0-9 lines,      |  |
|                       |     | |  only when conductor fires)      |  |
|                       |     | +----------------------------------+  |
+----------------------------------------------------------------------+
|  Status bar  (1 line)                                                |
+----------------------------------------------------------------------+
```

### Key observations

- The left panel **only appears when plans are actively running**. When idle, the
  right panel gets the full width. This is a good adaptive decision.
- The 38/62 split is reasonable but **38% can feel cramped** for the plan tree when
  plan IDs are long. The plan tree has its own column layout (progress bar, delta,
  verify icon, age) that gets squeezed at widths below ~140 chars.
- The 1px void gutter between panels is intentional (mirrors Mori). It works visually.
- Phase compact is hard-locked at 4 lines. This is fine -- it's a 2-line segmented bar
  plus 2 lines of border/detail. Never needs more.
- Plan tree and task progress share the remaining vertical space via content-aware
  sizing: `plan_content` vs `task_content` weighted against available height, with
  minimums (8 lines for plans, 8 lines for tasks when tasks exist). This is well done.

### Layout verdict

**Good but not great.** The left panel density is high and well-structured. The right
panel is where the problem lives: the sub-tab paradigm hides too much behind tab
switches. During a plan run, the most important information (agent output, gate
status, cost) is spread across 3 different sub-tabs. You must actively switch to see
each one.

---

## 2. Sub-tabs: What's in Each?

### (a) Agents -- the default sub-tab
- **Layout**: parallel_pool table (4-7 rows) + optional route_metrics table (up to 8
  rows) + output panel (flexible) + bottom strip (7 rows: token sparkline + sys metrics)
- **Content**: Shows agent roster, their current tasks, model routing, live output,
  token burn sparkline, and system resource gauges
- **Assessment**: **Overcrowded.** This single sub-tab tries to show 4 independent
  widgets stacked vertically. On a 40-row terminal, each widget gets maybe 6-8 lines.
  The output panel (the most important part) gets whatever is left after the pool
  table, routes table, and bottom strip take their share.

### (o) Output -- standalone output panel
- **Content**: Same `render_output_panel` as used inside Agents, but gets the full
  right panel area. Shows live agent output with ANSI color parsing, auto-tail
  scrolling, and gate output override.
- **Assessment**: **This is where you actually read what the agent is doing.** Having
  it as a separate tab is good, but it duplicates what's already in the Agents tab.
  The separation creates a "which tab am I on?" problem.

### (d) Diff -- git diff viewer
- **Content**: Syntax-colored unified diff output from `tui_state.git_diff`.
  Scrollable.
- **Assessment**: **Useful during gate failures.** Otherwise rarely checked. Could be
  merged with Git sub-tab.

### (e) Verify -- gate verdict dashboard
- **Layout**: 1-line intro + verdict summary or trend grid (60%+) + recent failures
  (33% height)
- **Content**: Per-gate pass ratios with rate bars, 24h timeline sparklines, and a
  reverse-chronological failure log with age/gate/task/detail columns.
- **Assessment**: **Important but only after gates run.** The trend grid with braille
  sparklines is genuinely useful for spotting flaky gates. The failures panel is
  critical for debugging. This is well-designed but mostly idle during implementation
  phases.

### (g) Git -- git status summary
- **Content**: Branch name, commit hash, last commit age, file status counts
  (M/A/D), recent commit log (up to 8), worktree count.
- **Assessment**: **Low value as a sub-tab.** This is reference info that changes
  slowly. The status bar already shows branch + commit. Most users will use their own
  terminal for git. Could be collapsed into a small section elsewhere.

### (m) MCP / Context -- MCP config + efficiency + cascade router
- **Content**: Three sections: MCP server config (path, server list), efficiency
  summary (token counts, cost, model usage), cascade route stats (model slugs,
  trial/success counts).
- **Assessment**: **Diagnostic, not operational.** Useful when setting up or debugging
  MCP config. The efficiency and cascade data overlap heavily with what header bar,
  token sparkline, and cost_by_model already show. This is a "settings inspection"
  tab, not a "monitor the run" tab.
- **Performance issue**: `load_mcp_config_view` does file I/O (reads roko.toml and
  MCP config) **on every render frame**. This should be cached.

### (L) Learning -- trend sparklines + experiments
- **Layout**: 60/40 vertical split. Top 60%: four stacked sparklines (C-Factor,
  Tokens/hr, Latency/hr, Cost/hr). Bottom 40%: concluded experiments table.
- **Content**: Each sparkline is an hourly-bucketed trend. The experiments table shows
  A/B test winners with win rates, sample sizes, confidence intervals, and rate bars.
- **Assessment**: **Well-designed but niche.** The sparklines need >12 lines of height
  and look thin when compressed. The experiments table is the most actionable part.
  Rarely needed during a plan run.

### (P) Procs -- process table
- **Content**: PID, Role, CPU%, MEM, State, Uptime, and braille trend sparklines for
  CPU and memory per process. Scrollable.
- **Assessment**: **Pure ops/debugging.** Useful when diagnosing hung agents or
  resource exhaustion. Not needed during normal operation. The header bar already shows
  CPU/MEM/agent count.

---

## 3. Which Sub-tabs Are Useful vs Rarely Needed?

### Tier 1 -- Always useful during a plan run
- **Agents** (minus the overcrowding)
- **Output** (the primary "what is happening" view)
- **Verify** (the primary "did it work" view)

### Tier 2 -- Occasionally useful
- **Diff** (useful when gates fail, otherwise idle)
- **Learning** (useful for long multi-plan runs)

### Tier 3 -- Rarely needed during a run
- **Git** (static reference info, mostly duplicated in status bar)
- **MCP** (setup/diagnostic only)
- **Procs** (ops debugging only)

### Recommendation
The 8-tab structure forces the user to hunt for information. The three Tier 1 tabs
should have their most critical data visible simultaneously, not hidden behind tabs.
Tier 3 tabs could be folded into an "Inspect" overlay or moved to the F7 Inspect tab
entirely.

---

## 4. Header Bar: What Info Is Shown? What's Missing?

### Currently shown (9 sections, left to right)

1. Health-aware pulsing dot (green/amber/red/grey) + "roko" label
2. Queue/plan name (truncated to 24 chars)
3. Wave indicator ("Wave 2/5")
4. Fire-gradient progress bar (15 chars wide)
5. Task count ("7/12") with percentage and in-flight agent count
6. ETA (critical-path or proportional), elapsed time, cost ($X.XX/$Y.YY (Z%)), tokens
7. System metrics: CPU%, MEM, agent count, gate pass rate, MCP connections, NET rate,
   disk free, FPS
8. Active agent spinner with role and model
9. F-key strip (right-aligned, with badge counts on inactive tabs)

### Assessment
The header bar is **remarkably information-dense** and well-executed. The fire gradient
progress bar, the cost/budget display, and the health dot are all useful at a glance.

### What's missing or could improve
- **Current task name**: The header shows the plan name and active agent role, but not
  the actual task being worked on. During a 30-task plan, knowing "implementing
  task t-017: Wire gate pipeline" matters more than knowing the plan ID.
- **Gate status indicator**: The gate pass rate is a cumulative percentage. A simple
  "GATE: PASS" / "GATE: FAIL" / "GATE: --" for the most recent gate would be more
  immediately actionable.
- **Compact mode drops useful info**: Below 120 chars, MCP count, NET rate, disk free,
  and FPS all disappear. These are fine to drop, but the percentage display also
  disappears, which is more useful than some things that remain.
- **ETA is speculative**: The proportional ETA assumes constant task duration, which is
  often wrong. The critical-path ETA is better but only available when the runner
  provides it. Consider showing both or labeling the estimate quality.

---

## 5. Phase Compact Widget: Clear at a Glance?

### What it shows
```
+-- Phase . implementer ---------------------+
| ||||||||||||||||||||@@@@@@-------------------|
| * implementer  42%  2m31s                    |
+----------------------------------------------+
```

Line 1: Segmented bar where each phase gets a proportional segment. Done = solid green,
Active = solid amber + animated spinner char, Failed = solid red, Pending = dashes.

Line 2: Active phase detail -- spinner icon, phase name (bold rose), percentage, elapsed
time (with heartbeat pulse color).

### Assessment
**Clear and effective.** The segmented bar gives instant "how far along are we" context.
The active phase detail line provides enough info without overwhelming. The color coding
is semantically consistent (SAGE=done, WARNING=active, EMBER=failed, ghost=pending).

### Minor issues
- The segmented bar uses equal-width segments per phase. If there are 2 phases vs 8
  phases, the visual weight changes dramatically. For 2 phases, each segment is huge
  and uninformative. For 8+, they get too narrow to distinguish.
- No ETA shown on line 2 (the code has the `step.pct` but no estimated remaining time
  for the current phase).
- The title shows "Phase . implementer" but the dot separator is subtle; at small widths
  the phase name gets truncated in the title bar, which is the only place it appears.

---

## 6. Token Sparkline: Visible and Useful?

### What it shows
A bordered "Efficiency" box containing:
- Line 1: `tokens XXk  cost $X.XX  avg/task XXk`
- Line 2 (if height > 3): `succ XX%  events XX  window XX`
- Line 3: `XXk [braille sparkline chart] XX.Xk/min`
- Lines 4-6: T0/T1/T2 tier distribution bars with labels and counts

### Assessment
**Good data, poor placement.** The sparkline is buried in the bottom strip of the Agents
sub-tab, competing with sys_metrics for horizontal space (58/42 split). At typical
terminal sizes, it gets maybe 40-50 chars wide and 5-6 lines tall. The braille chart is
tiny but readable.

### Issues
- **Only visible on Agents sub-tab.** If you switch to Output (to see what the agent is
  actually doing), the sparkline disappears. Cost and token info should be always-visible.
- The "Efficiency" title is vague. "Token Burn" or "Resource Usage" would be clearer.
- The tier distribution bars (T0/T1/T2 haiku/sonnet/opus) are useful but take 3 lines
  that could show more sparkline history.
- The fallback path (`build_snapshot_from_tui_state`) when efficiency events are empty
  works, but produces less accurate data since it distributes cost proportionally by
  token count rather than using actual per-model pricing.

---

## 7. Cost Display: Prominent Enough?

### Where cost appears

1. **Header bar**: `$X.XX/$Y.YY (Z%)` -- present but easy to miss among 15+ other
   data points on a single line
2. **Status bar**: `$X.XX / $Y.YY (Z%)` -- duplicated from header, also crowded
3. **Token sparkline**: `cost $X.XX` -- only on Agents sub-tab
4. **MCP sub-tab**: `cost: $X.XXXX` -- full precision but buried in diagnostic panel
5. **Cost by Model table** (`cost_by_model.rs`): Per-model cost breakdown -- exists
   as a widget but is NOT rendered anywhere in the F1 dashboard view. Only used on
   other tabs.

### Assessment
**Not prominent enough.** Cost is the most financially consequential metric on the
dashboard, yet it's presented as just another number in the header bar's information
soup. During a $50 plan run, the operator needs to see cost at the same prominence as
progress.

### Recommendation
Cost deserves its own mini-widget or a dedicated position in the header bar, not crammed
between elapsed time and token count. Consider: a colored cost pill that changes from
green (under budget) to amber (>50% budget) to red (>80% budget) with large-enough text
to see from arm's length.

---

## 8. Progress Indicators: Clear Status at All Times?

### What exists

1. **Header bar progress bar**: 15-char fire-gradient filled bar with count ("7/12")
2. **Phase compact**: Segmented bar showing phase-level progress
3. **Task progress widget**: Full progress bar + scrollable task checklist with per-task
   status icons, elapsed times, and summary badges (RUN/DONE/FAIL/WAIT)
4. **Wave progress ribbon** (`wave_progress.rs`): Proportional wave segments with ocean
   gradient animation. Exists as a widget but is NOT rendered in the dashboard view --
   only in the header wave indicator.

### Assessment
**Multiple progress signals, but fragmented.** The header bar has a progress bar. The
left panel has a separate progress bar in task_progress. The phase compact has yet
another progress representation. These tell the same story at different granularities
but aren't visually connected.

### Issues
- When no plans are active, the left panel disappears entirely and the header still
  shows "0/0". There's no "idle" dashboard state -- just empty right panel content.
- The task progress checklist is the most detailed view but it's in the left panel (38%
  width), which means task titles get truncated. The right panel (62%) has more space
  but uses it for output.
- The `wave_progress` ribbon has a nice animated ocean gradient but is never actually
  rendered on the F1 dashboard. Only the header's text-only "Wave 2/5" is shown.

---

## 9. What Would Make the Dashboard "At-a-Glance" Useful During a Plan Run?

The operator's core questions during a plan run are:

1. **What's happening right now?** (current task + agent output)
2. **How far along are we?** (progress + ETA)
3. **Is it working?** (gate pass/fail)
4. **How much is it costing?** (cost + budget utilization)
5. **Is anything broken?** (errors, failures, retries)

Currently, answering all five requires switching between at least 3 sub-tabs (Agents,
Output, Verify). The ideal dashboard answers all five with zero tab switches.

### The "single-screen" principle
The most useful dashboard is one where the operator can glance at the screen and answer
all five questions without any interaction. This means:

- Agent output should be the largest area (it's the "TV" -- what you watch)
- Progress, cost, and gate status should be immediately visible in fixed positions
- Errors should surface as overlays or alerts, not hidden in tabs
- The sub-tabs should be for deep-dive, not for essential information

---

## 10. What Should Be the "Hero" Element?

**The agent output stream should be the hero.**

During a plan run, the agent output is what the operator watches 90% of the time. It
shows whether the agent is making progress, getting stuck, producing errors, or going
off-track. Everything else is metadata about the output stream.

Currently, the agent output competes with 3 other widgets for vertical space in the
Agents sub-tab, or takes the full right panel in the Output sub-tab (but loses all the
metadata context).

The ideal hero layout makes the output stream the dominant visual element, with progress,
cost, and gate status as compact permanent fixtures around it.

---

## Redesigned Layout Proposal

### Principles
1. **Output is the hero** -- largest single area, always visible
2. **Progress + cost + gates are permanent** -- never hidden behind tabs
3. **Sub-tabs are for deep-dive only** -- not for essential monitoring info
4. **Information density increases toward edges** -- the center is calm, the margins
   are dense

### Proposed layout (active plan run)

```
+----------------------------------------------------------------------+
|  Header bar  (unchanged, 1 line)                                     |
+----------------------------------------------------------------------+
|  LEFT SIDEBAR (30%)   | RIGHT MAIN (70%)                             |
|  +------------------+ | +------------------------------------------+ |
|  | Progress Card    | | |  HERO: Agent Output                      | |
|  | [=====>   ] 7/12 | | |                                          | |
|  | ETA ~4m  $2.31   | | |  (auto-tailing, ANSI-colored,            | |
|  | GATE: PASS (83%) | | |   scrollable, with gate output override) | |
|  +------------------+ | |                                          | |
|  | Phase Bar         | | |                                          | |
|  | [@@@@@@@@------] | | |                                          | |
|  | * implementer 42%| | |                                          | |
|  +------------------+ | |                                          | |
|  | Task Checklist    | | |                                          | |
|  | * t-001 Wire SPB | | |                                          | |
|  | > t-002 Episodes | | |                                          | |
|  | . t-003 Refactor | | |                                          | |
|  | x t-004 Clippy   | | |                                          | |
|  |    [scrollbar]   | | |                                          | |
|  +------------------+ | +------------------------------------------+ |
|  | Active Agent      | | Sub-tabs: d:Diff  e:Verify  L:Learn  P:Sys| |
|  | opus implementer | | +------------------------------------------+ |
|  | 23k in / 4k out  | | | [collapsed detail panel, ~6 lines]       | |
|  +------------------+ | | gate failures / diff / learning trends   | |
|                        | +------------------------------------------+ |
+----------------------------------------------------------------------+
|  Status bar  (unchanged, 1 line)                                     |
+----------------------------------------------------------------------+
```

### Key changes

1. **"Progress Card" replaces the plan tree as the top-left widget.** Instead of a
   tree of all plans (which is mostly useful on F2), show a compact summary card:
   progress bar, ETA, cost, and latest gate result. This puts the 4 key metrics in a
   permanently visible, scannable card.

2. **Agent output becomes the hero** with 70% width and ~80% of the right panel
   height. No more splitting it with the parallel pool table and token sparkline.

3. **The sub-tab area shrinks to a collapsed detail strip** at the bottom of the right
   panel (~6 lines). Only 4 sub-tabs remain: Diff, Verify, Learning, and System.
   The Agents, Output, Git, and MCP tabs are either absorbed into the main view or
   moved to F7 Inspect.

4. **"Active Agent" widget** replaces the parallel_pool table. Shows just the currently
   active agent: model, role, token usage. If multiple agents are running in parallel,
   this becomes a compact roster (1 line per agent, no table headers).

5. **Cost gets a color pill** in the Progress Card: green text when under 50% budget,
   amber at 50-80%, red above 80%.

6. **Gate status is a single prominent badge** in the Progress Card: "GATE: PASS (83%)"
   in green, or "GATE: FAIL" in red, rather than requiring a tab switch to Verify.

### When no plan is active (idle state)

```
+----------------------------------------------------------------------+
|  Header bar                                                          |
+----------------------------------------------------------------------+
|                                                                      |
|  +--------------------------------------+                            |
|  |  roko                                |                            |
|  |                                      |                            |
|  |  No active plans.                    |                            |
|  |  roko plan run plans/ --engine v2    |                            |
|  |                                      |                            |
|  |  Last run: 2h ago  7/12 tasks        |                            |
|  |  Cost: $2.31  Gate pass: 83%         |                            |
|  +--------------------------------------+                            |
|                                                                      |
|  Sub-tabs: e:Verify  L:Learn  g:Git  m:MCP  P:Procs                 |
|  +------------------------------------------------------------------+|
|  | [full-width detail panel]                                        ||
|  +------------------------------------------------------------------+|
+----------------------------------------------------------------------+
|  Status bar                                                          |
+----------------------------------------------------------------------+
```

### Sub-tab consolidation

| Current | Proposed | Rationale |
|---------|----------|-----------|
| Agents | Absorbed into main view | Pool table + output are now the hero area |
| Output | Absorbed into main view | Output IS the main view |
| Diff | Keep (in collapsed strip) | Needed for gate failure diagnosis |
| Verify | Keep (in collapsed strip) | Gate results and failure log |
| Git | Move to F7 Inspect | Static info, already in status bar |
| MCP | Move to F7 Inspect or F6 Config | Setup/diagnostic, not monitoring |
| Learning | Keep (in collapsed strip) | Trend data is useful during long runs |
| Procs | Move to F7 Inspect | Ops debugging, not monitoring |

This reduces the sub-tab count from 8 to 3-4, eliminating the "which tab am I on?"
problem and ensuring the hero output area is always visible.

---

## Additional Findings

### Performance concern: MCP sub-tab file I/O on every frame

```rust
// dashboard_view.rs line 823
let mcp_config = load_mcp_config_view(&tui_state.workdir);
```

`load_mcp_config_view` calls `Config::from_file()` and `McpConfig::load()` -- file I/O
on every render frame. This should be cached in TuiState and refreshed periodically (on
file watcher event or every 30s), not on every 30fps render cycle. Even though this only
fires when the MCP sub-tab is active, a stuck-on-MCP-tab user would trigger thousands of
file reads per minute.

### Unused widget: cost_by_model

`cost_by_model.rs` (428 LOC) defines `render_cost_by_model_table` but it is NOT called
anywhere in the F1 dashboard view. It exists as a compiled widget but is only used on
other tabs. This is either a wiring oversight or intentional -- but given that cost
visibility is a weakness of the current dashboard, integrating this table into the
Learning or a dedicated cost sub-tab would add value.

### Unused widget: wave_progress

`wave_progress.rs` (119 LOC) with its ocean gradient animation is rendered nowhere in
the F1 view. The header bar has a text-only "Wave 2/5" indicator. The animated wave
ribbon would be visually effective as a 1-line strip between the header and the main
content area, replacing or supplementing the warning bar position when no warnings are
active.

### The AffectView override

When `view_state.active_sub_view(Tab::Dashboard) == SubView::AffectView`, the entire
dashboard layout is replaced by `affect_view::render`. This means the affect view
completely takes over F1, losing all monitoring context. This should either be a separate
tab or an overlay, not a replacement of the entire dashboard.

### Diagnosis panel: dynamically sized but rarely seen

The diagnosis panel at the bottom of the right panel only appears when the conductor
circuit breaker fires. When it does appear, it takes 6-9 lines -- a significant chunk
of vertical space that pushes the main content upward. This is correct behavior (alerts
should demand attention) but could use a dismiss mechanism so it doesn't linger after
the operator has acknowledged it.

---

## Summary of Priorities

1. **Make output the hero** -- stop splitting it with 3 other widgets in the Agents tab
2. **Add a Progress Card** -- cost + ETA + gate badge in a permanently visible sidebar
3. **Consolidate sub-tabs from 8 to 4** -- move Git/MCP/Procs to Inspect
4. **Fix MCP sub-tab file I/O** -- cache config, don't read on every frame
5. **Wire wave_progress** into the dashboard layout
6. **Wire cost_by_model** into Learning or a cost overlay
7. **Show current task name** in the header bar or Progress Card
8. **Make cost more prominent** -- color-coded budget utilization pill

---

## Implementation Status (2026-09-02 swarm)

F1 Dashboard improvements (task #12): layout restructured, panel organization improved,
idle dashboard state handling. NET/DSK metrics corrected (task #6). Critical-path ETA
wired into header bar (task #5).
