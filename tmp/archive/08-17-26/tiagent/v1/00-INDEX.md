# tiagent: Master Index

## Executive Summary

tiagent is a **general-purpose, self-improving coding agent harness** --- a direct
alternative to Claude Code, Codex, Cursor, and Windsurf. It is written in Rust,
model-agnostic (works with any LLM: Claude, GPT, Gemini, Ollama, local models), and
designed to get better the more you use it. Run single tasks, execute multi-step plans
with parallel agent dispatch and automated quality gates, or drive entire PRD-to-code
workflows autonomously. tiagent works as a standalone local coding agent with no external
dependencies beyond an LLM.

What makes tiagent different from other coding agents is **collective self-improvement**.
Through optional integration with Celestia's data availability (DA) layer, tiagent instances
can share learning artifacts --- routing weights, efficiency patterns, behavioral
fingerprints, successful strategies --- so the entire network of agents gets smarter, not
just yours. This is not a prerequisite: tiagent works perfectly well without Celestia. But
with it, every agent execution contributes to a shared, verifiable, append-only memory layer
that enables a new class of collaborative agents --- ones that improve not just from their
own experience, but from every agent's experience on the network.

tiagent is composable (it speaks MCP, A2A, AITP, and x402), open source, and not locked
into any vendor's ecosystem.

---

## Document Suite

| # | Document | Description |
|---|----------|-------------|
| 00 | **00-INDEX.md** | This document --- master index, reading order, and project orientation. |
| 01 | **01-vision-and-overview.md** | What tiagent is, why it exists, who it is for (developers first), everyday use cases, and how it compares to Claude Code, Codex, Cursor, and prior art. |
| 02 | **02-architecture.md** | Core architecture: the universal loop, trait system, signal model, and runtime structure. Applies to all users --- Celestia is one pluggable substrate. |
| 03 | **03-crate-structure.md** | Rust workspace layout, crate dependency graph, and rationale for each crate's scope. |
| 04 | **04-celestia-integration.md** | How tiagent integrates with Celestia: blob submission, namespace management, light node embedding, and fee economics. |
| 05 | **05-da-storage-patterns.md** | Patterns for storing vector embeddings, HDC fingerprints, agent state snapshots, and shared learning artifacts on Celestia's DA layer. |
| 06 | **06-tool-system.md** | Tool calling architecture, MCP server/client integration, and the built-in Celestia developer tool suite. |
| 07 | **07-tracecommons-integration.md** | Integration with TraceCommons for trace quality scoring, trajectory retrieval-augmented generation (RAG), and cross-agent learning. |
| 08 | **08-ironclaw-integration.md** | Integration with IronClaw for WASM-sandboxed tool execution, TEE-backed agent isolation, and verifiable compute. |
| 09 | **09-interop-protocols.md** | Protocol interoperability: MCP (Model Context Protocol), A2A (Agent-to-Agent), AITP (AI Transfer Protocol), and x402 (paid API access). |
| 10 | **10-design-patterns.md** | Catalog of design patterns used throughout tiagent: composition strategies, error handling, state management, and extension points. |
| 11 | **11-self-improving-loop.md** | The cybernetic self-improvement loop: how agents get better with use --- cascade routing, adaptive gates, playbook extraction, efficiency tracking, and optional shared learning via DA. |
| 12 | **12-prd-core-harness.md** | Product Requirements Document for the core agent harness MVP --- the standalone coding agent that works without Celestia. |
| 13 | **13-prd-celestia-native.md** | Product Requirements Document for the Celestia integration layer --- DA storage, namespace tooling, light node embedding, shared learning. |
| 14 | **14-on-chain-agent-survey.md** | Survey of existing on-chain agent frameworks (Eliza, Rig, polkagent, ARC, Zerebro, etc.) with comparison matrix. |
| 15 | **15-deep-research-queries.md** | Structured research queries for further investigation into open technical questions. |
| 16 | **16-grant-proposals.md** | Near-submission-ready grant proposals (Celestia Foundation, Interchain Foundation, Modular Fellows/Mammothon) with strategy and supporting materials. |

---

## How to Read This

The documents are numbered in a logical dependency order, but different audiences will want
different paths through the material. Pick the reading order that matches your role:

### For general software developers

Start here if you want to use tiagent as a coding agent --- writing code, running plans,
automating development workflows. No blockchain knowledge needed.

1. **00-INDEX.md** (this document) --- orient yourself
2. **01-vision-and-overview.md** --- understand what tiagent is and how it compares to Claude Code, Codex, Cursor
3. **02-architecture.md** --- learn the universal loop and trait system (applies to all usage, not just Celestia)
4. **11-self-improving-loop.md** --- understand how tiagent gets better the more you use it
5. **06-tool-system.md** --- understand tool calling and MCP integration
6. **12-prd-core-harness.md** --- review the standalone agent harness MVP scope
7. **10-design-patterns.md** --- patterns for extending tiagent
8. *(Optional)* **04-celestia-integration.md** --- if you want to enable shared learning via DA

### For executives and product stakeholders

Start here for the "what" and "why" before diving into any technical detail.

1. **00-INDEX.md** (this document) --- orient yourself
2. **01-vision-and-overview.md** --- understand what tiagent is and why it exists
3. **14-on-chain-agent-survey.md** --- see how tiagent compares to alternatives
4. **12-prd-core-harness.md** --- review the MVP scope and milestones
5. **13-prd-celestia-native.md** --- review the Celestia-specific feature roadmap

### For Rust engineers building tiagent

Start with the vision, then move into architecture and implementation details.

1. **00-INDEX.md** (this document) --- orient yourself
2. **01-vision-and-overview.md** --- understand goals and design philosophy
3. **02-architecture.md** --- learn the core abstractions (signals, traits, universal loop)
4. **03-crate-structure.md** --- understand the workspace layout and where code lives
5. **10-design-patterns.md** --- learn the patterns you will use daily
6. **06-tool-system.md** --- understand tool calling and MCP integration
7. **04-celestia-integration.md** --- learn how Celestia DA is wired in
8. **05-da-storage-patterns.md** --- understand what goes on-chain and how
9. **11-self-improving-loop.md** --- understand the cybernetic feedback loop

### For Celestia ecosystem developers

Start with why Celestia, then understand the integration surface.

1. **00-INDEX.md** (this document) --- orient yourself
2. **01-vision-and-overview.md** --- understand what tiagent brings to Celestia
3. **04-celestia-integration.md** --- detailed Celestia integration design
4. **05-da-storage-patterns.md** --- how agent data maps to blobs and namespaces
5. **06-tool-system.md** --- Celestia-specific developer tools
6. **09-interop-protocols.md** --- how tiagent interoperates with other systems

### For agent/AI researchers

Start with the self-improvement loop and shared learning mechanisms.

1. **00-INDEX.md** (this document) --- orient yourself
2. **01-vision-and-overview.md** --- understand the vision for shared agent learning
3. **11-self-improving-loop.md** --- the cybernetic self-improvement architecture
4. **07-tracecommons-integration.md** --- trace quality and trajectory RAG
5. **05-da-storage-patterns.md** --- vector stores and HDC fingerprints on DA
6. **08-ironclaw-integration.md** --- verifiable compute and sandboxed execution
7. **15-deep-research-queries.md** --- open questions worth investigating

### For integration engineers

Start with the interop surface and work inward.

1. **00-INDEX.md** (this document) --- orient yourself
2. **01-vision-and-overview.md** --- understand goals and composability philosophy
3. **09-interop-protocols.md** --- MCP, A2A, AITP, x402 protocol support
4. **06-tool-system.md** --- tool calling and MCP server/client architecture
5. **08-ironclaw-integration.md** --- IronClaw runtime integration
6. **07-tracecommons-integration.md** --- TraceCommons integration surface

---

## Terminology

A few terms appear throughout the document suite. They are defined fully in
**02-architecture.md**, but here is a quick glossary for orientation:

| Term | Meaning |
|------|---------|
| **Signal** | The universal data type --- every piece of information flowing through tiagent is a Signal (a content-addressed, typed, scored datum). |
| **Gate** | A quality check that runs after an agent completes a task --- compilation, tests, linting, diff review. Gates ensure agent output actually works. |
| **Plan / Plan DAG** | A directed acyclic graph of tasks that tiagent executes, potentially in parallel. Plans can be generated from PRDs or written manually. |
| **PRD** | Product Requirements Document --- a spec that tiagent can read and automatically generate implementation plans from. |
| **Cascade router** | The model selection system that learns which LLM backend works best for which task type, improving routing over time. |
| **Playbook** | A reusable strategy extracted from successful agent runs --- tool call sequences, prompt patterns, task decompositions. |
| **Episode** | A structured trace of an agent's execution: every turn, tool call, and outcome. Used for learning and audit. |
| **DA layer** | Data Availability layer --- in Celestia's architecture, this is the layer that stores blob data and makes it retrievable. tiagent can optionally use it as shared agent memory. |
| **Blob** | A binary large object submitted to Celestia. When DA integration is enabled, agents write traces, embeddings, and state as blobs. |
| **Namespace** | Celestia's partitioning mechanism for blobs. tiagent uses namespaces to organize agent data by type and purpose. |
| **HDC fingerprint** | A Hyperdimensional Computing vector that compactly represents an agent's behavioral signature or a task's semantic identity. |
| **MCP** | Model Context Protocol --- Anthropic's open standard for connecting LLMs to tools and data sources. tiagent implements both MCP client and server. |
| **A2A** | Agent-to-Agent protocol --- Google's standard for agent interoperability. |
| **AITP** | AI Transfer Protocol --- Near's standard for AI-to-AI communication. |
| **x402** | HTTP 402-based protocol for paid API access, enabling agents to pay for and sell services. |
| **TraceCommons** | A system for scoring trace quality and enabling trajectory retrieval-augmented generation across agents. |
| **IronClaw** | A WASM/TEE runtime for sandboxed, verifiable agent execution. |
| **Cybernetic loop** | The self-improvement cycle: observe performance, identify gaps, generate plans, execute improvements, validate results, repeat. |
| **Universal loop** | The core execution pattern: query, score, route, compose, act, verify, write, react. |

---

## Project Status

tiagent is in the **design phase**. These documents constitute the initial design
specification. No code has been written yet. The documents define the architecture,
integration points, and implementation plan that will guide the first code drops.

---

## Related Projects

tiagent does not exist in isolation. It competes with, draws on, and integrates with
several other projects:

### Coding agent alternatives

tiagent competes directly with these tools for everyday software development:

| Tool | How tiagent differs |
|------|---------------------|
| **Claude Code** | Locked to Anthropic models. No self-improvement loop --- same behavior on run 1 and run 1000. No plan DAGs, no gates, no shared learning. |
| **Codex (OpenAI)** | Locked to OpenAI models. Single-task execution only --- no multi-step plans, no PRD workflows, no persistent learning. |
| **Cursor** | IDE-coupled, closed-source, subscription-locked. No agent orchestration, no plan execution, no self-improvement. |
| **Windsurf** | Similar to Cursor --- IDE-coupled, no autonomous plan execution, no learning loop, no open-source extensibility. |

tiagent is model-agnostic (use any of the above providers plus local models), open source,
self-improving, and supports autonomous multi-step plan execution with quality gates.

### Architectural influences

| Project | Relationship |
|---------|-------------|
| **roko** | A Rust toolkit for self-building agents (~177K LOC, 18 crates). tiagent's architecture is inspired by roko's universal loop, signal model, and trait system, but targets a more minimal design. |
| **polkagent** | A 90-crate Rust workspace for building agents on Polkadot. tiagent takes a similar "natively on-chain" approach but for Celestia, and aims for a smaller crate count. |

### Integration targets

| Project | Relationship |
|---------|-------------|
| **Celestia** | The modular blockchain whose DA layer tiagent optionally integrates with. Celestia separates data availability from execution and consensus, providing cheap, verifiable blob storage for shared agent learning. |
| **TraceCommons** | A trace quality and trajectory RAG system. tiagent integrates TraceCommons to enable shared learning across agents. |
| **IronClaw** | A WASM/TEE agent runtime. tiagent can run inside IronClaw for sandboxed, verifiable execution. |
