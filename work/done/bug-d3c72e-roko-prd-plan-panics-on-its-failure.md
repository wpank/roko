+++
id = "bug-d3c72e"
kind = "bug"
title = "roko prd plan panics on its failure path when byte 2000 of the model output falls inside a multi-byte character"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/prd"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "4dc345a29"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-9681d2"
anchors = ["crates/roko-cli/src/prd.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-9681d2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib prd_failure_output_cuts_at_a_char_boundary"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T00:39:03Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T22:34:26Z"
forced = false
evidence = "Gate 6e on ddf47dbd6 plus its fixes, re-run at 3ac297a00 and merged as 4dc345a29 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 10 crates; lib tests pass (roko-cli 3420, roko-agent 2289, roko-core 1984, roko-learn 1230, roko-serve 1013, roko-graph 488, roko-conductor 316, roko-acp 220, roko-execution 193, roko-dreams 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp integration, smoke, graph_plan_callers, graph_timeout_matrix and plan_conversion pass; bin 445; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

`prd.rs` (around line 2212) prints `&output[..output.len().min(2000)]` when plan generation fails. That byte slice panics when byte 2000 falls inside a multi-byte character, so a model reply with non-ASCII text near that point crashes `roko prd plan` on its failure path.

## Plan

Cut at a char boundary (for example `floor_char_boundary`, or `char_indices().nth(2000)`), and add a test named `prd_failure_output_cuts_at_a_char_boundary`.

## Done when

- `cargo test -p roko-cli --lib prd_failure_output_cuts_at_a_char_boundary` passes.

## Notes

- Reported on 2026-10-01 by wk-filer4, working on gap-9681d2.
- 2026-10-01 (wk-filer4): implemented on work/gap-cd51b7; cargo verification deferred to the batch check.
- 2026-10-01 (wk-filer4): What changed: prd.rs had three fixed byte cuts of model output. They were the failure print (`&output[..output.len().min(2000)]`), the non-retriable error preview (`min(500)`), and the retry prompt (`&last_output[..2000]`, a third one the item did not name). All three now cut by characters with `crate::run::truncate`, which was private in run.rs, and named constants give their lengths. The new test `prd_failure_output_cuts_at_a_char_boundary` puts a two-byte character across byte 2000.
