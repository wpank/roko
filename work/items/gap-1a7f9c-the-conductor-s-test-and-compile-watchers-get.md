+++
id = "gap-1a7f9c"
kind = "gap"
title = "The conductor's test and compile watchers get no gate signal on the Graph path, so RG2 stays PARTIAL"
status = "open"
triage = "unverified"
severity = "p3"
goal = "cybernetic"
size = "M"
subsystem = ["roko-conductor/watchers", "roko-cli/graph_execution"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-repin's report)"
anchors = ["crates/roko-conductor/src/watchers/test_failure_budget.rs", "crates/roko-conductor/src/watchers/compile_fail_repeat.rs", "crates/roko-cli/src/episode.rs"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["gap-08d9b2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn conductor_watchers_receive_graph_gate_verdicts' crates/roko-cli/src/ && cargo test -p roko-cli --lib conductor_watchers_receive_graph_gate_verdicts"
+++

## Problem

The conductor's `test_failure_budget` watcher scans its signal stream for `Kind::GateVerdict` signals (`crates/roko-conductor/src/watchers/test_failure_budget.rs:4`, :74), and `compile_fail_repeat` watches for repeated compile failures. On the Graph path, the gate verdicts never reach the conductor's stream: the GateVerdict signal is built in `crates/roko-cli/src/episode.rs:109` and read by the TUI (`tui/verdicts.rs`). So both watchers see nothing, and the status matrix keeps RG2 (the conductor's gate-driven interventions) at PARTIAL (wk-repin).

## Why it matters

Cybernetic core (epic spec-6ac537): the conductor is the regulator that should react to repeated test and compile failures. Without gate signals it can't. p3.

## Where

Where Graph verification settles a gate verdict, and the conductor's input stream.

## Plan

1. Publish each settled gate verdict as a `Kind::GateVerdict` signal onto the stream the conductor watches, with the fields the watchers read (the gate name, pass or fail, the failure class).
2. Add `conductor_watchers_receive_graph_gate_verdicts`.

## Done when

- [ ] A failing Graph gate reaches both watchers, and RG2 can move to WIRED.
- [ ] The `[[verify]]` command passes.
