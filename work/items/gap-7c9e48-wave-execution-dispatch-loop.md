+++
id = "gap-7c9e48"
kind = "gap"
title = "Wave Execution Dispatch Loop"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/commands"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/396-wave-execution-dispatch.md#396 — Wave Execution Dispatch Loop"
discovered_from = "audit:tmp/backlog/archive/396-wave-execution-dispatch.md#396 — Wave Execution Dispatch Loop"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs:1197", "crates/roko-cli/src/graph_execution/plan_runner.rs::graph_plan_execution_order", "crates/roko-cli/src/runner/plan_dag.rs::CrossPlanDag::compute", "crates/roko-cli/src/commands/plan.rs:67"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'CrossPlanDag' crates/roko-cli/src/graph_execution/plan_runner.rs"
+++
largest single performance gap; 30-plan batches take 5x longer without wave parallelism. `CrossPlanDag::compute` (in `crates/roko-cli/src/runner/plan_dag.rs`) uses Kahn's algorithm to group plans into execution waves. Wave 0 contains all plans with no cross-plan dependencies; wave N contains all…

Imported without verification from:
- `tmp/backlog/archive/396-wave-execution-dispatch.md#396 — Wave Execution Dispatch Loop`

Some cited files are gone: `crates/roko-core/src/config.rs`.

How to verify: Check: `roko plan run plans/` with 30 independent plans dispatches all 30 graph engines before any one completes (confirmed via `RUST_LOG=info` showing concurrent `running plan via Graph Engine` lines).; `roko plan run plans/` with a diamond… [evidence: no status line; no index/roll-up evidence]

Verified 2026-09-28: crates/roko-cli/src/graph_execution/plan_runner.rs:1197 still runs plans strictly one after another (`for plan_id in &plan_execution_order`, each awaited); CrossPlanDag::compute is used only for `roko plan list --waves` and summaries (commands/plan.rs:67, :382). Severity lowered p0 -> p1: a throughput gap, not a broken core loop.
