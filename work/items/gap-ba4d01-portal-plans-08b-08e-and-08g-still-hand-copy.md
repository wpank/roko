+++
id = "gap-ba4d01"
kind = "gap"
title = "Portal plans 08b–08e and 08g still hand-copy their acceptance tests instead of pinning them with [task.accept]"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["plans/portal-programme", "roko-cli/plan_validate"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-accept's report on gap-d14a43, branch work/gap-d14a43 at 37b6d7c95)"
anchors = ["plans/portal-programme/", "crates/roko-cli/src/plan_validate.rs", ".github/workflows/plan-validate.yml"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = ["gap-d14a43"], blocks = [], related = ["gap-1b5636", "bug-b0fd73"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn portal_plans_have_no_hand_copied_acceptance_tests' crates/roko-cli/src/ && cargo test -p roko-cli --lib portal_plans_have_no_hand_copied_acceptance_tests"
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
