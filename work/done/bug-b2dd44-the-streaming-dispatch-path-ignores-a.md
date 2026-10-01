+++
id = "bug-b2dd44"
kind = "bug"
title = "The streaming dispatch path ignores a substituted pinned model, which the batch path fails as model_substituted"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "9c0b9aed0"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on branch work/bug-31438d at ee6a541ef)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/graph_task_dispatch/served_model.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["bug-31438d"], blocks = [], related = ["bug-31438d", "bug-35379d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn streaming_dispatch_fails_a_substituted_pinned_model' crates/roko-cli/src/graph_task_dispatch/ && cargo test -p roko-cli --lib streaming_dispatch_fails_a_substituted_pinned_model"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 38521c19f. The streaming path fails a substituted pinned model as the batch path does. Batch 15a gate on 9005da604, re-assembled as 8a2ee8bca with only settle's rustfmt commit changing two files' formatting (MAIN 9c0b9aed0 has the same code): cargo check --workspace --tests clean; nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-compose -p roko-core -p roko-learn -p roko-serve --keep-going -D warnings clean; lib tests pass: roko-cli 3190 (two flakes, the turn_policy escalated-timeout test and graph_run_routing_observations_survive_a_crash, pass alone and in their module), roko-agent 2268, roko-core 1952, roko-learn 1204, roko-serve 986, roko-compose 560; cargo test -p roko-cli --test learning_wiring_census: 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

On bug-31438d's branch, `check_served_model` (`graph_task_dispatch/served_model.rs:108`) detects that a pinned model was substituted. The batch path fails the attempt on it (`graph_task_dispatch.rs:1204-1219`, category `model_substituted`). The streaming path calls it and discards the result: `let _ = self.check_served_model(…)` (`streaming.rs:245`). A streamed attempt served by another model therefore completes as if the pin held.

## Why it matters

One settled record per attempt (epic spec-b7303f): the pin guarantee depends on which path a task happens to take. Benchmarks that pin a model can't trust streamed attempts.

## Where

`streaming.rs:245` and `served_model.rs`.

## Plan

1. Handle the result in the streaming path exactly as the batch path does.
2. Add `streaming_dispatch_fails_a_substituted_pinned_model`.

## Done when

- [ ] A substituted pinned model fails the attempt on both paths.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on bug-31438d's branch.
- Implemented on `work/bug-b8af02` at `d44ba2b72`; cargo verification deferred to the batch check. `streaming_dispatch_fails_a_substituted_pinned_model` (targeted `cargo test` passed at the branch head). The streaming path now handles `check_served_model` as the batch path does. Verification is skipped, and the attempt settles as a provider failure that carries the pin message, with its feedback emitted. After the failed terminal receipt and event, the dispatch returns the non-retryable `model_substituted` error.
