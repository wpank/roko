+++
id = "bug-430473"
kind = "bug"
title = "The TUI's PatternMiner learning row counts CrossEpisodeConsolidator patterns"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/tui"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "4dc345a29"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-4adfa7"
anchors = ["crates/roko-cli/src/tui/dashboard_model.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-4adfa7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'PatternMiner' crates/roko-cli/src/tui/dashboard_model.rs && grep -q 'rendered.contains(\"MetaPatterns\")' crates/roko-cli/src/tui/dashboard.rs && cargo test -p roko-cli --lib learning_page_renders_learning_system_status"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T00:38:59Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T22:34:38Z"
forced = false
evidence = "Gate 6e on ddf47dbd6 plus its fixes, re-run at 3ac297a00 and merged as 4dc345a29 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 10 crates; lib tests pass (roko-cli 3420, roko-agent 2289, roko-core 1984, roko-learn 1230, roko-serve 1013, roko-graph 488, roko-conductor 316, roko-acp 220, roko-execution 193, roko-dreams 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp integration, smoke, graph_plan_callers, graph_timeout_matrix and plan_conversion pass; bin 445; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

The learning row labelled PatternMiner (`dashboard_model.rs:879`) counts CrossEpisodeConsolidator meta-patterns. gap-4adfa7 deleted the runtime miner, so the label names something that no longer exists.

## Plan

Rename the row after what it counts.

## Done when

- The verify passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on gap-4adfa7, during the evening close-out round.
- 2026-10-02 (wk-tuiv): implemented on work/bug-6c11d1; cargo verification deferred to the batch check.
  The row is now `MetaPatterns` (it counts the meta-patterns `CrossEpisodeConsolidator::discover` finds across the
  loaded episodes, computed on each render), with a comment saying so; its health reads `found` instead of
  `mining`, since nothing mines. `learning_page_renders_learning_system_status` asserts the new label and the
  old one's absence; the verify now runs it.
