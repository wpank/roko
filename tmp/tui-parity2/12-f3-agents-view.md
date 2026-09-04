# F3 Agents View Audit

**Files reviewed:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/agents_view.rs` (~1325 lines)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/agent_status_grid.rs` (~436 lines)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/parallel_pool.rs` (~148 lines)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` (AgentRow, AgentSummary, AgentStream, AgentStatus, LogSearchState)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/display_utils.rs` (shorten_model, display_model)
- Mori reference: `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/views/agents.rs` + `widgets/agent_output.rs`

---

## 1. Agent list/grid: visual density, status indicators

**Rating: Good with minor density issues**

The left panel (32% width) renders a full agent roster with header columns:
agent, model, status, task, tokens, cost, elapsed, last evt. Each row includes:
- Cursor indicator (triangle) for selected row
- Status icon: active=filled-triangle (warning color), done=checkmark (green), failed=cross (red/bold), idle=middot (muted)
- Role-accent coloring per agent label (7 distinct role colors)
- Model slug via `display_model()` (strips `claude-`/`gpt-` prefixes)
- Status chip: LIVE/DONE/FAIL/idle with colored background badges
- Token count, cost in USD, elapsed uptime, time since last event
- Gradient context gauge bar with percentage

When an agent is selected, an inline detail line appears below it showing plan, task,
turns, uptime, and last output line.

**Issues:**
- The roster header rendering uses hardcoded column widths that adapt to `content_width`,
  but there is no horizontal scroll or column collapsing -- at widths under ~100 columns
  the right columns (elapsed, last evt, gauge) will clip or overlap
- Agent IDs are truncated via `truncate_middle()` to `agent_w` (min 14), which can still
  be too wide for narrow terminals
- No row-count limit or virtualization; all agents render into a `Paragraph` of `Line` items,
  relying on ratatui's clip-to-area behavior. Not a problem at 10 agents, but at 50+ the
  Vec allocation happens every frame

**What works well:**
- Sorting: active first, then idle, then done, then failed -- matches mori
- Role colors are distinctive and consistent with the tab bar
- Gradient context bar is visually rich (teal -> accent -> warning)
- Active breathing effect on bar for running agents
- Summary line below the roster: active count, total tokens, total cost

## 2. Agent output panel: streaming output readable? scrollable? searchable?

**Rating: Mostly good; search is MISSING**

The right panel (68% width) is split into:
1. Route metrics bar (1 line): model badge, ctx usage, focus score, tier
2. Output body (flexible): agent output lines
3. Live Stream panel (7 lines, conditional): WebSocket or runner-collected tail

**Readability:**
- Output is rendered via `render_agent_output_lines()` on `TuiState` which caches parsed
  output and applies semantic styling (errors in red, success in green, headings bold, etc.)
- Text wraps via `Wrap { trim: false }` so long lines don't clip

**Scrolling:**
- Vertical scroll with `agent_scroll: Option<usize>`:
  - `None` = auto-follow tail (shows `[TAIL]` badge in green)
  - `Some(offset)` = pinned scroll (shows `[PINNED line N]` in warning color)
- Max scroll is clamped to `total_lines - visible_height`
- Good: scroll state distinguishes tail-follow vs manual

**Search: NOT WIRED**
- `LogSearchState` exists in `TuiState` with pattern, regex, mode (Highlight/Filter),
  match indices, and navigation -- but it is only used for the F4/Log tab
- The agent output panel has zero search integration: no `/` binding, no highlight, no
  match navigation
- This is a significant gap: when an agent produces 500+ lines of output, finding a
  specific error or tool call requires manual scrolling

**Live Stream panel:**
- Shows live WebSocket chunks or falls back to runner-collected output
- Connection status label: connected/done/connecting/no stream
- Auto-tails to most recent N visible lines
- Good fallback hierarchy: WS stream > runner output > episode output

## 3. Role tabs: clear what each role does?

**Rating: Adequate but not self-documenting**

Seven fixed role tabs in the right panel header: `1:impl 2:strat 3:arch 4:audit 5:crit 6:cond 7:res`

- Active tab: inverted (black text on role accent background)
- Tab with active agent: role accent text
- Tab without agent: muted text
- Numbering (1-7) implies keyboard shortcuts exist for quick switching

**Issues:**
- Tab labels are abbreviations only -- there is no tooltip, help text, or description of
  what each role does. A new user seeing `5:crit` or `6:cond` has no context
- Mori has the same abbreviations but also shows status icons (spinner/check/dot) per tab
  which roko does not
- The tabs currently filter nothing -- switching tabs changes `view_state.sub_tab` but the
  output body does not filter by role. The role tab selection appears decorative: the output
  panel shows whatever agent is selected in the roster, regardless of which role tab is
  active. This is a functional gap vs mori where each tab shows that role's output

## 4. Agent lifecycle states: visually distinct?

**Rating: Good**

Four states are defined in `AgentStatus`:
| State | Icon | Color | Badge |
|---|---|---|---|
| Active | filled triangle | warning/bold | LIVE (accent bg) |
| Idle | middot | muted | idle (transparent bg) |
| Done | checkmark | success | DONE (success bg) |
| Failed | cross | danger/bold | FAIL (danger bg) |

Additional visual cues:
- Active agents are **bolded** in the roster
- Active agents get a "breathing" brightness effect on their context gauge
- Failed agents turn the entire grid border red (`theme.danger`)
- Selection highlight with `selection_background` color

The `agent_status_grid.rs` widget uses a separate icon set: filled circle (active),
empty circle (idle), checkmark (done), cross (failed) -- slightly inconsistent with the
roster's triangle/middot icons but both are readable.

**Missing states:**
- No "starting" / "spawning" transitional state
- No "retrying" state (would be useful when attempt > 1)
- No "waiting for approval" state (the runner has approval gates)

## 5. Token/cost per agent: visible?

**Rating: Good**

Per-agent token and cost data is shown in multiple places:

1. **Roster row**: tokens (compact: k/M format), cost (USD), elapsed, last-event age
2. **Route metrics bar**: context used/limit with color-coded utilization
3. **Summary line**: aggregate active count, total tokens, total cost
4. **Token sparkline** (left panel bottom, 6 rows): time-series chart of token usage
5. **Selected agent detail line**: plan, task, turns, uptime

Cost is displayed as `$X.XX` when above $0.001, otherwise `-`. Total cost gets a warning
color when above $1.00.

**Missing:**
- No per-agent cost breakdown (input cost vs output cost)
- No cumulative cost delta / burn rate
- Mori shows cache-hit, research-prepass, and verify-artifacts-fresh flags in the detail
  header -- roko does not expose per-agent efficiency metadata flags

## 6. Attempt/retry information: shown?

**Rating: Partially shown**

The `AgentRow.attempt` field (1-based attempt number) is displayed in the output panel
title bar when `attempt > 0`:

```
Output . agent-id . running (attempt 2)
```

**Issues:**
- Attempt info is only visible in the right-panel title for the selected agent
- Not shown in the roster rows at all -- if 3 of 10 agents are on retries, you can't
  tell without clicking each one
- No history of previous attempts: if an agent failed attempt 1 and is on attempt 2,
  there's no way to see what attempt 1's output was
- Max retry count and retry reason are not displayed

## 7. How does it handle many agents (10+)?

**Rating: Adequate but fragile**

The agent status grid (`agent_status_grid.rs`) has an overflow indicator:
```rust
if entries.len() > visible_rows {
    let remaining = entries.len() - visible_rows;
    lines.push("... +{remaining} more");
}
```

The main roster in `agents_view.rs` renders all agents as `Paragraph` lines and relies
on ratatui's area clipping to hide overflow. There is:
- No explicit overflow indicator in the roster itself
- No scroll offset tracked for the roster (unlike mori which has `agent_scroll` + `agent_list_cursor`)
- Selection via `view_state.selected` indexes into `agent_summaries` but there is no
  visible-window calculation to keep the selected row in view

**At 10+ agents:**
- Summary line and sparkline still render (2+6 rows), leaving roster height = panel height - 8
- In a 40-row terminal, that's ~30 rows for agents: workable
- At 30+ agents, many will be off-screen with no scroll indicator

**Mori comparison:**
- Mori tracks `agent_list_cursor` and `agent_scroll` together and explicitly slices the
  visible window: `list_lines[start..end]`
- Mori supports parallel mode with a dedicated `parallel_pool` widget in the left panel

## 8. Is there an agent timeline/activity view?

**Rating: Partial -- topology only**

There is an **Agent Topology** panel (toggled via Ctrl+T) that replaces the output body.
It renders a tree view:
```
pool: default
    agent-1 [active]
        task: implement-foo (running)
    agent-2 [done]
        no active tasks
```

This shows the relationship between agents and their assigned tasks, but it is NOT a
timeline. There is no:
- Gantt chart or timeline visualization of agent activity
- History of when each agent started/stopped
- Turn-by-turn activity log with timestamps
- Overlap visualization (which agents ran concurrently)

The topology is fetched on demand and scrollable, with status labels per node.

## 9. What's missing compared to mori's agent view?

| Feature | Mori | Roko | Gap |
|---|---|---|---|
| Role-filtered output tabs | Each tab shows that role's output | Tabs are decorative; output follows roster selection | **Functional gap** |
| Agent spinner animation | `atmosphere.spinner()` in roster + title | No animated spinner; static icons only | **Cosmetic gap** |
| Detail header block | 6-line dedicated header: plan, task, thread, turns, tokens gauge, in/out/cost/iter | Route metrics bar (1 line) has model/ctx/focus/tier; less info | **Density gap** |
| Per-agent provider/route/strategy | Model, provider, route source, context strategy in detail header | Only model and tier shown | **Missing** |
| Scrollbar widget | `widgets::scrollbar::render_scrollbar` on output panel | No scrollbar rendered; only TAIL/PINNED text indicator | **Missing** |
| Output segment parsing | Full parser: thinking/heading/tool-use/code/success/error/blank/turn-marker with bubble backgrounds | Basic semantic coloring via `render_agent_output_lines` | **Simpler** |
| Turn-alternating bubble backgrounds | Alternating `BG_SECONDARY` / `BG_BUBBLE_ALT` per turn | No turn-based background alternation | **Missing** |
| Parallel mode left panel | Dedicated `parallel_pool::render` replaces roster when parallel agents exist | `parallel_pool.rs` exists but is not called from agents_view.rs left panel | **Not wired** |
| Cache-hit / research / verify flags | Shown in parallel detail header per agent | Not shown | **Missing** |
| "Completed in prior run" state | Distinct display for `CompletedPrior` plans | Not supported | **Missing** |
| Verification sub-panel | Separate `[verify]` tab group with per-plan verify entries | Not present | **Missing** |
| Inline markdown parsing | `parse_inline_markdown()` for bold/code/links | Not implemented | **Missing** |
| Empty state messages per role | Role-specific empty messages (conductor shows its consultation triggers) | Generic "waiting for agent output..." | **Less helpful** |
| Scroll position indicator | "N lines above" + "[End] to resume" in output | `[TAIL]` / `[PINNED line N]` in title | **Equivalent** |

## 10. Proposals

### A. Agent activity sparklines (per-agent)

Add a mini-sparkline (8-12 chars wide) to each roster row showing token usage over the
last N turns. Data source: `efficiency_events` already has per-agent per-turn token counts.

Implementation sketch:
- In `render_agent_roster`, after the gradient gauge bar, append a mini sparkline using
  braille or block characters showing the last 8 turns' token deltas
- Color: role accent for active, muted for inactive
- This gives at-a-glance "is this agent working hard or idle" without selecting it

### B. Live typing indicator

When an agent's output is actively growing (i.e., the `last_output_line` changed since
last frame, or `AgentStream.last_chunk_at` is within the last 2 seconds), show a pulsing
indicator:

- Roster: append a blinking dot or ellipsis after the status badge
- Output panel title: append a typing indicator like `...` with breathing brightness
- Could reuse the `atmosphere.breathing_brightness()` pattern already in token_sparkline

Implementation: in `render_agent_roster`, check `agent_row.last_event_at_ms` against
current time. If within 2000ms, append a pulsing span. The `AgentStream.last_chunk_at`
provides even more granular data for the live stream panel.

### C. Model badge per agent

Replace the plain model text in the roster with a styled badge:

```
[opus-4-6]  [s4]  [haiku-4-5]  [gpt-4o]
```

- Background: muted role color at low opacity
- Text: shortened model slug via existing `shorten_model()`
- Color-code by provider: Anthropic=rose, OpenAI=green, Gemini=blue, Cerebras=amber
- Already have `display_model()` in display_utils.rs; need a `provider_color()` helper

This makes it instantly visible which model each agent is using without reading the
route metrics bar.

### D. Additional high-value proposals

**D1. Wire role tabs to filter output** (severity: functional gap)
Currently the role tabs change `sub_tab` but don't filter. Wire them so selecting a role
tab shows all output from agents of that role, concatenated. The roster selection and tab
selection should be independent dimensions.

**D2. Wire parallel_pool.rs** (severity: dead code)
`parallel_pool.rs` renders a proper table (agent id, role, model, task, progress,
cumulative usage) but is never called from `agents_view.rs`. Mori switches to it when
`!parallel_agents.is_empty()`. Add the same condition.

**D3. Add scrollbar to output panel**
Mori renders a scrollbar widget. Roko's output panel only has a title indicator. Add a
thin scrollbar on the right edge of the output area when content overflows.

**D4. Search in agent output** (severity: significant gap)
The `LogSearchState` infrastructure exists and works for F4/Log. Wire the same `/` keybind
and highlight/filter modes into the agent output panel. Reuse `LogSearchState` or add a
per-view search state to `ViewState`.

**D5. Attempt badge in roster rows**
Show attempt number in the roster for agents on retry: append `(R2)` or similar after
the status badge, colored in warning when attempt > 1.

---

## Summary

The F3 Agents view is one of roko's most complete TUI screens. The left roster is
information-dense with good color coding, gradient gauges, and sorted prioritization.
The right panel has working scroll, live stream fallback, and route metrics. The main
gaps vs mori are: role tabs don't filter output (decorative only), no animated spinners,
no scrollbar widget, no search in agent output, no per-agent provider/route detail header,
and the `parallel_pool.rs` widget is built but not wired. The proposed sparklines, typing
indicator, and model badges would add meaningful at-a-glance density without clutter.

---

## Implementation Status (2026-09-02 swarm)

F3 Agents view improvements (task #13): agent roster density, output rendering, role tab
switching. Agent output streaming improvements (task #26): semantic parsing, auto-scroll.
Active agent animated braille spinners added (task #22, effects).
