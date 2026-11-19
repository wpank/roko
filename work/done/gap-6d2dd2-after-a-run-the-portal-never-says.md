+++
id = "gap-6d2dd2"
kind = "gap"
title = "After a run the portal never says where the work went: a passed run sits on its own branch with main untouched, and the portal shows no branch, no changes and no merge command, and offers Run again; a failed row shows the traceback's first line; GET /api/plans/{id} names the plan's last run only while it can be resumed, so the portal can't find it after a reload"
status = "done"
triage = "verified"
severity = "p1"
goal = "visibility"
size = "M"
subsystem = ["roko-serve/plans"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "a60b70588"
source = "round v8 portal-v8 audit (tmp/hermes-roko-v8/design/portal/before, main bd93521ee), the lead's approved proposal (tmp/hermes-roko-v8/design/portal/README.md, 2026-11-12)"
discovered_from = "agent:portal-v8 (round v8, bd93521ee)"
anchors = ["crates/roko-serve/src/routes/plans/reads.rs::get_plan", "crates/roko-serve/src/routes/run_result.rs::run_result", "apps/portal/src/api/contracts.ts::WirePlanDetail", "apps/portal/src/lib/checks.ts::digestHeadline"]
links = { depends_on = ["gap-60f6b6"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_detail_names_its_last_run' crates/roko-serve/src && grep -rqF 'a passed run says where its work is' apps/portal/src && cargo test -p roko-serve --lib plan_detail_names_its_last_run && (cd apps/portal && npx vitest run src/components/plan/result.accept.test.tsx)"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T16:55:00Z"
commit = "a60b70588"
by = "portal-v8 (round v8)"
executor = "claude-agent"
via = "manual"
model = "claude-opus-5-5"
size = "M"
claimed_at = "2026-11-12T14:52:00Z"
forced = false
evidence = "roko serve: GET /api/plans/{id} returns last_run beside resumable_run. Rust in roko-work-gap-6d2dd2 (heavy.sh cargo, CARGO_INCREMENTAL=0): cargo test -p roko-serve --lib for routes::plans::tests, plan_detail_names_its_last_run and a_plan_says_whether_it_has_a_run_a_resume_would_continue, 71 passed; clippy -p roko-serve clean; nightly fmt. Portal: the plan's detail and the run's summary give 'Ready to merge: 3 files changed, +14 lines, on its own branch...', the merge command with copy, review (orange) opening the run's diff, 'Merged into main.', a failed check's command and real error line (lib/reason.ts), and a passed task's changes. At a60b70588's tree: tsc clean, vitest 720/720 (result.accept 7 cases, reason.test 7), orphans none, build:export OK, roko plan validate over every portal plan without errors, and the CI browser check PASS 37 of 37 against this tree's own roko build. A run nobody watched, then the page: this build says 'Ready to merge' with the merge command; roko-main-bd93521ee (no last_run) says only 'All 3 tasks passed their checks.'. Shots: tmp/hermes-roko-v8/design/portal/item3/."
+++

## Problem

After a run, the portal never says where the work went or why a run failed.

- **A passed run:** the greeting plan's work landed on `roko/plan/greeting` (batch `roko/batch/<run>`), and the
  checkout's `main` is untouched. The portal says "Finished in 20s — 3 of 3 verified." and nothing about the branch,
  what changed or how to merge it. Its main key after a pass is `▶ Run again`, which pays for the same work twice.
- **A failed run:** the failed row shows `Traceback (most recent call last):`, the first line of the check's output;
  the reason (`SyntaxError: invalid syntax`, settings.py line 2) is eight lines down, after an absolute worktree path.
- **After a reload** the portal can't find a plan's last run at all: `GET /api/plans/{id}` names it only while a
  resume could continue it (`resumable_run`), and the dashboard snapshot carries no run ids.

Screens: `tmp/hermes-roko-v8/design/portal/before/08-done.png`, `09-failed.png`.

## Why it matters

Goal visibility. "Where is my work, and is it right?" is the question every run ends on. roko serve already answers
it: `GET /api/runs/{id}/summary` has `result` (`branch`, `commit`, `diffstat`, `files_changed`, `merge_command`,
`merged`), and `GET /api/runs/{id}/diff` and `GET /api/plans/{id}/tasks/{task}/diff` have the patches. The portal
just never asks.

## Where

- `crates/roko-serve/src/routes/plans/reads.rs::get_plan` reads the plan's checkpoint run and keeps it only when
  resumable.
- `crates/roko-serve/src/routes/run_result.rs::run_result` (the summary's `result`) and `run_diff`.
- The portal: `apps/portal/src/api/contracts.ts` (`WirePlanDetail`), `src/api/queries.ts`, the plan page (gap-60f6b6),
  `src/lib/checks.ts` (`digestHeadline`, which picks the traceback's first line).

## Plan

1. **roko serve:** `GET /api/plans/{id}` also returns `last_run`, the checkpoint run whatever its state (run id,
   status, passed and left tasks), beside `resumable_run`, which keeps its meaning. A test in
   `routes/plans/tests.rs`: `plan_detail_names_its_last_run`.
2. **Passed:** "Ready to merge: 3 files changed, +14 lines, on its own branch. Your checkout hasn't changed." from
   the summary's `result`; the merge command with a `copy` key; `run again` and the orange **`review`**, which opens
   the run's changes (file by file, the patch folded) from `GET /api/runs/{id}/diff`. When `result.merged` is true:
   "Merged into main." and no merge command.
3. **Failed:** "Task 1's check failed after 2 tries: `<the check>` stopped at settings.py, line 2: invalid syntax."
   The reason line comes from the check's output: the last error line of a traceback (`…Error: …`), a test runner's
   failure summary, else the last non-empty line; paths inside the project are shown relative to it. Keys: `revise`
   and the orange **`run again`** (or **`resume`** when the run can be resumed). The full output folds.
4. A task's own changes in its detail, from `GET /api/plans/{id}/tasks/{task}/diff`.

## Done when

- `cargo test -p roko-serve --lib plan_detail_names_its_last_run` passes, and the route's existing tests too.
- CI's portal job passes; `src/components/plan/result.accept.test.tsx` passes ("a passed run says where its work is",
  "a failed run names the check's real error line").
- Screenshots, day and night, match the mock (`design/portal/after/passed-*`, `failed-*`).

## Notes

- Depends on gap-60f6b6 (the plan page).
- No merge from the portal this round: the lead's call (2026-11-12) is the merge command with `copy`, and `review`
  as the orange key. The merge route is its own held item, gap-ba20b0.
