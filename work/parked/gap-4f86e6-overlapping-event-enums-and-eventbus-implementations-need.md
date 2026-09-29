+++
id = "gap-4f86e6"
kind = "gap"
title = "Overlapping event enums and EventBus implementations need consolidation"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["workspace/events"]
created = 2026-09-15
updated = 2026-09-28
source = "gaps-md#2026-09-15-refactoring-audit-batch/eventbus-audit"
anchors = ["crates/roko-runtime/src/event_bus.rs:239", "crates/roko-serve/src/event_bus.rs:25", "crates/roko-learn/src/events.rs:113", "crates/roko-agent/src/task_runner.rs:101"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

The 2026-09-15 EventBus audit counted about 30.7K lines of event plumbing:
- 47 event enums across 42 files;
- 4 `EventBus` structs, confirmed on 2026-09-28 in `roko-runtime`, `roko-serve`, `roko-learn` and `roko-agent`;
- 2 bus trait systems.

The costliest overlaps are `RuntimeEvent`/`GraphExecutionEvent`, `ServerEvent`/`DashboardEvent` and three separate `AgentEvent` enums. A four-phase migration plan was drafted but not started.

Fix: choose canonical event types for each layer (runtime, graph, dashboard), convert the others to adapters, and delete the duplicate buses.
