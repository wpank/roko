# End-to-End Workflow Tests

Friction log for every user journey through the demo app.

---

## Workflow 1: Investor Landing → Demo

**Steps:**
1. Navigate to `/`
2. See the NieR title screen
3. Click "START" or "Watch Demo"
4. Navigate to `/demo`
5. Select a scenario
6. Press Play
7. Watch scenario run
8. Press Next to advance steps
9. Press Reset when done

**Friction points:**

| Step | Friction | Severity |
|------|---------|----------|
| 1 | ConnectScreen overlay blocks page for up to 2 seconds. Black flash before 3D scene loads. | High |
| 3 | "START" button navigates to `/demo`. "Watch Demo" also goes to `/demo`. Two buttons, same destination. | Low |
| 6 | If server is offline, Play appears to work (green status on first-check) but then hangs for ~8s while `waitForOpen` times out in `setupWorkspace`. | High |
| 7 | Speed button does nothing. Clicking `2x` or `4x` has no visible effect on timing. | High |
| 8 | Next button only works at `waitForStep()` boundaries. During long commands, button is unresponsive. | High |
| 8 | In `race`, `providers`, `explore` scenarios, Next button never activates at all. | Blocker |
| 9 | After Reset, old PTY output continues streaming briefly into new terminal. | Medium |

**Overall:** The core happy path works but the presenter control surface (speed, pause, step) is largely non-functional.

---

## Workflow 2: Dashboard Tour

**Steps:**
1. Navigate to `/dashboard`
2. View Cost tab
3. Click Fleet tab
4. Click Knowledge tab
5. Click Entries tab
6. Click Routing tab
7. Click Chain tab

**Friction points:**

| Step | Friction | Severity |
|------|---------|----------|
| 2 | Cost tab looks good offline — demo data fills in. But Provider Health pane is empty with no message. | Medium |
| 3 | Fleet tab: no loading skeleton, agent grid is blank when empty. Topology graph freezes after 2s. | Medium |
| 4 | Knowledge tab: canvas is mostly empty. Tiny dots barely visible. Placeholder text labels. | High |
| 5 | **Entries tab: text is likely invisible.** Undefined CSS vars (`--text`, `--font-serif`, `--glass-2-border`) cause invisible text, wrong fonts, transparent backgrounds. Uses `useApi` not `useApiWithFallback` so shows error instead of demo data. | Blocker |
| 6 | **Routing tab: same CSS variable issues.** Title in Times New Roman. Text may be invisible. All stats at 0. | Blocker |
| 7 | Chain tab: "Phase 2" status is confusing (development status, not system status). Timer leak on navigation. | Medium |

**Overall:** Cost and Fleet tabs are presentable. Knowledge is sparse. **Entries and Routing are visually broken** due to undefined CSS variables. Chain is functional but confusing.

---

## Workflow 3: Benchmark Run

**Steps:**
1. Navigate to `/bench`
2. Configure a test suite, model, strategy
3. Start a benchmark run
4. Watch live results
5. View completed results
6. Check history
7. Try to view a specific run detail
8. Try to compare runs

**Friction points:**

| Step | Friction | Severity |
|------|---------|----------|
| 3 | Double-clicking Start leaks a polling interval (B8). Old poll overwrites new run data. | High |
| 6 | History table has no "View" link to `/bench/run/:id`. The detail page exists but is unreachable. | High |
| 7 | Must manually type `/bench/run/<id>` in URL bar to see run detail. No UI path to get there. | Blocker |
| 8 | `/bench/compare` exists but has no link from the Bench page. Must type URL manually. | High |
| 3 | Gates configuration hardcoded — doesn't reflect selected suite's actual gates. | Low |

**Overall:** Config and live-run work well. But run detail and comparison are navigational dead ends.

---

## Workflow 4: Bench Showroom

**Steps:**
1. Navigate to `/bench/showroom`
2. Select a scenario
3. Press Play
4. Watch progressive results reveal
5. View cost chart and activity tree

**Friction points:**

| Step | Friction | Severity |
|------|---------|----------|
| 2 | If no demo data exists, no empty-state message. Just blank. | Medium |
| 3 | No speed control — playback always at 800ms per task. | Medium |
| 4 | When not playing, all results are immediately visible — the progressive reveal only works during active playback. | Low |

**Overall:** Works well for its purpose. Missing speed control is the main gap.

---

## Workflow 5: Explorer Deep Dive

**Steps:**
1. Navigate to `/explorer`
2. Check Health tab
3. Switch to Cost tab
4. Switch to Signals tab
5. Switch to Episodes tab
6. Switch to Events tab

**Friction points:**

| Step | Friction | Severity |
|------|---------|----------|
| 2 | Mosaic shows correct stats but the body below is empty — enormous dark void. | High |
| 3–6 | Same pattern on every tab: mosaic at top, empty void below when no data. | High |
| 5 | Episodes silently truncated to 200 — no pagination, no "showing N of M". | Medium |
| 6 | Events silently truncated to 500. | Medium |
| 2 | Provider health names are fabricated from count — mapping to actual providers is wrong. | High |
| All | No auto-polling — data only refreshes on manual click or tab switch. | Medium |
| All | `position: fixed` pseudo-element border on left edge bleeds into other pages. | Low |

**Overall:** Explorer is a data viewer that shows empty views when there's no data. Needs demo data fallback or meaningful empty states.

---

## Workflow 6: Builder Task

**Steps:**
1. Navigate to `/builder`
2. Type a task description
3. Select a model
4. Click Submit
5. Watch terminal output
6. Review detected files

**Friction points:**

| Step | Friction | Severity |
|------|---------|----------|
| 3 | **Model picker is cosmetic** — selected model is never passed to the roko CLI command. | Blocker |
| 2 | Preset buttons overflow horizontally. Some presets may be off-screen with no scroll indicator. | Medium |
| 4 | If `handle.current` is null (first render), setup silently fails. `setupDoneRef` is set to `true` prematurely. | High |
| 5 | Terminal background is bright white against dark theme — jarring contrast. | Medium |
| 6 | File detection regex produces false positives ("created 3 entries" → file "entries" detected). | Low |

**Overall:** Core flow works but the model picker being cosmetic is a significant issue for demos where model selection is the point.

---

## Workflow 7: Terminal Session

**Steps:**
1. Navigate to `/terminal`
2. Click + to add terminal
3. Type commands
4. Add more terminals
5. Click Clear All

**Friction points:**

| Step | Friction | Severity |
|------|---------|----------|
| 1 | Empty state says nothing useful — just dark void. | Medium |
| 5 | No per-terminal close button. Can only clear all. | Medium |
| 5 | `clearAll` relies on unmount for WS cleanup — not explicitly closed. | Low |
| 3 | Terminal color bar at top is `tput colors` output, not decorative — confusing. | Low |

**Overall:** Simple and functional. Missing per-terminal management.

---

## Workflow 8: Jobs Page

**Steps:**
1. Click "Jobs" in top nav
2. View job market

**Friction points:**

| Step | Friction | Severity |
|------|---------|----------|
| 2 | **Page is completely empty — black void.** No route defined, no component rendered. Dead nav link. | Blocker |

**Overall:** Non-functional. Remove from nav or add placeholder.

---

## Workflow 9: Share Receipt

**Steps:**
1. Navigate to `/share/:token`
2. View shared run receipt

**Friction points:**

| Step | Friction | Severity |
|------|---------|----------|
| 1 | Returns `null` while loading — invisible flash, no loading indicator. | Medium |
| 1 | Duration fallback hardcoded to `'4s'` when timestamps are missing — misleading. | Low |
| 1 | Dashboard-level `/dashboard/share/:token` route is unreachable from UI — no links point to it. | Medium |

**Overall:** Functional but rough loading experience.

---

## Summary

| Workflow | Verdict | Key Blockers |
|----------|---------|-------------|
| Landing → Demo | Partially works | Speed, step mode non-functional |
| Dashboard Tour | Mostly works | Entries + Routing visually broken (CSS vars) |
| Benchmark Run | Mostly works | Run detail unreachable from UI |
| Bench Showroom | Works | Missing speed control |
| Explorer | Barely works | Empty voids, no demo data |
| Builder | Partially works | Model picker cosmetic |
| Terminal | Works | Minor UX gaps |
| Jobs | **Broken** | Page doesn't exist |
| Share | Partially works | No loading state |

**Workflows ready for VC demo today:** Dashboard Cost tab, Bench config, Terminal.
**Workflows that need work:** Everything else.
