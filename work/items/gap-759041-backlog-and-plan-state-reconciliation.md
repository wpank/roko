+++
id = "gap-759041"
kind = "gap"
title = "Backlog and Plan State Reconciliation"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/229-backlog-plan-state-reconciliation.md#229 — Backlog and Plan State Reconciliation"
discovered_from = "audit:tmp/backlog/archive/229-backlog-plan-state-reconciliation.md#229 — Backlog and Plan State Reconciliation"
anchors = ["crates/roko-cli/src/commands/backlog.rs::build_audit_report", "crates/roko-cli/src/commands/backlog.rs::read_executor_plan_phases", "crates/roko-cli/src/commands/backlog.rs::read_run_state_task_terminals"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
stale indexes and plan status can re-dispatch completed work or hide unfinished work. Dogfooding found 11 `tasks.toml` files still marked ready after their implementation had merged. The 2026-08-31 backlog index also continued to list several PR #64–#72 items as open even though the current code…

Imported without verification from:
- `tmp/backlog/archive/229-backlog-plan-state-reconciliation.md#229 — Backlog and Plan State Reconciliation`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.6 Proof Case 6: Backlog/plan state reconciliation`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `scripts/check_backlog_integrity.*`.

How to verify: Check: `roko backlog audit --json` reports every mismatch with stable codes, paths, IDs, evidence, and; A fully completed plan cannot remain silently `status = "ready"` without an audit failure.; An archived spec with a stale root link is reported… [evidence: own status: Overall: NOT STARTED. No implementation work has landed for `roko backlog audit` or reconciliation logic. No…; 00-STATUS-SUMMARY 3. Open / P1 -- High…]

Verified 2026-09-28: `roko backlog audit` exists (commands/backlog.rs::cmd_backlog_audit, --json/--fix-safe, landed 72e0a76b8), but runner-side evidence comes only from Runner-v2 files (state-snapshot.json, executor snapshot, run-state.json). Graph checkpoints under .roko/state/graph/ are never read, and the Graph path does not write task status back to tasks.toml, so a Graph-completed plan left `ready` is not detected.
