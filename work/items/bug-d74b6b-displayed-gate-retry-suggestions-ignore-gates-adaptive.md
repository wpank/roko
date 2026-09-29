+++
id = "bug-d74b6b"
kind = "bug"
title = "Displayed gate retry suggestions ignore [gates] adaptive_min_retries/adaptive_max_retries, so they differ from plan-run budgets"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/tui", "roko-serve/learning", "roko-cli/commands"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:feat/learning-verify-loops 99adacd6d"
anchors = ["crates/roko-cli/src/tui/dashboard.rs::gate_threshold_rows", "crates/roko-cli/src/tui/dashboard_model.rs::render_optimizer_page", "crates/roko-cli/src/commands/util.rs::cmd_status", "crates/roko-serve/src/routes/learning/mod.rs::build_adaptive_thresholds_response", "crates/roko-cli/src/graph_task_dispatch/retry_budget.rs::TaskRetryBudgets::load", "crates/roko-gate/src/adaptive_threshold.rs::AdaptiveThresholds::apply_gates_config"]
links = { depends_on = [], blocks = [], related = ["find-4b4344", "gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn displayed_retry_suggestions_follow_gates_config' crates/roko-cli/src && cargo test -p roko-cli --lib displayed_retry_suggestions_follow_gates_config && grep -rqw 'fn adaptive_thresholds_route_applies_gates_config' crates/roko-serve/src && cargo test -p roko-serve --lib adaptive_thresholds_route_applies_gates_config"
+++

## Problem

Since `99adacd6d`, plan runs set the retry budget of a task without an authored `max_retries` from the adaptive
gate thresholds. That budget is bounded by `[gates] adaptive_min_retries..=adaptive_max_retries`, which default to
3..=5 since `41c7ffbd6`. Every place that *shows* a retry suggestion loads `gate-thresholds.json` as
`roko_gate::AdaptiveThresholds` and never calls `apply_gates_config`. It therefore uses the crate's built-in bounds
`MIN_RETRIES = 1` and `MAX_RETRIES = 5` (`adaptive_threshold.rs:21-24`), which are the serde defaults of the
skipped fields (`:233-245`, `:259-266`).

The displays disagree with what the next plan run does:
- A rung with EMA 0.95 and at least 5 observations shows 1 retry. A plan run gives such a task 3.
- A cold rung (under 5 observations) shows the midpoint 3. A plan run gives 4.

`roko learn gates` (`inspect_gates`) does not show retries: it shows only the EMA and the observation count.
The retry numbers appear in the places listed under Where.

## Why it matters

Goal `learning`. Retry budgets are now a real decision driven by learned state. An operator who reads the dashboard
or `/api/learn/adaptive-thresholds` sees budgets that plan runs do not use, and `[gates]` edits appear to have no
effect.

Related items:
- `find-4b4344`: P3-15 retry alignment;
- `gap-7a3527`: the adaptive `[gates]` keys and `from_gates_config`.

## Where

- `crates/roko-cli/src/commands/util.rs:1055-1075` (`roko status`, bin only): `AdaptiveThresholds::load_or_new`,
  then `suggested_max_retries` and `should_skip_rung`.
- `crates/roko-cli/src/tui/dashboard.rs`:
  - `gate_threshold_rows` (:3148-3165) feeds the gate-results page;
  - the thresholds are loaded with `load_json_opt::<AdaptiveThresholds>` at :586, :754 and :819.
- `crates/roko-cli/src/tui/state/snapshot.rs:946-955`: connected mode parses the pushed JSON the same way.
- `crates/roko-cli/src/tui/dashboard_model.rs::render_optimizer_page` (:945-968): the "retries" column.
- `crates/roko-serve/src/routes/learning/mod.rs`:
  - `adaptive_thresholds` (:131-138, `GET /api/learn/adaptive-thresholds` and `/api/learning/adaptive-thresholds`)
    uses `load_or_new`;
  - `build_adaptive_thresholds_response` (:581-598) reports `suggested_max_retries`.
- Reference behaviour: `crates/roko-cli/src/graph_task_dispatch/retry_budget.rs::TaskRetryBudgets::load`
  (:60-77) loads the same file and calls `apply_gates_config(gates)`. That is the only production caller of
  `apply_gates_config`.

## Current state

Checked at `33e107da1`: `apply_gates_config` and `from_gates_config` are called only by `retry_budget.rs:76` and by
roko-gate's own tests. The TUI already loads the workspace config for its budget view (`dashboard.rs:481`).
roko-serve has `AppState::roko_config`.

## Plan

1. Add one helper that loads the thresholds for display with `[gates]` applied. For example, in roko-gate:
   `AdaptiveThresholds::load_with_gates(path, &GatesConfig)`, or `load_or_new` followed by `apply_gates_config`.
   Use it in all the places listed under Where. In connected mode, apply the config after parsing the pushed JSON.
2. Optionally label the column as the budget for tasks without an authored `max_retries`, since authored budgets
   win.
3. Tests:
   - `displayed_retry_suggestions_follow_gates_config` (roko-cli lib): with `adaptive_min_retries = 3`, a
     high-pass rung row shows 3, not 1;
   - `adaptive_thresholds_route_applies_gates_config` (roko-serve lib): the same for the route response.

## Done when

- The TUI gate-results and optimizer pages, `roko status` and `/api/learn/adaptive-thresholds` show the budget
  that `TaskRetryBudgets` would give an unauthored task on that rung.
- The `[[verify]]` command passes.
- Manual check: `roko status` with `[gates] adaptive_min_retries = 4` prints no `retries=` value below 4.

## Notes

- Also out of step: `should_skip_rung` in these displays uses the default `skip_streak_threshold` rather than
  `[gates]`. And because the Graph writer (`runner::persist::GateThresholds`) never records
  `consecutive_passes`, the skip advisory on a Graph-written file always reads "no". Fix the threshold with the
  same helper. Leave the missing streak to `find-4b4344` (P1-12).
