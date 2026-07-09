# 02-Agents Parity Analysis

Gap analysis of `docs/02-agents/` (16 documents) vs the actual Roko agent stack. Covers what is shipped, what is duplicated, what is not yet wired into runtime, and what should be deferred to later parity batches instead of being forced into agent batch `02`.

Generated: 2026-04-16

---

## How To Use This Batch

This batch should be treated as **agent runtime integration + shared type ownership hardening**, not as permission to implement every research idea described in docs `10-12`.

- Prefer wiring agent infrastructure that already exists over inventing new agent frameworks.
- Prefer smaller batches that touch one conflict group at a time.
- If a task starts depending on orchestration-policy, verification-policy, or learning-policy semantics, record the seam and defer.
- For overnight runs, each batch should be able to stop with a clear pass/fail/block result and leave behind concrete evidence: files changed, commands run, unresolved blockers, and explicit deferrals.

Recommended single-agent serial order inside batch `02`:

`G6 -> G1 -> G3 -> G4 -> G5 -> G2 -> G7 -> G8`

Reasoning:

- `G6` is a narrow safety cleanup with low conflict risk.
- `G1` removes local type duplication before broader shared-type work.
- `G3` closes backend coverage that makes `G4` safer.
- `G4` activates the main runtime agent gap: dispatcher/tool-loop universality.
- `G5` hardens the newly-active tool path.
- `G2` is cross-crate shared-type work and should happen after the live path is clearer.
- `G7` and `G8` are a foundation-then-propagation pair for temperament.

---

## Document Index

| File | Docs Covered | Items | Status |
|------|-------------|-------|--------|
| [A-core-abstractions.md](A-core-abstractions.md) | 00, 03, 04 (Agent trait, chat types, roles) | A.01-A.16 | 13/16 DONE |
| [B-provider-system.md](B-provider-system.md) | 01, 02, 14 (Registry, adapters, integrations) | B.01-B.21 | 15/21 DONE |
| [C-tool-loop.md](C-tool-loop.md) | 07, 09 (Tool loop, format translation) | C.01-C.40 | 31/40 DONE |
| [D-lifecycle-infrastructure.md](D-lifecycle-infrastructure.md) | 05, 06, 13 (Pools, MCP, creation sites) | D.01-D.14 | 13/14 DONE |
| [E-routing-temperament.md](E-routing-temperament.md) | 08, 10, 11 (Routing, temperament, harness) | E.01-E.19 | 10/19 DONE |
| [F-advanced-capabilities.md](F-advanced-capabilities.md) | 12, 00-adv (Extensibility, composition, metamorphosis) | F.01-F.16 | 12/16 DONE |
| [BATCHES.md](BATCHES.md) | — | 8 batches | Execution contract |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | Verified code anchors | Reference |

Doc 15 (`15-status-gaps.md`) is absorbed into this index. Its gaps map into A, C, D, and E below.

---

## Overall Parity: 93/126 items DONE (74%)

The agent layer is in a different state from `00` and `01`:

- the **provider, translator, tool-loop, safety, MCP, and pool infrastructure is mostly real**,
- but several critical agent abstractions are **duplicated or owned by the wrong crate**,
- and the main orchestration path still **bypasses the strongest agent runtime surfaces**.

### Tier 1 — Should exist now (self-hosting relevant)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| A.08 | Shared response surface still lives in `roko-agent`, not `roko-core` | NOT DONE | HIGH |
| C.17 | ToolDispatcher / ToolLoopAgent not universal on plan-execution path | PARTIAL | HIGH |
| E.18a | `validate before executing` principle bypassed in `orchestrate.rs` | NOT DONE | HIGH |
| E.10 | Typed temperament config does not exist | NOT DONE | HIGH |
| D.14b | Research creation paths bypass scoped safety helpers | PARTIAL | MEDIUM |

### Tier 2 — Should exist soon (operational quality)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| A.05 | Two competing `ChatResponse` structs | DONE (duplication remains) | MEDIUM |
| A.07 | `ResponseMetadata` duplicated with drift | PARTIAL | MEDIUM |
| A.03 | `Usage` missing `model` attribution field | PARTIAL | MEDIUM |
| C.13a | Anthropic HTTP backend coverage incomplete for tool-loop factory | PARTIAL | MEDIUM |
| C.36 | `max_tools` / degrade-cap not enforced | PARTIAL | MEDIUM |
| F.07b | `MetacognitiveMonitor` is wired into `ToolLoop` but not the orchestrator path | PARTIAL | MEDIUM |

### Tier 3 — Future / theoretical (Phase 2+)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| B.14 | `ProviderOptimizations` / `StreamingMode` | NOT DONE | LOW |
| C.30-C.34 | Reflexion / ToT / MCTS / ToolRAG / speculative tool research | NOT DONE | LOW |
| E.15 | Meta-routing | NOT DONE | LOW |
| E.16 | Collapse-avoidance / anti-monoculture | NOT DONE | LOW |
| F.12 | Darwin-Godel / agent archive | NOT DONE | LOW |
| F.14 | Shared agent memory | NOT DONE | LOW |

### Already shipped

| ID | Title | Status |
|----|-------|--------|
| A.01 | Agent trait | DONE (100%) |
| A.04 | Concrete agent implementations | DONE (19 impls vs 7 documented) |
| A.10 | `AgentRole` enum | DONE (28 variants) |
| B.07 | `ProviderAdapter` trait + registry | DONE (6 adapters) |
| B.15-B.19 | Perplexity, Gemini, GLM, Kimi, OpenRouter integrations | DONE |
| C.01-C.12 | Core tool-loop primitives | DONE |
| C.14-C.16 | ToolDispatcher + SafetyLayer | DONE |
| D.07-D.13 | Full MCP discovery / registry / handler pipeline | DONE |
| E.02-E.09 | Cascade router, LinUCB, Pareto, anomaly, active inference, persistence | DONE |
| F.06 | `CompositeAgent` | DONE |
| F.09 | OTP-style supervision strategies | DONE |
| F.11 | `MorphableAgent` | DONE |

---

## Execution Boundaries

Items that are real gaps but should usually be handled in later batches rather than forced into `02`:

| Item | Better Home | Why |
|------|-------------|-----|
| pool wiring into runtime (`AgentPool`, `MultiAgentPool`) | `01-orchestration` follow-on | the owning runtime loop lives there |
| gate strictness semantics by temperament | `04-verification` | gate policy ownership lives there |
| adaptive reward tuning and routing economics | `05-learning` | learning owns policy adaptation |
| domain/plugin scaffolding and prompt-template rollout | `03-composition` | that is composition and domain activation work, not agent runtime hardening |
| concrete feedback collectors (GitHub/Slack/CI) | `05-learning` | collectors matter because of learning ingestion, not agent construction itself |
| supervision-tree runtime recovery wiring | `01-orchestration` | restart behavior is executor-owned even if types live in agent/runtime crates |
| Darwin-Godel / shared agent memory / archive systems | post-parity roadmap | these are Phase 2+ systems, not unattended parity activation work |

Batch `02` should generally produce:

- a canonical shared response surface,
- runtime use of the existing agent safety / tool-loop stack,
- a typed temperament foundation with at least some live behavior,
- and explicit deferrals for broader learning, verification, or orchestration work.

---

## Critical Agent Issues

1. **Shared response types are duplicated and crate-owned incorrectly**. `ChatResponse`, `ResponseMetadata`, and related response concepts are split across `translate/mod.rs` and `chat_types.rs`, while the broader codebase wants them to be usable outside `roko-agent`.
2. **The best agent runtime path is not the main runtime path**. `run.rs` uses `ToolDispatcher`, safety, and scoped agent creation; `orchestrate.rs` still mostly bypasses that stack.
3. **Temperament is still a concept, not a typed runtime input**. There is a free-form string in `AgentIdentity`, but no shared enum, config field, or durable propagation contract.
4. **Creation-site consolidation is close but not complete**. Most paths are migrated, but research entrypoints still bypass scoped safety helpers.
5. **Advanced agent features are mostly “built but not called.”** `MetacognitiveMonitor`, `CompositeAgent`, warrants, and supervision types exist; the main question is which one bounded runtime path should activate them first.

---

## Key Insight

Unlike batch `00`, the agent layer is not missing its major primitives.

Unlike batch `01`, the main problem is not dormant executor state machines.

The core batch-`02` problem is:

**the strongest agent abstractions already exist, but ownership and runtime entrypoints are inconsistent.**

That means the highest-value work here is usually:

1. remove duplicate or misplaced shared types,
2. route more production execution through the existing safety/tool-loop stack,
3. turn “temperament” from a doc concept into a typed runtime contract,
4. avoid widening into learning, verification, or orchestration design work.

---

## Batch 02 Success Definition

Batch `02` is successful when:

- the response surface is canonicalized and no longer split across competing definitions,
- at least one real plan-execution path uses the same dispatcher/safety/tool-loop machinery as the single-run path,
- the remaining direct creation-site safety bypasses are closed,
- temperament exists as a typed config/runtime concept with at least one tested behavioral effect,
- and the more speculative agent ideas are cleanly deferred instead of being half-built.
