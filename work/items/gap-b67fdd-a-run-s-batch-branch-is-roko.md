+++
id = "gap-b67fdd"
kind = "gap"
title = "A run's batch branch is roko/batch/<run-uuid>, so every merge command roko prints (the terminal's ending, roko status, the portal's Ready to merge, the plugin's merge dialog) shows a UUID; name it from the run's plans, roko/work/<slug>, with -2 only on a clash"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/graph_execution", "roko-serve"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round v8 lead's ask (terminal output overhaul, wave 2 item 1); round v8 roko-recovery captures (tmp/hermes-roko-v8/roko-recovery/runs/item5b/plan-status.*)"
discovered_from = "agent:roko-recovery (round v8)"
anchors = ["crates/roko-cli/src/graph_execution/batch.rs::BatchIntegration::open", "crates/roko-cli/src/graph_execution/batch.rs::resumed_batch_run", "crates/roko-serve/src/routes/run_result.rs::result_range"]
links = { depends_on = [], blocks = [], related = ["gap-2c87f0", "gap-5043c6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_batch_branch_is_named_from_its_plans' crates/roko-cli/src && grep -rqw 'fn a_run_result_names_the_recorded_batch_branch' crates/roko-serve/src && cargo test -p roko-cli --lib a_batch_branch_is_named_from_its_plans && cargo test -p roko-serve --lib a_run_result_names_the_recorded_batch_branch"
+++

## Problem

A run's batch branch is `roko/batch/<run-id>`, and the run id is a UUID. Every merge command roko prints shows it:
- the terminal's ending: `Next  git merge --ff-only roko/batch/graph-dd2189fe-7e1b-4149-9689-9144b4f2886e`;
- `roko status` and `roko plan status`;
- the portal's "Ready to merge" line, through serve's run result;
- the Hermes plugin's merge dialog.

The design system's never-on-screen list (`$V8/design/roko-design-system.md` §6) bars run UUIDs. A person can't read
the branch aloud or recognise it in `git branch`.

`roko/plan/<plan>` can't be used: it is already the per-plan work branch (gap-3b5361). Accepted tasks land there, the
batch merges it, and the GitHub integration's PR branches and `roko github status` use it.

## Why it matters

Goal tooling. The merge command is the last thing a run says and the one thing the person must type. Round v8's
lead asked for it (terminal output overhaul, wave 2 item 1).

## Where

- `crates/roko-cli/src/graph_execution/batch.rs`:
  - `BatchIntegration::open` builds the name from the run id;
  - `resumed_batch_run` parses the run id back out of a recorded branch name.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`, which opens the run's batch.
- `crates/roko-serve/src/routes/run_result.rs::result_range`, which builds `roko/batch/{run_id}` again rather than
  reading the branch the run recorded.
- Outside roko: the Hermes plugin (`workspace.py` BATCH_PREFIX, `rundata.py`, `runwork.py`, `views.py`,
  `plugin.js` `shortBranch`) and the portal's contract notes.

## Plan

1. Name a new run's batch from its plans:
   - one plan: its id;
   - several: their directory's name;
   - a one-task run: its request, as a slug.

   The branch is `roko/work/<slug>`, with `-2`, `-3` and so on only when that branch already exists.
2. The run id stays the batch's identity everywhere else: receipts, the batch record's `run_id`, the
   `roko/run/<run-id>` tag and the parked delivery ref.
3. A resumed run continues the branch its batch record names, with that record's run id, rather than deriving
   either from the other. Old records (`roko/batch/<run-id>`) resume as before.
4. serve's run result reads the branch from the run's batch records, falling back to `roko/batch/<run-id>` for runs
   recorded without one.
5. Tell plugin-wow-v8 and portal-v8 which fields to read instead of building the name.

## Done when

- A new run's merge command reads `git merge --ff-only roko/work/<slug>`.
- A second run of the same plan, while the first branch exists, gets `roko/work/<slug>-2`.
- A resumed run continues its own branch.
- serve's run result names the recorded branch.
- plan_branch_integration passes, and the `[[verify]]` passes.
