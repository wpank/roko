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
updated = 2026-10-19
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e11"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (GP)"
anchors = ["crates/roko-cli/tests/golden_path_acceptance.rs", "crates/roko-cli/tests/fixtures/golden_path/"]
lane = "rust-cold"
parent = "spec-f09094"
links = { depends_on = ["gap-3aa9cb"], blocks = [], related = ["gap-e21595", "gap-af00b1", "gap-d14a43", "find-4003b9", "find-5bcba7", "find-8b6828", "bug-2110e8", "bug-ba0c40", "bug-1f8db7", "find-fc5cdd", "bug-2504b3", "bug-3f40d2"], supersedes = [], duplicate_of = "" }

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

## Decision (2026-10-18, Will)

Closes on the rerun that find-4003b9's option 2 asks for (tightened fixture, pass^3), not on the 2026-10-16 record.

## Rerun (2026-10-18): 2 of 3, so this stays open

The rerun on the tightened fixture (`15837120f`, work/find-4003b9) is recorded in
`work/history/golden-path-live-2026-10-18.md`. It ran three live runs for $0.70 by the wire count ($0.69 by roko),
with 0 interventions:

- **Run 1** (`graph-09f0bd03-b7ec-463f-b311-5d6409516cac`) **failed.** All nine tasks passed their own checks, but
  T7 put `VERSION` after alpha's test module. roko's whole-plan check failed clippy, the delivery settled
  `regression_failed`, and nothing was delivered (find-5bcba7).
- **Run 2** (`graph-a9cdb3e8-b765-497e-9d3c-54e7decc9383`) **passed**, $0.214916, $0.0239 per verified task.
  Review: CLEAN.
- **Run 3** (`graph-338e9cf3-3be5-41c2-9162-c9fe5171b355`) **passed**, $0.254900, $0.0283 per verified task.
  Review: one low-severity T5 escape (find-8b6828).

There were no escalations in any run. The new non-ASCII case rejected T5's ASCII-only first attempt in runs 2 and 3,
and the retry on the cheap rung passed. pass^3 therefore needs find-5bcba7 settled, and find-8b6828 if reviews must
find no escape, then a new pass^3.

Two other findings came out of the rerun:

- A ~$1 plan budget with the workspace's $0.50 turn cap stops attempts at $0 spent (bug-2110e8). The runs set
  `max_turn_usd = max_plan_usd / 8`.
- `RequireToolBeforeEdit` admits an edit to an unread file (gap-78cf31).

## Decision (2026-10-19, Will)

Closes on the pass^3 rerun after find-5bcba7 (whole-plan failures handed back to their tasks) lands, on the tightened fixture.

## Rerun (2026-10-19): 3 of 3 on the live test, 2 of 3 by the review bar, so this stays open

The rerun after find-5bcba7 (`11bbb817f`) on the fixture tightened again by find-8b6828 (`74a24a836`) is
recorded in `work/history/golden-path-live-2026-10-19.md`. It ran at `4535b3914` (`work/find-8b6828`), under a $2
cap with `max_turn_usd = max_plan_usd / 8` (bug-2110e8 is open), and spent $0.747 by the wire count ($0.737 by
roko), with 0 interventions.

- **Run 1** (`graph-d282b763-e911-4ffb-a22f-fc34ff59abe2`) passed all twelve checks, $0.212433, $0.0236 per
  verified task. But its delivered branch, which `main` fast-forwarded to, holds T7's 233 cargo build files
  (7.4 MB) under `crates/alpha/target/`, outside T7's `files` (bug-ba0c40). Both reviews refuse the merge. One
  also counts T4's accepted commit as an escape: not fmt-clean, and a doc example that cannot compile, both
  repaired by T6 before delivery (find-fc5cdd).
- **Run 2** (`graph-4146c90c-baf6-4e70-9d37-63b831f8b922`) passed, $0.266352, $0.0296 per verified task.
  Review: CLEAN.
- **Run 3** (`graph-da87db5b-72da-4eb7-907c-6f414694db0d`) passed, $0.258572, $0.0287 per verified task. Two
  reviews: CLEAN, CLEAN.

There were no escalations. Run 3's T9 failed over from Cerebras to `glm-4.7` after two 429s: a provider
failover, not a failed check.

No whole-plan check failed, so the hand-back was not needed live. It was proven at $0 in rehearsal r2, where T7's
misplaced `VERSION` was handed back and fixed, and the plan was delivered. The ß case rejected T5's ASCII-only
first attempts in runs 2 and 3, and no review found find-8b6828's escape.

**What pass^3 needs next.** bug-ba0c40 fixed, so that an attempt's undeclared build output is neither committed
nor delivered. `golden_path_live` should also drop its relative `CARGO_TARGET_DIR` and check that every
delivered path is in a task's `files`. Then a new pass^3.

Filed from the rerun:

- bug-ba0c40 (p1): undeclared build output is committed and delivered;
- bug-1f8db7 (p1): `run_tests` was refused or failed in all 30 calls, the immune boundary isolating it;
- find-fc5cdd (p2): a Rust task's gate is V0, though its spec asks for fmt and a working doc example;
- bug-2504b3 (p2): the audit charged T4's defect to T5 (`false_green` in runs 1 and 2);
- bug-3f40d2 (p3): a retry diagnosis named a file that does not exist.
