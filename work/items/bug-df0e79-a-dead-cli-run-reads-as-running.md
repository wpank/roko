+++
id = "bug-df0e79"
kind = "bug"
title = "A dead CLI run reads as running forever: only serve settles a run whose process died, so plan status, diagnose and show plans keep saying running, and serve's settle misses the run's events id"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-cli/commands", "roko-serve/runs"]
created = 2026-10-26
updated = 2026-10-27
last_verified = 2026-10-26
source = "round-v7 roko-recovery kill matrix (tmp/hermes-roko-v7/findings/roko-recovery.md, tmp/hermes-roko-v7/roko-recovery/runs/roko-main-4d22b94e8), the lead's ask"
discovered_from = "agent:roko-recovery (round v7, 4d22b94e8)"
anchors = ["crates/roko-serve/src/interrupted_runs.rs::settle", "crates/roko-cli/src/commands/diagnose.rs::build_report", "crates/roko-cli/src/commands/plan.rs::plan_status_label", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan"]
links = { depends_on = ["bug-c872f6"], blocks = [], related = ["bug-dceafd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_status_reports_a_dead_runs_interruption' crates/roko-cli/src && cargo test -p roko-cli --bin roko -- plan_status_reports_a_dead_runs_interruption"
+++

## Problem

When the process running a `roko run plans/<p>` (or `roko plan run`) dies, nothing on the CLI side ever ends the
run. It reads as running forever:
- `roko plan status plans/<p>` says `status: running`, with the tasks "ready".
- `roko plan status` gives the frozen phase from `.roko/state/status.json`, with "active agents: 1" when the kill
  landed in the agent. That file keeps the dead pid.
- `roko diagnose <plan>` says "Plan <p>: running … The checkpoint still says `running`. If no `roko plan run` is
  active for this plan, the run stopped without recording an outcome." It doesn't say whether the process is gone,
  though the manifest names it.
- `roko show plans` says `<p> pending | tasks 0/2 done`, even with both tasks passed.
- The manifest (`.roko/runs/<run>/manifest.json`) stays open, and the event log has no `run_completed`.

Only `roko serve` settles a dead run (bug-dceafd, `interrupted_runs::settle`), and it misses a CLI run's events:
- It closes the manifest under the records id `graph-<plan>-<uuid>`.
- The run's events are under `graph-<uuid>` (bug-c872f6), so its `run_completed` goes to an index nobody reads.
- `GET /api/runs/<records id>/summary` is a 404. `GET /api/runs/<events id>/summary` says `state: running` with the
  dead run's cost, and `/api/dashboard/runs` lists it as `observed`, for good.

The $0 kill matrix reproduced this at every kill point on 4d22b94e8 (round v7: `tmp/hermes-roko-v7/roko-recovery/`,
`runs/roko-main-4d22b94e8/summary.md`, the probe rows). Serve-run crashes are fine: their summary says `interrupted`
with "roko stopped while this run was going (machine restart or crash)".

## Why it matters

Goals visibility and truth. After a crash, the person's first commands (plan status, diagnose, the dashboard)
report a live run. Hosts reading serve show a CLI run as running forever. bug-dceafd fixed this for serve runs only.

## Where

- `crates/roko-serve/src/interrupted_runs.rs::settle` (with `settle_open_runs`): the pid and start-time test, and
  the manifest and index end. It lives in roko-serve, so the CLI can't call it.
- `crates/roko-cli/src/commands/plan.rs` (`PlanCmd::Status`), `crates/roko-cli/src/commands/diagnose.rs::build_report`,
  and `crates/roko-cli/src/commands/show.rs` (plans).
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan`, where a resume reopens the run.
- The second id: `crates/roko-cli/src/graph_execution/event_log.rs::graph_run_id` against
  `crates/roko-cli/src/graph_checkpoint.rs::create_fresh_checkpoint` (bug-c872f6).

## Plan

1. Move `settle` and its process test into a crate both sides use, roko-learn's telemetry or roko-fs. Leave a thin
   wrapper in roko-serve.
2. Call it from `plan status`, `diagnose` and `show plans`:
   - they say `interrupted` with the reason and the dead pid;
   - `diagnose`'s next step gives the exact command that resumes, or the one that starts over when it can't.
3. Call it at the start of `run_graph_plan` too, so a resume first closes the dead invocation, with the interruption
   recorded in the manifest as serve records it. A CLI resume today records `interruptions: 0`.
4. Mark `status.json` stale when its pid is gone (`plan status` global).
5. One run id: bug-c872f6. This item depends on it for serve's view. Without it, settling the manifest still leaves
   the events index open.

## Progress

**2026-10-27, the CLI views** (round v7 roko-recovery, after bug-693bb3; the lead's ask). `roko plan status <dir>`,
`roko plan show`, `roko diagnose` and `roko show plans` now read a dead run's plan as `interrupted`, the way `roko
serve` and `roko plan list` have since bug-693bb3. The test is the same: the run's manifest is closed, or names a
process that is gone, or, without a manifest, no process holds the runner lock.
- `plan status` and `diagnose` count the tasks the checkpoint passed.
- `diagnose` no longer says "the checkpoint still says `running`" about a dead run.
- The test lives in the binary, so the `[[verify]]` runs `--bin roko`.

What's left:
- serve's summary of a CLI run, whose events go under a second id (bug-c872f6);
- `status.json`'s dead pid in the global `roko plan status`;
- settling the dead invocation when a resume starts (plan steps 3 and 4).
- Until then, the kill matrix's E4 fails on the `cli` rows, where serve summarizes the events id as running
  (`tmp/hermes-roko-v7/roko-recovery/runs/fix-693bb3-96ff7f0e0`).

## Done when

- After a SIGKILL of a CLI run, `roko plan status plans/<p>` and `roko diagnose <p>` say interrupted
  (test `plan_status_reports_a_dead_runs_interruption`).
- Serve's summary of the run (by its one id) says `interrupted`.
- The kill matrix's E4 passes (`harness/check.py --only E4`).

## Notes

- Never signal anything: settling only records.
- Related:
  - bug-dceafd: serve's settle.
  - bug-c872f6: the second id.
  - The roko-recovery p1: what a resume does after a kill.
