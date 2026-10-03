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
