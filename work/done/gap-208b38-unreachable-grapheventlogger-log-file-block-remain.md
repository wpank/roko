+++
id = "gap-208b38"
kind = "gap"
title = "Unreachable GraphEventLogger --log-file block remains in the Graph plan runner"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan", "crates/roko-cli/src/graph_execution/event_log.rs::run_recorded"]
links = { depends_on = [], blocks = [], related = ["bug-230de6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -q "GraphEventLogger::open" crates/roko-cli/src/graph_execution/plan_runner.rs'

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:34Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:57Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`run_graph_plan` hands every run with `--log-file` to `event_log::run_recorded` (plan_runner.rs:702-705), which writes the JSONL event log and runs the plan with no `log_file` (event_log.rs:126-133). The older block that opens `runner::structured_log::GraphEventLogger` for the same flag (plan_runner.rs:1268) can therefore never see a log file.

Fix: delete the block, and `GraphEventLogger` if nothing else uses it, so there is one `--log-file` writer.

## Notes

2026-10-01 (wk-planrun): implemented on work/gap-dd4826; cargo verification deferred to the batch check.
Deleted the unreachable `--log-file` wiring: the `GraphEventLogger` block in `run_graph_plan_body` (its `log_file` is now bound as `_` with a comment saying why it is always `None` there), the `PlanRunContext.graph_event_logger` field and its initializer, and the `with_event_sink` attach in `run_one_plan`. `runner/structured_log.rs` loses `GraphEventLogger` and `FanOutGraphEventSink` (no other users; the fan-out's only use was its own test, which paired two `GraphEventLogger`s) and their two tests; `StructuredLogger` stays. `event_log::run_recorded` is now the only `--log-file` writer; its tests (`log_brackets_hub_events_with_one_start_and_one_terminal` and the rest of `event_log.rs`) cover it. This is also item 2 of gap-8921a3 (wk-scheduler was told); `GraphEngine::with_event_sink` itself and `build_fix_prompt` stay with that item.
2026-10-01 (wk-planrun): wk-scheduler's gap-8921a3 commit `1245fd3cd` (work/bug-28b604) made the same deletion independently. Both are in gate 6b (`work/rust-batch-6`), where the coordinator resolved the overlap. The batch keeps one `log_file: _` and has no `GraphEventLogger`, `FanOutGraphEventSink` or `graph_event_logger` left under `crates/roko-cli/src`, so either commit is evidence for this item.
