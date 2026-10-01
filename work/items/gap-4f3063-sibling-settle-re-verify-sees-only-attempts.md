+++
id = "gap-4f3063"
kind = "gap"
title = "Sibling-settle re-verify sees only attempts in the same process and re-runs a failed step once"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w4a-reverify"
anchors = ["crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs::InFlightTasks", "crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs::InFlightTasks::settle_failed_step", "crates/roko-cli/src/workspace_lock.rs::acquire_runner_lock"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_sibling_that_edits_again_after_the_rerun_is_waited_for' crates/roko-cli/ && cargo test -p roko-cli --lib sibling_settle::tests::a_sibling_that_edits_again_after_the_rerun_is_waited_for"
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
