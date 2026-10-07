+++
id = "bug-11556f"
kind = "bug"
title = "The portal shows the new 'interrupted' task outcome as unknown"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["portal"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "efb9acf44"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-60ccba"
anchors = ["apps/portal/src/lib/runState.ts", "apps/portal/src/lib/taskRows.ts"]
lane = "frontend"
links = { depends_on = [], blocks = [], related = ["bug-60ccba"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'its own label (bug-11556f)' apps/portal/src/lib/taskRows.test.ts && cd apps/portal && npx vitest run src/lib/taskRows.test.ts src/lib/runState.test.ts src/lib/glyphs.test.ts"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T01:38:29Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-02T00:41:46Z"
forced = false
evidence = "Gate 6g on 698ca793d, merged as efb9acf44 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-cli, roko-core, roko-learn and roko-neuro; lib tests pass (roko-cli 3431, roko-core 1986, roko-learn 1234, roko-neuro 239); all eight canaries, golden_path_suite, secret_canary and C2 pass; graph_timeout_matrix 7/7 including the worktree-mode case; bin 445; portal tsc clean and vitest 807/807; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

bug-60ccba added the task outcome `interrupted` (TASK_OUTCOME_INTERRUPTED), but the portal's run-state mapping doesn't know it, so it renders as an unknown outcome.

## Plan

Map `interrupted` as a failed (not passed) outcome with its own label, and add a vitest case.

## Done when

- The portal tests pass, and an interrupted task shows its label.

## Notes

- Reported on 2026-10-02 by wk-streams, working on bug-60ccba, during the overnight close-out round.
- 2026-10-02 (wk-streams): implemented on work/gap-b35a57; vitest and tsc deferred to the gate's portal checks.
  `interrupted` is now its own `TaskStatus`, terminal and counted in `tasksFailed`, with its own glyph (`↯`, red)
  and label. The live `run_completed` fold ends a running task as the server does: cancelled when the run was
  cancelled, else interrupted and counted as failed, so a reload matches. An interrupted row takes focus like a
  failed one. Tests: `taskRows.test.ts` (live and reloaded rows), `runState.test.ts`, `glyphs.test.ts`.
