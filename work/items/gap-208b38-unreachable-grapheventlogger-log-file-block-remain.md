+++
id = "gap-208b38"
kind = "gap"
title = "Unreachable GraphEventLogger --log-file block remains in the Graph plan runner"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan", "crates/roko-cli/src/graph_execution/event_log.rs::run_recorded"]
links = { depends_on = [], blocks = [], related = ["bug-230de6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -q "GraphEventLogger::open" crates/roko-cli/src/graph_execution/plan_runner.rs'
+++

`run_graph_plan` hands every run with `--log-file` to `event_log::run_recorded` (plan_runner.rs:702-705), which writes the JSONL event log and runs the plan with no `log_file` (event_log.rs:126-133). The older block that opens `runner::structured_log::GraphEventLogger` for the same flag (plan_runner.rs:1268) can therefore never see a log file.

Fix: delete the block, and `GraphEventLogger` if nothing else uses it, so there is one `--log-file` writer.

## Notes

2026-10-01 (wk-planrun): implemented on work/gap-dd4826; cargo verification deferred to the batch check.
Deleted the unreachable `--log-file` wiring: the `GraphEventLogger` block in `run_graph_plan_body` (its `log_file` is now bound as `_` with a comment saying why it is always `None` there), the `PlanRunContext.graph_event_logger` field and its initializer, and the `with_event_sink` attach in `run_one_plan`. `runner/structured_log.rs` loses `GraphEventLogger` and `FanOutGraphEventSink` (no other users; the fan-out's only use was its own test, which paired two `GraphEventLogger`s) and their two tests; `StructuredLogger` stays. `event_log::run_recorded` is now the only `--log-file` writer; its tests (`log_brackets_hub_events_with_one_start_and_one_terminal` and the rest of `event_log.rs`) cover it. This is also item 2 of gap-8921a3 (wk-scheduler was told); `GraphEngine::with_event_sink` itself and `build_fix_prompt` stay with that item.
2026-10-01 (wk-planrun): wk-scheduler's gap-8921a3 commit `1245fd3cd` (work/bug-28b604) made the same deletion independently. Both are in gate 6b (`work/rust-batch-6`), where the coordinator resolved the overlap. The batch keeps one `log_file: _` and has no `GraphEventLogger`, `FanOutGraphEventSink` or `graph_event_logger` left under `crates/roko-cli/src`, so either commit is evidence for this item.
