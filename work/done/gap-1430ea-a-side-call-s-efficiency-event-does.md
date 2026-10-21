+++
id = "gap-1430ea"
kind = "gap"
title = "A side call's efficiency event does not name its try: side_call_usd.failover_refused reaches the stream without its attempt, so a reader cannot count a refused call once (inside its try's live call_cost figure, then as a side call) without guessing, as the run summary's running_try does"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "7fb5eb880"
source = "round-v7 plugin-wow and the lead, on de778b612 (bug-11e54b)"
discovered_from = "agent:plugin-wow (round v7)"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent", "crates/roko-cli/src/graph_task_dispatch/helper_calls.rs::write_side_call_rows", "crates/roko-serve/src/routes/runs.rs::summarize_events", "crates/roko-cli/src/runner/tui_bridge.rs::efficiency_event"]
links = { depends_on = [], blocks = [], related = ["bug-11e54b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_side_calls_cost_reaches_the_hub' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_side_calls_cost_reaches_the_hub"

[[verify]]
command = "grep -rqw 'fn a_side_call_event_names_its_try_and_the_summary_keeps_its_cost' crates/roko-serve/src && cargo test -p roko-serve --lib -- a_side_call_event_names_its_try_and_the_summary_keeps_its_cost"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T03:53:43Z"
commit = "7fb5eb880"
by = "roko-speed"
executor = "unknown"
forced = false
evidence = "7fb5eb880. DashboardEvent::EfficiencyEvent and serve's ServerEvent::EfficiencyEvent carry attempt: Option<u32> beside plan_id and task_id (serde default, absent from the JSON when None, so older events read and write as before; every other literal sets None); TuiBridge::try_efficiency_event publishes it, and SideCallRows::write sets it from the call's attempt key (AttemptKey::parse) for failover_refused, helper and audit calls alike. The run summary holds a refused call against the try its event names, and falls back to the task's latest call_cost try only for events without one; cost_usd and side_call_usd are unchanged. The literal verify passes: a_side_calls_cost_reaches_the_hub (a helper call after a failed gate reaches the hub as side_call_usd.helper with attempt 1, and the hub's spend equals the plan ledger's $0.02014) and a_side_call_event_names_its_try_and_the_summary_keeps_its_cost (T2's first try refused at $0.0068 and settled at $0.01, its second try at $0.02 so far: $0.0368 and failover_refused 0.0068 both with the event in order and read after the second try began, where the old guess would have held it against the second try and read $0.0300). Also an_efficiency_event_names_its_try_when_it_has_one (serde), a_runs_summary_cost_includes_its_failover_refused_calls (events without attempt), newer_dashboard_events_bridge_both_ways. Lib suites: roko-agent 2389, roko-cli 3962, roko-core 2047, roko-learn 1391, roko-runtime 213, roko-serve 1078; fmt and clippy clean."
+++

## Problem

A side call's cost reaches the event stream without the try it belongs to. de778b612 (bug-11e54b) publishes each side
call (a call refused before a failover, a helper call, an audit check) as an `efficiency_event` with metric
`side_call_usd.<role>`, but the event has only `plan_id`, `task_id`, `metric` and `value`
(`TuiBridge::efficiency_event`, `crates/roko-cli/src/runner/tui_bridge.rs:438`). The rows in `.roko/learn` are keyed
by their attempt (`<attempt_key>/refused-N`), the event is not.

A `failover_refused` call is one of its try's own calls: while the try runs, its spend is also inside that try's
`call_cost` `attempt_cost_usd`. To count it once, a reader must know which try it belongs to. roko's own run summary
guesses: the task's latest try seen in `call_cost` (`running_try`, `crates/roko-serve/src/routes/runs.rs:2176`). The
Hermes plugin (plugin-wow) must not guess the same way, so it leaves refused calls out.

## Where

- `crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent::EfficiencyEvent`: no attempt field.
- `crates/roko-cli/src/graph_task_dispatch/helper_calls.rs::SideCallRows::write`: publishes the side call's cost; it
  holds the attempt key.
- `crates/roko-serve/src/routes/runs.rs::summarize_events`: attributes a refused call to the task's latest
  `call_cost` try.

## Plan

- `EfficiencyEvent` gains `attempt: Option<u32>` beside `plan_id` and `task_id`, absent from the JSON when there is
  none, so existing events read as before.
- `SideCallRows::write` sets it from the call's attempt key (`AttemptKey::parse`), for every side-call role.
- The run summary takes a refused call's try from the event, and falls back to `running_try` only for events without
  one. Its `side_call_usd` and `cost_usd` are unchanged.
- serve's `ServerEvent::EfficiencyEvent` carries the field across the bridge.

## Done when

A side call's efficiency event names its try (`attempt`), the run summary uses it, and the summary's `side_call_usd`
is unchanged. Checked by the tests in `[[verify]]`.
