+++
id = "bug-e0f472"
kind = "bug"
title = "Concurrent Graph verify runs lose gate-threshold updates: gate-thresholds.json is loaded and saved without a lock"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/graph-dispatch", "roko-cli/runner"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:feat/learning-verify-loops 99adacd6d"
anchors = ["crates/roko-cli/src/graph_task_dispatch/gate_learning.rs::GraphTaskDispatcher::settle_gate_learning", "crates/roko-cli/src/runner/persist.rs::GateThresholds::update_locked", "crates/roko-cli/src/graph_task_dispatch/verification.rs::GraphTaskDispatcher::settle_task_verification", "crates/roko-cli/src/runner/persist.rs::GateThresholds::load_or_default", "crates/roko-cli/src/runner/persist.rs::GateThresholds::save", "crates/roko-fs/src/atomic.rs::with_locked_json_transaction"]
links = { depends_on = [], blocks = [], related = ["reg-c7ecf6", "find-4b4344", "gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn concurrent_verify_updates_keep_every_gate_observation' crates/roko-cli/src && cargo test -p roko-cli --lib concurrent_verify_updates_keep_every_gate_observation"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:27Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:09Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

After each task's verify steps, Graph dispatch updates `.roko/learn/gate-thresholds.json` with an unlocked
read-modify-write:
1. `GateThresholds::load_or_default` (`graph_task_dispatch.rs:2453`);
2. `observe_verify_steps` (:2467);
3. `save` (:2477).

Tasks of a plan run concurrently, up to `max_parallel`, on a multi-thread runtime (`main.rs:3325`, `:3451`). Plans
also run concurrently (`max_parallel_plans`) and share the file, as do separate `roko plan run` or `roko serve`
processes in one workspace. When two tasks finish verify close together, both load the same state and the second
save discards the first task's observations. `save` goes through `atomic_write`, so the file is never torn, but the
last writer wins.

Expected: every verify step's observation reaches the EMA.

## Why it matters

Goal `learning`. Since `99adacd6d`, these EMAs set the retry budget of every task that does not author
`max_retries` (`retry_budget.rs`). They also feed the oracle residual, the dashboard and `roko status`. A lost
update skews all of them, and it happens most often in parallel plans. `ebf274ada` fixed the same kind of lost
update for `experiments.json`, which parallel attempts also write.

Related items:
- `reg-c7ecf6`: honour `learning.gate_threshold_flush_interval` at this same save;
- `gap-7a3527`: this writer also ignores `[gates] ema_alpha`, because `GateThresholds::observe` hard-codes alpha
  0.1 (`persist.rs:337-346`). Wiring it is step 1 of gap-7a3527, so it is not repeated here;
- `find-4b4344`: the other gate-threshold closures.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs:2438-2500`: the "P2-LRN-6 Loop 1" block in the Graph verify path.
  Its comment ("a missed flush is non-fatal; the next task will attempt its own update") does not cover lost
  observations.
- `crates/roko-cli/src/runner/persist.rs`:
  - `GateThresholds::load_or_default` (:505) and `save` (:496);
  - `observe` (:337) and `observe_verify_steps` (:378).
- `crates/roko-fs/src/atomic.rs::with_locked_json_transaction` (:55): the existing locked read-modify-write
  helper. `roko-learn/src/prompt_experiment.rs:687` and roko-agent's immune ledgers already use it.

## Current state

Checked at `33e107da1`. There is no in-process mutex and no file lock around the block, and it is the only writer
on the Graph path (`plan_runner.rs:1081` sets the path). Not reproduced live. The window is short: a JSON read,
an update and an atomic write.

## Plan

1. Wrap load, observe and save in `roko_fs::with_locked_json_transaction::<GateThresholds, ..>` on the thresholds
   path. Call `fill_default_rungs` inside the transaction, as `load_or_default` does. Move the block into a small
   testable function, for example `GateThresholds::update_locked(path, |t| t.observe_verify_steps(..))`.
2. Keep the TUI notification (`gate_thresholds_updated`) with the state the transaction saved.
3. If `reg-c7ecf6` is done at the same time, batch observations in memory and flush them through the same locked
   transaction.
4. Test `concurrent_verify_updates_keep_every_gate_observation`: N threads each record one observation on one
   path, and the final `total_count` of that rung is N.

## Done when

- Concurrent verify updates keep every observation.
- The `[[verify]]` command passes.

## Notes

- `find-4b4344` asks not to add a second writer for `gate-thresholds.json`. Change this writer in place.
- Readers (`TaskRetryBudgets::load`, TUI, serve) read without the lock. `atomic_write`'s rename keeps each read
  consistent, so they need no change.
- 2026-10-01 (wk-honestbench): implemented on work/bug-730243; cargo verification deferred to the batch check.
  At BASE the block had moved to `gate_learning.rs::settle_gate_learning`, behind an in-process mutex only (no
  file lock across processes). New `GateThresholds::update_locked` (persist.rs) runs load, `fill_default_rungs`,
  update and save in `roko_fs::with_locked_json_transaction`; `settle_gate_learning` uses it, with the ratchet
  loaded and saved inside the same lock, and tells the dashboard the state as saved. A malformed thresholds file is
  now reported and left as it is instead of being replaced with defaults. Test:
  `concurrent_verify_updates_keep_every_gate_observation` (16 threads, one observation each).
