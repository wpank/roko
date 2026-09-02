# 23 -- Empty States, Loading States, and Error States Audit

Audited: `crates/roko-cli/src/tui/` (all views and widgets)
Date: 2026-09-01

---

## Summary

The TUI has **33 identified empty/loading/error state render paths** across 10 views
and 8 widgets. Quality is mixed: views added later (Marketplace, Atelier, Error Digest)
have notably better empty states with contextual guidance and CLI hints, while older
widgets and dashboard sub-panels default to terse one-liners. Loading states are the
weakest area -- there is exactly one animated spinner in the entire TUI
(`task_progress.rs`), and no skeleton screens or shimmer effects exist anywhere. Error
states are generally well-colored (red/danger) but rarely offer recovery guidance.

**Totals:**
- Empty states found: **25** (17 informative, 8 generic/terse)
- Loading states found: **3** (1 animated, 2 static)
- Error display states found: **5** (0 with recovery guidance in-situ)
- Completely blank paths: **4** (silent `return` on small terminals)

---

## 1. Empty State Inventory

### 1.1 Task Progress Widget
- **File:** `widgets/task_progress.rs:214-218`
- **Condition:** `tasks.is_empty()`
- **Current text:** `" {spinner} waiting for tasks..."`
- **Informative?** Yes -- animated spinner character from `atm.spinner()`, phrased as an ongoing activity.
- **Animation:** Yes -- the `Atmosphere` spinner cycles through Unicode frames.
- **Proposal:** Good as-is. This is the gold standard for TUI empty states. Could add a secondary line: "Tasks appear when a plan starts executing."

### 1.2 Cost By Model Widget
- **File:** `widgets/cost_by_model.rs:159-161`
- **Condition:** `models.is_empty()` (after both efficiency events and agent fallback)
- **Current text:** `"  no efficiency data"`
- **Informative?** Partially -- says what's missing but not why or what to do.
- **Animation:** None (static `Paragraph`).
- **Proposal:** Change to: "No efficiency data yet. Cost breakdowns appear after agents complete task turns."

### 1.3 Diff Panel Widget
- **File:** `widgets/diff_panel.rs:33-38`
- **Condition:** `diff_text.is_empty()`
- **Current text:** `"no diff"`
- **Informative?** No -- minimal, no context.
- **Animation:** None.
- **Proposal:** Change to: "No diff available. Diffs appear when agents modify files during task execution."

### 1.4 Parallel Pool Widget
- **File:** `widgets/parallel_pool.rs:36-41`
- **Condition:** `agents.is_empty()`
- **Current text:** `"no parallel agents"`
- **Informative?** Minimal -- tells you the state but not what would populate it.
- **Animation:** None.
- **Proposal:** Change to: "No parallel agents running. Agents spawn when plans execute tasks."

### 1.5 Wave Progress Widget
- **File:** `widgets/wave_progress.rs:22-24`
- **Condition:** `state.execution_waves.is_empty()`
- **Current text:** Nothing -- silent `return`, renders zero pixels.
- **Informative?** No -- completely blank area with no feedback.
- **Animation:** None.
- **Proposal:** Render a dim placeholder line: "No execution waves" or a single dim horizontal rule.

### 1.6 Marketplace View -- Empty State
- **File:** `views/marketplace_view.rs:88-117`
- **Condition:** `jobs.is_empty()` and not in CreateJob sub-view
- **Current text:** Multi-line centered:
  ```
  No jobs posted.

  Jobs appear when agents or operators post work items to .roko/jobs/.
  Press 'n' to create a new job manually.
  ```
- **Informative?** Excellent -- explains both why it's empty and what action to take.
- **Animation:** None.
- **Proposal:** This is the model empty state. Add a dim `roko job create` CLI hint below the keyboard shortcut.

### 1.7 Marketplace -- Empty Form Fields
- **File:** `views/marketplace_view.rs:523-526`
- **Condition:** `value.is_empty() && !is_editing`
- **Current text:** `"(empty)"`
- **Informative?** Adequate for a form field placeholder.
- **Animation:** None.
- **Proposal:** OK as-is. Could use field-specific placeholders ("Enter job title...", etc.).

### 1.8 Marketplace -- Job Description
- **File:** `views/marketplace_view.rs:360-361`
- **Condition:** `job.description.is_empty()`
- **Current text:** `"No description provided."`
- **Informative?** Yes.
- **Proposal:** OK as-is.

### 1.9 Marketplace -- Job Priority
- **File:** `views/marketplace_view.rs:299-301`
- **Condition:** `job.priority.is_empty()`
- **Current text:** em dash character (`\u{2014}`)
- **Informative?** Minimal but acceptable for a metadata field.
- **Proposal:** OK as-is.

### 1.10 Marketplace -- Tags, Posted By, Created At
- **File:** `views/marketplace_view.rs:309-343`
- **Condition:** Various `.is_empty()` checks
- **Current text:** `"\u{2014}"` (em dash) for posted_by/created_at, `"(unassigned)"` for assigned_to, `"(none)"` for tags
- **Informative?** Adequate.
- **Proposal:** OK as-is.

### 1.11 Atelier View -- Empty PRDs
- **File:** `views/atelier_view.rs:201-230`
- **Condition:** `prds.is_empty()`
- **Current text:** Multi-line centered:
  ```
  No PRDs found.

  Create one with: roko prd idea "your idea"
  Then draft: roko prd draft new "your-slug"
  ```
- **Informative?** Excellent -- shows exact CLI commands to get started.
- **Animation:** None.
- **Proposal:** Good as-is. Could add a third line: "Use F8:market to browse available work."

### 1.12 Atelier View -- Empty Tasks for PRD
- **File:** `views/atelier_view.rs:478-485`
- **Condition:** `tasks.is_empty()`
- **Current text:** `"no tasks -- run 'roko prd plan <slug>' to generate"`
- **Informative?** Yes -- actionable CLI guidance.
- **Animation:** None.
- **Proposal:** Good as-is.

### 1.13 Logs View -- Empty Entries
- **File:** `views/logs_view.rs:176-182`
- **Condition:** `filtered_entries.is_empty()`
- **Current text:** `"no log entries -- run agents to generate signals and episodes"`
- **Informative?** Yes.
- **Animation:** None.
- **Proposal:** Good. Could add: "Check filter levels -- current filter may be hiding entries."

### 1.14 Git View -- No Branch Data
- **File:** `views/git_view.rs:161-166`
- **Condition:** `git_data.branches.is_empty()`
- **Current text:** `"no branch data"`
- **Informative?** Minimal.
- **Animation:** None.
- **Proposal:** Change to: "No branch data available. Ensure this is a git repository." Or: "Waiting for git data..." with a spinner if the background refresh hasn't completed.

### 1.15 Git View -- No Worktrees
- **File:** `views/git_view.rs:228-233`
- **Condition:** `git_data.worktrees.is_empty()`
- **Current text:** `"no worktrees"`
- **Informative?** Minimal.
- **Animation:** None.
- **Proposal:** Change to: "No additional worktrees. Create one with: git worktree add <path>"

### 1.16 Git View -- Not a Git Repository
- **File:** `views/git_view.rs:288-293` and `434-439`
- **Condition:** `git_data.is_not_a_git_repository()`
- **Current text:** `"not a git repository"`
- **Informative?** Adequate -- clearly states the problem.
- **Animation:** None.
- **Proposal:** Add: "Run 'roko init' in a git repository to use this view."

### 1.17 Git View -- Clean Working Tree
- **File:** `views/git_view.rs:296-301`
- **Condition:** `git_data.status_lines.is_empty()` and is a git repo
- **Current text:** `"clean working tree"` (styled with `theme.success()`)
- **Informative?** Excellent -- positive confirmation with green styling.
- **Animation:** None.
- **Proposal:** Perfect as-is.

### 1.18 Git View -- No Commit History
- **File:** `views/git_view.rs:381-386`
- **Condition:** `git_data.commits.is_empty()`
- **Current text:** `"no commit history"`
- **Informative?** Minimal.
- **Animation:** None.
- **Proposal:** Change to: "No commit history. Make your first commit to see the graph."

### 1.19 Learning View -- No Router Data
- **File:** `views/learning_view.rs:57-66`
- **Condition:** `router.model_slugs.is_empty()`
- **Current text:** `"No cascade router data. Run tasks to populate."`
- **Informative?** Yes -- explains what to do.
- **Animation:** None.
- **Proposal:** Good as-is.

### 1.20 Learning View -- No Observations
- **File:** `views/learning_view.rs:303-305`
- **Condition:** `total_trials == 0`
- **Current text:** `"  No observations yet. Run tasks to see transitions."`
- **Informative?** Yes.
- **Animation:** None.
- **Proposal:** Good as-is.

### 1.21 Learning View -- No Efficiency Events
- **File:** `views/learning_view.rs:419-429`
- **Condition:** `events.is_empty()`
- **Current text:** `"No efficiency events recorded yet."`
- **Informative?** Yes.
- **Animation:** None.
- **Proposal:** Good as-is.

### 1.22 Config View -- No Provider Data
- **File:** `views/config_view.rs:531-536`
- **Condition:** `providers.is_empty()`
- **Current text:** `"no provider data \u{2014} run agents to populate"`
- **Informative?** Yes -- em dash separator, actionable.
- **Animation:** None.
- **Proposal:** Good as-is.

### 1.23 Config View -- No Model Data
- **File:** `views/config_view.rs:587-592`
- **Condition:** `cascade_router.model_slugs.is_empty()`
- **Current text:** `"no model data \u{2014} run agents to populate cascade router"`
- **Informative?** Yes.
- **Animation:** None.
- **Proposal:** Good as-is.

### 1.24 Affect View -- No Affect Data
- **File:** `views/affect_view.rs:33-37`
- **Condition:** `tui_state.affect.is_none()`
- **Current text:** `"No affect data yet.\nWaiting for first task turn..."`
- **Informative?** Yes -- two-line message, explains the temporal condition.
- **Animation:** None (static text, but the "Waiting for..." phrasing implies patience).
- **Proposal:** Could add the spinner character from `Atmosphere` to make the waiting feel active.

### 1.25 Affect View -- No Somatic Markers / No Biases
- **File:** `views/affect_view.rs:110-113` and `142-145`
- **Condition:** `affect.recent_markers.is_empty()` / `affect.active_biases.is_empty()`
- **Current text:** `"(no markers)"` / `"(none)"`
- **Informative?** Minimal but adequate for sub-sections.
- **Animation:** None.
- **Proposal:** OK as-is.

---

## 2. Loading State Inventory

### 2.1 Task Progress Spinner (ONLY animated loading state)
- **File:** `widgets/task_progress.rs:216`
- **Mechanism:** `atm.spinner()` returns cycling Unicode characters from the `Atmosphere` animation system.
- **Verdict:** This is the only genuinely animated loading indicator in the entire TUI. It cycles through a set of spinner frames per-tick. Effective.

### 2.2 Header Bar -- Active Agent Spinner
- **File:** `widgets/header_bar.rs:469-474`
- **Mechanism:** `state.atmosphere.spinner()` prefixed to active agent role and model display.
- **Verdict:** This shows an active spinner when any agent is running, but it's a status indicator, not a loading state. Functionally serves as a "something is happening" signal.

### 2.3 Header Bar -- Pulsing Health Dot
- **File:** `widgets/header_bar.rs:180-195`
- **Mechanism:** Heartbeat animation cycling between filled/open circle with brightness modulation.
- **Verdict:** Provides persistent "alive" feedback. Not a loading indicator per se, but it does communicate system liveness during long waits.

### 2.4 Missing: Git View Initial Load
- **File:** `views/git_view.rs:91`
- **Condition:** `tui_state.git_view_data` is `None` on first render (before background thread delivers data)
- **Current behavior:** Falls back to `GitViewData::default()` which is all-empty, showing "no branch data", "no worktrees", "no commit history" simultaneously.
- **Proposal:** Should show a dedicated "Loading git data..." message with a spinner instead of three separate "no data" messages.

### 2.5 Missing: Dashboard Right Panel Initial Load
- **Condition:** No active plans and no data
- **Current behavior:** Renders the sub-tabbed right panel directly. When there's no data at all, each sub-panel renders its own empty state independently.
- **Proposal:** When `tui_state` has zero plans, zero agents, and zero efficiency events, show a single "Getting started" panel instead of empty sub-tab panels.

---

## 3. Error State Inventory

### 3.1 Error Digest -- Compact Panel
- **File:** `widgets/error_digest.rs:103-132`
- **Shows:** Gate pass/fail ratio + recent error list.
- **Empty variant:** `"No gates evaluated"` for no gates, `"No errors"` for no errors.
- **Recovery guidance:** None in-situ. The gate failures show plan/task IDs but no "what to do next."
- **Proposal:** Add: "Run 'roko plan run --resume-plan' to retry failed tasks."

### 3.2 Error Digest -- Aggregation Panel (F5:Errors tab)
- **File:** `widgets/error_digest.rs:143-220`
- **Shows:** Categorized errors from all sources with timestamps, icons, and category counts.
- **Empty variant:** Centered multi-line:
  ```
  no errors recorded

  errors from gates, agents, and runtime
  will appear here when they occur
  ```
- **Recovery guidance:** None -- errors are displayed but no recovery actions are suggested.
- **Proposal:** Best-in-class empty state for errors. For populated states, add per-category recovery hints: "Gate failures: check compile/test output in F1:Verify" or "Agent failures: check agent logs in F3."

### 3.3 Header Bar -- Error Count Display
- **File:** `widgets/header_bar.rs:260-270`
- **Shows:** `" ERR:{done}/{total}"` when any plan has failures.
- **Recovery guidance:** None in the header itself; relies on user navigating to details.
- **Proposal:** OK as-is -- the header is space-constrained.

### 3.4 Status Bar -- Error Summary
- **File:** `widgets/status_bar.rs:78-83,110-121`
- **Shows:** `"ERR:{count}"` and flailing/failure icons.
- **Recovery guidance:** Keybinding hints include `R:retry` and `D:diag` when failures are detected.
- **Proposal:** Good -- this is the only place that offers contextual error recovery hints.

### 3.5 Warning Bar
- **File:** `widgets/header_bar.rs:584-609`
- **Shows:** Active warnings with `[n] dismiss` hint.
- **Recovery guidance:** Dismissal only, no fix guidance.
- **Proposal:** Could link warnings to relevant tabs: "Low disk: see F6:cfg > disk"

---

## 4. Completely Blank Render Paths

These are cases where the TUI renders absolutely nothing in a region, providing zero feedback.

### 4.1 Wave Progress -- Empty Waves
- **File:** `widgets/wave_progress.rs:22-24`
- **Condition:** `execution_waves.is_empty()`
- **Behavior:** `return` -- no pixels rendered in the allocated area.
- **Impact:** The wave progress ribbon area appears as raw background color with no explanation.

### 4.2 Wave Progress -- Zero Total Plans
- **File:** `widgets/wave_progress.rs:27-29`
- **Condition:** `total_plans == 0`
- **Behavior:** `return` -- blank area.

### 4.3 System Metrics -- Small Terminal
- **File:** `widgets/sys_metrics.rs:126-128`
- **Condition:** `inner.width < 12 || inner.height < 2`
- **Behavior:** `return` after rendering the block border -- empty bordered box.
- **Impact:** Minor -- this only happens at very small terminal sizes.

### 4.4 Multiple Widgets -- Area Too Small
- **Files:** Various (marketplace_view:153-155, 457-464; config_view:60-62)
- **Condition:** `inner.height < N || inner.width < M`
- **Behavior:** Silent return or "Terminal too small for form."
- **Impact:** The marketplace form does show a message; others are completely blank.

---

## 5. First-Run Experience Analysis

When the TUI starts with no `.roko/` data, no running plans, and no prior history:

### What the user sees:

| Tab | First-Run Content |
|-----|-------------------|
| **F1 Dashboard** | Right panel only (no left panel since no active plans). Sub-tabs show individual empty states per panel type. |
| **F2 Plans** | Empty wave list on left, blank detail on right. No guidance text. |
| **F3 Agents** | Empty agent roster. No guidance text about how to spawn agents. |
| **F4 Git** | Falls back to "no branch data" / "no worktrees" / "no commit history" if git data hasn't loaded; "clean working tree" if it has. |
| **F5 Logs** | "no log entries -- run agents to generate signals and episodes" -- informative. |
| **F6 Config** | Shows config fields from `roko.toml` (always present after `roko init`). Runtime sections show zeroes. |
| **F7 Inspect** | All four sections show "no data" variants. Health summary shows zeroes. |
| **F8 Marketplace** | Excellent empty state with CLI hints. |
| **F9 Atelier** | Excellent empty state with CLI workflow commands. |
| **F10 Learning** | "No cascade router data. Run tasks to populate." -- informative. |

### Verdict:
The first-run experience is **adequate but inconsistent**. F8 (Marketplace) and F9
(Atelier) are exemplary. F2 (Plans) and F3 (Agents) are the weakest -- they show
structural elements (borders, headers) but no guidance on how to populate them.

---

## 6. Transition Analysis: Empty to Populated

### 6.1 Smooth transitions
- **Task progress:** Animated spinner transitions to task list naturally as tasks appear.
- **Logs view:** Auto-tail mode scrolls in new entries smoothly.
- **Header bar:** Progress bar grows from zero filled to full as tasks complete.
- **Marketplace:** Job list appears when files are written to `.roko/jobs/`.

### 6.2 Potentially jarring transitions
- **Dashboard left panel:** The entire left panel (38% of screen) appears/disappears based on `has_active_plans`. When the first plan activates, the right panel suddenly shrinks from 100% to 62% width. When the last plan completes, it snaps back to full width.
- **Git view:** If background thread delivers data mid-view, three "no data" panels simultaneously populate. No progressive reveal.
- **Plans view (F2):** No transition animation between empty wave list and populated one.

---

## 7. Specific Recommendations

### 7.1 Immediate Fixes (low effort, high impact)

| # | File | Change |
|---|------|--------|
| 1 | `wave_progress.rs:22-24` | Render a dim `"No execution waves"` line instead of silent return |
| 2 | `diff_panel.rs:34` | Change `"no diff"` to `"No diff available -- diffs appear when agents modify files"` |
| 3 | `parallel_pool.rs:37` | Change `"no parallel agents"` to `"No parallel agents running -- agents spawn when plans execute"` |
| 4 | `git_view.rs:162` | Change `"no branch data"` to `"Loading branch data..."` when `git_view_data.is_none()`, keep current for non-git repos |
| 5 | `git_view.rs:229` | Change `"no worktrees"` to `"No additional worktrees"` |
| 6 | `git_view.rs:382` | Change `"no commit history"` to `"No commits found"` |
| 7 | `cost_by_model.rs:160` | Change `"  no efficiency data"` to `"  No cost data yet -- breakdowns appear after agent turns"` |

### 7.2 Medium Effort Improvements

| # | Change | Benefit |
|---|--------|---------|
| 8 | Add first-run guidance to F2 (Plans) empty state: "No plans loaded. Run 'roko plan run plans/ --engine runner-v2' to start." | Eliminates blank-screen confusion for new users |
| 9 | Add first-run guidance to F3 (Agents) empty state: "No agents online. Agents appear when plans execute or when started with 'roko agent start'." | Same |
| 10 | Add `Atmosphere::spinner()` to `affect_view.rs:34` waiting text | Visual consistency with task_progress.rs |
| 11 | Add recovery hints to error digest entries based on category (gate -> "check F1:Verify", agent -> "check F3:Agents") | Helps users self-diagnose |
| 12 | Animate the dashboard left-panel show/hide with a 2-frame transition instead of instant snap | Reduces layout jarring |

### 7.3 Skeleton Screens and Progressive Reveal (high effort)

| # | Proposal | Where |
|---|----------|-------|
| 13 | **Skeleton screen for git view:** Render dim placeholder lines (e.g., dim rectangles or dots) for branches/commits while the background thread is loading. Replace with real data on delivery. | `git_view.rs` |
| 14 | **Shimmer loading effect:** Extend `Atmosphere` with a `shimmer(x, width)` method that returns a per-cell brightness modifier. Use it on skeleton placeholder lines to create a left-to-right sweep effect. | `atmosphere.rs` + all empty states |
| 15 | **Progressive reveal for dashboard sub-panels:** When switching sub-tabs, fade in the new panel's content over 2-3 frames instead of instant render. Could be done by interpolating text opacity (dim -> normal). | `dashboard_view.rs` |
| 16 | **First-run onboarding overlay:** When `tui_state` has zero of everything (no plans, agents, signals, episodes), render a centered "Getting Started" panel over the active tab, showing the self-hosting workflow steps from CLAUDE.md. Dismiss on any keypress. | `app.rs` |

---

## 8. Quality Tier Classification

### Tier 1: Exemplary (contextual, actionable, CLI hints)
- Marketplace empty state (`marketplace_view.rs:88-117`)
- Atelier empty PRDs (`atelier_view.rs:201-230`)
- Atelier empty tasks (`atelier_view.rs:478-485`)
- Error digest aggregation panel empty (`error_digest.rs:184-209`)
- Logs view empty (`logs_view.rs:176-182`)
- Config provider health empty (`config_view.rs:531-536`)
- Status bar error hints (`status_bar.rs` with `R:retry` / `D:diag`)

### Tier 2: Adequate (tells you the state, maybe hints at cause)
- Learning view empties (3 instances)
- Config model comparison empty
- Affect view "No affect data yet" / "Waiting for first task turn..."
- Git clean working tree (positive confirmation)
- Task progress spinner (animated, good)

### Tier 3: Terse (minimal, no guidance)
- `"no diff"` (diff_panel.rs)
- `"no parallel agents"` (parallel_pool.rs)
- `"no branch data"` (git_view.rs)
- `"no worktrees"` (git_view.rs)
- `"no commit history"` (git_view.rs)
- `"  no efficiency data"` (cost_by_model.rs)
- `"No errors"` (error_digest.rs compact)

### Tier 4: Silent (no feedback at all)
- Wave progress empty return
- Small terminal blank returns (multiple widgets)

---

## 9. Cross-Cutting Observations

1. **No loading spinners except task_progress.** The `Atmosphere` struct already provides
   `spinner()` and `breathing_brightness()` methods, but only `task_progress.rs` and
   `header_bar.rs` use them. Every "Waiting for..." or "Loading..." message should use
   the spinner.

2. **No skeleton screens anywhere.** The git view is the most obvious candidate since it
   has a background refresh thread with a visible delay.

3. **Inconsistent empty state phrasing.** Some use "no X", some use "No X found.", some
   use "No X data yet." Should standardize on: "No {thing} yet. {What to do}."

4. **Em dash (`\u{2014}`) used for missing scalar values.** Consistent across
   marketplace, learning, and config views. Good convention.

5. **`"(none)"` used for missing collection values.** Seen in affect biases and
   marketplace tags. Good convention.

6. **Form field empty state:** Only marketplace job form uses `"(empty)"` for empty
   fields. Consistent within that view.

7. **Dashboard left panel toggle is the biggest layout jump.** Goes from 0% to 38% of
   screen width instantly. This is the most jarring transition in the entire TUI.

8. **Error states never show stack traces or detailed diagnostics inline.** The error
   digest shows categorized summaries but users must navigate to F1:Verify or F5:Logs
   for full output. This is arguably correct for a dashboard but could benefit from an
   "expand" action on error rows.

9. **The `"Terminal too small for form."` message** in `marketplace_view.rs:459` is the
   only explicit small-terminal message. All other widgets silently return. Consider
   adding a similar message to other widgets that bail on small terminals.

---

## Implementation Status (2026-09-02 swarm)

Empty state messages added across all views (task #9): contextual help text replacing
generic placeholders.
