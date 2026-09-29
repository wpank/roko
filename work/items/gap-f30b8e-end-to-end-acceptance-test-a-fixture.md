+++
id = "gap-f30b8e"
kind = "gap"
title = "End-to-end acceptance test: a fixture plan runs through the ladder on real cheap models and merges green"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/tests"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e11"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (GP)"
anchors = ["crates/roko-cli/tests/golden_path_acceptance.rs", "crates/roko-cli/tests/fixtures/golden_path/"]
lane = "rust-cold"
parent = "spec-f09094"
links = { depends_on = ["gap-3aa9cb"], blocks = [], related = ["gap-e21595", "gap-af00b1", "gap-d14a43"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn golden_path_fixture_plan_merges_green' crates/roko-cli/tests/ && cargo test -p roko-cli --test golden_path_acceptance golden_path_fixture_plan_merges_green"
+++

## Problem

No run has tested the golden path's central claim: a plan written by a frontier model executes on cheap models,
escalates only when a check fails, is integrated, and merges green with nobody stepping in. All 210 portal attempts
pinned `claude-sonnet-4-6`, and the cheap-model runs were on different tasks (research note B7).

## Why it matters

This is assessment W8's GP: the golden-path goal's acceptance test and the whitepaper's evidence for the claim. The
canaries prove each mechanism with fake agents; real models fail differently (W8 risk 5). Part of epic spec-f09094.

## Where

- **New file:** `crates/roko-cli/tests/golden_path_acceptance.rs`.
- **New directory:** `crates/roko-cli/tests/fixtures/golden_path/`, holding:
  - a small seed repo: a two-crate Cargo workspace, a Python module and a TypeScript file;
  - a spec-first plan of 8–10 tasks written by the frontier planner: 1 Python, 1 TypeScript, 2 leaf-Rust, a
    cross-crate pair, and mechanical tasks for the rest. Every task has a planner-written `[task.accept]` test, and
    the plan has a `[meta] verify`.

## Current state

Checked at `41c7ffbd6`: no such fixture or test exists. It needs epics E2–E10 in place.

## Plan

1. Write the fixture plan and the seed repo. Check that every accept test is red on the seed (gap-b3fa0a).
2. `golden_path_fixture_plan_merges_green` runs in CI with the shared scripted provider, which replays patches. One
   task fails twice on the first rung and passes one rung up. Assert that:
   - the plan branch is delivered and merged into the seed's main branch;
   - in the merged seed, `cargo fmt --check`, clippy with `-D warnings`, `cargo test --workspace`, and the Python and
     TypeScript tests all pass.
3. `golden_path_live` is `#[ignore]`d and run by hand with keys, on real models through the D11 ladder. It makes the
   same assertions, plus 0 interventions and an executed-model and cost record for every attempt. It writes its
   report outside the repo.
4. Run it three times; all three must pass (pass^3). An independent frontier review of each merged diff looks for
   escapes.

## Done when

- [ ] The scripted test passes in CI.
- [ ] Three live runs pass. The closing evidence lists their run ids, the cost per verified task, the escalations and
      the review verdicts.
- [ ] The `[[verify]]` command passes.

## Notes

- Live runs cost money and share the Claude workers' limits; budget them (W8 risk 7).
- Spec-first: freeze the plan before writing any solution or replay patch (W8, "Honesty").
