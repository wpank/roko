# Roko Agent Architecture — Design Documents

## Overview

These documents describe the architecture for **Roko**, an open-source Rust toolkit
for building autonomous agents that improve themselves over time. They are self-contained
and assume no prior context about the project.

## Documents

| File | Contents | Length |
|---|---|---|
| [00-OVERVIEW.md](00-OVERVIEW.md) | What is Roko, history, core thesis, glossary, architecture diagram | Context & orientation |
| [01-ARCHITECTURE.md](01-ARCHITECTURE.md) | Full runtime design: heartbeat pipeline, extension system, cognitive gating, context engineering, event fabric, type-state lifecycle, inference gateway, native tool loop | Technical specification |
| [02-DOMAINS.md](02-DOMAINS.md) | Domain specialization (blockchain, research, coding, custom), native harness design, blue ocean competitive analysis, deployment & UX | Domain & strategy |
| [03-IMPLEMENTATION.md](03-IMPLEMENTATION.md) | Current state audit, gap analysis, crate layout, 5-phase migration path, testing strategy, risk assessment | Implementation roadmap |

## Reading Order

1. Start with **00-OVERVIEW.md** for context and glossary
2. Read **01-ARCHITECTURE.md** for the technical design
3. Read **02-DOMAINS.md** for domain-specific applications and competitive positioning
4. Read **03-IMPLEMENTATION.md** for the concrete build plan

## Key Architectural Decisions

1. **Universal runtime, domain profiles** — One AgentRuntime trait handles all domains. Profiles control frequency, extensions, gates, and tools.
2. **Extension-based composition** — 22 hooks across 8 layers. Behavior comes from layered extensions, not monolithic code.
3. **Cognitive gating** — 80% of ticks cost $0 (T0 deterministic). Only novel situations escalate to LLM reasoning.
4. **Learnable context** — Context assembly is a feedback-loop control system. Prompt quality improves autonomously.
5. **Type-state lifecycle** — Invalid agent state transitions are compile errors, not runtime checks.
6. **Mortality as optional extension** — Economic pressure to be efficient. Configurable per domain.

## Legacy Documents (Superseded)

The following files in this directory are the original research notes used to develop
these consolidated documents. They can be removed or kept for reference:

- `01-current-state.md` → merged into 03-IMPLEMENTATION.md §1
- `02-golem-vision.md` → merged into 00-OVERVIEW.md + 01-ARCHITECTURE.md
- `03-gap-analysis.md` → merged into 03-IMPLEMENTATION.md §2
- `04-agent-runtime-design.md` → merged into 01-ARCHITECTURE.md
- `05-domain-specialization.md` → merged into 02-DOMAINS.md
- `06-extension-model.md` → merged into 01-ARCHITECTURE.md §3 + 02-DOMAINS.md §5
- `07-native-harness-design.md` → merged into 02-DOMAINS.md §6-7
