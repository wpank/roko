---
title: "Readiness Audit: Tools (§18)"
section: analysis
subsection: readiness-audit
id: ra-subsystem-18
source: 31-implementation-readiness-audit.md (§18)
score: 28/30
tags: [tools, roko-std, capability-tokens, MCP, plugin-SDK, wired]
---

# Readiness Audit: Tools (§18)

**Score**: 28/30 | **Crate**: roko-std (Stable, 33 files, ~3,500 LOC, ~120 tests)

## Criterion Scores

| Criterion | Score | Notes |
|---|---|---|
| rust_structs | 5 | Capability<T> token system: security-by-construction |
| pseudocode | 5 | Tool loop fully documented |
| config_params | 5 | 16 builtin tools configured |
| error_handling | 4 | Tool error types; plugin error less specified |
| integration_wiring | 4 | MCP client built; plugin SDK not in any crate |
| test_criteria | 5 | 66 eval tests; golden tool tests verify schema stability |

## Strengths

- `Capability<T>` token system: unforgeable, single-use, compile-time (security-by-construction)
- MCP client: actually built
- 18 agent templates immediately usable
- 4-layer tool testing with 66 eval tests
- 16 builtin tools; role profiles; mock dispatcher

## Gaps

- MCP servers scaffold — GitHub, Slack not yet built (G22)
- Plugin SDK specified but not in any crate
- WASM plugins designed but no `wasm32` target validated

## Cross-References

- [../integration-map/safety-x-agents.md](../integration-map/safety-x-agents.md) — Tools and safety are tightly coupled
