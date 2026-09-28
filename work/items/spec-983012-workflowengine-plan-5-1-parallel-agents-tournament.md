+++
id = "spec-983012"
kind = "spec"
title = "WorkflowEngine plan 5.1 Parallel Agents (Tournament)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/runner"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#5.1 Parallel Agents (Tournament)"
discovered_from = "audit:tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#5.1 Parallel Agents (Tournament)"
anchors = ["ParallelWorkflowTemplate", "EffectDriver", "EventBus", "SwarmProjection", "SwarmUpdated"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
7 open items (2026-04-28 plan written for the since-retired WorkflowEngine; retarget to Graph engine or close): 5.1.1 `ParallelWorkflowTemplate`: spawns N agents with diffe; 5.1.2 Worktree isolation: `git worktree add` per agent, clea; 5.1.3 Per-agent `EffectDriver` instance, all sharing the sam…

Imported without verification from:
- `tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#5.1 Parallel Agents (Tournament)`

How to verify: WorkflowEngine was retired (#276); check whether a Graph-engine/config equivalent exists for: 5.1.1 `ParallelWorkflowTemplate`: spawns N agents with different approaches; 5.1.2 Worktree isolation: `git worktree add` per agent, cleanup on completion; 5.1.3 Per-agent `EffectDriver` instance, all sharing the same `EventBus`; 5.1.4 `SwarmProjection`…
