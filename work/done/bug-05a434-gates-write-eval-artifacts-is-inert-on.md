+++
id = "bug-05a434"
kind = "bug"
title = "gates.write_eval_artifacts is inert on the Graph path, and a test no longer proves the suppression it claims"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-017c2d"
anchors = ["crates/roko-core/src/config/gates.rs", "crates/roko-cli/src/graph_task_dispatch.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-017c2d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn write_eval_artifacts_is_reported_inert_on_graph' crates/roko-cli/src && cargo test -p roko-cli --lib write_eval_artifacts"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:04Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:14:43Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

After bug-017c2d, the Graph path has no property-body source, so `gates.write_eval_artifacts` does nothing. `skip_enrichment_plan_meta_is_read_and_suppresses_eval_artifacts` no longer proves any suppression. `EpisodeView::succeeded` has no reader (gap-d0f52f's notes).

## Plan

List write_eval_artifacts as inert, or give it a source again. Rewrite the test to check what still happens, and delete `EpisodeView::succeeded` or give it a reader.

## Done when

- `cargo test -p roko-cli --lib write_eval_artifacts` passes, and the setting is honest.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-017c2d, during the evening close-out round.
- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e; cargo verification deferred to the batch check.
  - `gates.write_eval_artifacts` is listed in `graph_engine_inert_settings`, with the reason (no assertion body to
    render, nothing executes the files), so `plan run` warns and `roko config doctor` reports it; the config doc says so.
    I did not give it a source: no plan-task field holds a Rust assertion body. New test
    `write_eval_artifacts_is_reported_inert_on_graph`; the inert-settings test no longer lists the key as wired.
  - Tests: the dispatch test is renamed `write_eval_artifacts_writes_nothing_without_a_property_body`. The
    skip-enrichment test is now `skip_enrichment_plan_meta_is_read_once_per_plan`: it checks the meta is read once per
    plan and that a flagged plan still dispatches. The verify gained a grep guard, so the filter cannot pass on zero tests.
  - `EpisodeView::succeeded` is deleted. It had no reader, and `PatternMiner`'s output is not read in production
    either, so giving it a reader would serve nothing. Updated the trait docs and doctest, roko-learn's
    `EpisodeActions`, roko-neuro's `EpisodeActionView` (tier_progression.rs), and the miner tests' `Ep` fixture.
