+++
id = "gap-ba4d01"
kind = "gap"
title = "Portal plans 08b–08e and 08g still hand-copy their acceptance tests instead of pinning them with [task.accept]"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["plans/portal-programme", "roko-cli/plan_validate"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "1bf49188d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-accept's report on gap-d14a43, branch work/gap-d14a43 at 37b6d7c95)"
anchors = ["plans/portal-programme/", "crates/roko-cli/src/plan_validate.rs", ".github/workflows/plan-validate.yml"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = ["gap-d14a43"], blocks = [], related = ["gap-1b5636", "bug-b0fd73"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn portal_plans_have_no_hand_copied_acceptance_tests' crates/roko-cli/src/ && cargo test -p roko-cli --lib portal_plans_have_no_hand_copied_acceptance_tests"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T09:00:15Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20c gate on fcdaf32ae/ca5645373 (MAIN 1bf49188d has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/gate/learn/neuro/serve; lib tests roko-cli 3273, roko-agent 2278, roko-core 1962, roko-learn 1209, roko-serve 989, roko-gate 692, roko-neuro 239, roko-acp 199, roko-dreams 100 all pass; extras: C1 1/1, C7 2/2, learn_paths 7, cost_comparison 1, bin 429, verify loop 10/10, speclint 91, including portal_plans_have_no_hand_copied_acceptance_tests (assertion fixed on work/gap-ba4d01-fix b7cac84f5: a clean plan is checked but not listed) and plan_validate 16/16. Merged 27b97a7a8."
+++

## Problem

gap-d14a43's branch (37b6d7c95, not merged yet) makes three changes:

- it moves 08f's acceptance tests to `[task.accept]` (115e87d36);
- it adds PLAN_038, which makes a `[task.accept]` problem an error and a hand copy out of `accept/` a warning (plan_validate.rs:1685-1687 on the branch);
- it leaves 08b, 08c, 08d, 08e and 08g on the old convention. At 37b6d7c95 their tasks.toml files reference `accept/` 70, 18, 43, 27 and 11 times, and none has a `[task.accept]` table.

After the merge, those five plans get PLAN_038 warnings. CI tolerates them, because `.github/workflows/plan-validate.yml` (:58) runs `roko plan validate "$plan_dir"` without `--strict`.

## Why it matters

Specs a cheap model can execute (epic spec-e57870): a hand copy is the unpinned convention that gap-d14a43 replaces. An agent can edit the copy it is judged by. The warnings also train readers to ignore PLAN_038.

## Where

- The five plans' `tasks.toml` under `plans/portal-programme/`.
- PLAN_038 in `plan_validate.rs`.
- The CI job in `.github/workflows/plan-validate.yml`.

## Current state

08f is migrated on the branch; the other five are not.

## Plan

1. Migrate each plan's hand-copied acceptance tests to `[task.accept]`, as 115e87d36 did for 08f.
2. Run `roko plan validate --strict --dag` on each plan, and fix what it reports.
3. Add `portal_plans_have_no_hand_copied_acceptance_tests`, which validates the six portal plans and finds no PLAN_038.
4. Consider making the CI job `--strict`. That is a separate decision, because other plans may still carry other warnings.

## Done when

- [ ] None of 08b–08e or 08g hand-copies an acceptance test, and none gets PLAN_038.
- [ ] The `[[verify]]` command passes.

## Notes

- These plans are finished. Their replays and field evidence refer to them, so keep each task's behaviour, and change only how its tests are pinned.
- Merge gap-d14a43 first.
- Implemented on `work/gap-ba4d01` at `f7bf1a103`; cargo verification deferred to the batch check.
- `roko plan validate --strict --dag` (batch binary 1ce6526f8) now reports no PLAN_038 for any of the six plans
  (there were 30, 8, 20, 10 and 2 warnings for 08b, 08c, 08d, 08e and 08g). The other diagnostics are unchanged. 08b
  T01's PLAN_TIER_SIZE warning lost its "5 verify steps" clause. The PLAN_CONTEXT_RANGE errors in 08d and 08f predate
  this item: their read_files ranges run past files that shrank since.
- PLAN_038 only sees `cp accept/…` and `cp …/accept/…`. It misses copies through a variable (`A=…/accept`,
  `cp $A/…`, the `for p in …` loops of the final tasks, all of 08e T01 and 08g T02). Those were migrated as well. The
  new test checks PLAN_038 only, so a variable copy added later would pass it.
- 08b's `accept/dom.tsx` is a helper, not a test, and an entry needs a runner and a count. T01 pins it first with the
  node suite its own steps require (formatters.test.ts, 22), because the harness test is not in the tree until the next
  entry. T16 pins it with the harness test (8).
- Item plan step 4 (making `.github/workflows/plan-validate.yml` run `--strict`) is left to a separate decision.
