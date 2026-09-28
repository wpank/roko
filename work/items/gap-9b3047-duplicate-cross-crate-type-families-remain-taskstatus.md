+++
id = "gap-9b3047"
kind = "gap"
title = "Duplicate cross-crate type families remain (TaskStatus, GateFeedback, AgentEvent, PlanStatus, ...)"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["workspace/types"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "gaps-md#cross-crate-duplicate-type-families----partial"
anchors = ["crates/roko-core/src/task.rs:22", "crates/roko-gate/src/feedback.rs:53", "crates/roko-compose/src/gate_feedback.rs:9", "crates/roko-learn/src/events.rs:24", "crates/roko-cli/src/inline/agent_events.rs:16", "crates/roko-agent/src/task_runner.rs:75"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

About 14 conceptual type families still have more than one public definition, including `AgentState`, `TaskStatus`, `GateFeedback`, `EventBus`, `Cell` and `Plan`. Each needs either consolidation or an explicit statement of why the types differ. Duplicate `pub enum`/`pub struct` definitions found on 2026-09-28:
- `crates/roko-core/src/task.rs:22` and `:66`
- `crates/roko-gate/src/feedback.rs:53`
- `crates/roko-compose/src/gate_feedback.rs:9`
- `crates/roko-cli/src/dispatch/prompt_builder.rs:905`
- `crates/roko-learn/src/events.rs:24`
- `crates/roko-cli/src/inline/agent_events.rs:16`
- `crates/roko-agent/src/task_runner.rs:75`
- `crates/roko-acp/src/types.rs:881`
- `crates/roko-cli/src/tui/state/mod.rs:224`

This item absorbs backlog #42.

Fix: for each family, pick one canonical owner (usually `roko-core`) or document the semantic split. Turn the other definitions into re-exports or adapters.
