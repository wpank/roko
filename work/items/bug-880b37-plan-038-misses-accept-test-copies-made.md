+++
id = "bug-880b37"
kind = "bug"
title = "PLAN_038 misses accept-test copies made through a variable or a loop (A=…/accept, cp $A/…, for p in …)"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/task_accept"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report on gap-ba4d01, branch work/gap-ba4d01 at 0a68f0643)"
anchors = ["crates/roko-cli/src/task_accept.rs::copies_accept_test_by_hand"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["gap-ba4d01", "gap-d14a43"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn variable_and_loop_accept_copies_are_plan_038' crates/roko-cli/src/ && cargo test -p roko-cli --lib variable_and_loop_accept_copies_are_plan_038"
+++

## Problem

`copies_accept_test_by_hand` (`crates/roko-cli/src/task_accept.rs`) flags a verify step only when a segment starts with `cp` and one of its words starts with `accept/` or contains `/accept/`. A copy made through a variable or a loop passes unseen:

- `A=plans/x/accept && cp $A/foo.test.ts src/`
- `for p in a b; do cp "$ACC/$p.test.ts" src/; done`

The portal plans used both forms until gap-ba4d01 migrated them by hand (wk-runstate). A copy like that added later slips past PLAN_038 and past `portal_plans_have_no_hand_copied_acceptance_tests`.

## Why it matters

Specs a cheap model can execute (epic spec-e57870): a hand copy is the unpinned convention `[task.accept]` replaces, and an agent can edit the copy it is judged by. The detector should not depend on how the path is spelled.

## Where

`crates/roko-cli/src/task_accept.rs::copies_accept_test_by_hand`, called from `accept_issues` (PLAN_038 in `plan_validate.rs`).

## Current state

At 712ed9536 the check is the word test above; its unit test covers only literal paths.

## Plan

1. Track shell variables assigned a path that contains `accept` (`A=…/accept`, `ACC=$PLAN/accept`) within the command, and treat `cp $A/…`, `cp "${A}/…"` and `cp` inside a `for … do … done` body as copies when their source expands to an `accept/` path.
2. Keep it a warning, as now. Prefer a false positive over a miss, but don't flag `cp` of files that are not under an accept directory.
3. Add `variable_and_loop_accept_copies_are_plan_038`: the two forms above are flagged, a literal copy still is, and `cp src/a.ts src/b.ts` is not.

## Done when

- [ ] Copies through a variable or a loop are PLAN_038 warnings.
- [ ] The `[[verify]]` command passes.
