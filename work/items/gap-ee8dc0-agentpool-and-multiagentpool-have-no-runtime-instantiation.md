+++
id = "gap-ee8dc0"
kind = "gap"
title = "AgentPool and MultiAgentPool have no runtime instantiation"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-agent/pool"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "gaps-md#built-but-unwired-subsystems/agentpool"
anchors = ["crates/roko-agent/src/pool.rs", "crates/roko-agent/src/multi_pool.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rn --include='*.rs' -E 'AgentPool::new|MultiAgentPool::new' crates/roko-cli/src crates/roko-serve/src crates/roko-execution/src | grep -q . || { ! test -e crates/roko-agent/src/pool.rs && ! test -e crates/roko-agent/src/multi_pool.rs; }"
+++

Pool management is built and the TUI has a modal for it. Nothing outside `crates/roko-agent/src/` constructs `AgentPool` or `MultiAgentPool`, so no runtime path uses warm pooled agents. Backlogs #55 (pool runtime integration) and #72 (three overlapping pool designs) are archived without a status.

Fix: either wire one pool into Graph task dispatch (warm reuse per provider and role) and reconcile the overlapping pool types, or remove the unused pools and the TUI modal.

## Notes

- 2026-10-01 (wk-guard2): implemented on work/bug-a70def; cargo verification deferred to the batch check.
- Removed rather than wired: wiring one into Graph dispatch with warm reuse per provider and role is a design task, not a static change, and roko-cli's `WarmPool` is the production pool. Deleted `roko-agent/src/pool.rs` and `multi_pool.rs` (1,831 lines, tests included) and their `lib.rs` modules and re-exports; nothing outside them used `AgentPool`, `MultiAgentPool`, `AgentInstanceId`, `AgentTask`, `InstanceStatus`, roko-agent's `TaskOutcome`, `KillReport` or `WarmEntry`. Doc comments updated in `roko-cli/src/dispatch/warm_pool.rs` and `roko-compose/src/context_mesh.rs`.
- Left for the TUI area: `ModalState::AgentPool` is not this type but a roster modal (`tui/modals/agent_pool_modal.rs`) that only tests open; removing it touches `tui/input.rs`, `tui/app/actions.rs`, `tui/widgets/status_bar.rs` and `tui/modals/mod.rs`.
