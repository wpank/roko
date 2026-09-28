+++
id = "gap-6b6ae7"
kind = "gap"
title = "No per-task agent handle map for targeted cancellation"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/graph-execution"]
created = 2026-08-31
updated = 2026-09-28
source = "gaps-md#not-started-2/139"
anchors = ["crates/roko-cli/src/graph_execution/control_adapter.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

UX #139: there is no `HashMap<TaskId, AgentHandle>` (0 matches on 2026-09-28), so an operator cannot cancel the agent of one running task without cancelling the whole plan. It has not been checked whether Graph cancellation (`crates/roko-cli/src/graph_execution/control_adapter.rs`) already supports per-task cancellation another way. Backlog #139 is archived without a status.

Fix: confirm the granularity of the Graph control adapter's cancellation. If it only cancels whole plans, add per-task agent handles and a `plan cancel --task` path.
