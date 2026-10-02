+++
id = "gap-1d8a49"
kind = "gap"
title = "The Graph timeout matrix runs only the shared tree and never compares the stop cause with interrupted_by"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/tests"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "efb9acf44"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "q-1faa0c"
anchors = ["crates/roko-cli/tests/graph_timeout_matrix.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["q-1faa0c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --test graph_timeout_matrix worktree"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T01:38:33Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-02T00:42:12Z"
forced = false
evidence = "Gate 6g on 698ca793d, merged as efb9acf44 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-cli, roko-core, roko-learn and roko-neuro; lib tests pass (roko-cli 3431, roko-core 1986, roko-learn 1234, roko-neuro 239); all eight canaries, golden_path_suite, secret_canary and C2 pass; graph_timeout_matrix 7/7 including the worktree-mode case; bin 445; portal tsc clean and vitest 807/807; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

graph_timeout_matrix pins the shared tree, so nothing exercises the per-task worktree mode, now the default (gap-4ec59f). No case compares the checkpoint's stop cause (`roko.run.stop@1`, gap-fab2cc) with `run.completed.interrupted_by` either.

## Plan

Add a worktree-mode variant of the timeout and SIGTERM cases, and assert the stop cause and interrupted_by agree.

## Done when

- The new cases pass.

## Notes

- Reported on 2026-10-02 by wk-honestbench, working on q-1faa0c, during the overnight close-out round.
- 2026-10-02 (wk-honestbench): implemented on work/bug-730243; cargo verification deferred to the batch check.
  `terminal_projections_agree_in_task_worktrees` (graph_timeout_matrix.rs) runs the timeout and SIGTERM cases with
  `--worktree-per-task`, and checks that each agent ran in a checkout under `.roko/worktrees/` (and that the
  shared-tree cases did not), so a fallback to the shared tree cannot pass unnoticed. `Projections` now also
  holds `run.completed.interrupted_by` and the checkpoint's stop cause (`canonical_stop_cause`, the reader of
  `roko.run.stop@1`); every case expects both to be absent or both to name the same stop: `SIGTERM` for the
  SIGTERM cases and `deadline` for the FAST deadline. `interrupt_settles_when_agent_ignores_sigterm` checks that
  the checkpoint names `SIGTERM`, and that `run.completed`, when the run wrote one before exiting, names it too.
