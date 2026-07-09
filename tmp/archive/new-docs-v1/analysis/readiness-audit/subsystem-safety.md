---
title: "Readiness Audit: Safety (§11)"
section: analysis
subsection: readiness-audit
id: ra-subsystem-11
source: 31-implementation-readiness-audit.md (§11)
score: 27/30
tags: [safety, capability-tokens, taint-tracking, tool-dispatcher, critical-gap-G1]
---

# Readiness Audit: Safety (§11)

**Score**: 27/30 | **Crate**: Types in roko-orchestrator's safety sub-module (Built + tested; not invoked)

## Criterion Scores

| Criterion | Score | Notes |
|---|---|---|
| rust_structs | 5 | `Capability<T>` token system: security-by-construction |
| pseudocode | 4 | 7-step ToolDispatcher pipeline documented |
| config_params | 5 | All 6 guards configured |
| error_handling | 4 | Good; forensic replay partial |
| integration_wiring | 5 | Best integration documentation — maps every guard to pipeline location |
| test_criteria | 4 | Guards tested; activation from orchestrate.rs untested |

## The Critical Gap: G1

SafetyLayer + ToolDispatcher are built and wired to each other, but **ToolDispatcher is never invoked from `orchestrate.rs`**. All six safety guards are dormant:

- `BashPolicy` — restricts dangerous shell commands
- `GitPolicy` — prevents unauthorized git operations
- `NetworkPolicy` — controls network access
- `PathPolicy` — confines file system access
- `ScrubPolicy` — sanitizes sensitive data in outputs
- `RateLimiter` — prevents runaway API costs

**This is Gap G1 — the highest-priority gap in the entire system.**

## Strengths

- `Capability<T>` token system: unforgeable, single-use, compile-time
- Best integration documentation: doc 00 maps every safety guard to its location in the 7-step pipeline
- Doc 16 provides exact file locations, line counts, and three resolution options for the wiring gap

## Resolution Path (G1)

Three options:
1. Subprocess interception (intercept bash/git at OS level)
2. Settings passthrough (pass safety settings to LLM backend)
3. In-process API dispatch (recommended — route all tool calls through ToolDispatcher)

## Cross-References

- [../integration-map/safety-x-agents.md](../integration-map/safety-x-agents.md) — Critical gap
- [../integration-map/safety-x-composition.md](../integration-map/safety-x-composition.md) — M13
