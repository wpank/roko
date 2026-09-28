+++
id = "q-1faa0c"
kind = "question"
title = "Dev-audit runtime fixes unverified on the Graph engine after Runner-v2 deletion"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-14
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dev-audit/09-additional-live-run-findings.md#Timeout loses provider usage and cost"
discovered_from = "audit:tmp/dev-audit/09-additional-live-run-findings.md#Timeout loses provider usage and cost"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs", "crates/roko-cli/src/graph_checkpoint.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Timeout usage/provider-identity preservation, idempotent terminal projections (snapshot/status/PID/ledger), bounded conductor settlement, timeout-diff gate salvage and FAST deadlines were built in Runner-v2 event_loop.rs; their kill-point/timeout matrix never ran and Graph parity is unproven (TD-...

Imported without verification from:
- `tmp/dev-audit/09-additional-live-run-findings.md#Timeout loses provider usage and cost`
- `tmp/dev-audit/09-additional-live-run-findings.md#Final persistence contradicts the terminal event`
- `tmp/dev-audit/11-implementation-status.md#Status Update (2026-09-01)`
- `tmp/docs-audit/07-TECH-DEBT.md#TD-08: Graph Engine / Runner-v2 Parity Gap`
- `tmp/dogfood/2026-09-19-session.md#Next Steps`
- `tmp/dogfood/2026-09-20-final-session.md#P2 (Medium)`

Some cited files are gone: `.roko/state/state-snapshot.json`.

How to verify: Graph-engine fixture: provider emits usage then hangs past deadline; verify usage/model/provider persisted and event/snapshot/status/PID/ledger agree.

Verified 2026-09-28: No Graph-path counterpart found: graph_task_dispatch.rs has no timeout usage/cost salvage, and nothing under graph_execution/ or graph_checkpoint.rs writes agent-pids.json or run-ledger.jsonl. The Runner-v2 implementations went with runner/event_loop.rs (6b5da8616), and the proposed Graph-engine timeout fixture does not exist. The question stays open.
