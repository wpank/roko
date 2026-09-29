+++
id = "spec-9230a9"
kind = "spec"
title = "Epic: check each attempt's diff for tampering and scope"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "L"
subsystem = ["roko-gate", "roko-cli/graph_task_dispatch"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e9"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P1 #13; tldr/04 design rule 4)"
anchors = ["crates/roko-gate/src/diff_gate.rs::analyze_diff", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::settle_task_verification"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "rust-cold"
links = { depends_on = ["gap-abbd22", "gap-b72761", "gap-b954ad"], blocks = [], related = ["gap-d14a43"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn c5_tampering_attempt_is_flagged' crates/roko-cli/tests/ && grep -rqw 'fn c5_empty_diff_is_rejected_before_verify' crates/roko-cli/tests/ && cargo test -p roko-cli --test attempt_diff_canary"
+++

## Problem

After an agent finishes, the only check is the task's own verify commands, which the agent can see (research note
B4). Nothing looks at what the attempt changed:

- An agent can edit or delete tests, add `#[ignore]`, rewrite a verify script, or edit the pinned `accept/` sources
  or the gate settings, and still pass.
- Nothing compares the changed files with the task's `files`. The dispatcher records `changed_files: Vec::new()` for
  every attempt (`graph_task_dispatch.rs:4651`).
- An attempt that changed nothing, or returned malformed or runaway output, still goes through every verify step.

## Why it matters

tldr/04 design rule 4: assume visible checks will be gamed, most of all by cheap models. The gap between visible and
held-out tests is larger for smaller models (`zhao2026specbench`), and coding agents edit tests to pass
(`zhong2025impossiblebench`, `gabor2025evilgenie`). The golden path puts cheap models in front of exactly these
checks. This epic is tldr/05 P1 #13, S05's inline check A1, and gate G5 in assessment W8.

## Where

- `crates/roko-gate/src/diff_gate.rs`: `DiffGate` and `analyze_diff` already reject empty and stub-only diffs. Only
  the orphaned rung pipeline (`runner/gate_dispatch.rs`) uses them.
- `crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::settle_task_verification`: the one place where
  both dispatch paths (batch and streaming) settle an attempt. The new checks run at its top, before any verify step.

## Current state

Checked at `41c7ffbd6`:

- No tamper, scope or empty-diff check exists on the Graph path (S05 §3: "ABSENT").
- `DiffGate` exists, but Graph runs never call it.
- Today's merges `ce3bdcbb8`, `99adacd6d` and `c7023a60b` changed the verify-settle path. Re-read it before wiring.

## Plan

This is the implementation plan.

1. **Red flags first** (gap-b72761, can start now): compute the attempt's diff once. Before the verify steps, reject
   an empty diff (with `analyze_diff`) and malformed or overlong output.
2. **Tamper and scope** (gap-abbd22, after gap-d14a43's `[task.accept]`): a pure `roko_gate::attempt_diff` check over
   the same diff, wired next to step 1. It also fills `changed_files`.
3. **Prove it** with canary C5 (gap-b954ad), which joins the golden-path suite (epic E11).

## Done when

- [ ] gap-abbd22: Attempt diff check: flag edits to tests, verify scripts, accept/ or gate config, and to files
      outside the task
- [ ] gap-b72761: Reject empty diffs and malformed or overlong agent output before the gates run
- [ ] gap-b954ad: Integration test C5: a tampering attempt is flagged and an empty diff is rejected
- [ ] The epic's `[[verify]]` command (test C5) passes on the merged branch.

## Notes

- **For the author to decide** (B4 question 2): does a tamper or scope finding fail the attempt, or only label it for
  audit? tldr/05 decision 6's default fails the attempt for changes to existing or planner-written tests. The items
  assume that default, and record scope findings without failing unless `[gates]` enables it.
- **Hot file:** the wiring edits `graph_task_dispatch.rs`, so both code items wait for the dispatch-file split (E15.4,
  gap-c8e1f1) and go one at a time. Their logic lives in new modules, which can be written earlier.
- The audit battery that re-checks accepted commits offline (S05) belongs to epic E17, not here.
- **Decided 2026-09-29 (Will):** edits to tests, verify scripts, `accept/` or gate config fail the attempt. Edits outside the task's `files` are recorded, and fail the attempt only with `[gates] diff_scope = "enforce"`.
