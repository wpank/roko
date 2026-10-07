+++
id = "find-8872ad"
kind = "finding"
title = "Server workspace admits one run: a second independent set is refused 409 — submit together"
status = "done"
triage = "verified"
severity = "p3"
subsystem = ["roko-serve/plans"]
created = 2026-09-29
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "248d279c7"
source = "plan:portal-programme/03b-backend-workspace-server#T16"
discovered_from = "plan:portal-programme/03b-backend-workspace-server#T16"
anchors = ["crates/roko-serve/src/routes/plans/run_control.rs::active_run_conflict", "crates/roko-serve/src/routes/plans/run_control.rs::start_plan_run", "crates/roko-serve/src/state.rs::AppState"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn second_plan_run_is_queued_not_refused' crates/roko-serve/ && cargo test -p roko-serve second_plan_run_is_queued_not_refused"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T19:21:52Z"
commit = "54c31ff46"
forced = false
evidence = "9104 (3d7441310) + 9111 (54c31ff46): a second POST /api/plans/{id}/execute (or /api/plans/execute) now queues on AppState.plan_queue and returns 202 {run_id, queued:true, position} instead of 409; the queued run starts when the active one ends. Verified by the already-passing test second_plan_run_is_queued_not_refused (routes/plans/tests.rs:1746); the item's stale verify (a grep of the deleted routes/plans.rs handlers) is replaced with a positive check on that test."
+++

The server enforces one active run per workspace: `POST /api/plans/{id}/execute` and `POST /api/plans/execute` both return 409 while any run is active, regardless of whether the pending plans are independent of the running set.

**Operational implication.** A user who has two genuinely independent plan sets (e.g. a research plan and a code plan) cannot run them concurrently by submitting two separate requests. They must submit a single `POST /api/plans/execute {plans: [id1, id2]}` request; the server will then run independent plans within that set in parallel up to `max_parallel_plans`.

This is the intended design (documented in §2.3 of `03-CONTRACT.md`): the server admits one set run per workspace. The finding records the UX implication so it can be considered for a future relaxation if the one-lock rule is ever changed.

Re-checked 2026-09-29: unchanged. The handlers are execute_plans (crates/roko-serve/src/routes/plans.rs:291) and execute_plan (:578); both refuse with 409 via active_run_conflict (:213) at :395 and :494. The existing verify pipes grep into head, so it always passes and greps for the current behaviour rather than a relaxation.

## Notes

2026-10-01 (wk-runstate): blocked: needs a product decision, and runtime work before the server check can go. The one-run rule is the contract (`tmp/portal-audit/03-CONTRACT.md` §2.3: "409 while any run is active (one run per workspace)"). The runtime enforces it too: `run_plan_with_options` takes the exclusive workspace runner lock (`crates/roko-cli/src/serve_runtime.rs:836`, `workspace_lock::acquire_runner_lock`, `.roko/runtime/roko.runner.lock`), which `roko plan run` also takes. Dropping `active_run_conflict(&active)` at `routes/plans.rs:395` and `:494` would therefore turn the 409 into a run that fails at once with a lock error. Next step: Will decides whether independent sets may run side by side in one workspace. If so, scope per-set runner locks, a refusal for sets that share a plan or output files, and per-run status and event files, then remove the 409.

2026-10-02 (filer): Will decided — queue it instead (backlog tldr P25/B9 B1). 9104 (`3d7441310`) gave a plan-run
handle a terminal status; 9111 (`54c31ff46`) built on it: `active_run_conflict` (moved to
`crates/roko-serve/src/routes/plans/run_control.rs` by the `5772b602b` route split, no longer in `plans.rs`) now
enqueues a second run on `AppState.plan_queue` (a bounded `VecDeque<QueuedPlanRun>`, capacity
`PLAN_RUN_QUEUE_CAPACITY`) and returns 202 `{run_id, queued: true, position}` instead of 409; the next queued run
starts when the active one ends. Test `second_plan_run_is_queued_not_refused` (`routes/plans/tests.rs:1746`)
covers it. The old `[[verify]]` (`! grep -q '...' routes/plans.rs`) passed vacuously once the code moved out of that
file, which is why `next`/`DRIFT.md` never caught this closing — replaced with a positive check on the new test.
`tmp/portal-audit/03-CONTRACT.md` §2.3 updated to say "queued" in the same pass (per 9111's own Notes).
