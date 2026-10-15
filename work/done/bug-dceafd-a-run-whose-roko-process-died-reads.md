+++
id = "bug-dceafd"
kind = "bug"
title = "A run whose roko process died reads running forever: its summary stays active, cancel answers 404, hosts keep its cap held"
status = "done"
triage = "verified"
severity = "p1"
goal = "core"
size = "M"
subsystem = ["roko-serve/runs"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "bd000231d"
source = "tmp/hermes-roko-v5/findings/repro-will.md#F1 (repro-will, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: Will's Mac crashed at 17:02 with roko runs in flight"
anchors = ["crates/roko-serve/src/routes/runs.rs::run_summary", "crates/roko-serve/src/routes/plans/run_control.rs::cancel_plan_run", "crates/roko-serve/src/routes/mcp.rs::cancel_run", "crates/roko-learn/src/telemetry/records.rs::RunClosed", "crates/roko-serve/src/lib.rs::run_server_with_state"]
links = { depends_on = [], blocks = [], related = ["gap-36f3fb", "bug-1d09e8", "bug-06bd18"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_run_whose_process_died_reads_interrupted' crates/roko-serve/ && cargo test -p roko-serve a_run_whose_process_died_reads_interrupted"

[closed]
at = 2026-10-15
at_ts = "2026-10-15T19:53:59Z"
commit = "bd000231d"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "M"
model = "claude-opus-5-5"
forced = false
evidence = "roko-serve interrupted_runs::settle ends a run whose manifest is open and whose last invocation's process is gone (pid free, or recycled: started after the invocation; ps etime tells other users' processes): the manifest closes interrupted with a RunInterruption record (roko-learn), and the run's own index gets run_completed {outcome: interrupted, reason}. Serve settles open runs at start and on summary, detail, dashboard list or cancel of a run it holds no handle of; RunState::Interrupted; cancel answers 200 state interrupted, idempotently; an unknown id still 404s. cargo test -p roko-serve a_run_whose_process_died_reads_interrupted passes; roko-learn 1352 and roko-serve 1022 lib tests plus every integration test pass; clippy -p roko-learn -p roko-serve -D warnings clean. Live: kill -9 of a real serve mid-attempt (gpt-5.4-mini, $0.010), restart: summary interrupted, run_cancel clean twice, the re-run succeeded with the crashed checkout set aside (tmp/hermes-roko-v5/findings/roko-recovery.md)."
+++

## Problem

A run whose roko process dies before the run ends (a machine restart, a kernel panic, `kill -9` of `roko serve`
or of a `roko plan run`) reads as running forever. Its manifest keeps `closed: null`, its event index never gets a
`run_completed`, and serve reports it from neither:

- `GET /api/runs/<id>/summary` (and the `/mcp` tool `run_status`) say `state: running, active: true` with no end;
- `POST /api/plans/<id>/cancel` answers 404 "no active execution for this plan", and the `/mcp` tool
  `run_cancel` answers the same text as an error, so nothing changes;
- a host such as the Hermes plugin keeps the run's clock counting, keeps its `max_usd` cap committed against the
  day's limit and shows "1 running".

Seen by repro-will after the 2026-10-15 17:02 crash (run 46b3d3d7, Will's binary 216f88c84) and reproduced on
main 7b6a5af44 by roko-recovery with a zero-cost harness (`tmp/hermes-roko-v5/roko-recovery/zc/run.sh`): start
`crash-demo` through serve, `kill -9` the serve while T2's agent works, start serve again. The summary still says
`running`/`active: true`, `run_cancel` and the cancel route answer "no active execution for this plan", and the
manifest stays open.

## Why it matters

Goal `core`. After any crash every host shows a run that is not running, holds its budget, and offers a cancel that
fails. With approvals off, a chat model "repaired" roko's state itself and started paid runs (repro-will F3): roko
must close its own runs so nobody else has to.

## Where

- `crates/roko-serve/src/routes/runs.rs::run_summary`: a run with neither an in-memory handle nor a recorded end
  "is still running, as a run another process writes the index of is".
- `crates/roko-serve/src/routes/plans/run_control.rs::cancel_plan_run`: only live or queued in-memory handles.
- `crates/roko-serve/src/routes/mcp.rs::cancel_run`: answers `state: cancelled` from the route.
- `crates/roko-learn/src/telemetry/records.rs::RunClosed`: how a run ended; `RunInvocation` records the owning
  process (`pid`, `started_at`).
- `crates/roko-serve/src/lib.rs::run_server_with_state`: serve start.

## Current state

The run manifest (`.roko/runs/<run>/manifest.json`) records, per invocation, the process that ran it (`pid`,
`host`, `started_at`). `roko_agent::process::process_identity` reads a pid's kernel start time, which tells a
recycled pid apart (the agent pid registry uses the same rule to kill orphaned agents). Nothing reads the
manifest's owner to end a run.

## Plan

1. A run is interrupted when its manifest is open, its last invocation ran on this host, and that process is gone:
   no process has the pid, or the process that has it started after the invocation did (a recycled pid).
   Anything unverifiable counts as alive. Never signal anything.
2. Close such a run: the manifest gets `closed` with `status: interrupted`, the attempt tallies and the reason
   "roko stopped while this run was going (machine restart or crash)"; the run's own event index (only when it
   has one, never the shared hub or `events.jsonl`, as bug-09e21e's refused runs do) gets a `run_completed` row
   with `outcome: interrupted` and the reason, stamped at the run's last sign of life.
3. Serve closes them at start (open manifests, newest first, bounded) and whenever a run's summary or detail is
   read while it has no handle and no recorded end.
4. `RunState::Interrupted` ("interrupted", terminal): the summary says `state`/`verdict: interrupted`,
   `active: false` and `interrupted: {reason, at}`, so a host stops its clock and releases the cap.
5. Cancel of an interrupted run answers 200 with `state: interrupted` (idempotent); `run_cancel` returns that state.
   An unknown id still answers 404.

## Done when

`[[verify]]` passes: a run whose owner process is gone reads `interrupted` (not active, with the reason) from its
summary, cancel of it answers 200 and leaves it interrupted, and serve start closes it. Checked through a real serve
killed with SIGKILL mid-attempt (`tmp/hermes-roko-v5/roko-recovery/`).

## Notes

- roko-checks owns cancel semantics of live runs (bug-1d09e8, bug-7b62ff); this changes only the case where no live
  or queued handle exists and the run's owner is gone.
- A resumed run reopens its manifest (`begin_invocation` clears `closed`); its new events land after the
  interrupted row, and the summary takes the last end.
- gap-36f3fb (record and reconcile an interrupted attempt's workspace) is related but separate.
