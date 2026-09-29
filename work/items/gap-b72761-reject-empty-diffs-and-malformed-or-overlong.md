+++
id = "gap-b72761"
kind = "gap"
title = "Reject empty diffs and malformed or overlong agent output before the gates run"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e9"
discovered_from = "tmp/cybernetic-harness/tldr/research/C1-research-planning-decomposition-cascades.md (implication 5: red-flag pre-gates)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/red_flags.rs", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::settle_task_verification"]
lane = "rust-cold"
parent = "spec-9230a9"
links = { depends_on = [], blocks = [], related = ["gap-abbd22"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn empty_diff_fails_before_verify_runs' crates/roko-cli/src/ && cargo test -p roko-cli --lib empty_diff_fails_before_verify_runs"
+++

## Problem

Every attempt that the provider reports as successful goes through all of its verify steps, however suspicious it
is:

- an implementer attempt that changed no file, where a weak verify step can then pass on the untouched tree;
- output that is malformed for the role, for example a planner or reviewer reply with no parsable result;
- runaway output far longer than the task needs.

`roko_gate::diff_gate::analyze_diff` already rejects empty and stub-only diffs, but only the orphaned rung pipeline
(`runner/gate_dispatch.rs`) calls it.

## Why it matters

Red flags cost almost nothing and stop bad attempts before Roko pays for compiles and test runs. In one study,
discarding outputs that were malformed or over 750 tokens let a cheap model run a million steps without an error
(`meyerson2025solving`; C1 implication 5). tldr/05 P1 #13. Part of epic spec-9230a9.

## Where

- **New file:** `crates/roko-cli/src/graph_task_dispatch/red_flags.rs`, beside `sibling_settle.rs` and
  `retry_feedback.rs`.
- `GraphTaskDispatcher::settle_task_verification`: one call at the top, before the verify loop. It covers both the
  batch and the streaming dispatch paths.
- Reuse `roko_gate::diff_gate::{DiffPayload, analyze_diff}`.

## Current state

Checked at `41c7ffbd6`: no empty-diff, output-size or output-format check on the Graph path. First check which
malformed provider streams already fail the attempt, for example a stream with no `result` event, and add only what
is missing.

## Plan

1. Compute the attempt's changed paths once; gap-abbd22 will reuse the same snapshot. For a task that declares
   `files`, an empty or stub-only diff fails the attempt as "no changes", with that message as retry feedback.
2. Overlong output: fail when output tokens exceed a cap per role (`[gates] max_output_tokens`, generous by default),
   and record it.
3. Malformed output: for roles whose output is the product (planner TOML, reviewer verdict), fail when it does not
   parse.
4. A red-flag failure counts as a failed attempt, never as a gate pass or a skip.

## Done when

- [ ] An implementer attempt with no changes fails without running its verify steps.
- [ ] The test `empty_diff_fails_before_verify_runs` covers that case; companion tests cover the overlong and
      malformed cases.
- [ ] The `[[verify]]` command passes.

## Notes

- The module can land now (rust-cold). The one-line call in `graph_task_dispatch.rs` waits for the dispatch-file
  split (E15.4, gap-c8e1f1).
- Tasks without `files`, and refactor tasks, are exempt from the empty-diff rule.
- **From wk-filer (2026-09-29):** reuse `SafetyLayer::post_dispatch_check` (`crates/roko-agent/src/safety/mod.rs`, about line 1107), which today only ACP calls.
