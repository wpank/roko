+++
id = "bug-b8af02"
kind = "bug"
title = "Helper calls after a failed gate still reach the cascade router as successes; bug-31438d only tags their cost rows"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-cli/dispatch"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "9c0b9aed0"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-telemetry2's report, branch work/gap-8cb382 at c5090e9a5)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/routing_context.rs", "crates/roko-cli/src/dispatch/factory.rs"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["bug-31438d", "bug-62e3f4", "bug-f68404"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn helper_calls_give_the_cascade_router_no_credit' crates/roko-cli/src/ && cargo test -p roko-cli --lib helper_calls_give_the_cascade_router_no_credit"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 38521c19f. The premise was fixed earlier by bug-07bc75 (57ea5133f); this adds the regression test that helper calls give the cascade router no credit. Batch 15a gate on 9005da604, re-assembled as 8a2ee8bca with only settle's rustfmt commit changing two files' formatting (MAIN 9c0b9aed0 has the same code): cargo check --workspace --tests clean; nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-compose -p roko-core -p roko-learn -p roko-serve --keep-going -D warnings clean; lib tests pass: roko-cli 3190 (two flakes, the turn_policy escalated-timeout test and graph_run_routing_observations_survive_a_crash, pass alone and in their module), roko-agent 2268, roko-core 1952, roko-learn 1204, roko-serve 986, roko-compose 560; cargo test -p roko-cli --test learning_wiring_census: 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

After a failed gate, Graph dispatch makes its helper calls (quality judge, error enrichment, gate reflection) through `CheapFactoryAgent` (`graph_task_dispatch/routing_context.rs:15-66`). The model is `select_cheap_model_key`'s fast model, and the call goes through `SharedAgentFactory::run_shared_agent_bridge` (`dispatch/factory.rs:455`). The bridge hands the dispatcher's cascade router to the call path (`factory.rs:385`), so each helper call that returns counts as a routing success for the fast model. wk-telemetry2's loop-census fixture shows 6 extra router trials from them.

bug-31438d's branch (`work/bug-31438d` at 7107e29eb) records the helper calls' cost and efficiency rows with `role "helper"` (`graph_task_dispatch/helper_calls.rs:21`, :281, :302). It doesn't touch `routing_context.rs` or the bridge's router wiring, so the router credit remains.

## Why it matters

Cybernetic core (epic spec-6ac537): a helper call succeeding says nothing about whether that model can do the task. Crediting it biases routing towards the fast model on exactly the tasks that failed. bug-f68404 is the same kind of error for manual `--model` overrides.

## Where

`CheapFactoryAgent::run`, `run_shared_agent_bridge` and the router hand-off at `factory.rs:385`.

## Plan

1. Mark helper dispatches (for example with `agent_id "cheap-factory-agent"` or an explicit request flag), and skip router observation for them, or record them under a separate helper context that routing doesn't read.
2. Add `helper_calls_give_the_cascade_router_no_credit`: a failed gate followed by its helpers leaves the router's trial counts unchanged.

## Done when

- [ ] Helper calls give the router no credit, and their cost rows stay tagged as helper rows.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-b8af02` at `bd2dab70a`; cargo verification deferred to the batch check. `helper_calls_give_the_cascade_router_no_credit` (targeted `cargo test` passed at the branch head). The premise no longer held at BASE b128de876. bug-07bc75 (`57ea5133f`) had already switched the provider bridge to `ModelCallFeedbackRecorder::without_cascade_router`, so helper calls through `run_shared_agent_bridge` no longer reach the router, which learns only from settled verdicts through `RoutingObservationSink`. The commit adds only the regression test: after a failed gate and its three helper calls, the helper model has no router trials, the attempt's model has one failed trial, the bridge writes no `cascade-router.json`, and the three helper cost rows keep role `helper`.
