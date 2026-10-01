+++
id = "gap-9b3047"
kind = "gap"
title = "Duplicate cross-crate type families remain (TaskStatus, GateFeedback, AgentEvent, PlanStatus, ...)"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["workspace/types"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d9e79e9d8"
source = "gaps-md#cross-crate-duplicate-type-families----partial"
anchors = ["crates/roko-core/src/task.rs:22", "crates/roko-gate/src/feedback.rs:53", "crates/roko-compose/src/gate_feedback.rs:9", "crates/roko-learn/src/events.rs:24", "crates/roko-cli/src/inline/agent_events.rs:16", "crates/roko-agent/src/task_runner.rs:75"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "for t in TaskStatus PlanStatus GateFeedback AgentEvent; do test \"$(git grep -nE \"pub (enum|struct) $t( |<|\\{)\" -- 'crates/*.rs' ':!*/tests/*' | wc -l)\" -le 1 || exit 1; done"
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

Re-checked 2026-09-29: unchanged. The parked, unverified gap-c2a751 (created 2026-09-15, from docs/v3/39-ROADMAP.md#3.1, 'Consolidate ~14 duplicate type families and unify the event system') and find-2cc755 (created 2026-09-19, TaskStatus only) describe the same problem. Mark them duplicate_of this verified item, or fold them in.

## Notes

- 2026-10-01 (wk-filer4): partial, implemented on work/gap-2bc1b9; cargo verification deferred to the batch check. First step, PlanStatus: roko-acp's `PlanStatus` was the Agent Client Protocol's plan-entry state (pending, in_progress, completed), not a copy of roko-core's plan lifecycle. It is renamed to `PlanEntryStatus`, the protocol's own name, in types.rs, event_forward.rs and runner.rs (21 references, all inside roko-acp; no other crate used it), and its doc says why it differs. The serialized form is unchanged: serde names the variants, not the type. PlanStatus now has one definition, roko-core's. Left (the verify still counts them): TaskStatus (roko-core and roko-cli tui/state, 132 tui references), GateFeedback (roko-gate, roko-compose and roko-cli prompt_builder) and AgentEvent (roko-agent task_runner, roko-cli inline and roko-learn events), plus the other families in the description. Each needs a per-family call, consolidate or rename, best done with a compiler at hand.
