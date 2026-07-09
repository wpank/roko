---
title: "Agents × Verification"
section: analysis
subsection: integration-map
id: im-agents-x-verification
source: 24-cross-section-integration-map.md (§3.1, §3.3, §4.1)
tags: [agents, verification, gate-verdict, safety, tool-dispatcher, wired]
---

# Agents × Verification

**Direction**: 02-Agents → 04-Verification (agent output to gate pipeline); 04-Verification → 02-Agents (verdict informs retry)  
**Status**: **Wired** (agent output → gate); **Critical Gap**: ToolDispatcher never invoked from `orchestrate.rs` (G1, G13)  
**Interface**: `roko-agent::AgentOutput` → `roko-gate::GatePipeline`

## What Flows

| Signal | From | To | Status |
|---|---|---|---|
| `Kind::AgentOutput` | `roko-agent` | `GatePipeline::verify()` | **Wired** |
| `Kind::GateVerdict` | `GatePipeline` | `roko-orchestrator` (pass/fail) | **Wired** |
| `Kind::ToolCall`, `Kind::ToolResult` | `roko-agent` | `roko-std::ToolDispatcher` | **Wired** (agent→tools) |
| Safety checks on tool calls | `SafetyLayer` | `ToolDispatcher` | **Built but not invoked from orchestrate.rs** |

## The Critical Safety Gap

SafetyLayer and ToolDispatcher are built and wired to each other, but **ToolDispatcher is never invoked from `orchestrate.rs`**. All six safety guards (BashPolicy, GitPolicy, NetworkPolicy, PathPolicy, ScrubPolicy, RateLimiter) are dormant in the production code path. This is Readiness Audit Gap G1 — the highest-priority gap in the system.

Three resolution options (from source file 31, §11):
1. **Subprocess interception**: intercept bash/git calls at the OS level
2. **Settings passthrough**: pass safety settings to the LLM backend
3. **In-process API dispatch**: route all tool calls through ToolDispatcher in-process

## Invariants of the Interaction

1. Every agent output goes through the gate pipeline before the orchestrator accepts it.
2. Gate verdicts include the gate name that failed, for debugging.
3. Safety guards should intercept all tool calls before execution (not yet enforced — G1 gap).

## Enhancement Opportunities

- [code-intel-x-verification.md](./code-intel-x-verification.md) — M16: semantic diff as gate input
- [neuro-x-verification.md](./neuro-x-verification.md) — M14: knowledge-informed thresholds
- [../architectural-analysis/08-novel-proposals.md](../architectural-analysis/08-novel-proposals.md) — Proposal 2: gradient gate feedback

## Cross-References

- Readiness audit: [RA-02: Agents](../readiness-audit/subsystem-agents.md), [RA-04: Verification](../readiness-audit/subsystem-verification.md), [RA-11: Safety](../readiness-audit/subsystem-safety.md)
