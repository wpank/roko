+++
id = "gap-0fad03"
kind = "gap"
title = "A ship run can't ask for a cold delivery check without editing its plan: plan run --cold-delivery-check would make the run whose check backs \"builds from a clean clone\" cold, as [meta] cold_verify does per plan"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "round-v7 roko-speed, the lead's ask after gap-dc24dd"
discovered_from = "agent:roko-speed (round v7)"
anchors = ["crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs::deliver_plan_to_batch"]
links = { depends_on = [], blocks = [], related = ["gap-dc24dd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cold_delivery_check_flag_makes_every_delivery_check_cold' crates/roko-cli/src && cargo test -p roko-cli --lib a_cold_delivery_check_flag_makes_every_delivery_check_cold"
+++

## Problem

gap-dc24dd (f9328a636) made a plan's delivery check warm by default: under `--worktree-per-task` the regression
checkout gets a warm copy of the person's build trees and keeps what earlier deliveries built there. A cold check,
from no build trees and an empty regression cargo target, is a plan property, `[meta] cold_verify = true` in the
plan's `tasks.toml`.

So a person who wants one run's delivery check cold, without changing the plan, has no way to ask. The case is the
end-of-phase ship run, the one whose check backs "the build works from a clean clone". They must edit the plan,
commit it, and remember to take it out again. A plan authored once and run many times, such as one phase of
Undertow, would need a second copy to run warm and cold.

## Why it matters

Golden path and truth. The clean-clone claim rests on a cold check having run. It should be one flag on the run that
makes the claim, not an edit to the plan.

## Where

- `crates/roko-cli/src/commands/plan.rs`: `plan run`'s arguments, beside `--worktree-per-task`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::deliver_plan_to_batch`: sets
  `.with_cold_check(plan.tasks.meta.cold_verify)`; `PlanRunContext` would carry the run's choice.
- `roko run plans/<dir>` takes `plan run`'s flags, and serve's run route may want the same switch for a chat host.

## Plan

1. `roko plan run --cold-delivery-check` (and `roko run plans/<dir> --cold-delivery-check`): every plan of the run
   delivers through a cold check, as if each set `[meta] cold_verify`. A plan's own `cold_verify = true` still
   holds without the flag. There is no flag to force a cold plan warm.
2. Without `--worktree-per-task` the flag fails before anything runs, saying why: the plan check runs in the
   person's checkout, which roko never clears. `cold_verify` there only logs a warning.
3. The run's manifest records the switch, so a run's record says its delivery checks were cold by request.
4. Docs: the Warm Build Trees section of `docs/v3/depth/04-execution/worktree-isolation.md` names the flag next to
   `cold_verify`.

## Done when

- `plan run --cold-delivery-check` gives every delivery check of the run a cold start. A test asserts that the
  regression backend gets `with_cold_check(true)` for a plan without `cold_verify`.
- Without per-task worktrees the flag is refused before the run starts.
- The `[[verify]]` passes.

## Notes

- Filed by roko-speed (round v7) at the lead's request, 2026-10-27, after gap-dc24dd. The lead: "the end-of-phase
  ship run, the one whose check backs 'builds from a clean clone', is where a person would want it without editing
  the plan". Not to be coded yet.
- Related: gap-dc24dd (the warm regression checkout and `cold_verify`).
