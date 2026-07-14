# SH01-T06C4 independent review 02 — b8bfd506d

## Assignment and independence

- Candidate tip: `b8bfd506d3169b7e02cf7bfc03b278c4f9c506f1`.
- Candidate parent / rejected first candidate: `f42df7d7ab105f4f401bc1cc7cedab0777ca0775`.
- Exact assigned base: `fc831c5542950470808ed876a29f4f841e7cd936`.
- Prior independent rejection: `3f083d6cd8830991beb8cff50a2ee38da6f90605`, retained on integration as `ca65e644cfea007dcf818b769d302832fb99fd03`.
- Review branch: `review/SH01-T06C4-r2-b8bfd506`.
- Review worktree: `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/reviews/SH01-T06C4-r2`.
- The reviewer did not author either candidate commit and did not change candidate source, implementation evidence, manifests, master status, or integration state.
- Cumulative review range: `fc831c5542950470808ed876a29f4f841e7cd936..b8bfd506d3169b7e02cf7bfc03b278c4f9c506f1`.
- Cumulative changed paths: `crates/roko-runtime/src/run_ledger.rs`, `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/runner/projection.rs`, and `tmp/status-quo/execution-evidence/SH01-T06C4.md`.

## Independent requirement reconstruction and source trace

The task requires one exact ownership winner between completion and expiry; typed, exact-key, idempotent timeout durability; cleanup before durability, durability before claim closure, and claim closure before terminal publication; fail-closed recovery; replay equivalence for pre-timeout and already-timed-out snapshots; stable aggregate failure counts; reconstructed failed IDs, reasons, downstream blocking and dispatch eligibility; projection equality; no wall-clock-to-monotonic conversion; stale/duplicate/gate-map independence; and bounded sibling drainage.

The cumulative production trace satisfies that contract:

- `AttemptOwnership::claim_phase` and `claim_cancellation_exact` compete for the same exact attempt/phase/effect owner. `enforce_owned_deadlines_at` revalidates each sorted scan candidate with `event_is_eligible` before claiming it, so completion and expiry have one winner and stale effects cannot consume replacement owners.
- `cancel_exact_attempt` retains the exact claim during resource cleanup and `persist_timeout_terminal`. A persistence or conversion failure restores the claim and publishes only cancellation failure. Successful claim closure precedes the timeout event. Thus a crash after the ledger append can be recovered without falsely publishing before durability.
- `TimeoutTerminalReplay` keys by run/plan/task/attempt, accepts an identical duplicate as a no-op, rejects a different fact for the occupied key, and rejects a run mismatch through `RunLedger::record_timeout_terminal`. The CLI loader decodes this runtime-owned serde type and ignores non-task global timeout audit entries.
- `replay_timeout_terminals` now fixes the prior rejection: lifecycle event idempotence is separate from failed-ID and reason reconciliation. A `Started` snapshot applies the terminal and increments `tasks_failed` once. An already-`TimedOut` snapshot preserves its restored aggregate count while rebuilding the non-snapshotted failed ID and reason. `seed_task_dag_from_run_state` then blocks dependents, and `ready_tasks_for_plan` cannot redispatch either the failed task or its downstream task.
- Live and reconstructed timeout events both use `timeout_runner_event` and `Projection::normalize_runner_event`; the projection regression compares the normalized values exactly.
- Deadline restart intent retains only configured and previously observed monotonic durations. Changing `observed_at_ms` from zero to `u64::MAX` does not alter it; no wall timestamp is converted to `Instant` or `MonotonicTime`.
- Sibling drainage snapshots the initial owner set once. An unconfirmable sibling remains owned with one cancellation-failed event and is not recursively retried. Unrelated gate-map state neither hides exact expiry nor gets consumed by it.

No acceptance-blocking finding remains after adversarial review of the changed lines and the unchanged ownership, lifecycle, snapshot, DAG-ready, persistence, and projection callers.

## Independent verification

Working directory for all commands was the review worktree above. Rust commands used the single isolated target `CARGO_TARGET_DIR=/private/tmp/roko-sh01-t06c4-r2-review-target`, created empty for this review and removed after verification.

- `git merge-base fc831c5542950470808ed876a29f4f841e7cd936 b8bfd506d3169b7e02cf7bfc03b278c4f9c506f1` — PASS; returned the exact assigned base.
- `git diff --check fc831c5542950470808ed876a29f4f841e7cd936..b8bfd506d3169b7e02cf7bfc03b278c4f9c506f1` — PASS, no diagnostics.
- `CARGO_TARGET_DIR=/private/tmp/roko-sh01-t06c4-r2-review-target cargo test -p roko-runtime run_ledger` — PASS; 7 passed, 0 failed, 225 filtered. Three unrelated pre-existing test warnings were emitted in heartbeat modules.
- `CARGO_TARGET_DIR=/private/tmp/roko-sh01-t06c4-r2-review-target cargo test -p roko-cli runner::event_loop` — PASS; 51 passed, 0 failed, 1,284 filtered. The existing missing-docs warning for `tests/plan_validation.rs` was emitted. This includes both resume orderings, exact race orderings, durable ordering/failure recovery, stale and duplicate behavior, gate-map independence, and bounded sibling drainage.
- `CARGO_TARGET_DIR=/private/tmp/roko-sh01-t06c4-r2-review-target cargo test -p roko-cli runner::projection` — PASS; 7 passed, 0 failed, 1,328 filtered. The same existing missing-docs warning was emitted.
- `cargo fmt --all -- --check` — PASS.
- Final `git status --short` after committing this evidence and removing the isolated target — clean.

No persistent test fixture, process, generated index, or target remains from review. The test-created Git commit shown in the event-loop output occurred inside its temporary fixture, not in this worktree.

## Verdict

**ACCEPTED** for the exact cumulative candidate ending at `b8bfd506d3169b7e02cf7bfc03b278c4f9c506f1`.

Confidence: high. The previous deterministic resume defect is directly covered in production logic and by a snapshot/DAG/redispatch regression, while the complete focused suite passes from an empty target.

Next action: the coordinator may merge the exact accepted candidate and this review commit, run post-merge focused proof, and only then reconcile canonical task status. This verdict does not mark SH01-T06C4, its plan, Wave 1, or the programme complete.
