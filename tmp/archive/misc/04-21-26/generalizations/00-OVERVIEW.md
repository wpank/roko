# Roko: A Self-Improving Agent Kernel

## What Is Roko?

Roko is an open-source Rust toolkit (~177K lines of code, 18 crates) for building **autonomous agents that improve themselves over time**. It is not a wrapper around an LLM API. It is a complete cognitive architecture — a runtime that gives agents perception, memory, affect, learning, and dreams.

The project exists because every existing agent framework (LangChain, CrewAI, Claude Code, Codex CLI, Cursor, Aider) shares the same fundamental architecture:

```
System prompt + context → LLM API call → Parse tool calls → Execute tools → Loop until done
```

This loop is stateless, memoryless, and maximally expensive. Agent session #1000 is no smarter than session #1. Every tick costs money (full LLM call). There's no learning, no adaptation, no embodied experience. The agent is a function call with extra steps.

Roko replaces this with a fundamentally different model: **agents as long-lived processes** with adaptive heartbeat loops, cognitive gating (80% of ticks cost $0), learnable context assembly, dream-based consolidation, and somatic markers that create "gut feelings" about risky actions.

## History and Lineage

Roko descends from a project called **Bardo** (previously called Mori), which was an orchestration system for autonomous DeFi agents. Bardo had extensive Product Requirement Documents (PRDs) describing a sophisticated cognitive architecture called the **Golem** — a long-lived autonomous entity with:

- A 9-step heartbeat decision cycle running on three timescales
- 28 composable extensions across 7 dependency layers
- Type-state lifecycle enforcement at compile time (impossible to tick a dead agent)
- Economic mortality (finite budget = finite life = pressure to be efficient)
- Dream consolidation (offline pattern discovery between sessions)
- Pheromone-based inter-agent communication

Much of this original vision was **lost during migration** from Bardo to Roko. The current Roko codebase has the individual subsystems built (daimon for affect, neuro for knowledge, dreams for consolidation, conductor for anomaly detection) but they're **wired into a monolithic orchestrator** rather than composed into an agent runtime. Agents in current Roko are spawn-execute-die LLM wrappers — the orchestrator holds all cognitive state.

This document set describes the architecture to **recover and generalize** the original Golem vision, making it work not just for DeFi but for **any domain**: coding, blockchain, research, writing, security, data science, and custom user-defined domains.

## The Core Thesis

> **Agent session #1000 should be categorically better than session #1 — not because the model improved, but because the harness learned.**

This is achieved through five architectural innovations that no other agent framework offers:

| # | Innovation | What It Does | Economic Impact |
|---|---|---|---|
| 1 | **Cognitive Gating** | 80% of agent ticks are pure Rust (no LLM, $0). Only novel situations escalate to expensive reasoning. | 35x cost reduction vs. always-on LLM |
| 2 | **Learnable Context** | Context assembly is a feedback loop. Sections that correlate with success get more budget. | Prompt quality improves autonomously |
| 3 | **Dream Consolidation** | Between sessions, the system replays episodes, discovers patterns, and consolidates into playbooks. | Knowledge compounds at $0 marginal cost |
| 4 | **Somatic Markers** | Actions matching past failures generate hesitation signals — increasing verification or forcing human review. | Safety as a gradient, not binary gates |
| 5 | **Native Rust Runtime** | Zero-overhead tool execution, type-state lifecycle safety, and multi-timescale processing in a single compiled binary. | Sub-millisecond tool calls, compile-time correctness |

## Key Concepts (Glossary)

| Term | Meaning |
|---|---|
| **Engram** | The universal data type. Every signal, event, episode, and message is an Engram. Think of it as a typed, hashable, content-addressed envelope. |
| **Extension** | A composable unit of agent behavior. Implements lifecycle hooks (on_tick, assemble_context, before_tool_call, etc.). Extensions are layered with dependency ordering. |
| **Heartbeat** | The agent's main loop. Runs on configurable frequency (gamma=fast/perception, theta=medium/decision, delta=slow/consolidation). Each tick is a 9-step pipeline. |
| **Cognitive Gating** | The decision of whether a tick needs an LLM call. T0=no (pure Rust, $0), T1=cheap model, T2=full reasoning. Based on prediction error (how surprising is the current state?). |
| **CognitiveWorkspace** | The typed, budgeted, audited context package assembled for each LLM call. Sections have priorities and allocations that evolve based on outcome feedback. |
| **Prediction Error** | How much the observed state differs from what the agent expected. High PE = novel situation = escalate to expensive reasoning. Low PE = familiar = handle deterministically. |
| **CorticalState** | A lock-free (atomic) shared perception surface where extensions read/write signals concurrently without locks. Contains affect, vitality, attention, and perception signals. |
| **Event Fabric** | A broadcast channel (tokio) with a 10K-event ring buffer. Agents subscribe to event categories (chain blocks, file changes, pheromones). Enables reactive behavior. |
| **Daimon** | The affect engine. Implements the ALMA temporal model (Pleasure-Arousal-Dominance). Creates somatic markers — embodied "gut feelings" about actions based on past outcomes. |
| **Neuro** | The durable knowledge store. Six kinds of knowledge (Insight, Heuristic, AntiKnowledge, Warning, CausalLink, StrategyFragment) with tier progression and temporal decay. |
| **Dreams** | Offline consolidation. Replays episodes, generates counterfactuals, rehearses threats, and promotes validated insights into durable knowledge. Triggered by sleep pressure. |
| **Domain Profile** | Configuration that controls agent behavior for a specific domain (coding, blockchain, research, custom). Sets tick frequency, extensions, gates, tools, and event subscriptions. |
| **Type-State Lifecycle** | Compile-time enforcement of valid agent state transitions via Rust's type system. A dead agent literally cannot be ticked — the compiler rejects the code. |
| **Mortality** | (Optional extension) Three death clocks — economic (budget), epistemic (knowledge decay), stochastic (random). Creates behavioral pressure to be efficient and share knowledge. |
| **Playbook** | A validated, reusable pattern extracted from multiple successful episodes. Injected into context at dispatch time for similar future tasks. |
| **Genome** | Compressed knowledge (≤2048 entries) extracted when an agent dies. Inherited by successor agents with confidence discounting. Knowledge transfer across generations. |

## Architecture at a Glance

```
┌─────────────────────────────────────────────────────────────────┐
│                        AGENT RUNTIME                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ HEARTBEAT PIPELINE (9 steps, 3 timescales)               │   │
│  │                                                           │   │
│  │  OBSERVE → RETRIEVE → ANALYZE → GATE → [conditional]     │   │
│  │                                   │                       │   │
│  │                              T0: suppress ($0)            │   │
│  │                              T1: cheap inference           │   │
│  │                              T2: full reasoning            │   │
│  │                                   │                       │   │
│  │                    SIMULATE → VALIDATE → EXECUTE → VERIFY │   │
│  │                                   │                       │   │
│  │                              REFLECT (learn)              │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌─────────────────────────┐  ┌────────────────────────────┐   │
│  │ EXTENSION CHAIN         │  │ EVENT FABRIC               │   │
│  │ (22 hooks, 8 layers)    │  │ (broadcast + ring buffer)  │   │
│  │                         │  │                            │   │
│  │ L0: Heartbeat, Clock    │  │ Chain events               │   │
│  │ L1: Probes, Subscribers │  │ File changes               │   │
│  │ L2: Neuro, Memory       │  │ Agent pheromones           │   │
│  │ L3: Daimon, Attention   │  │ Gate verdicts              │   │
│  │ L4: Tools, Safety       │  │ Timer ticks                │   │
│  │ L5: Pheromones, Chat    │  │ Custom events              │   │
│  │ L6: Dreams, Playbooks   │  │                            │   │
│  │ L7: Recovery, Shutdown  │  │                            │   │
│  └─────────────────────────┘  └────────────────────────────┘   │
│                                                                  │
│  ┌─────────────────────────┐  ┌────────────────────────────┐   │
│  │ CORTICAL STATE          │  │ COGNITIVE WORKSPACE        │   │
│  │ (lock-free atomics)     │  │ (learnable context)        │   │
│  │                         │  │                            │   │
│  │ Affect (PAD)            │  │ Typed sections             │   │
│  │ Vitality                │  │ Priority-based allocation  │   │
│  │ Prediction error        │  │ Feedback loop              │   │
│  │ Attention               │  │ Cache-aligned prefix       │   │
│  │ Perception signals      │  │ Affect modulation          │   │
│  └─────────────────────────┘  └────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ TYPE-STATE LIFECYCLE                                      │   │
│  │                                                           │   │
│  │ Provisioning → Active ↔ Dreaming → Terminal → Dead        │   │
│  │ (compile-time enforced: invalid transitions are errors)   │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│ INFERENCE GATEWAY                                                │
│   L3 Cache (hash) → L2 Cache (semantic) → L1 Cache (prefix)    │
│   → Intent-based provider routing → Backend API                  │
├─────────────────────────────────────────────────────────────────┤
│ DOMAIN PROFILES                                                  │
│   Coding | Blockchain | Research | Docs | Security | Custom      │
│   (tick freq, extensions, gates, tools, event subscriptions)     │
└─────────────────────────────────────────────────────────────────┘
```

## What This Document Set Covers

| Document | Contents |
|---|---|
| **01-ARCHITECTURE.md** | The complete agent runtime design: heartbeat pipeline, extension system, cognitive gating, context engineering, event fabric, type-state lifecycle, inference gateway. Full Rust trait definitions and algorithms. |
| **02-DOMAINS.md** | Domain specialization (blockchain, research, coding, custom), the native harness vs. external harness question, blue ocean competitive analysis, deployment and UX patterns. |
| **03-IMPLEMENTATION.md** | Current state audit, gap analysis (what exists vs. what's needed), crate layout, migration path from the monolithic orchestrator, concrete implementation phases. |

## Current Codebase Structure (For Reference)

Roko is a Cargo workspace with 18 crates:

| Crate | Purpose | Status |
|---|---|---|
| `roko-core` | Signal (Engram) + 6 core traits (Substrate, Scorer, Gate, Router, Composer, Policy) | Stable kernel |
| `roko-agent` | 6+ LLM backends (Claude CLI, Anthropic API, OpenAI, Gemini, Perplexity, Ollama), tool loop, safety | Working |
| `roko-runtime` | ProcessSupervisor, event bus, cancellation | Needs redesign → AgentRuntime |
| `roko-gate` | 11 gates, 7-rung pipeline, adaptive thresholds | Working |
| `roko-compose` | Prompt assembly, 9 templates, enrichment | Working (needs CognitiveWorkspace) |
| `roko-learn` | Episodes, playbooks, bandits, model routing, experiments | Working |
| `roko-daimon` | Affect engine (ALMA PAD model), somatic markers | Working (orchestrator-side) |
| `roko-neuro` | Durable knowledge store, tier progression, distillation | Working (partial) |
| `roko-dreams` | Offline consolidation (replay, imagination, rehearsal) | Working (basic cycle) |
| `roko-conductor` | 10 anomaly watchers, circuit breaker, stuck detection | Working |
| `roko-primitives` | HDC vectors (10,240-bit), tier routing | Working |
| `roko-chain` | ChainClient/ChainWallet traits, MEV gate, tx simulation | Built (not running) |
| `roko-cli` | CLI binary: all subcommands, ratatui TUI, orchestrator | Main entry (19K LOC monolith) |
| `roko-serve` | HTTP control plane (~85 routes on :6677) | Working |
| `roko-agent-server` | Per-agent HTTP sidecar | Working |
| `roko-std` | 19 builtin tools, mock dispatcher | Stable |
| `roko-fs` | FileSubstrate (JSONL persistence), GC, layout | Stable |
| `roko-plugin` | Plugin manifest, event sources, feedback collectors | Built |

The workspace compiles with `cargo build --workspace` (requires Rust 1.91+ for alloy dependencies).
