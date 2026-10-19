+++
id = "bug-cbdb6b"
kind = "bug"
title = "A plain plan_run over a failed run's resumable checkpoint silently archives it (checkpoint.json -> .bak.<ms>): no warning, nothing in the result, no way back from chat, so a 'resume' without the resume flag throws away the passed tasks' progress"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph-checkpoint"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-hermes-pilot#C-p0-resume"
discovered_from = "agent:hermes-pilot (round v7 phase 0, runs 6988f0b8 -> 5ebd3310)"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs::archive_checkpoint_files"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_chat_plan_run_over_a_resumable_failed_run_is_refused_with_the_resume_hint' crates/ && cargo test -p roko-serve -- a_chat_plan_run_over_a_resumable_failed_run_is_refused_with_the_resume_hint"
+++

## Problem

Round v7, Undertow phase 0, `~/dev/undertow` (roko f33fd217c, plugin 0.6.0).

- Run `6988f0b8` ended `failed`, with T1 and T2 passed and T3 failed. Its Graph checkpoint
  (`.roko/state/graph/p0-undertow-foundation/checkpoint.json`) was resumable: a resume would have run only T3.
- The person asked in chat "Use roko to resume the plan p0-undertow-foundation: only T3 is left". The chat model
  called `roko_plan_run(plan_id, max_usd=2)` without `resume: true`.
- roko started a **fresh** run (`5ebd3310`, `resumed: false`) at T1. As it started, `archive_checkpoint_files`
  renamed the resumable checkpoint to `checkpoint.json.bak.1792422172343`, along with `activities.jsonl` and
  `costs.json`.
- The tool result said only "running". Nothing in it, on the page or in the run's events said that run
  `6988f0b8`'s progress (2 passed tasks, 6 tries, $0.65 of work) had been set aside.
- The fresh run was then cancelled, and its interrupted checkpoint is now the plan's current one. So a correct
  "resume" from chat would now start at T1 too. The only way back to run 1's progress is hand-copying the `.bak`
  files, or a terminal `roko plan run … --resume-plan <.bak file>`, which a chat host can't do.

## Why it matters

A person who asks to continue loses the passed work and pays again for tasks that already passed. Here that's about
$0.25 and 25 minutes for T1 and T2. It happens silently, and only because one optional flag was left out. A
plan-level checkpoint is the most expensive state roko keeps, so discarding it shouldn't take less than a
confirmation.

## Where

- `crates/roko-cli/src/graph_checkpoint.rs::archive_checkpoint_files`: a fresh run renames the plan's checkpoint
  files to `.bak.<ms>`.
- The serve and MCP `plan_run` path that starts the fresh run, and its result: nothing in it names the archived run.

## Plan

1. When a `plan_run` without resume finds a resumable checkpoint (a failed or interrupted run whose tasks partly
   passed):
   - from a chat host: refuse it, unless the call says `fresh: true` or `resume: true`, and name the resumable run
     ("run 6988f0b8 can be resumed: T1, T2 passed; pass resume=true to continue, or fresh=true to start over");
   - from a person at a terminal: say so before starting.
2. Whenever a run does start over, say what it archived, in the result and in an event: the run id, its passed
   tasks, and the `.bak` name.
3. Let a resume take an archived checkpoint by run id (`roko resume <run-id>`, and MCP `plan_run` with `resume`
   plus `from_run`).

## Done when

A chat-host `plan_run` over a resumable failed run is refused with the resume hint, and a fresh run reports what it
archived. Checked by the test in `[[verify]]`.

## Notes

- Evidence:
  - `~/dev/undertow/.roko/state/graph/p0-undertow-foundation/` (the `.bak.1792422172343` files are kept as
    evidence);
  - the runs `6988f0b8`, `5ebd3310` and `e578cf71`;
  - `tmp/hermes-roko-v7/findings/hermes-pilot.md` §C.
- Related: the plugin side, that the chat didn't use `resume` and wasn't told, is the plugin's P-14 (routed to
  plugin-wow).
