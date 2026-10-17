+++
id = "bug-dfe4c5"
kind = "bug"
title = "roko plan import refuses while roko serve runs in the workspace: it takes the exclusive workspace lock, though a fresh import writes only new plan directories"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/plan"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "081cbaf37"
source = "tmp/hermes-roko-v6 (dogfood-desktop's mori-import scene, via team-lead)"
anchors = ["crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/plan_import.rs::write_mori_import"]
links = { depends_on = [], blocks = [], related = ["bug-1f5406", "bug-808700", "gap-624658"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_import_runs_beside_the_serve_that_holds_the_workspace' crates/roko-cli/ && cargo test -p roko-cli --lib plan_import_runs_beside_the_serve_that_holds_the_workspace"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T19:24:55Z"
commit = "c062ebf85"
forced = false
evidence = "A fresh roko plan import takes no workspace lock, as roko plan generate takes none (#226): it writes each new plan into a hidden .<name>.import-<pid> directory beside its place, which plan discovery skips, and renames it in whole, so a reader such as roko serve sees all of the plan or none. --force, which replaces plans, keeps the exclusive lock, and when serve holds it the error names serve and points to a new --out or --set. Beside serve the report ends with roko serve (PID n) runs in this workspace and lists the new plans. Live at zero cost, a serve on a copy of the pocket-sequencer workspace: before (roko-main-b2970cb12) both imports exit 2 with the workspace lock error and /api/plans is empty; after, the import writes plans/pocket-sequencer (5 tasks, 0 diagnostics) and exits 0, --force exits 2 naming serve, /api/plans lists pocket-sequencer with its 5 tasks, and no staging directory is left. On 081cbaf37 (main 56ed8657b merged in): the verify, the 4 importer tests, the corpus test (156 of 156 validate), the bug-ce2967 and gap-6605f7 verifies, clippy -D warnings (roko-serve, roko-cli) and nightly fmt pass."
+++

## Problem

`roko plan import --mori <dir>` takes the exclusive workspace lock. A running `roko serve` holds that lock for its whole
life, so the import refuses whenever serve is up in the same workspace:

```
$ roko plan import --mori .mori/plans/pocket-sequencer
error: Another roko process holds an exclusive lock on this workspace.
  hint: wait for it to finish or kill it.
```

dogfood-desktop hit this in round v6 while recording the mori-import scene: with Hermes connected, the only way to
import was to stop serve. bug-1f5406 (`roko doctor`) and bug-808700 (`roko learn`) are the same class.

## Why it matters

- Goal `hermes`: when Hermes uses roko, serve runs all day.
- The mori import is how a mori plan becomes a roko plan. The scene shows the import in a terminal pane and then the
  plan in Plans, which is the serve the import could not run beside.

## Where

- `crates/roko-cli/src/commands/plan.rs`, the `PlanCmd::Import` arm, which calls `acquire_workspace_lock`.
- `crates/roko-cli/src/plan_import.rs::write_mori_import`, which writes `tasks.toml` and then its companions in place.
- Precedent: `roko plan generate` takes no workspace lock (#226). It writes one new plan directory and reads the rest.
- `crates/roko-cli/src/doctor.rs::serve_holding_workspace` recognises the serve that holds a workspace (bug-1f5406).

## Current state

An import reads the mori directories and the workspace, then writes new plan directories under `plans/`. It touches
no `.roko` state, so the lock guards nothing a fresh import needs. `--force` is different: it replaces plans, and a run
may be using them.

## Plan

1. A fresh import takes no workspace lock, as `plan generate` takes none.
   - Each new plan is written to a hidden directory beside its place. Plan discovery skips names that do not start
     with a letter or a digit. The directory is then renamed into place, so serve lists all of the plan or none of it.
   - A plan that appeared meanwhile makes the rename fail, and the import refuses as it does today.
2. `--force` keeps the exclusive lock. When serve holds the lock, the error names serve and says what to do instead:
   import into a new `--out` or `--set`, or stop serve.
3. Beside serve, the report says that serve lists the new plans.
4. Test `plan_import_runs_beside_the_serve_that_holds_the_workspace`.

## Done when

- The `[[verify]]` passes.
- With a `roko serve` up, `roko plan import --mori <dir>` writes the plan, and `GET /api/plans` lists it.
