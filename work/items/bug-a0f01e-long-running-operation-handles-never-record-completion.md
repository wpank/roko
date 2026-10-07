+++
id = "bug-a0f01e"
kind = "bug"
title = "Long-running operation handles never record completion"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-serve/operations"]
created = 2026-09-28
updated = 2026-10-02
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/dream.rs:107", "crates/roko-serve/src/routes/research.rs:257", "crates/roko-serve/src/routes/templates.rs:207", "crates/roko-serve/src/routes/plans.rs:1151", "crates/roko-serve/src/routes/gateway.rs:708"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn every_producer_records_terminal_status' crates/roko-serve/ && cargo test -p roko-serve operations::tests::every_producer_records_terminal_status"
+++

Nine handlers register an entry in `AppState.operations` (dream `routes/dream.rs:107`, research `:257`, PRDs `prds.rs:534/709/947`, templates `:207`, plans `:948/:1591`, inference batch `gateway.rs:728`), but only the inference batch ever sets `Completed` (`gateway.rs:708`).
A finished dream, research or plan-generation job reports `"Running"` until handle GC removes it, after which `GET /api/operations/{id}` is 404; the status is a Rust `Debug` string, and nothing survives a restart.
Fix: one registry wrapper that records Running -> Completed | Failed | Cancelled for every producer, returns a JSON status object and persists a bounded record.

**Partial fix landed 2026-09-29 (plan 04 T11):** The plan-generation producer (`routes/plans.rs`) now finalizes its handle with `Completed{result: {slug, task_count}}` or `Failed{error}`. `GET /api/operations/{id}` returns a JSON status object `{id, kind, status, result?, error?}` (no longer a Rust `Debug` string). The other producers — dream (`routes/dream.rs`), research (`routes/research.rs`), PRDs (`routes/prds.rs`), templates, and inference batch — are unchanged: they still report `"running"` until GC removes the handle.

Re-verified 2026-09-29 at d9e79e9d8. Beyond the plan-generation producer, plan 04b's plan_revise op (routes/plans.rs:2154) also records Completed/Failed (6c9b8dc7a). Still open: the producers at routes/dream.rs:107, routes/research.rs:257, routes/prds.rs:537, :712 and :950, routes/templates.rs:207 and the plan_chat op at routes/plans.rs:1151 insert OperationStatus::Running and never update it; there is still no shared registry wrapper, and operation records are not persisted, so nothing survives a restart.

## Notes

- 2026-10-01 (wk-serve2): PARTIAL, implemented on work/bug-1cb461; cargo verification deferred to the batch check.
  New `crates/roko-serve/src/operations.rs::spawn_operation` registers the handle under the operations lock (so the
  work cannot finish first), records `Completed { result }` or `Failed { error }` when the work returns (a panic
  records `Failed`), and only then publishes `OperationCompleted`. Dream, research, the three PRD producers
  (`prd_draft`, `prd_consolidate`, `prd_plan`), template deploy and `plan_chat` (`routes/plans.rs`, one hunk) now go
  through it. Tests: `spawn_operation_records_completed_failed_and_panicked_work` and
  `every_producer_records_terminal_status` (drives each producer route through `build_router` with a succeeding stub
  runtime and checks `GET /api/operations/{id}` says `completed`; dream is covered by the wrapper only, because the
  test would run real dream consolidation).
- Still open: (1) a persisted, bounded record of finished operations, so `GET /api/operations/{id}` survives handle GC
  (`gc_completed_handles` drops finished handles every 60 s) and restarts; (2) a `Cancelled` status, which nothing
  produces yet; (3) the inference batch (`routes/gateway.rs`) still registers its handle after spawning and records
  only `Completed`; `plan_generate` and `plan_revise` keep their own recording.
- 2026-10-02 (roko-7d): the workflow-audit migration (merge bfd36512f) removed the PRD pipeline, `roko do` and `roko develop`; `roko run` is the one entry point and plans come from a prompt. `routes/prds.rs` and its three producers (`prd_draft`, `prd_consolidate`, `prd_plan`) were deleted, so they drop out of this item.
