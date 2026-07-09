---
title: "Readiness Audit: Agents (§02)"
section: analysis
subsection: readiness-audit
id: ra-subsystem-02
source: 31-implementation-readiness-audit.md (§02)
score: 21/30
tags: [agents, roko-agent, LLM-backends, tool-loop, safety, MCP]
---

# Readiness Audit: Agents (§02)

**Score**: 21/30 | **Crate**: roko-agent (Stable/Wired, 97 files, ~9,500 LOC, 567 tests)

## Criterion Scores

| Criterion | Score | Notes |
|---|---|---|
| rust_structs | 4 | Core types complete; temperament system unbuilt |
| pseudocode | 4 | 5 LLM backends documented with code |
| config_params | 4 | Provider config complete; temperament config absent |
| error_handling | 3 | Safety layer errors specified; backend errors partial |
| integration_wiring | 3 | ToolDispatcher never invoked from orchestrate.rs (G1) |
| test_criteria | 3 | Core path tested; advanced paths not |

## Strengths

- 5 LLM backends (Claude CLI, Anthropic API, Gemini, Perplexity, Ollama/OpenAI-compat)
- Safety layer: bash, git, path, network, rate_limit, scrub — all complete
- MCP bridge wired
- ToolLoop/Safety path live for all major provider families

## Critical Gaps

- **G1** (Tier 0): ToolDispatcher never invoked from `orchestrate.rs` — all 6 safety guards dormant
- **G4** (Tier 0): Role prompts average ~20 tokens; Meta-Harness research shows harness quality dominates model quality
- Temperament system fully specified but not propagated to runtime
- LlmBackend coverage: Gemini grounding/code-execution and Perplexity deep-research use dedicated paths

## Wiring Gap Detail (G1)

SafetyLayer + ToolDispatcher are built and wired to each other, but ToolDispatcher is never invoked from `orchestrate.rs`. Resolution options:
1. Subprocess interception
2. Settings passthrough
3. In-process API dispatch (recommended)

## Cross-References

- [../integration-map/safety-x-agents.md](../integration-map/safety-x-agents.md) — Critical safety wiring gap
- [../integration-map/agents-x-composition.md](../integration-map/agents-x-composition.md) — Wired
- [../integration-map/agents-x-verification.md](../integration-map/agents-x-verification.md) — Wired
