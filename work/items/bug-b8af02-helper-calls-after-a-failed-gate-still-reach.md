+++
id = "bug-b8af02"
kind = "bug"
title = "Helper calls after a failed gate still reach the cascade router as successes; bug-31438d only tags their cost rows"
status = "open"
triage = "unverified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-cli/dispatch"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-telemetry2's report, branch work/gap-8cb382 at c5090e9a5)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/routing_context.rs", "crates/roko-cli/src/dispatch/factory.rs"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["bug-31438d", "bug-62e3f4", "bug-f68404"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn helper_calls_give_the_cascade_router_no_credit' crates/roko-cli/src/ && cargo test -p roko-cli --lib helper_calls_give_the_cascade_router_no_credit"
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
