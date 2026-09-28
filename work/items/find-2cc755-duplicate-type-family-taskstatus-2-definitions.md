+++
id = "find-2cc755"
kind = "finding"
title = "Duplicate type family: TaskStatus (2 definitions)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core"]
created = 2026-09-19
updated = 2026-09-28
source = "tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#1. TaskStatus"
discovered_from = "audit:tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#1. TaskStatus"
anchors = ["crates/roko-core/src/task.rs:66", "crates/roko-cli/src/tui/state/mod.rs:224", "roko_core::TaskStatus", "crates/roko-runtime/src/run_registry.rs:34", "crates/roko-cli/src/runner/types.rs:522", "RunStatus", "RunnerRunStatus", "roko_runtime::RunStatus::Pending", "crates/roko-gate/src/feedback.rs:53", "crates/roko-compose/src/gate_feedback.rs:9"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
P2-DT-1 audit (2026-09-19) family #1: TaskStatus has 2 independent definitions. Proposed action: Add Failed to core; remove TUI copy. Difference: The TUI copy adds `Failed` as a distinct variant; core omits it (uses `Blocked` for terminal failure in the current model).

Imported without verification from:
- `tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#1. TaskStatus`
- `tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#3. RunStatus/RunnerRunStatus`
- `tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#5. GateFeedback`
- `tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#7. CostRecord`
- `tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#2. PlanStatus`
- `tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#4. AgentState`
- `tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#6. ProviderHealth/ProviderStatus`
- `tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#8. WorkspaceInfo`
- `tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#11. PlanOutcome/PlanLifecycleStatus`
- `tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#13. DashboardSnapshot vs CommandDashboardSnapshot`
- `tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#14. TaskStatusDetail`

How to verify: grep the duplicate definitions of TaskStatus across crates; open if more than one independent definition remains (action: Add Failed to core; remove TUI copy). CONSOLIDATED P2-DT-1 'done' only covered the audit itself. / grep the duplicate definitions of RunStatus/RunnerRunStatus across crates; open if more than one independent definition remains (action: Alias CLI to runtime variant). CONSOLIDATED P2-DT-1 'done' only covered the audit itself. / grep the duplicate definitions of GateFeedback across crates; open if more than one independent definition remains (action: Canonicalize to roko-gate; add conversion). CONSOLIDATED P2-DT-1 'done' only covered the audit itself.

Merged 11 mined candidates: m1-198, m1-200, m1-202, m1-204, m1-199, m1-201, m1-203, m1-205, m1-207, m1-208, m1-209.
