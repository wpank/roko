# 127 — F7 Inspect View Parity (Three-Panel Layout: MCP + Learning + Prompt Stats)

**Priority**: P2 — The F7:inspect tab exists (1,110 LOC) but is missing the MCP runtime status panel, learning metrics panel, and prompt stats panel that made Mori's F7 the "single pane of glass" for understanding what the system knows and how it's routing.
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/tui/tabs.rs`, context view rendering files
**Depends on**: #121 (TUI data model unification provides the single source)
**Sources**: `tmp/backlog/_checklist-gaps.md` §3.1, `tmp/backlog/_mori-old-gaps.md` MO-20, MO-12

---

## Background

Mori's F7:inspect panel was a three-column dense view showing three categories of runtime intelligence simultaneously:

**Column 1 — MCP Runtime**:
- MCP server config paths (which `.cursor/mcp.json` is active)
- Tool inventory count and names
- Backend status (healthy/unhealthy)
- Per-worktree routing status
- AST index: file count, symbol count, reference count, resolution %

**Column 2 — Learning State**:
- Episode count (pass vs fail), rolling accuracy
- Playbook rule count (learned vs manual)
- Cascade router stage and routing coverage %
- Gate threshold values per rung
- Model stats: pass rate, avg duration, retry rate per model

**Column 3 — Prompt Stats**:
- Avg prompt tokens per role (from efficiency events)
- Avg context window utilization per role
- Top 3 sections by token cost
- Section effectiveness scores (from `SectionEffectivenessRegistry`)

Roko's current F7 (`context_view.rs`, 1,110 LOC) shows: system health, token burn per role, cost per model, cascade router state, and alerts. The MCP server status, playbook rule count, routing coverage %, and AST index stats are absent. All data exists in files/memory; the gap is the panel composition.

## Current State

- `crates/roko-cli/src/tui/` — `context_view.rs` implements the current F7 rendering.
- Data sources:
  - Episodes: `.roko/episodes.jsonl` — readable by TUI.
  - Playbook: `.roko/learn/playbook.json` — readable by TUI.
  - Cascade router: `.roko/learn/cascade-router.json` — readable by TUI.
  - Gate thresholds: `.roko/learn/gate-thresholds.json` — readable by TUI.
  - MCP tool count: available from `roko-mcp-code` if queried.
  - AST index stats: available from `roko-index` if queried.
  - Efficiency events: `.roko/learn/efficiency.jsonl` — readable by TUI.
  - Section effectiveness: `SectionEffectivenessRegistry` — needs to be serialized to disk.
- Missing: the three-column layout and the specific panels listed above.

## Implementation Plan

1. **Three-column layout**: Use `ratatui::layout::Layout::horizontal` with three equal-width columns. Each column is a bordered panel.

2. **Column 1 — MCP Runtime panel**:
   - Read MCP config path from `roko.toml` and display whether it exists.
   - Display tool count from `roko-mcp-code` stats file (write a stats file at MCP server startup to `.roko/state/mcp-stats.json`).
   - Display AST index stats from `.roko/index/stats.json` (if available).

3. **Column 2 — Learning State panel**:
   - Episode count: read from `.roko/episodes.jsonl` (count lines, filter pass/fail).
   - Playbook rule count: read `.roko/learn/playbook.json` and count rules.
   - Routing coverage: from `.roko/learn/cascade-router.json` (fraction of models with non-default weights).
   - Gate thresholds: read `.roko/learn/gate-thresholds.json`, show threshold per rung.
   - Model stats: aggregate from cascade router data.

4. **Column 3 — Prompt Stats panel**:
   - Avg prompt tokens per role: aggregate from `.roko/learn/efficiency.jsonl` by role.
   - Avg context window utilization: `avg_prompt_tokens / role_context_limit`.
   - Top sections by token cost: aggregate section token costs from efficiency events.
   - Section effectiveness: from `SectionEffectivenessRegistry` if serialized to `.roko/learn/section-effectiveness.json`.

5. **Refresh cadence**: F7 panel data is refreshed once per 5 seconds (not every frame) since it requires disk reads. Cache in `TuiModel.inspect_data`.

6. **Preserve existing content**: The current F7 content (system health, cost, alerts) moves to a fourth panel below the three-column layout, or is merged into the three columns where it fits logically.

## Acceptance Criteria

1. F7 tab shows a three-column layout with MCP, Learning, and Prompt Stats panels.
2. MCP config path is displayed and shows whether the file exists.
3. Episode count (pass/fail) is non-zero after at least one plan run.
4. Playbook rule count is non-zero if any rules exist in `.roko/learn/playbook.json`.
5. Gate threshold values are displayed per rung.
6. Avg prompt tokens per role shows non-zero values after at least one agent dispatch.
7. Existing system health and cost display is preserved (not removed).

## Verification Checklist

- [ ] Run a plan; open F7; verify episode count matches `wc -l .roko/episodes.jsonl`.
- [ ] If playbook.json has rules, verify the count is displayed.
- [ ] Verify gate threshold values match `.roko/learn/gate-thresholds.json`.
- [ ] Verify MCP config path is displayed (even if file is missing, show the path and "not found").
- [ ] Verify three-column layout renders without overflow on an 120-column terminal.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/context_view.rs` | Rebuild as three-column layout; add MCP, learning, prompt panels |
| `crates/roko-cli/src/tui/app.rs` | Add `inspect_data: InspectData` to `TuiModel` with refresh logic |
| `crates/roko-cli/src/tui/mod.rs` | Wire 5-second refresh timer for inspect data |
