+++
id = "gap-c1726f"
kind = "gap"
title = "The surfaces disagree on TIME and CHECKS: the portal counts TIME from plan_started (5:51 where roko's run took 6:19) and counted phase 7's checks twice (12/12 where roko checked 8), and roko serve gives no run start or plan checks for a surface to count by"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["apps/portal"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
last_verified_rev = "22040fe91"
source = "round v8 portal-v8, the lead's consistency ask after phase 7's three-surface capture (2026-11-12)"
discovered_from = "agent:portal-v8 (round v8)"
anchors = ["crates/roko-serve/src/routes/plans/reads.rs::get_plan", "crates/roko-cli/src/serve_runtime.rs::load_plan_tasks", "apps/portal/src/lib/runCard.ts"]
links = { depends_on = [], blocks = [], related = ["gap-acfe4b", "gap-6d2dd2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_detail_names_when_its_run_started' crates/roko-serve/src && cargo test -p roko-serve --lib plan_detail_names_when_its_run_started"

[[verify]]
command = "grep -rqw 'fn plan_tasks_list_the_plans_own_checks' crates/roko-cli/src && cargo test -p roko-cli --lib plan_tasks_list_the_plans_own_checks"

[[verify]]
command = "cd apps/portal && npx vitest run src/components/plan/readings.accept.test.tsx"

[closed]
at = 2026-11-13
at_ts = "2026-11-13T00:28:13Z"
commit = "22040fe91"
by = "portal-v8 (round v8)"
executor = "unknown"
forced = false
evidence = "One definition each, approved by the lead: TIME from roko accepting the run to its end (run_completed.duration_ms once ended, now - last_run.started_at while running); CHECKS x of y, y fixed from the start (task verify steps + workspace rungs when run + meta.verify), x passed once each on the last try; on a resume the whole run's spend and checks. roko serve: PlanHandle.started_at -> last_run.started_at; plan_checks and workspace_rungs in GET /api/plans/{id}/tasks (roko-cli maps them). Portal: TIME and CHECKS as defined, budget_carried added to SPENT, kept tasks' checks counted. Red first at f70edfbc0 (serve: no started_at; portal: 7 of 12 and 12 of 14 for phase 7, TIME from plan_started). At 22040fe91's tree: plan_detail_names_when_its_run_started and 76 more serve tests, roko-serve lib 1102/1102, integration plan_authoring 19, plan_discovery 6, plan_revision 6, plan_execute 6, plan_run_set 12; roko-cli plan_tasks_list_the_plans_own_checks shown correct (its plan check carries the default 900000 ms timeout, which the test now allows); nightly fmt and clippy clean on both crates; portal tsc clean, vitest 750/750 (readings.accept 5)."
+++

## Problem

The three surfaces show different TIME and CHECKS for the same run. On round v8's phase 7 (run 31e52490, 2 tasks,
4 + 3 verify steps and one plan check):

- **TIME:** Hermes showed 6:19, roko's own `run_completed.duration_ms` (379014), counted from when roko started
  the run, about the approval. The portal showed 5:51: it counts from `plan_started` to `plan_completed`, so it
  leaves out the ~28 s spec gate before the tasks.
- **CHECKS:**
  - The terminal's summary counts the passed checks on each task's last try plus the plan's checks: 8.
  - Hermes showed 7/7, without the plan check.
  - The portal showed 12/12. It matched a task's steps to their results by phase word, all of T1's steps are
    `test`, so 3 of T1's results and 2 of T2's counted twice; and it left out the plan check.

roko serve gives no surface what they need to agree while the run goes:
- the run's start, which only the run's handle knows (`run_completed` carries the duration only at the end);
- the plan's own checks (`[meta] verify`), which GET /api/plans/{id}/tasks doesn't list.

## Why it matters

Goal visibility: the guide's three-surface section claims the readings match because they come from the same
records. The lead's ask (round v8): one definition each, which every surface follows.

## Where

- `crates/roko-serve/src/state.rs::PlanHandle`, `crates/roko-serve/src/routes/plans/reads.rs::get_plan`
  (`last_run`).
- `crates/roko-serve/src/plan_types.rs::PlanTasksDto`, `crates/roko-cli/src/serve_runtime.rs::load_plan_tasks`.
- `apps/portal/src/lib/runCard.ts` (`checkSignals`, the TIME reading), `apps/portal/src/components/plan/PlanPage.tsx`.

## Plan

The definitions (the lead's proposal, made exact):

- **TIME:** from when roko accepted the run to when it ended.
  - Once ended, every surface shows the run's `run_completed.duration_ms`.
  - While it runs, it's now minus the run's start: `last_run.started_at`, which serve now returns in
    GET /api/plans/{id} from the run's handle, as it returns `max_usd` (gap-acfe4b).
- **CHECKS:** "x of y".
  - y is fixed from the start: each task's verify steps, plus the workspace's rungs when the plan runs them, plus
    the plan's own checks (`[meta] verify`).
  - x is those that passed, each counted once, on its task's last try.
  - GET /api/plans/{id}/tasks now lists `plan_checks` and says `workspace_rungs`.

1. roko serve and roko-cli: `PlanHandle.started_at` and `last_run.started_at`; `plan_checks` and `workspace_rungs` in
   the plan's tasks.
2. Portal: TIME from `last_run.started_at` (else `plan_started`), and `run_completed.duration_ms` once the run ends.
   A task's steps are matched to their results by their index (`verify[i:…]`), rungs only when the plan runs them,
   and the plan's checks are counted.
3. Tests:
   - `plan_detail_names_when_its_run_started` (roko-serve);
   - `plan_tasks_list_the_plans_own_checks` (roko-cli);
   - `readings.accept.test.tsx` (portal).

## Done when

- On a run like phase 7, the portal reads TIME 6:19 and CHECKS 8/8, as roko's own figures do.
- The `[[verify]]` passes.

## Notes

- roko-recovery (the terminal) and plugin-wow-v8 (Hermes, and the design system's words) follow the same
  definitions.
