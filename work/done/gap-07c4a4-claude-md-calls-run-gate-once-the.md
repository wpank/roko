+++
id = "gap-07c4a4"
kind = "gap"
title = "CLAUDE.md calls run_gate_once the per-task gate entry point, but only tests call it; Graph runs verify in graph_task_dispatch/verification.rs"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["docs/claude-md"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "c1a537d2e"
source = "status-quo-2026-10-15#finding-38"
anchors = ["CLAUDE.md", "crates/roko-cli/src/runner/gate_dispatch.rs::run_gate_once", "crates/roko-cli/src/graph_task_dispatch/verification.rs"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-fd9dfb", "gap-c42c8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'the per-task entry point is `run_gate_once`' CLAUDE.md && grep -q 'graph_task_dispatch/verification.rs' CLAUDE.md"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T14:00:51Z"
commit = "c1a537d2e"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T08:07:03Z"
forced = false
evidence = "c1a537d2e rewrites CLAUDE.md's Gates row: it no longer calls run_gate_once (crates/roko-cli/src/runner/gate_dispatch.rs) the per-task entry point — confirmed at HEAD that every call to run_gate_once outside its own doc comment sits inside that file's #[cfg(test)] mod tests (from line 2078) — and instead says a Graph run checks a task through its authored verify steps and then [[gates.rungs]] in crates/roko-cli/src/graph_task_dispatch/verification.rs, matching run_verify_steps's own doc comment; run_gate_once is kept only until gap-c42c8e ports its seven test-pinned capabilities into ProductionGateService, which runs the rich-topology plan.gate cells (graph_execution/plan_runner.rs::plan_cell_resources) and the audit re-run (audit/rerun.rs, decision 7102). The same commit adds a roko safety controls/release row to the CLI reference (gap-fd9dfb's CLAUDE.md half; its docs/v3/26-HTTP-API.md half is still open, so gap-fd9dfb stays open). Both verify clauses pass at c1a537d2e: CLAUDE.md no longer contains the string the per-task entry point is run_gate_once, and it now contains graph_task_dispatch/verification.rs."
+++

## Problem

CLAUDE.md's Components table (line 25, row "Gates") says "the per-task entry point is `run_gate_once`". At HEAD
`run_gate_once` (`crates/roko-cli/src/runner/gate_dispatch.rs`) is called only from that file's test module (from
`:2080` on). A Graph plan run runs a task's verify steps in `crates/roko-cli/src/graph_task_dispatch/verification.rs`,
and the M4 audit re-run goes through `roko_gate::ProductionGateService`. An agent following CLAUDE.md to change how a
task is checked edits dead code. Status-quo assessment 2026-10-15 at 7b6a5af44, finding 38
(`tmp/status-quo/2026-10-15/programme/E2a-matrix-retag-AU-EX-IS-QA-RC.md` row QA3 and line 215).

## Why it matters

Goal `tooling`: CLAUDE.md is the first thing every agent reads, and its rule 2 ("WIRE, don't build") depends on it
naming the live entry points.

## Where

`CLAUDE.md` line 25; the same file's Absolute paths and Key crates tables mention gates too.

## Current state

gap-c42c8e tracks deleting `run_gate_once` once `ProductionGateService` ports its seven pinned capabilities.
gap-fd9dfb tracks CLAUDE.md's missing `roko safety controls/release` rows. The finding's third point, that "one
substrate / `roko replay`" does not hold for plan runs, is already stated correctly in CLAUDE.md (the Signal log row
and the Absolute paths table say plan runs do not write it); the conflict is in the research paper's §4 (claim
X4-01), which Will's decision 2 keeps as written, so it belongs to gap-ac1aed's review, not here.

## Plan

1. Rewrite the Gates row: per-task verify steps run in `crates/roko-cli/src/graph_task_dispatch/verification.rs` on
   Graph runs; `run_gate_once` (runner/gate_dispatch.rs) is kept only for its tests until gap-c42c8e removes it; the
   rung pipeline lives in `crates/roko-gate/` and `ProductionGateService` serves the audit re-run.
2. While there, add the `roko safety controls/release` rows if gap-fd9dfb is still open, and close it with the
   same commit if its docs/v3 half is done too.

## Done when

The `[[verify]]` command passes.

## Notes

Docs only. Keep CLAUDE.md's rule that it makes no status claims.
