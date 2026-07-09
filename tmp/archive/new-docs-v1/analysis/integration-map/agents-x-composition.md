---
title: "Agents × Composition"
section: analysis
subsection: integration-map
id: im-agents-x-composition
source: 24-cross-section-integration-map.md (§3.1, §3.3, §4.1)
tags: [agents, composition, system-prompt, context-assembly, wired]
---

# Agents × Composition

**Direction**: 03-Composition → 02-Agents (system prompt delivery); 01-Orchestration → 03-Composition (role spec)  
**Status**: **Wired**  
**Interface**: `roko-compose::SystemPromptBuilder` → `roko-agent::LlmBackend`

## What Flows

| Signal | From | To | Status |
|---|---|---|---|
| `Kind::Prompt` (system prompt) | `SystemPromptBuilder` | Agent `execute()` call | **Wired** |
| Role spec (task description, domain) | `roko-orchestrator` | `SystemPromptBuilder` as build input | **Wired** |
| `Kind::AgentOutput` | `roko-agent` | Verification and Learning | **Wired** |

## Invariants of the Interaction

1. Every agent execution is preceded by a composition call — no bare agent execution without a built system prompt.
2. The composition budget (`Budget`) constrains token count for the system prompt; agents receive the composed result within budget.
3. System prompts are deterministic for the same input (no hidden statefulness in builder).
4. The `Kind::Prompt` Engram is logged with its content hash, enabling replay.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| Budget exceeded during composition | Prompt truncated; quality loss | Alert when composition exceeds budget by >10% |
| Role spec missing (orchestrator bug) | Prompt uses default template | Default template fallback; log missing role spec |
| LlmBackend rejects prompt (too long) | Agent call fails | Gate pipeline catches; retry with smaller budget |

## Enhancement Opportunities

- [learning-x-composition.md](./learning-x-composition.md) — M4: inject accumulated skills
- [neuro-x-composition.md](./neuro-x-composition.md) — M5: inject knowledge entries
- [daimon-x-composition.md](./daimon-x-composition.md) — M2: affect-modulated weights (partially wired)
- [code-intel-x-composition.md](./code-intel-x-composition.md) — M8: inject code symbols
- [safety-x-composition.md](./safety-x-composition.md) — M13: inject safety constraints

## Cross-References

- Readiness audit: [RA-02: Agents](../readiness-audit/subsystem-agents.md), [RA-03: Composition](../readiness-audit/subsystem-composition.md)
