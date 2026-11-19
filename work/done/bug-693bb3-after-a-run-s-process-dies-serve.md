+++
id = "bug-693bb3"
kind = "bug"
title = "After a run's process dies, serve's plan rows still say running: the plan list, the plan detail and resumable_run.status read the Graph checkpoint's stale running and tasks.toml's tasks_done 0, so a chat host tells the person the plan is still running and never offers the resume"
status = "done"
triage = "verified"
severity = "p1"
goal = "core"
size = "M"
subsystem = ["roko-cli/plan", "roko-cli/serve_runtime", "roko-serve/runs"]
created = 2026-10-26
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "9c8c78abb"
source = "round-v7 hermes-pilot P-51 window (2026-10-26 22:33, scratch serve :6697, roko-main-2ca53cb61), the lead's ask"
discovered_from = "agent:roko-recovery (round v7)"
anchors = ["crates/roko-cli/src/plan.rs::overlay_graph_checkpoint_status", "crates/roko-cli/src/serve_runtime.rs::checkpoint_run_of", "crates/roko-serve/src/interrupted_runs.rs"]
links = { depends_on = [], blocks = [], related = ["bug-b7f6a8", "bug-df0e79", "bug-dceafd", "bug-f6cd5a", "bug-4fadf8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_dead_runs_plan_reads_interrupted_everywhere' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_dead_runs_plan_reads_interrupted_everywhere"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T01:47:31Z"
commit = "9f7a84116"
by = "roko-recovery (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged 9f7a84116 (branch work/bug-693bb3 at 96ff7f0e0, with bug-f6cd5a). Post-merge verify at 9f7a84116 in my worktree: the grep finds the test and a_dead_runs_plan_reads_interrupted_everywhere passes (the plan list, the plan detail and resumable_run as the run goes dead, live, failed). Branch at 96ff7f0e0: lib suites roko-cli 4054, roko-serve 1087, roko-learn 1411, roko-core 2049 passed, none failed; roko-cli cli_fallback 26/26 and tui_tabs 26/26 (plan list, TUI); nightly fmt; clippy --workspace -D warnings clean; also a_run_without_a_manifest_runs_while_the_runner_lock_is_held, passed_tasks_count_a_task_once_across_a_resume and a_run_has_stopped_when_its_process_is_gone_or_it_closed. $0 kill matrix (tmp/hermes-roko-v7/roko-recovery/runs/fix-693bb3-96ff7f0e0, binary bin/roko-693bb3-96ff7f0e0): new check E7 (after the crash serve's plan rows say interrupted with the committed passed tasks) 22/22, E1/E2/E3/E5 88/88; E4 (bug-df0e79's CLI plan status, bug-c872f6) and E6 (bug-4fadf8, c-verify vacuous) stay with their items. Live: the pilot's second P-51/P-53 pass (10-27 02:39-02:45, chat with all models fake, the fenced serve on roko-693bb3-96ff7f0e0) passed: recov read 'interrupted (3s ago)' with T1 kept and $0.205 carried and resumed on the first ask; fails resumed with $0.492 carried; the serve-side watcher (p51/p51-4f829cc10/watch.jsonl) matches every turn."
+++

## Problem

After the process running a plan dies, every plan row in `roko serve` still says the plan is running:
- `GET /api/plans`, `GET /api/plans/{id}` and the plan detail's `resumable_run.status` all read `status: "running"`;
- `tasks_done` is 0, although a task passed.
- Only the run summary (`GET /api/runs/{id}/summary`) says `interrupted`.

**Seen 2026-10-26 22:33, the pilot's P-51 window** (round v7, the scratch serve on :6697, roko-main-2ca53cb61):
- Plan `recov`'s run 0a7e3e7b had been SIGKILLed with T1 passed and T2 mid-call.
- The chat believed the plan rows twice: "recov is already running; there isn't a paused run to resume right now".
- It never offered to resume. The plan then ran again from scratch as 30d43e0e, setting the dead run's progress aside
  and redoing T1.

This is Will's reboot case: after a restart, the chat says his plan is still running.

## Why it matters

Goals truth and core ("plan runs work reliably"):
- bug-b7f6a8 made a crash resumable, but a host that reads the plan rows never offers the resume, and the person is
  told their plan is still running.
- The rows are the first thing a chat host, the portal and the dashboard read.

## Where

- `crates/roko-cli/src/plan.rs::overlay_graph_checkpoint_status`:
  - it maps a Graph checkpoint's `status` onto the plan rows, and a SIGKILL leaves that at `running`;
  - it counts `tasks_done` from `tasks.toml`, which a Graph run never updates.
  - Serve's plan list and plan detail (`serve_runtime.rs::list_plans`, `::load_plan_summary`), the TUI dashboard and
    `roko plan list` all use it.
- `crates/roko-cli/src/serve_runtime.rs::checkpoint_run_of`: `resumable_run.status` is the checkpoint's `status`.
  The run counts as stopped only while no process holds the workspace's runner lock, even when another plan holds it.
- `crates/roko-serve/src/interrupted_runs.rs`: the test whether a run's process is gone (bug-dceafd). Serve uses it
  to settle the run manifest and summary, but never the checkpoint's view.

## Plan

1. **A run's liveness from its own manifest.** A `running` checkpoint's run has stopped when its run manifest is
   closed, or names a process that is certainly gone (`interrupted_runs`' test, read-only). Without a manifest, it has
   stopped when no process holds the workspace's runner lock.
2. **The overlay and `checkpoint_run_of` report such a run `interrupted`.** A live run, including a resumed one
   whose manifest it reopened, stays `running`. Cancelled and failed keep their state.
3. **`tasks_done` counts the tasks whose output the checkpoint committed with a passing verdict**, read from the
   Activity log (`inspect_canonical_checkpoint`), since a crashed run's manifest has no verdict summary.
4. **Fold in bug-f6cd5a:** the summary's `carried_usd` folds from 0.0.

## Done when

- A killed run's plan reads `interrupted` in the plan list, the plan detail and `resumable_run.status`, with
  `tasks_done` 1 of 2 (test `a_dead_runs_plan_reads_interrupted_everywhere`).
- A live run's plan still reads `running`, and failed and cancelled plans keep their state.
- On the $0 kill matrix (`tmp/hermes-roko-v7/roko-recovery`), the new check E7 passes at every kill point on both
  paths: after the crash, the plan rows say `interrupted` with the committed passed tasks. E1, E2, E3 and E5 still
  pass. E4 and E6 stay with bug-df0e79 and bug-4fadf8.
