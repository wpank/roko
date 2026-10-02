+++
id = "bug-2a31bc"
kind = "bug"
title = "Two plan validate integration tests fail on main: their diagnostic counts predate PLAN_041"
status = "open"
triage = "verified"
severity = "p2"
size = "S"
subsystem = ["roko-cli/plan_validate"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "roko-7d: cargo nextest run --workspace for the workflow-audit merge (2026-10-02)"
discovered_from = "merge:bfd36512f"
anchors = ["crates/roko-cli/tests/plan_validate.rs", "crates/roko-cli/src/plan_validate.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_validate_warns_on_known_model_aliases' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_validate plan_validate_"
+++

## Problem

Two `plan validate` integration tests fail on main (checked on a build of main at f49242f63):

- `plan_validate_warns_on_known_model_aliases` expects "3 diagnostics in 1 plan"; the run prints 6: the three
  PLAN_012 alias warnings plus a PLAN_041 warning for each task ("sets model_hint = 'haiku', which pins a model
  and bypasses the routing ladder; use `rung`").
- `plan_validate_preserves_unknown_model_warning` expects "1 diagnostics in 1 plan"; the run prints 2: PLAN_009
  plus PLAN_041.

PLAN_041 arrived with 5b8efa148 (2026-09-30, "routing: parse TaskDef hints and let plans pin a ladder rung");
the tests were not updated.

## Why it matters

A red `plan_validate` suite hides real regressions in plan validation. The batch gates run lib tests only, so
nothing flags it.

## Where

- `crates/roko-cli/tests/plan_validate.rs` (the two tests)
- `crates/roko-cli/src/plan_validate.rs` (PLAN_041)

## Plan

Decide whether PLAN_041 should fire on a task that already gets PLAN_009 or PLAN_012 for the same `model_hint`.
If yes, update the expected counts to 6 and 2 and assert that PLAN_041 is present. If not, suppress it in that
case and keep the counts.

## Done when

- Both tests pass. Verify:
  `grep -rqw 'fn plan_validate_warns_on_known_model_aliases' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_validate plan_validate_`

## Notes

- Found by roko-7d's full `cargo nextest run --workspace` for the workflow-audit merge (2026-10-02).
- 2026-10-02 (roko-90): both tests already failed at backlog gate 1 on a43288b5f, where main's prebuilt roko
  printed the same 6 and 2 diagnostics, so they predate this week's waves.
