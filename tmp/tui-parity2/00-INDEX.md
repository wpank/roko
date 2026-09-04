# TUI Parity Phase 2: Usability, Performance & Visual Audit — 2026-09-01

> **Status (2026-09-04):** ~35/38 items verified. Remaining: workflow contract proof (#116), PNG renderer (#151), resizable panes (#199). Near-archival.

> **Implementation handoff:** CLI-relevant P0/P1/P2 findings are mapped to mechanical packets and serialized file lanes in [`../cli-audit/IMPLEMENTATION-CHECKLIST.md`](../cli-audit/IMPLEMENTATION-CHECKLIST.md); coverage is recorded in [`../cli-audit/FINDINGS-COVERAGE-MATRIX.md`](../cli-audit/FINDINGS-COVERAGE-MATRIX.md). The key owners are #128 and #365–#369.

> Phase 2 re-verifies the earlier `tui-parity/` completion claim against production producers,
> consumers, persisted run evidence, and full-frame captures. It also uses Mori as the visual
> baseline and Bardo as design context. Source presence and checked backlog boxes are not accepted
> as end-to-end completion.

---

## Executive Summary

**2026-09-02 swarm session: all 30 implementation tasks completed.** A parallel swarm of
specialized agents addressed every actionable finding from the Phase 2 audit in a single session.
The verified count moves from the original 21/11/6 to materially improved across all categories.

The 2026-09-01 audit found the verified result was **21 verified, 11 partial, and 6 not operational**
at baseline `fced716b6`. The 2026-09-02 swarm session addressed the following:

### Key accomplishments (2026-09-02)

- **Performance:** Theme::from_env() per-frame rebuild eliminated (#366); render-path waste reduced.
- **Modal/input:** Modal input precedence fixed (#365); mouse scroll/click routing corrected (#368).
- **Navigation:** Breadcrumb trail added below header; sub-tab bar extended to all 10 tabs; focus
  zone labels added; consistent Tab:panel hints across all views; input mode badges (FILTER/SEARCH/
  INJECT/EDIT) in breadcrumb and status bar.
- **Scroll/focus:** Independent Diff/Procs scroll (#P7.3, #6.5) fixed; focus zone isolation per tab.
- **Data pipeline:** Critical-path ETA wired (#196, P1.5); NET/DSK metrics correctness fixed (#240,
  P4.3); plan detail enrichment wired (#238, P5.1-P5.5).
- **Notifications:** Notification history with bounded retention added (#369).
- **Empty states:** Contextual empty state messages added across all views (#236).
- **Per-view improvements:** All 10 tab views (F1-F10) received targeted layout, widget, and
  interaction improvements based on their individual audit reports.
- **Visual:** ROSEDUST palette fixes; unicode/typography improvements; responsive layout fixes;
  animation/effects tuning; widget visual quality improvements; data visualization enhancements.
- **Status bar:** Input mode indicator; elapsed time; budget utilization.
- **Agent output:** Streaming improvements and output rendering enhancements.
- **Number keys:** Fixed shadowing on Plans and remaining tabs (#237).
- **Queue overview:** Fixed modal key binding and source (#216).

The highest-impact remaining work is an acknowledged command contract, true gate-line streaming,
terminal source convergence, and deterministic styled/PNG visual comparison. See `36-OPEN-GAPS.md`.

---

## File Index

| # | File | Topic | Priority |
|---|------|-------|----------|
| 00 | `00-INDEX.md` | This file: master index, priority categories, cross-references | — |
| 01 | `01-performance-render-loop.md` | Render loop timing, frame budget, jank, adaptive framerate tuning | P0 |
| 02 | `02-performance-data-pipeline.md` | Data flow bottlenecks, channel backpressure, memory growth, StateHub throughput | P0 |
| 03 | `03-modal-behavior.md` | Modal dismiss bugs, focus trapping, overlay layering, z-order | P0 |
| 04 | `04-keybindings-complete.md` | Full keybinding audit: conflicts, dead keys, missing bindings, help accuracy | P1 |
| 05 | `05-navigation-information-architecture.md` | Tab hierarchy, sub-tab discovery, drill-down paths, breadcrumbs | P1 |
| 06 | `06-color-theme-system.md` | ROSEDUST palette audit, WCAG contrast, 256-color fallback, semantic color usage | P2 |
| 07 | `07-widget-visual-quality.md` | Per-widget visual polish: borders, padding, alignment, truncation, wrapping | P2 |
| 08 | `08-data-visualization-graphs.md` | Sparklines, progress bars, gauges, bar charts, DAG rendering, data density | P3 |
| 09 | `09-animations-effects.md` | Atmospheric effects, transitions, spinners, cursor blink, demoscene potential | P3 |
| 10 | `10-f1-dashboard-view.md` | F1 Dashboard: layout, header bar, system metrics, overview panels | P2 |
| 11 | `11-f2-plans-view.md` | F2 Plans: plan list, task tree, detail modal, progress display | P1 |
| 12 | `12-f3-agents-view.md` | F3 Agents: agent list, output streaming, role tabs, attempt tracking | P1 |
| 13 | `13-f4-git-view.md` | F4 Git: diff display, branch info, worktree status, commit history | P2 |
| 14 | `14-f5-logs-view.md` | F5 Logs: signal log, episode log, search/filter, tail behavior | P1 |
| 15 | `15-f6-config-view.md` | F6 Config: config editor, provider health, MCP status, secrets display | P2 |
| 16 | `16-f7-inspect-view.md` | F7 Inspect: sub-tabs, data explorer, playbook viewer, three-panel layout | P2 |
| 17 | `17-f8-f9-f10-views.md` | F8 Marketplace, F9 Atelier, F10 Learning: consolidated audit of lower-traffic tabs | P2 |
| 18 | `18-bardo-prd-visual-vision.md` | Bardo PRD 18 (interfaces) and 25 (mori) visual design intent extraction | Ref |
| 19 | `19-bardo-screen-specs-rendering.md` | Bardo screen specifications: layout grids, component specs, rendering contracts | Ref |
| 20 | `20-ratatui-capabilities.md` | ratatui feature audit: what is available that roko does not use | Ref |
| 21 | `21-mori-tui-reference.md` | Mori TUI implementation reference: what works, what to adopt, what to skip | Ref |
| 22 | `22-scroll-focus-selection.md` | Scroll behavior, focus management, list selection, cursor persistence | P1 |
| 23 | `23-empty-loading-error-states.md` | Empty states, loading indicators, error display, contextual help text | P2 |
| 24 | `24-layout-responsive.md` | Terminal size handling, responsive breakpoints, minimum viable layout | P4 |
| 25 | `25-unicode-icons-typography.md` | Unicode symbols, box-drawing, Nerd Font icons, text density, line spacing | P4 |
| 26 | `26-agent-output-streaming.md` | Live agent output rendering, semantic parsing, render cache, auto-scroll | P1 |
| 27 | `27-status-progress-notifications.md` | Status bar, progress indicators, toast notifications, alert hierarchy | P2 |
| 28 | `28-existing-backlog-relevant-items.md` | Triage of Phase 1 leftovers (PX, MX items) and GAPS.md TUI entries | Ref |
| 30 | `30-VERIFIED-CLAIM-MATRIX.md` | Re-verification of all 38 P0-P7 claims | Evidence |
| 31 | `31-LIVE-RUN-EVIDENCE.md` | Persisted successful-run timeline and failure reconstruction | Evidence |
| 32 | `32-SCREENSHOT-HARNESS-STATUS.md` | Static/continuous screenshot implementation and evidence limits | Evidence |
| 33 | `33-MORI-BARDO-DECISIONS.md` | Patterns adopted, adapted, and rejected | Decision |
| 34 | `34-BACKLOG-CROSSWALK.md` | Acceptance-criterion crosswalk for UX/TUI/workflow backlog | Evidence |
| 35 | `35-IMPLEMENTED-FOLLOWUPS.md` | Bounded source fixes completed during this audit | Result |
| 36 | `36-OPEN-GAPS.md` | Prioritized remaining implementation and live-fixture queue | Plan |
| 37 | `37-MORI-WORKFLOW-PARITY-AUDIT.md` | Mori/Bardo workflow parity: queue, DAG, parallelism, worktrees, merge, recovery | P0/P1 |
| — | `evidence/README.md` | Checked-in all-tab full-frame text capture matrix | Evidence |

---

## Priority Categories

### P0: Critical — Performance, Crashes, Modal Bugs

Issues that cause data loss, hangs, panics, or make the TUI materially unusable. Must fix
before any other work.

| Report | Scope |
|--------|-------|
| `01-performance-render-loop.md` | Frame budget violations, render jank, dropped frames |
| `02-performance-data-pipeline.md` | Channel saturation, unbounded memory growth, StateHub bottlenecks |
| `03-modal-behavior.md` | Modal dismiss failures, focus escapes, overlay corruption |

### P1: Usability — Keybindings, Navigation, Scroll, Core Views

Issues that prevent productive use. The TUI runs but the operator cannot efficiently
navigate, interact, or understand what is happening.

| Report | Scope |
|--------|-------|
| `04-keybindings-complete.md` | Dead keys, conflicts, missing bindings, help text accuracy |
| `05-navigation-information-architecture.md` | Tab/sub-tab discovery, drill-down, breadcrumbs |
| `11-f2-plans-view.md` | Plan list usability, task tree interaction, detail modal UX |
| `12-f3-agents-view.md` | Agent output readability, role tab switching, attempt display |
| `14-f5-logs-view.md` | Log search, filter, tail, signal/episode differentiation |
| `22-scroll-focus-selection.md` | Scroll persistence, focus management, list cursor behavior |
| `26-agent-output-streaming.md` | Live output rendering, semantic parsing, auto-scroll |

### P2: Visual — Color, Layout, Widget Quality, Per-View Polish

Issues that make the TUI look unfinished or hard to read. Functional but aesthetically
deficient.

| Report | Scope |
|--------|-------|
| `06-color-theme-system.md` | Palette consistency, WCAG compliance, terminal compatibility |
| `07-widget-visual-quality.md` | Border styles, padding, alignment, text truncation |
| `10-f1-dashboard-view.md` | Dashboard layout, header bar density, metric panels |
| `13-f4-git-view.md` | Diff rendering, branch display, worktree indicators |
| `15-f6-config-view.md` | Config editor layout, provider health cards, MCP panel |
| `16-f7-inspect-view.md` | Inspect sub-tab layouts, data explorer, playbook viewer |
| `17-f8-f9-f10-views.md` | Learning/Knowledge/Extensions tab visual quality |
| `23-empty-loading-error-states.md` | Contextual empty states, loading skeletons, error formatting |
| `27-status-progress-notifications.md` | Status bar layout, progress indicators, toast design |

### P3: Wow Factor — Animations, Effects, Graphs, Demoscene

Features that elevate the TUI from functional to impressive. Data visualizations that
provide genuine insight. Atmospheric effects that create presence.

| Report | Scope |
|--------|-------|
| `08-data-visualization-graphs.md` | Sparkline resolution, DAG rendering, bar charts, gauges |
| `09-animations-effects.md` | Effect presets, transitions, spinners, ambient effects |

### P4: Refinement — Icons, Typography, Responsive Layout

Fine-grained polish that matters for long sessions but is not blocking.

| Report | Scope |
|--------|-------|
| `24-layout-responsive.md` | Small terminal handling, column collapse, minimum size |
| `25-unicode-icons-typography.md` | Icon consistency, Nerd Font usage, text density tuning |

### Reference — External Context, Not Actionable Alone

Research material that informs the actionable reports but does not itself contain issues
to fix.

| Report | Scope |
|--------|-------|
| `18-bardo-prd-visual-vision.md` | Design intent from bardo PRDs (sections 18, 25) |
| `19-bardo-screen-specs-rendering.md` | Screen specs, layout grids, component contracts |
| `20-ratatui-capabilities.md` | Unused ratatui features and widgets |
| `21-mori-tui-reference.md` | Mori TUI patterns to adopt or avoid |
| `28-existing-backlog-relevant-items.md` | Phase 1 leftovers and GAPS.md TUI entries |

---

## Cross-Reference: Phase 1 (tui-parity/)

Phase 1 (`tmp/tui-parity/`) ran three parallel audit campaigns (v1: 19 agents, v2: 15
agents, parity checklist: 53 items) and produced a consolidated tracker of 68 items across
P0-P7, PX, and MX tiers.

### Completed Phase 1 Work

| Phase 1 Item | Status | Relevance to Phase 2 |
|---|---|---|
| **P0 (dead TUI fix)** | 4/5 implemented, 1 partial | P2 inherits P0.5 (learning/efficiency bridge) via `28-existing-backlog-relevant-items.md` |
| **P1 (interaction)** | 4/6 implemented, 2 partial | P2 inherits P1.1 (command channel) and P1.5 (critical-path ETA) |
| **P2 (gate output)** | 0/3 operational | P2 inherits all three gate-output items |
| **P3 (performance)** | 2/3 implemented, 1 partial | P2 deepens with `01-performance-render-loop.md` and `02-performance-data-pipeline.md` |
| **P4 (visual polish)** | 6/7 verified, 1 partial | P2 extends visual polish with per-view and per-widget reports |
| **P5 (plan detail)** | 0/5 operational | P2 inherits via `11-f2-plans-view.md` |
| **P6 (keyboard)** | 2/5 implemented, 3 partial | P2 deepens with `04-keybindings-complete.md` |
| **P7 (sub-tabs)** | 2/4 implemented, 2 not operational | P2 inherits via per-view reports |
| **PX (parity)** | 0/11 implemented | P2 triages via `28-existing-backlog-relevant-items.md` |
| **MX (medium-term)** | 0/10 tracked | P2 inherits selectively; MX.8 (AgentOutputWidget) covered by `26-agent-output-streaming.md` |

### Phase 1 Root Causes (for context)

| ID | Root Cause | Phase 2 Coverage |
|---|---|---|
| RC-1 | Two disconnected data models (DashboardData vs TuiState) | `02-performance-data-pipeline.md` |
| RC-2 | Recovery keybindings facade-only | `04-keybindings-complete.md` |
| RC-3 | Gate output stripped before TUI | `27-status-progress-notifications.md`, per-view reports |
| RC-4 | Disk I/O in render path | `01-performance-render-loop.md` |
| RC-5 | Built-but-not-rendered features | Per-view reports (10-17) |

---

## Cross-Reference: Bardo PRDs Consulted

The following bardo PRD sections inform the visual vision and screen specifications
analyzed in reports 18 and 19:

| PRD Section | Path | Relevance |
|---|---|---|
| **00-vision/** | `bardo-backup/prd/00-vision/` | Overall product vision, aesthetic direction |
| **18-interfaces/** | `bardo-backup/prd/18-interfaces/` | TUI interface specifications, layout contracts, interaction patterns |
| **25-mori/** | `bardo-backup/prd/25-mori/` | Mori orchestrator UI design, the reference implementation roko replaces |
| **07-tools/** | `bardo-backup/prd/07-tools/` | Tool display and interaction specifications |
| **13-runtime/** | `bardo-backup/prd/13-runtime/` | Runtime visualization requirements |
| **19-agents-skills/** | `bardo-backup/prd/19-agents-skills/` | Agent display, skill visualization, output streaming specs |
| **22-oneirography/** | `bardo-backup/prd/22-oneirography/` | Visual language, symbol system, atmospheric design |
| **24-sonification/** | `bardo-backup/prd/24-sonification/` | Audio-visual correspondence (informs effect design) |

### Additional Reference Material

| Source | Path | What |
|---|---|---|
| Mori TUI source | `bardo/apps/mori/src/tui/` | 24K LOC reference implementation |
| Mori agent output | `bardo/apps/mori/src/tui/agent_output.rs` | 1,679 LOC, the gold standard for report 26 |
| Mori command output | `bardo/apps/mori/src/tui/command_output.rs` | 218 LOC gate output widget |
| Phase 1 v1 audit | `tmp/tui-parity/archive/ux-audit/SUMMARY.md` | 19-agent architectural audit |
| Phase 1 v2 audit | `tmp/tui-parity/archive/ux-audit/v2/00-MASTER-SUMMARY.md` | 15-agent issue-level audit |
| Phase 1 parity checklist | `tmp/tui-parity/archive/ux-mori-parity-audit/00-AUDIT-SUMMARY.md` | 53-item parity checklist |
| Phase 1 post-merge audit | `tmp/tui-parity/audits/post-merge-live-audit-2026-08-31.md` | Live verification of PR #74 claims |
| Phase 1 visual density | `tmp/tui-parity/audits/visual-density-effects.md` | Effects/layout redesign audit |
| Graph TUI integration | `tmp/engine-audit/19-graph-tui-integration.md` | Graph engine TUI adapter requirements |
| GAPS.md | `.roko/GAPS.md` | Canonical gap tracker (TUI section) |

---

## Report Ownership

Reports are created by parallel audit agents. Each report should contain:

1. **Scope statement** — what exactly is being audited
2. **Current state** — what exists today, with file:line evidence
3. **Issue list** — numbered, with severity (P0-P4), file paths, and reproduction steps
4. **Mori comparison** — where applicable, what mori does differently
5. **Recommendations** — specific changes with effort estimates
6. **Dependencies** — which other reports this one connects to

---

## Methodology

Phase 2 audits the TUI by reading the actual source code in `crates/roko-cli/src/tui/`,
cross-referencing against mori's TUI in `bardo/apps/mori/src/tui/`, and evaluating against
the bardo PRD visual specifications. Each report examines a specific slice (performance,
view, capability) and produces actionable issues with file:line evidence.

The priority tiers (P0-P4) are assigned at the report level but individual issues within
each report may carry different severities. The master implementation plan (to be generated
after all reports complete) will sequence work by individual issue severity, not report-level
priority.
