+++
id = "gap-9084e7"
kind = "gap"
title = "Plan-Scoped Run Lifecycle and Humane Resume UX"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/runner"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/327-plan-scoped-run-lifecycle-ux.md#327 — Plan-Scoped Run Lifecycle and Humane Resume UX"
discovered_from = "audit:tmp/backlog/archive/327-plan-scoped-run-lifecycle-ux.md#327 — Plan-Scoped Run Lifecycle and Humane Resume UX"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs::DEFAULT_RUNNER_RESUME_PATHS", "crates/roko-cli/src/commands/plan.rs::cmd_plan_run_engine"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
evidence = "Premise gone: plan runs no longer open the Runner-v2 singleton .roko/state/state-snapshot.json (Runner-v2 deleted in 6b5da8616; Graph runs use per-plan checkpoints under .roko/state/graph/; source marked Implemented 2026-09-04). The merged DF-0925 P3-4 sub-bug (bare --resume-plan on multi-plan runs) is fixed in the working tree (uncommitted): graph_checkpoint.rs DEFAULT_RUNNER_RESUME_PATHS maps the clap default .roko/state/state-snapshot.json to the canonical checkpoint root (test near graph_checkpoint.rs:1386)."
+++
normal repeated plan execution currently requires knowledge of an internal recovery flag and one bad run can disrupt another. `roko plan run <selection>` currently opens the workspace singleton `.roko/state/state-snapshot.json` on every Runner-v2 invocation. This is implicit even when the user did…

Imported without verification from:
- `tmp/backlog/archive/327-plan-scoped-run-lifecycle-ux.md#327 — Plan-Scoped Run Lifecycle and Humane Resume UX`
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Missing`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P3-4. Bare `--resume-plan` is broken for multi-plan runs`

Warning: every file this item cites is gone (`.roko/state/state-snapshot.json`, `roko/attempt/*`) — likely obsolete or moved.

How to verify: Check: Running a plan after any terminal run starts new work without `--fresh` and without restoring; Killing a run after a durable checkpoint and invoking the same command resumes the exact; A clean or corrupt run for plan A cannot block plan B. [evidence: own status: Implemented (2026-09-04) -- RunId type in roko-core/run_id.rs, plan-scoped lifecycle wired; 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): L | engine DAG /…] / Check resume/--fresh/--new semantics for multi-plan runs.

Merged 2 mined candidates: m1-087, m4-165.

Verified 2026-09-28: closed as superseded; see [closed].evidence.
