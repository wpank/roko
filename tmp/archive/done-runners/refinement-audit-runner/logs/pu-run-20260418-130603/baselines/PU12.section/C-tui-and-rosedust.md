# C — Rosedust Design, TUI Layout, 29 Screens (Docs 07, 08, 09)

Parity of the three TUI chapters: Rosedust design language (400
lines), TUI main layout (577 lines), TUI 29 screens (1,589 lines).

The TUI is **massively shipping** — `crates/roko-cli/src/tui/`
totals **25,449 LOC** across 20+ module files, 13 modals, and ~10
widgets. What's shipping is a 7-tab TUI (not the full 29-screen
design); the rosedust color palette is a shipping theme constant set
but not the full rendering language described in Doc 07.

Generated: 2026-04-16.

---

## C.01 — TUI entry point via `roko dashboard` (Doc 08 §"TUI Layout", CLAUDE.md)

**Status**: DONE
**Severity**: —
**Doc claim**: `roko dashboard` launches the interactive TUI.
**Reality**: CLAUDE.md row: `Interactive TUI (ratatui) | Wired | crates/roko-cli/src/tui/, F1–F7 tabs, roko dashboard`. Shipping.

---

## C.02 — 7 tabs mapped to F1-F7 (Doc 08 §"Main Layout", Doc 09 §"Screen List")

**Status**: DONE
**Severity**: —
**Doc claim**: Top-level tab bar for F1-F7 navigation.
**Reality**: `crates/roko-cli/src/tui/tabs.rs:1-45` ships exactly 7 `Tab` variants mapped to `KeyCode::F(1)..F(7)`:
- F1 Dashboard — "Overview dashboard with health gauges, plan progress, cost"
- F2 Plans — "Plan tree, task progress, wave overview"
- F3 Agents — "Agent output, diffs, token burn, parallel pool"
- F4 Git — "Git branch tree, commit graph, worktree list"
- F5 Logs — "Scrollable log viewer with filtering"
- F6 Config — "Config editor / effective config view"
- F7 Inspect — "Engram DAG inspector, episode replay"

`Tab::ALL` array at `:30-38` enumerates them. File comment at `:1-4` explicitly cites "Mori-style TUI navigation".

---

## C.03 — 29 screens vs 7 tabs reality (Doc 09 §"29 Screens")

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 09 (1,589 lines!) tables 29 distinct TUI screens covering dashboard, plan-tree, task-detail, agent-output, git-branch-tree, log-viewer, config-editor, inspect-DAG, plus ~20 more specialized screens (search, filter, command-palette, wave-overview, etc.).
**Reality**: The shipping TUI has **7 primary tabs + 13 modal overlays**, not 29 distinct screens:
- 7 top-level tabs (C.02)
- 13 modals at `crates/roko-cli/src/tui/modals/`: `agent_pool_modal.rs, approval.rs, batch_review.rs, confirm.rs, help.rs, inject.rs, notification.rs, plan_detail.rs, queue_overview.rs, quit.rs, task_detail.rs, task_picker.rs, wave_overview.rs`

Total 20 views, not 29. The remaining 9 screens Doc 09 describes are either (a) sub-views within a tab (no distinct file, handled by widget composition), or (b) frontier. The doc's 29-screen enumeration is richer than the shipping tab+modal surface.
**Fix sketch**: Doc 09 should reconcile its 29-screen enumeration with the shipping 7-tab + 13-modal reality. Either re-classify the "screens" into (tab, modal-overlay, sub-view) categories or mark unshipped screens frontier.

---

## C.04 — Tab widgets: plan_tree, task_progress, token_sparkline, wave_progress (Doc 08 §"Widgets")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 08 describes core widgets for each tab.
**Reality**: `crates/roko-cli/src/tui/widgets/` ships 10+ widgets, with substantial ones:
- `plan_tree.rs` (1,015 LOC)
- `task_progress.rs` (505 LOC)
- `phase_compact.rs` (359 LOC)
- `token_sparkline.rs` (216 LOC)
- `sys_metrics.rs` (216 LOC)
- `status_bar.rs` (180 LOC)
- `parallel_pool.rs` (171 LOC)
- `wave_progress.rs` (119 LOC)
- `header_bar.rs` (visible in grep)
- `rosedust.rs` (9 LOC — theme-color stub only)

Widget surface is deep. The 1,015-LOC `plan_tree.rs` alone indicates substantial per-tab rendering.

---

## C.05 — 13 modal overlays ship (Doc 09 §"Modals")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 09 describes modal overlays for task detail, confirmation, help, etc.
**Reality**: `tui/modals/` ships 13 modal types covering task picker, task detail, plan detail, wave overview, queue overview, agent pool, approval, batch review, confirm, help, inject, notification, quit. This matches Doc 09's modal enumeration.

---

## C.06 — Rosedust color palette ships as theme constants (Doc 07 §"Color Palette")

**Status**: DONE (palette only; not full design language)
**Severity**: —
**Doc claim**: Doc 07 §"Color Palette" specifies the Rosedust palette: void (#1A1520), fg (#E8DFD5), fg-muted (#8A7F8E), rose (#D4778C), teal (#5DB8A3), gold (#D4A857), danger (#C45C50), sapphire (#6B8FBD), twilight (#221D2A), selection (#2D2838). WCAG AA contrast verified (Doc 17 §"Perceivable").
**Reality**: `Grep 'rosedust\|Rosedust' crates/roko-cli/src/tui/` hits `dashboard.rs`, `widgets/rosedust.rs`, `widgets/header_bar.rs`, `widgets/token_sparkline.rs`, `widgets/plan_tree.rs`, `widgets/wave_progress.rs`, `widgets/mod.rs`. The test `theme_defaults_to_rosedust_palette` at `dashboard.rs:4951` pins the palette. `widgets/rosedust.rs` is 9 LOC — just color constants + theme enum. Palette shipping at the theme-constants level.

---

## C.07 — Rosedust full design language (typography, spacing, motion) is PARTIAL (Doc 07 §"Typography", §"Motion")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 07 (400 lines) describes a full design language — typography scale, spacing rhythm, motion primitives (duration curves, easing), component patterns.
**Reality**: TUI is text-mode — typography and most motion concepts don't apply. What ships: color palette (C.06), theme constants, status-bar patterns, widget layout conventions. The "full design language" framing in Doc 07 applies more to the web portal (Doc 13) than the TUI.
**Fix sketch**: Doc 07 should scope the "design language" to cross-interface concerns (palette + iconography + tone-of-voice) and defer typography/motion primitives to the web portal chapter (which is itself frontier; see E.01).

---

## C.08 — PostFX / effects pipeline (Doc 08 §"Effects")

**Status**: DONE (additional shipping surface)
**Severity**: —
**Doc claim**: Doc 08 mentions post-processing effects in passing.
**Reality**: `crates/roko-cli/src/tui/` ships `postfx.rs`, `postfx_pipeline.rs`, `effects_config.rs`, `atmosphere.rs` — a substantial post-processing pipeline. These aren't in Doc 08's core chapters but are real shipping surface.
**Fix sketch**: Doc 07 / 08 should add a §"PostFX Pipeline" subsection pointing at these files.

---

## C.09 — Modal input routing + hit_test + scroll (Doc 08 §"Input")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 08 describes input routing via modal stack + scroll handling.
**Reality**: `tui/input.rs`, `tui/hit_test.rs`, `tui/scroll.rs` ship. Real input routing.

---

## C.10 — Approval IPC (Doc 09 §"Approval")

**Status**: DONE
**Severity**: —
**Doc claim**: Approval flow for agent actions pending user confirmation.
**Reality**: `tui/approval_ipc.rs` ships + `modals/approval.rs` ships. The IPC side connects the TUI approval modal to the agent runtime.

---

## C.11 — Config editor tab (F6) (Doc 09 §"Config Tab")

**Status**: DONE
**Severity**: —
**Doc claim**: F6 config editor lets user view + edit effective config.
**Reality**: `tui/config_meta.rs` ships + F6 Config tab wiring at `tabs.rs:20`. Config editing UI present.

---

## C.12 — Inspect tab (F7) Engram DAG + episode replay (Doc 09 §"Inspect Tab")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: F7 Inspect tab shows Engram DAG + episode replay UI.
**Reality**: F7 Inspect tab ships at `tabs.rs:22` per the tab enum. Whether the full DAG visualization + episode replay UI ships or is stub view is unverified — would require reading state.rs + dashboard.rs. Flag partial.

---

## C.13 — Command palette, search, filter (Doc 09 §"Cross-Cutting UI")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 09 describes cross-cutting UI primitives: command palette, global search, per-view filter.
**Reality**: `Grep 'command_palette\|global_search' crates/roko-cli/src/tui/ --include=*.rs` returns no explicit matches. Specific screens not verified. Likely frontier.
**Fix sketch**: Doc 09 §"Cross-Cutting UI" should flag command palette / global search as frontier.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 9 (C.01 roko dashboard, C.02 7 tabs F1-F7, C.04 widgets, C.05 13 modals, C.06 rosedust palette, C.08 PostFX pipeline, C.09 input routing, C.10 approval IPC, C.11 config tab) |
| PARTIAL | 3 (C.03 7-tab+13-modal vs 29-screen doc, C.07 rosedust as palette-only, C.12 inspect tab depth) |
| NOT DONE | 1 (C.13 command palette / global search) |

Section C shows **a massively shipping TUI** that undersells itself
in Doc 17's "Scaffold" status. 25,449 LOC of TUI code + 7 tabs + 13
modals + 10 widgets + PostFX pipeline + atmosphere / effects config.
The biggest doc drift is Doc 09's "29 screens" — the shipping reality
is a 7-tab + 13-modal surface, richer than a single flat 29-screen
enumeration.

## Agent Execution Notes

### C.03 — Reconcile 29 screens with 7 tabs + 13 modals

Doc 09's 29-screen list should be re-classified. Each "screen"
becomes either (a) a top-level tab, (b) a modal overlay, (c) a
widget within a tab, or (d) frontier. The shipping 7 + 13 = 20
views covers most of it; the remaining 9 are widgets or frontier.

### C.07 — Rosedust palette vs language

The shipping `widgets/rosedust.rs` (9 LOC) is the palette constants
only. The full "design language" (typography, spacing, motion) is
mostly web-portal concern — move to Doc 13 or mark frontier.

Acceptance criteria:

- Doc 09 reconciles 29-screen enumeration with 7-tab + 13-modal reality,
- Doc 07 scopes design-language to shipping palette + theme,
- Doc 08 adds PostFX pipeline subsection.
