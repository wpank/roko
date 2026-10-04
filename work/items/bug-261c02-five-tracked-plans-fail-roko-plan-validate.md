+++
id = "bug-261c02"
kind = "bug"
title = "Five tracked plans fail roko plan validate, predating PK14"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-validate"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-6 follow-up reports 2026-10-03 (PK14 gap-997366)"
discovered_from = "gap-997366"
anchors = ["plans/portal-plan-execution/tasks.toml", "plans/portal-programme/08d-portal-legibility/tasks.toml", "plans/portal-programme/08f-final-polish/tasks.toml", "plans/wire-http-plan-execute/tasks.toml", "plans/workspace-doctor-improvements/tasks.toml", "crates/roko-cli/src/plan_validate.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn five_stale_plans_pass_plan_validate' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_validate five_stale_plans_pass_plan_validate"
+++

## Problem

Five tracked plan directories fail `roko plan validate`, and did before PK14's work (gap-997366) as well — PK14
only noticed them, it did not cause the failures: `plans/portal-plan-execution/`, `plans/portal-programme/
08d-portal-legibility/`, `plans/portal-programme/08f-final-polish/`, `plans/wire-http-plan-execute/`,
`plans/workspace-doctor-improvements/`.

## Why it matters

A plan directory that cannot pass `roko plan validate` cannot be run (`roko plan run`/`roko run plans/<dir>`
would refuse it), and `plans/INDEX.md`/`roko plan list` may misreport its state. If these five are meant to be
re-run or re-resumed later (several are portal-programme slices with historical checkpoints referenced elsewhere
in this wave's notes, e.g. gap-997366's own Progress section mentions stale checkpoints on
`portal-programme/02-backend-plan-execution` and `09-acceptance` needing `--fresh`), a stale, unvalidatable
`tasks.toml` blocks that silently until someone runs validate and reads the error.

## Where

- `crates/roko-cli/src/plan_validate.rs` — the validator whose rules these five plans fail.
- `plans/portal-plan-execution/tasks.toml`, `plans/portal-programme/08d-portal-legibility/tasks.toml`,
  `plans/portal-programme/08f-final-polish/tasks.toml`, `plans/wire-http-plan-execute/tasks.toml`,
  `plans/workspace-doctor-improvements/tasks.toml`.

## Current state

Confirmed: all five plan directories and their `tasks.toml` files exist at HEAD. Which specific validator rule(s)
each fails was not re-derived in this filing pass (no cargo run; `roko plan validate` itself was not executed) —
PK14's own report is the primary evidence that they fail, and that the failures predate PK14's own change set.

## Plan

1. Run `roko plan validate` against each of the five (the first real step of the fix, not part of this filing).
2. For each distinct failure reason, either fix the plan's `tasks.toml` (if it's a stale field/schema mismatch) or
   decide the plan is dead and should be archived/removed from `plans/` rather than fixed.
3. If all five share one root cause (e.g. a schema field that changed underneath them), fix the validator rule or
   migrate the field in one pass rather than five.

## Done when

- `roko plan validate` passes (or the plan is deliberately archived with a recorded reason) for all five.
- The `[[verify]]` command passes.

## Notes

- Discovered via PK14 (gap-997366); this item exists so these five don't quietly stay broken now that they have a
  name. Whoever picks this up should run validate first to learn the actual per-plan reasons before planning a fix.

## Progress

- Ran `plan validate` against each of the five with a copy of the batch binary first, per the Plan's own step 1;
  the actual per-plan reasons split into two very different categories.
- Three were genuinely stale line ranges/symbol names, fixed in place:
  - `portal-programme/08f-final-polish`: one `read_files` range (`255-330`) outside `StreamPane.tsx` (now 292
    lines) -- re-pointed at `185-241`, the two tab badges the task's own `why` names.
  - `portal-programme/08d-portal-legibility`: two ranges outside `RunBand.tsx` (224 lines) and `StreamPane.tsx`
    (292 lines) -- re-pointed at `136-224` and `185-263`.
  - `workspace-doctor-improvements`: a symbol anchor `run_checks` not found anywhere -- the function was renamed
    to `run_doctor` (confirmed: no `fn run_checks` anywhere in `doctor.rs`, `pub async fn run_doctor` at line 221).
    Renamed every reference (symbol anchors, prose, the two `[[task.verify]]` python checks) and re-pointed both
    stale `175-...` ranges at `221-260`, `run_doctor`'s own body past `check_layout_basics`'s call.
- The other two named code and files that were never built as described, not just moved -- confirmed by reading,
  not guessed:
  - `portal-plan-execution`: all three tasks modify `apps/portal/src/app/work/editor/page.tsx`, assuming it
    already exists with an existing `TaskEditorRow` row component and `StatusLED` atom to extend. None of the
    three exist anywhere under `apps/portal/src` (confirmed: no `editor/` directory, no `StatusLED.tsx`, no
    `AgentOutputStream.tsx`). This is not a stale range; the plan's whole premise does not hold, and no task
    creates these files first (`PLAN_031` already flagged them as missing, uncreated prerequisites). Archived:
    `[meta] status = "archived"`, moved to `plans/archive/portal-plan-execution/`, with a comment recording why.
  - `wire-http-plan-execute`: wants `execute_plan` to call `runtime.run_plan` (the graph engine) instead of
    `run_once`, and to forward live `DashboardEvent`s to the SSE bus. Both already happened, by later, unrelated
    work: `execute_plan` (moved to `routes/plans/run_control.rs`, confirmed by its own comment "The Graph run
    settles the plan itself. Publish PlanCompleted...") already uses the graph engine via `start_plan_run`, and
    `runtime_event_bridge.rs` is a standing DashboardEvent-to-ServerEvent bridge, superseding T02's per-request
    sibling-task design. Marked done/superseded per task (`status = "done"`, `superseded_by = "..."` naming what
    really did it) rather than patched with a review of work that was never actually done as the plan describes;
    moved to `plans/archive/wire-http-plan-execute/`.
- Ran `roko plan index` afterward: `plans/INDEX.md` dropped both from the executable table on its own (24 plans,
  197 tasks -> 22 plans, 191 tasks), confirming `[meta] status`-based archival is read by the real tooling, not
  just a paper convention.
- Added `five_stale_plans_pass_plan_validate` to `crates/roko-cli/tests/plan_validate.rs` (a roko-cli integration
  test, not Python): runs `roko plan validate` against the real repo tree (not a tempdir fixture, since these
  plans' `read_files` scatter across the real repository) for the three fixed plans, and for the two archived
  ones checks they moved out of their old, live location and that `[meta] status` is set as described. No cargo
  available this wave (static worker); manually simulated every assertion against the real tree with the batch
  binary and confirmed each one matches (exit codes, path existence, and the exact `status = "..."` substrings)
  before writing the Rust.
- Verify: named `[[verify]]` command needs `cargo test -p roko-cli --test plan_validate` to actually run and is
  deferred to the batch gate, per `BUILD-RULES.md` (no cargo for a static worker). The test's own logic was
  verified by hand against the real tree with the batch binary, matching every assertion precisely.
