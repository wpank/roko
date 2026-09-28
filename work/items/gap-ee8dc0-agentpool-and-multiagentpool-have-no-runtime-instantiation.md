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
updated = 2026-09-28
last_verified = 2026-09-28
source = "gaps-md#built-but-unwired-subsystems/agentpool"
anchors = ["crates/roko-agent/src/pool.rs", "crates/roko-agent/src/multi_pool.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''grep -rn --include='*.rs' -E 'AgentPool::new|MultiAgentPool::new' crates | grep -v 'crates/roko-agent/src' | grep -q .'''
+++

Pool management is built and the TUI has a modal for it. Nothing outside `crates/roko-agent/src/` constructs `AgentPool` or `MultiAgentPool`, so no runtime path uses warm pooled agents. Backlogs #55 (pool runtime integration) and #72 (three overlapping pool designs) are archived without a status.

Fix: either wire one pool into Graph task dispatch (warm reuse per provider and role) and reconcile the overlapping pool types, or remove the unused pools and the TUI modal.
