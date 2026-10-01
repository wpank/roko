+++
id = "gap-4f3063"
kind = "gap"
title = "Sibling-settle re-verify sees only attempts in the same process and re-runs a failed step once"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w4a-reverify"
anchors = ["crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs::InFlightTasks", "crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs::InFlightTasks::settle_failed_step", "crates/roko-cli/src/workspace_lock.rs::acquire_runner_lock"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_sibling_that_edits_again_after_the_rerun_is_waited_for' crates/roko-cli/ && cargo test -p roko-cli --lib sibling_settle::tests::a_sibling_that_edits_again_after_the_rerun_is_waited_for"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:39Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:00Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

When a verify step fails while sibling tasks are still writing, `sibling_settle` waits for those writers and re-runs the step. The in-flight registry (`InFlightTasks`) is an in-memory map, so two `roko plan run` processes on the same tree do not see each other's running attempts, and a failure caused by the other process's edits is charged to the task. The step is re-run only once: if a sibling's final edit still breaks the build, the task fails (and names the sibling via `blocked_by_sibling`) instead of waiting it out.

Fix: share in-flight state across processes (through the workspace hub or a lock directory), and decide whether to wait for later sibling edits.

2026-09-29: re-verified at d9e79e9d8. Unchanged. The cross-process half only applies if the exclusive runner lock is relaxed: `roko plan run` (commands/plan.rs:597) and serve runs (serve_runtime.rs:834) both take acquire_runner_lock, and a live serve receives CLI runs, so only one plan executor per workspace can run. The single re-run after siblings settle (sibling_settle.rs settle_failed_step) is unchanged.

## Notes

- 2026-10-01 (wk-scheduler): implemented on work/bug-28b604; cargo verification deferred to the batch check.
  `settle_failed_step` now loops while `[gates] sibling_settle_secs`, which bounds the whole settle, lasts. A re-run
  that fails while siblings edit again (a sibling's next attempt, say) waits for them and runs once more. `rerun` is
  now `FnMut`, and blame covers every sibling waited for. Test:
  `sibling_settle::tests::a_sibling_that_edits_again_after_the_rerun_is_waited_for`.
  The cross-process half stays moot: `roko plan run` (`commands/plan.rs:616`), serve runs (`serve_runtime.rs:836`)
  and `roko do` all take `acquire_runner_lock`, so one plan executor runs per workspace. Relaxing that lock would
  need this registry shared first.
