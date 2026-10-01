+++
id = "gap-9980c6"
kind = "gap"
title = "`roko run` drops --effort and --no-cascade on the Graph path, and --serve/--share are untested"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/run"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w3d-roko-run"
anchors = ["crates/roko-cli/src/run.rs:437", "crates/roko-cli/src/run.rs::resolve_workflow_model_selection", "crates/roko-cli/src/graph_execution/plan_runner.rs::GraphPlanRunParams", "crates/roko-cli/src/resolved_overrides.rs"]
links = { depends_on = [], blocks = [], related = ["gap-d60281"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -q "effort: run.overrides.effort.clone()" crates/roko-cli/src/run.rs && grep -q "no_cascade: run.overrides.cascade_enabled" crates/roko-cli/src/run.rs && grep -rqE "\"--(serve|share)\"" crates/roko-cli/tests && grep -rqw "fn a_run_effort_reaches_the_provider" crates/roko-cli/src/ && cargo test -p roko-cli --lib a_run_effort_reaches_the_provider && cargo test -p roko-cli --test run_serve_share'

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:43Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:57Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

Since w3d, `roko run` executes its one-task plan with `run_graph_plan`. `GraphPlanRunParams` has no effort or cascade field, and only the model override (`cli_model_override`) is passed, so `--effort` and `--no-cascade` (resolved in `resolved_overrides.rs`) are accepted and then dropped. `roko plan run` has the same problem with `--effort` (gap-d60281). The `--serve` and `--share` flags of `roko run` have not been exercised since the switch to the Graph engine.

Fix: carry effort and cascade settings in `GraphPlanRunParams` (shared with gap-d60281), and add a smoke test for `--serve` and `--share`.

## Notes

2026-10-01 (wk-planrun): implemented on work/gap-dd4826; cargo verification deferred to the batch check.
`GraphPlanRunParams` gains `effort: Option<String>` and `no_cascade: bool`. `run_graph_plan_body` sets `[agent] default_effort` from `effort` before the dispatcher is built (the run manifest's config fingerprint records the effective value), and with `no_cascade` hands `SharedAgentFactory` no cascade router, so the routing ladder (on by default), else the default model, picks each task's model while the run's feedback still trains the router. `run_prompt` (`roko run`, `roko do`'s simple path) passes the global `--effort` and `roko do --no-cascade`; `roko do`'s planned path (`run_plan_execution`, whose `no_cascade` parameter was unused) passes both. `plan run`, `roko serve`, `prd` and the cloud worker pass `None`/`false` (wk-taskdef's gap-d60281 makes `plan run --effort` an error). `roko run` itself has no `--no-cascade` flag; only `roko do` does.
Tests: `a_run_effort_reaches_the_provider` (plan_runner.rs: a run with `effort: Some("low")` calls the provider with `--effort low`, not the config's `medium`) and the new `crates/roko-cli/tests/run_serve_share.rs` (`roko --model scripted --effort low run --share` with the scripted provider: the control plane starts, the run succeeds, `.roko/shared/` gets one transcript, and the provider gets `--effort low`). No test pins `no_cascade`'s routing effect.
The old `[[verify]]` could never pass: its `sed` range starts at `run_graph_plan(GraphPlanRunParams {`, but `run_prompt` calls `run_graph_plan_in_run(` with the literal on the next line, at BASE too. It now checks the two fields in `run.rs`, the `--share` test and the new lib test, and runs both tests.
