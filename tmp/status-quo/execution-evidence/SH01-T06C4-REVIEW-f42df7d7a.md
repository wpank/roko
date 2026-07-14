# SH01-T06C4 independent review — f42df7d7a

Assignment:
- Candidate: `f42df7d7ab105f4f401bc1cc7cedab0777ca0775`
- Exact base: `fc831c5542950470808ed876a29f4f841e7cd936`
- Review branch/worktree: `review/SH01-T06C4-f42df7d7a` at `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/reviews/SH01-T06C4-f42df7d7a`
- Reviewed task: `SH01-T06C4` in `tmp/status-quo/self-heal/plans/SH01-runner-lifecycle/tasks.toml`
- Review scope: full four-file candidate diff plus the timeout ownership, persistence, resume, DAG, state, and projection call paths.

Independent reconstruction:
- A timeout terminal must be durable and exact before publication, have one ownership winner against completion, remain idempotent for stale or duplicate activity, and rebuild the same lifecycle, failed-DAG, and consumer projection state after restart.
- A normal resume must work both when the timeout ledger is newer than the last snapshot and when the last snapshot already contains the `TimedOut` lifecycle status. Wall-clock audit data must never become a process-local monotonic clock.

Changed-path review:
- The exact ownership claim is the completion-versus-expiry linearization point. Cleanup precedes the fsynced typed ledger append; claim completion precedes timeout publication. The candidate rechecks snapshot candidates before cancellation and bounds sibling drainage to the initial owner snapshot.
- The runtime timeout serde shape matches the prior runner JSON shape, exact keys include run/plan/task/attempt, identical duplicates are no-ops, and conflicting facts fail closed.
- Live and ledger-reconstructed timeout events use the same projection normalization path. No wall timestamp is converted into `Instant`/`MonotonicTime`.
- One restart path is nevertheless incorrect, as described below.

Adversarial finding:

## High — an already-TimedOut resume loses failed-DAG identity and can redispatch the task

Reproduction by production-path trace:
1. Let exact task attempt `plan/T1/1` time out normally. The typed ledger is appended and the timeout event makes the lifecycle attempt `TimedOut`; a later ordinary snapshot therefore contains that terminal lifecycle and `tasks_failed` count.
2. Resume from that snapshot. `restore_state_from_resume_snapshot` restores the aggregate count and lifecycle at `event_loop.rs:4879-4889`, but `RunStateSnapshot` has no `failed_tasks` IDs or `failure_reasons` fields (`persist.rs:95-145`), so the new in-memory failed-task map is empty.
3. `replay_timeout_terminals` observes `Some(TaskAttemptStatus::TimedOut)` and immediately `continue`s at `event_loop.rs:7445-7452`. It therefore skips `mark_task_failed` and `record_task_failure` at `event_loop.rs:7469-7479`.
4. `seed_task_dag_from_run_state` only seeds failed DAG nodes from `state.plan_failed_tasks` at `event_loop.rs:4959-4968`. Because the map is empty, the timed-out task and its dependents are not reconstructed as failed/blocked. `ready_tasks_for_plan` uses the unseeded DAG plus only `completed_tasks` at `event_loop.rs:8701-8712`, making the timed-out task eligible for work again.

Expected: replay is idempotent for lifecycle status while still reconciling every derived runtime/DAG structure needed for equivalent resume. An already-`TimedOut` snapshot must produce the same failed task IDs, failure reason, downstream blocking, and dispatch eligibility as replay into a pre-timeout snapshot.

Actual: the early return treats lifecycle idempotence as whole-state idempotence and omits the derived failed-DAG reconciliation. The candidate test `finished_owned_gate_expires_as_lost_effect_and_replays_from_ledger` initializes replay from `Started`, so it does not cover this normal post-timeout snapshot ordering.

Smallest correction:
- Do not skip derived reconciliation when lifecycle is already `TimedOut`. Separate “apply cancellation/timeout lifecycle events if needed” from idempotent reconciliation of `failed_tasks`, failure reason, and count, avoiding double-increment of `tasks_failed` restored from the snapshot.
- Add a deterministic regression that snapshots/restores an already-`TimedOut` lifecycle with an empty non-persisted failed map, replays the same exact ledger entry, seeds the DAG, and proves the task cannot become ready and downstream tasks are blocked. Preserve the existing conflict checks for other terminal statuses.

Independent verification:
- `git merge-base fc831c5542950470808ed876a29f4f841e7cd936 f42df7d7ab105f4f401bc1cc7cedab0777ca0775` — PASS, exact base returned; candidate range contains one commit.
- `git diff --check fc831c5542950470808ed876a29f4f841e7cd936..f42df7d7ab105f4f401bc1cc7cedab0777ca0775` — PASS.
- `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target cargo test -p roko-runtime run_ledger` — PASS, 7 passed / 0 failed (225 filtered); three pre-existing unrelated test warnings.
- `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target cargo test -p roko-cli runner::event_loop` — PASS, 50 passed / 0 failed (1,284 filtered); existing `plan_validation` missing-doc warning only.
- `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target cargo test -p roko-cli runner::projection` — PASS, 7 passed / 0 failed (1,327 filtered); existing `plan_validation` missing-doc warning only.
- The shared integration target was reused to avoid a second 20+ GiB cold cache; no isolated review target was created or left behind.

Verdict: **REJECTED**

Confidence: high. The focused tests validate the newly covered paths, but the source-level resume ordering above is deterministic and violates the task's replay/resume equivalence acceptance.

Required next action: correct the already-terminal snapshot replay path and add the described DAG-ready regression, then submit a new immutable candidate for independent review. Do not merge this candidate or mark the manifest/master task done.
