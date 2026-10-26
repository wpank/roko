+++
id = "bug-b7f6a8"
kind = "bug"
title = "A crash with a task in flight makes its plan unresumable: every CLI resume fails closed on the dead attempt's budget hold, and serve's resume (MCP, REST) silently starts the plan over, reruns its passed tasks and still answers that it resumed"
status = "done"
triage = "verified"
severity = "p1"
goal = "core"
size = "L"
subsystem = ["roko-cli/graph_checkpoint", "roko-serve/plans"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "2ca53cb61"
source = "round-v7 roko-recovery kill matrix (tmp/hermes-roko-v7/findings/roko-recovery.md, tmp/hermes-roko-v7/roko-recovery/runs/roko-main-4d22b94e8), the lead's ask"
discovered_from = "agent:roko-recovery (round v7, 4d22b94e8)"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs::validate_cost_ledger_state", "crates/roko-cli/src/graph_checkpoint.rs::GraphCostLedgerCheckpoint::load", "crates/roko-cli/src/graph_checkpoint.rs::prepare_graph_checkpoint_for_run", "crates/roko-cli/src/graph_checkpoint.rs::record_interrupted_attempts", "crates/roko-cli/src/serve_runtime.rs::checkpoint_run_of", "crates/roko-serve/src/routes/plans/run_control.rs::start_plan_run_with", "crates/roko-serve/src/routes/plans/run_control.rs::resume_plan"]
links = { depends_on = [], blocks = [], related = ["bug-8bcd98", "gap-36f3fb", "bug-dceafd", "bug-cbdb6b", "bug-bb6b73"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn resume_settles_a_dead_attempts_budget_hold' crates/roko-cli/src && grep -rqw 'fn a_resume_that_cannot_continue_is_refused_with_its_reason' crates/roko-serve/src && cargo test -p roko-cli --lib resume_settles_a_dead_attempts_budget_hold && cargo test -p roko-serve --lib a_resume_that_cannot_continue_is_refused_with_its_reason"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T20:31:55Z"
commit = "4cfc6be60"
by = "roko-recovery (round v7)"
executor = "claude-agent"
via = "manual"
size = "L"
claimed_at = "2026-10-26T12:05:15Z"
forced = false
evidence = "Merged 5e9af6682 (branch work/bug-b7f6a8; interim a85290f66): the plan budget commits each attempt's hold with every call it admits and records, and a resume under the runner lock settles the holds a stopped process left: finished calls billed, the call in flight estimated, an explicit abandoned verdict (blame harness, failure_class process_stopped, cost billed/estimated) and a learn/costs.jsonl row per dead attempt; a ledger from before holds were kept is released with a warning; serve answers 409 cannot_resume for a resume it cannot continue (only fresh starts over); CheckpointRun gains continues, reason, spent_usd and real passed_tasks, MCP plan_run resumes.left_usd, a budget_carried event, the run summary carried_usd and the CLI report plan_budgets[].carried_usd; the self-model ingest reads an explicit abandoned verdict as no verdict. Follow-up 4cfc6be60 (the lead's heads-up on bb6b73/2a5566): a refused dispatch's calls are marked booked on the hold, so a crash charges each call once with bug-8bcd98 parts 1-2, and the abandoned verdict carries them as refused_usd. Post-merge verify on 4cfc6be60: both greps find the tests; resume_settles_a_dead_attempts_budget_hold, a_resume_that_cannot_continue_is_refused_with_its_reason and a_crash_after_a_refused_dispatch_charges_each_call_once pass; lib suites roko-cli 4045 passed (8 ignored, share test skipped), roko-core 2049, roko-serve 1086, roko-learn 1405 + latency_health_persistence_round_trip failed once under cargo load (a fixed 250 ms wait for a debounced save; 5 of 5 passed on rerun of the same binary). Branch at 85d16339f: nightly fmt; clippy --workspace -D warnings clean. One hold commit costs 0.49 ms (measure_a_hold_commit: 400 commits, 3 holds in flight, no fsync). Evidence (harness, not a verify; tmp/ is local): tmp/hermes-roko-v7/roko-recovery/runs/fix-4f829cc10, every kill point on both paths with cli-resume and serve-mcp-resume: check.py --only E1,E2,E3,E5 passes 88 of 88 (40 of 88 failed on 4d22b94e8, at a, a', b, c' and d3); c-verify's trigger missed (today's gate skips a check the try's hand-off passed, gap-6df2bf), so its row is vacuous. At a' the dead T2#1 settles abandoned at $0.082 ($0.041 billed + $0.041 estimated), only T2 reruns, the plan ledger, attempts.jsonl and learn/costs.jsonl all read $0.287, and serve's resumes says continues, spent $0.164, $1.836 left of $2. Follow-up p3: gap-d0c5d9 (the kill matrix into tools/crash_matrix with a fast CI subset)."
+++

## Problem

If the roko process running a plan dies while a task is in flight, the plan can't be resumed.
- **What "in flight" covers:**
  - It starts at the task's dispatch: its plan-budget hold is taken before its worktree is made.
  - It continues through its agent's model calls and tool calls, to the end of its hand-off check.
  - That is most of a try's wall time.
- **Every CLI way back fails with exit 2:** `roko run plans/<p> --resume-plan`, `roko resume`, and even a plain
  `roko run plans/<p>`. The error is "validate Graph cost ledger …: the checkpoint committed 1000000 unresolved
  reserved micro-USD from an interrupted provider call". It names no way forward, and neither does
  `roko diagnose <plan>`. `roko run` has no `--force-resume`.
- **Every way that works starts the plan over and sets the checkpoint aside (`checkpoint.json.bak.<ms>`).** That
  covers:
  - serve's MCP `plan_run {resume: true}`;
  - `POST /api/plans/<p>/resume`, which answers HTTP 202 `"resumed": true`;
  - `POST /api/plans/<p>/execute {"resume": true}`;
  - `roko plan run --resume-plan --force-resume` and `--fresh`.
- **Passed tasks run again.**
  - The MCP answer still names the dead run under `resumes`, with `resumable: false, passed_tasks: [],
    tasks_left: []`.
  - Only serve's log says "set aside a Graph checkpoint to start the plan over".
- **The dead try's spend is in no ledger:** not the plan's, not the daily `learn/costs.jsonl`, and not the new
  run's cap. Only its `call_cost` events hold it.

It was found with a $0 kill matrix in round v7: `tmp/hermes-roko-v7/roko-recovery/` (`run_matrix.sh`,
`harness/check.py`; findings in `tmp/hermes-roko-v7/findings/roko-recovery.md`). Results on 4d22b94e8, CLI-run and
serve-run crashes alike:

| Kill point | Plan ledger at the kill | CLI resumes | serve `resume: true` / REST resume |
|---|---|---|---|
| a: T1's model stream | spent $0, reserved $1.00 | exit 2 | start over |
| a': T2's model stream, T1 passed | $0.082 / $1.00 | exit 2 | start over, **T1 runs again** |
| b: a bash tool call | $0 / $1.00 | exit 2 ("interrupted provider call", though no call ran) | start over |
| c': the hand-off check (`run_tests`) | $0 / $1.00 | exit 2 | start over |
| d3: T1 committed, T2 dispatched | $0.082 / $1.00 | exit 2 | start over, **T1 runs again** |
| c: the gate's verify after the hand-off | $0.082 / $0 | resume | resume |
| d: ~0.1 s between a failed verdict and the retry | $0.164 / $0 | resume | resume |
| e, e': delivery | $0.164 / $0 | resume | resume |

Kill points d' and d'' (T1 passed, its output not yet committed) resume, but T1 runs again. That is by design: an
uncommitted Activity is set aside.

## Why it matters

- **Goal core: "plan runs work reliably".**
  - Phase 4.1's run 6059db44 resumed after the 2026-10-26 reboot only because the reboot landed in its gate verify.
  - A reboot 3–17 minutes earlier would have landed in T1#1's 15-minute `run_tests`. The CLI would have stranded
    it, and the chat's resume would have redone the phase.
- A multi-task phase (phase 3 took 54 min) loses every passed task to a crash, and a person who asked to resume is
  told it resumed.
- **Goal truth:**
  - The REST route answers `resumed: true` for a run that starts over.
  - The MCP answer's `resumes` names a run it doesn't continue.
  - The dead try's spend escapes the plan cap and the daily cap.
- The killed-try part of bug-8bcd98 (no verdict or cost row for a try killed mid-call) is done here, by settling the
  dead attempt on resume. bug-8bcd98 keeps the failed-call rows and the refused calls in `billed_usd` (roko-speed).

## Where

- `crates/roko-cli/src/graph_checkpoint.rs`:
  - `validate_cost_ledger_state` bails on `reserved_micro_usd > 0`. A test pins this:
    `unresolved_crash_reservation_is_rejected_without_force`.
  - `GraphCostLedgerCheckpoint::load` takes the committed spend and reservation from the manifest.
  - In `prepare_graph_checkpoint_for_run`, the `Err(_) if force_resume` branch sets the checkpoint aside and starts
    fresh. Serve's `resume: true` maps to `force_resume`.
  - `record_interrupted_attempts` (gap-36f3fb's partial) already finds the attempts a stop cut off.
- `crates/roko-cli/src/graph_task_dispatch/budget.rs::try_reserve` takes the attempt hold and persists it.
  `::settle` releases it when the agent returns.
- `crates/roko-cli/src/serve_runtime.rs::checkpoint_run_of` blanks `passed_tasks` and `tasks_left` when the preview
  is not `Resume`.
- `crates/roko-serve/src/routes/plans/run_control.rs`:
  - `start_plan_run_with` starts the run whatever the preview says;
  - `resume_plan` answers `"resumed": true` regardless.
- `crates/roko-serve/src/routes/mcp.rs`: `plan_run`'s answer.

## Current state

- Resume is fail-closed by design when the ledger holds a reservation. `--force-resume`'s only remedy is to start
  over.
- bug-dceafd (done) settles a dead serve run's manifest as `interrupted`.
- gap-36f3fb's partial records the interrupted attempts in `roko.attempt.interrupted@1` and warns.
- Nothing settles the dead attempt itself:
  - its `attempt_open` line never gets a verdict;
  - the manifest counts it `abandoned`;
  - `learn/costs.jsonl` never gets its row.

## Plan

1. **Resolve a dead run's hold on resume.** The resume holds the runner lock, so the earlier process is gone.
   - Make the holds durable (the approved design; it replaced reading `call_cost` rows, which a kill loses from the
     run's event index): with each call it admits and each it records, the plan budget commits its holds to
     `costs.json` and the manifest's `CommittedGeneration.holds`. Each hold records its attempt, what its finished
     calls cost, and the call in flight at its estimate (the attempt's last call, else the plan's costliest).
   - In the resume path, a committed `reserved_micro_usd` > 0 no longer fails. Instead:
     - add each dead hold's spend and its call in flight to `spent`, as billed and estimated;
     - release the rest of the hold;
     - persist the ledger, then resume as at kill point c.
     - A ledger from before holds were kept has its reservation released, its dead spend unknown, with a warning.
   - Settle each dead attempt in the records:
     - an `abandoned` verdict row in `attempts.jsonl` with that cost, and `failure_class`/`blame` saying the process
       died;
     - its `learn/costs.jsonl` row;
     - the cost in `roko.attempt.interrupted@1`.
   - A live holder of the runner lock keeps today's refusal.
2. **Serve never silently starts over.**
   - When `resume` is set and the checkpoint's preview action is not `Resume`, `start_plan_run_with` answers 409 with
     the reason, unless `fresh: true`. The same goes for the MCP `plan_run` tool, `POST …/execute {"resume": true}`
     and `POST …/resume`.
   - `resumes` carries the real `passed_tasks` and `tasks_left`, with `continues`, `reason` and `spent_usd`; the MCP
     answer adds `left_usd`, and the run summary `carried_usd`.
   - `resume_plan` answers `resumed: true` only when it continued the run.
3. **Interim, as its own small commit first:** the reservation error and `diagnose` name the commands that work today,
   `roko run plans/<p> --fresh` (it redoes passed tasks) and `roko plan run plans/<p> --resume-plan --force-resume`.
4. **Commit the kill matrix as a regression test**, as a follow-up item (p3): move it from `tmp/` to
   `tools/crash_matrix/`, with a fast subset CI could run. Until then the harness check is evidence, not a
   `[[verify]]`, since `tmp/` is local.

What a cap on a resume means (the lead's question): the whole run, dead and resumed together.
- The person's words in the live case were "keep its $1 cap for the whole run".
- The plan ledger already enforces it this way when the dead try's spend was committed (kill point c): a $0.27 resume
  stops for budget where a fresh run finishes, on the CLI and serve alike.
- The answer and the summary should say so: "$0.022 spent before the restart, $0.978 of $1.00 left". Today the
  summary shows only the new run's cost.

## Done when

- A resume after a kill in flight continues the run. The dead attempt is settled `abandoned` with its cost, and the
  plan ledger holds that cost.
- A serve or REST resume that can't continue answers 409 with the reason.
- Test `resume_settles_a_dead_attempts_budget_hold` (roko-cli) and test
  `a_resume_that_cannot_continue_is_refused_with_its_reason` (roko-serve) pass.
- Evidence, not a `[[verify]]`: `harness/check.py --only E1,E2,E3,E5` passes at every kill point, on both paths,
  with the fixed binary (`ROKO_BIN=target/debug/roko`).

## Notes

- **What a hold commit costs:** 0.49 ms, measured by `measure_a_hold_commit` (roko-cli, `--ignored`) over 400
  commits with 3 holds in flight, on the branch at 4f829cc10, on the round's Mac under cargo load.
  - Each call an attempt admits or records writes `costs.json` and the manifest, each to a temp file renamed in
    place, under the plan ledger's lock. That lock serializes the commits of parallel attempts.
  - A model call takes seconds, so two commits per call cost well under 0.1% of it.
  - bug-4026d8's fsync will lengthen each commit, and the lock holds while it runs. Measure again then: if a synced
    commit runs to milliseconds on a 3-wide run, commit the holds off the lock (the latest state wins) rather than
    making a call wait.
- **The rule a crash settles by, with bug-8bcd98 and the bench's bug-bb6b73 and bug-2a5566:** each call is booked
  once, at what is known of it. An estimate is used only for the call in flight, whose outcome nothing knows.
  - bug-8bcd98 part 1: the tool loop records a call that failed, at what it streamed, else $0. That ends the
    hold's call in flight, so a crash after it charges no estimate.
  - Part 2: failover books a refused dispatch on the plan. The hold then marks its calls so far as booked
    (`booked_micro_usd`), so a resume charges only the calls since, and the `abandoned` verdict carries the booked
    part as `refused_usd`.
  - Test: `a_crash_after_a_refused_dispatch_charges_each_call_once` (roko-cli).
- The pilot's live resume (round v7, `tmp/hermes-roko-v7/findings/hermes-pilot.md`, 11:09–11:25) matches kill point c.
- Reproduce one kill point at zero cost:
  `cd tmp/hermes-roko-v7/roko-recovery && python3 harness/matrix.py --scenarios a-stream-t2 --paths serve --out runs/dev && python3 harness/show.py runs/dev/a-stream-t2-serve`.
- Related:
  - bug-8bcd98: the books; the killed-try part is here.
  - gap-36f3fb: what a resumed try does with the dead try's edits.
  - bug-dceafd: serve settles the manifest.
  - bug-cbdb6b: a plain plan_run over a resumable run.
  - bug-bb6b73: a SIGTERM'd task's reservation in a bench ledger.
