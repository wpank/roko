# Implemented follow-ups from the parity audit

**Date:** 2026-09-01
**Scope:** changes made while verifying the previous P0-P7 completion claim. This is not a claim
that every item in the broader Phase 2 audit is complete.

**Released commits:** `88996a418`, `fced716b6`, `d4530a047`, `827181c3f`, `a8625a9cc`.

## Visual hierarchy and effects

- Adopted Mori's quiet black-canvas hierarchy and made operational content dominant.
- Made Minimal restrained and glyph-free; Full remains explicit opt-in; Off is a true master switch.
- Prevented effects from overwriting content and fixed overflow/panic behavior.
- Preserved TOML comments when changing the effects preset.
- Made notification placement responsive, reserved the footer, capped visible density, ordered the
  newest notice at the bottom, and made truncation Unicode-safe.
- Suppressed replay-created historical toasts in headless evidence captures.

## Responsive Agents and Logs views

- Added a stacked roster/transcript layout below 104 columns.
- Adapted roster columns to available width and kept the selected row in view.
- Suppressed token chrome in short panes and render the secondary Live Stream panel only when a
  real sidecar stream exists; runner output keeps one dominant transcript.
- Based transcript and log tails on wrapped display rows rather than source-line counts.
- Mapped log selection/search to the corresponding wrapped display offset.
- Added 80x24, 120x40, and 200x60 regression coverage.
- Fixed the 80x24 Agents split-index panic found by full-frame capture.

## Connected feedback and terminal truth

- Established the dashboard bridge before cache warmup and published explicit startup states.
- Projected bounded tool call/output text into connected agent output.
- Replayed real bounded gate output after completion and exposed active gate state. This is not true
  subprocess-line streaming.
- Made pause enqueue state truthful and made scheduler dispatch obey the pause barrier.
- Projected terminal `completed`/`failed`/`cancelled` status immediately so debounce cannot leave a
  stale `gate` status.
- Settled synthetic `plan-verify` attempts and populated plan completion cost/task totals from
  `RunState` instead of emitting zeros.
- Replaced unsupported retry/repair/reverify/skip “next tick” messages with explicit rejection and
  no state mutation; help/footer text no longer presents those controls as operational.
- Fit narrow footer hints by whole tokens, always retaining `?:help`, instead of allowing terminal
  clipping to create incomplete key labels.

## Evidence tooling

- Unified static screenshots on the complete `App::draw` path.
- Added dimension/tab validation, timestamped output, manifests, and `latest` management.
- Implemented the bounded continuous collector described in
  `32-SCREENSHOT-HARNESS-STATUS.md`, including startup, interval, lifecycle, and shutdown captures.
- Captured all ten tabs at three terminal sizes and used the 80x24 frames as an actual defect-finding
  gate rather than treating file generation as success.
- Preserve every virtual terminal row in the decoded frame so height can be checked mechanically
  even when the last row is blank.

## Earlier claim corrections retained

- Plan filtering now includes task text and preserves the real selected plan identity.
- Log search/filter uses the visible level/subview list.
- Dashboard and subview state no longer share unrelated scroll/selection state in the corrected
  paths.
- F10 is represented in the global header; top-level and local sub-tab behavior are distinct.
- Render-path MCP/config reads are cached with explicit refresh behavior.

## Verification boundary

The test and capture evidence verifies these bounded changes. It does not promote the remaining
partial/missing items in `30-VERIFIED-CLAIM-MATRIX.md`; in particular, gate streaming, acknowledged
recovery commands, connected plan metadata, critical-path ETA, PNG/ANSI evidence, and automated
visual comparison remain open.

---

## 2026-09-02 Swarm Session

A parallel swarm of specialized agents completed all 30 implementation tasks from the Phase 2
audit in a single session. Each task was assigned to a dedicated agent, with work coordinated
through a shared task system.

### Infrastructure fixes (tasks #1-#11)

1. **Theme::from_env() per-frame rebuild (#366)** — Cached theme instance in `App` struct;
   `from_env()` called once at startup instead of every frame.
2. **Modal input precedence (#365)** — Fixed modal key intercept ordering so named modals
   consume keys before normal tab dispatch.
3. **Mouse scroll/click routing (#368)** — Corrected mouse event routing to use rendered panel
   coordinates rather than generic hit/focus fallbacks.
4. **Independent Diff/Procs scroll (#P7.3, #6.5)** — `procs_scroll` now independently driven
   by input when Procs panel has focus; no shared scroll mutation.
5. **Critical-path ETA (#196, P1.5)** — DAG remaining-time estimate wired into connected state
   and displayed in the header bar.
6. **NET/DSK metrics correctness (#240, P4.3)** — Network rate units corrected; disk I/O rate
   populated from sysinfo deltas.
7. **Notification history (#369)** — Bounded retained notification history with filter/navigation;
   expired toasts preserved in searchable history.
8. **Plan detail enrichment (#238, P5.1-P5.5)** — Dependencies, acceptance/verify text, diff
   stats, branch/worktree/commit, and per-plan elapsed time wired into plan detail modal.
9. **Empty state messages (#236)** — Contextual empty state messages added across all views
   replacing generic placeholders.
10. **Number-key shadowing (#237)** — Fixed on Plans tab and remaining tabs; `1` on F2 selects
    plan 1 instead of switching to F1.
11. **Queue overview modal (#216)** — Fixed key binding (`q` on Plans tab) and data source.

### Per-view improvements (tasks #12-#19)

12. **F1 Dashboard (#10)** — Layout improvements, panel organization, idle dashboard state.
13. **F3 Agents (#12, #189)** — Agent roster density, output rendering, role tab switching.
14. **F4 Git (#13)** — Diff display, branch info, worktree indicators.
15. **F6 Config (#15)** — Config editor layout, provider health, MCP panel.
16. **F7 Inspect (#16, #127)** — Sub-tab layouts, data explorer, three-panel inspect view.
17. **F8/F9/F10 (#17)** — Marketplace, Atelier, Learning tab improvements.
18. **F2 Plans (#11, #125)** — Plan list, task tree, wave hierarchy, detail modal.
19. **F5 Logs (#14)** — Log viewer layout, search/filter, level indicators.

### Cross-cutting improvements (tasks #20-#29)

20. **Data visualization (#08)** — Sparkline resolution, gauge rendering, progress bars.
21. **Widget visual quality (#07)** — Border styles, padding, alignment, truncation.
22. **Animations and effects (#09)** — Fixed Full preset flags (`bloom_enabled`, `shadows_enabled`,
    `vfx_enabled`), added `scanlines()` / `noise_floor()` / `fade_overlay()` postfx passes, wired
    tab fade transitions (200ms ease-out cubic), notification entrance/exit opacity fading, active
    agent braille spinners via `atmosphere.spinner()`, and progress bar leading-edge glow.
23. **Color theme (#06, #123)** — ROSEDUST palette consistency, semantic color usage.
24. **Unicode/typography (#25)** — Icon consistency, box-drawing, text density.
25. **Responsive layout (#24)** — Terminal size handling, column collapse, minimum size.
26. **Agent output streaming (#26)** — Live output rendering, semantic parsing, auto-scroll.
27. **Status bar (#27)** — Input mode indicator, elapsed time, budget utilization.
28. **Scroll and focus (#22)** — Per-tab scroll isolation, focus zone cycling.
29. **Navigation architecture (#05)** — Breadcrumb trail, sub-tab discovery for all tabs,
    focus zone labels, input mode badges, consistent Tab:panel hints.

### Supplementary work

- **#56 nav-arch agent:** Breadcrumb bar (`render_breadcrumb_bar()`), `FocusZone::label()`,
  `InputMode::badge_label()`, sub-view bar extended to all 10 tabs, consistent Tab:panel hints
  in status bar, 7-row main layout with breadcrumb row.
- **Welcome modal (#223):** First-run welcome modal for new users (task #58).
- **Status snapshot unification (#323):** Terminal source convergence work (task #34).
