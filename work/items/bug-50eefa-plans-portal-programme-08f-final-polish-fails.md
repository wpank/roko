+++
id = "bug-50eefa"
kind = "bug"
title = "plans/portal-programme/08f-final-polish and 08e-portal-refine fail roko plan validate since the portal's globals.css shrank (gap-195374): their contexts name lines 850-930 and 650-954 of a 644-line file, so roko-cli's five_stale_plans_pass_plan_validate (08f) fails on main"
status = "open"
triage = "unverified"
severity = "p1"
goal = "visibility"
size = "S"
subsystem = ["plans"]
created = 2026-11-12
updated = 2026-11-12
source = "round v8 portal-v8, roko plan validate on main 57f0fdf6f (2026-11-12)"
discovered_from = "agent:portal-v8 (round v8, 57f0fdf6f)"
anchors = ["plans/portal-programme/08f-final-polish/tasks.toml", "plans/portal-programme/08e-portal-refine/tasks.toml", "crates/roko-cli/tests/plan_validate.rs::five_stale_plans_pass_plan_validate"]
links = { depends_on = [], blocks = [], related = ["gap-195374", "gap-60f6b6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --test plan_validate five_stale_plans_pass_plan_validate"

[[verify]]
command = "cargo run -q -p roko-cli --bin roko -- plan validate plans/portal-programme/08e-portal-refine"
+++

## Problem

`roko plan validate` exits 1 on main for two portal plans since the portal's new look merged (gap-195374,
`57f0fdf6f`):

```
08f-final-polish  error PLAN_CONTEXT_RANGE line range `850-930` is outside `apps/portal/src/styles/globals.css` (644 lines)
08e-portal-refine error PLAN_CONTEXT_RANGE line range `650-954` is outside `apps/portal/src/styles/globals.css` (644 lines)
```

08f's T01 context names `globals.css` lines 850-930 ("rail bar, progress bar and the end of the components
layer"), and 08e's names lines 650-954 ("the band, notice and type classes"). gap-195374 moved the portal's
components into `roko-kit.css` and `portal.css`, and `globals.css` shrank from 1084 lines to 644 (and to about 330
with gap-60f6b6). So `crates/roko-cli/tests/plan_validate.rs::five_stale_plans_pass_plan_validate`, which runs
`roko plan validate` on 08d and 08f against the real tree, fails. No test runs 08e, but its validate fails the same
way. Before gap-195374 (`1e31e8905`) both plans validated without errors.

## Why it matters

A red workspace test on main: CI's test job fails, and so does any local `cargo test --workspace`.

## Where

- `plans/portal-programme/08f-final-polish/tasks.toml`, T01's `[task.context] read_files` entry for `globals.css`.
- `plans/portal-programme/08e-portal-refine/tasks.toml`, the same kind of entry.
- Every other portal-programme plan validates as it did before gap-195374 (checked at `1e31e8905` and `57f0fdf6f`).

## Plan

1. Drop both entries' `lines`, so they name the whole file: the rail rows, run band and progress bar they pointed
   at are gone from `globals.css`, and the portal overhaul (gap-60f6b6, gap-6d2dd2, gap-6c4d24) keeps reshaping it.
2. Say in each `why` where the rail's rows went.

## Done when

`cargo test -p roko-cli --test plan_validate five_stale_plans_pass_plan_validate` passes, and
`roko plan validate plans/portal-programme/08e-portal-refine` exits 0.

## Notes

- Found by portal-v8 (round v8) while checking gap-60f6b6's tree with `roko plan validate`; gap-195374's checks were
  the portal's own (types, tests, export), not the workspace's Rust tests.
