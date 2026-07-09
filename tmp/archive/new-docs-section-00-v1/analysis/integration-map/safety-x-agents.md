---
title: "Safety × Agents"
section: analysis
subsection: integration-map
id: im-safety-x-agents
source: 24-cross-section-integration-map.md (§3.1, §3.3, §4.1)
tags: [safety, agents, tool-dispatcher, capability-tokens, taint-tracking, critical-gap]
---

# Safety × Agents

**Direction**: 11-Safety ↔ 02-Agents (safety guards on tool execution)  
**Status**: **Built but not wired** — SafetyLayer and ToolDispatcher are complete and tested; ToolDispatcher is never invoked from `orchestrate.rs` (Readiness Audit G1 — highest priority gap)  
**Interface**: `roko-orchestrator::safety` (SafetyLayer, ToolDispatcher) → `roko-agent` tool call execution path

## What Flows

| Signal | From | To | Status |
|---|---|---|---|
| `Kind::ToolCall` | `roko-agent` | `ToolDispatcher` | **Not invoked from orchestrate.rs** |
| `Kind::ToolResult` | `ToolDispatcher` | `roko-agent` | **Not invoked from orchestrate.rs** |
| `Capability<T>` tokens | `SafetyLayer` | Agent context | **Built; wiring gap** |
| Taint propagation | `ScrubPolicy` | Agent output | **Built; wiring gap** |
| Audit chain | `AuditChain` | `roko-orchestrator` | **Built; wiring gap** |

## The Critical Gap (G1)

All six safety guards are **dormant**:
- `BashPolicy` — restricts dangerous shell commands
- `GitPolicy` — prevents unauthorized git operations
- `NetworkPolicy` — controls network access
- `PathPolicy` — confines file system access
- `ScrubPolicy` — sanitizes sensitive data in outputs
- `RateLimiter` — prevents runaway API costs

**Resolution options** (source file 31, §11):
1. **Subprocess interception**: intercept bash/git calls at OS level via process wrapper
2. **Settings passthrough**: pass safety settings to LLM backend as context
3. **In-process API dispatch**: route all tool calls through ToolDispatcher in-process (recommended)

## Invariants of the Interaction

Once wired:
1. Every tool call passes through all applicable safety guards before execution.
2. A failed safety check returns an error `ToolResult`, not a panic.
3. `Capability<T>` tokens are unforgeable and single-use (compile-time guarantee).
4. Scrubbed content is replaced, not deleted — the audit chain records what was scrubbed.

## Cross-References

- Composition constraint: [safety-x-composition.md](./safety-x-composition.md) — M13 (safety constraints should also surface in prompts)
- Verification: [agents-x-verification.md](./agents-x-verification.md) — gate pipeline follows tool execution
- Readiness audit: [RA-11: Safety](../readiness-audit/subsystem-safety.md), [RA-02: Agents](../readiness-audit/subsystem-agents.md)
- **Audit Gap G1**: highest priority gap in the entire system
