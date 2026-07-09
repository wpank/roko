# Vision and Core Thesis

> Roko is a Rust toolkit for building cognitive agents that build themselves. Its core
> thesis — "the scaffold IS the product" — asserts that given the same LLM, agent
> performance varies dramatically based on the surrounding harness: context engineering,
> verification, learning loops, and cognitive architecture. Roko is that scaffold, made
> composable and domain-agnostic.

**Status**: Shipping
**Crate**: `roko-core` (kernel) — see [`reference/11-crate-map.md`](../reference/11-crate-map.md) for the full picture
**Last reviewed**: 2026-04-17

---

## TL;DR

Most AI agent frameworks chain LLM calls. Roko provides the cognitive architecture that wraps
those calls: memory that decays, affect that modulates compute allocation, verification at every
step, and offline consolidation that transforms execution experience into reusable knowledge.
The core self-hosting loop works today — Roko reads its own product requirements, generates
plans, dispatches agents to execute them, validates outputs through an 11-gate pipeline, and
learns from outcomes. Every improvement to Roko makes the system that builds Roko better. That
feedback loop is the business.

---

## 1. The Problem: LLMs Are Necessary but Not Sufficient

Large language models can generate code, reason about strategy, and produce natural-language
plans. But the frontier of agent capability is not determined by the LLM alone. Empirical
evidence across multiple benchmarks demonstrates that the *harness* — the system wrapping the
LLM — accounts for a large and growing share of agent performance.

### 1.1 Empirical Evidence

- **SWE-bench Verified** (Jimenez et al. 2024): The same base model (Claude Sonnet 3.5)
  achieves between 30% and 65% solve rates depending on the harness wrapping it. The
  difference is not the model — it is the scaffolding. A 2× performance spread from
  harness design alone is the canonical demonstration that scaffolding is a first-class
  engineering concern.

- **AlphaCode** (Li et al. 2022): DeepMind achieved competitive programming results not
  through a single enormous model but through a compound system of generation, filtering,
  clustering, and selection. The harness architecture was the innovation. The model was
  necessary but insufficient.

- **Meta-Harness** (Lee et al. 2026, arXiv:2603.28052): Demonstrates +7.7 points on text
  classification and +4.7 points on IMO-level math from harness optimization alone, at 4×
  fewer tokens. This is the most direct empirical evidence that scaffolding is a first-class
  performance lever, measurable in points, not in vague qualitative terms.

- **FrugalGPT** (Chen et al. 2023, arXiv:2305.05176): Shows that cascading through models
  of increasing capability — routing easy queries to cheap models and hard queries to
  expensive ones — can match GPT-4 quality at 2% of the cost. The routing harness is the
  product. The insight is that which model runs matters far less than what routes to it.

- **DSPy** (Khattab et al. 2024): Replaces hand-written prompts with a compiler that
  optimizes prompt pipelines automatically. Performance improvements come not from bigger
  models but from better prompt assembly — a scaffold concern. DSPy shows that prompts
  are a learnable, optimizable artifact, not configuration to be hand-tuned once.

- **Compound AI Systems** (Zaharia et al. 2024): The Berkeley AI Research thesis that
  state-of-the-art results increasingly come from systems of multiple components rather
  than from individual models. The system architecture is the differentiator. Building the
  best component in isolation is the wrong race; building the best system is the right one.

The pattern is clear: the scaffold IS the product. Given any fixed LLM, the quality of the
surrounding context engineering, verification pipeline, learning loops, model routing, and
cognitive architecture determines agent performance. Roko exists to be that scaffold —
composable, domain-agnostic, and self-improving.

---

## 2. The Scaffold Thesis

Roko's core thesis can be stated precisely:

> **Given the same LLM, agent performance varies dramatically based on the surrounding
> harness — context engineering, verification, learning loops, cognitive architecture.
> Roko is that scaffold, made composable.**

This thesis has three concrete implications.

### 2.1 Scaffolding Is a Durable Competitive Moat

LLM capabilities improve rapidly. Any advantage based on model access is temporary — today's
frontier model is tomorrow's commodity. The scaffold wrapping that model, however, compounds
its value over time:

- **Context engineering** improves as the agent learns which information to include in prompts
  and which to omit. This is a data advantage that deepens with usage. An agent that has
  assembled 10,000 prompts has learned what works for its domain; this knowledge does not
  transfer to a competitor who builds a fresh harness.

- **Verification pipelines** become more calibrated as gate thresholds adapt to observed
  pass rates. A harness that has verified 10,000 tasks has better-calibrated gates than one
  that has verified 10. Gate calibration is a function of experience, not of the underlying
  model.

- **Learning loops** extract heuristics, playbook rules, and skill libraries from execution
  episodes. These accumulate and transfer across agent lifetimes. A playbook built from 1,000
  successful coding sessions encodes engineering judgment that took a human team years to
  develop.

- **Model routing** improves as contextual bandits learn which models work best for which
  task types. Routing quality is a function of experience, not of model capability. Once a
  router has learned that Claude Haiku handles symbol-lookup reliably and Sonnet handles
  architecture questions, the system extracts near-frontier quality at commodity cost.

Each of these scaffold components improves independently of the underlying LLM. When the LLM
is upgraded, the scaffold's accumulated knowledge amplifies the new model's capabilities.
This is the compound advantage Roko provides.

### 2.2 Composability Is the Design Requirement

If the scaffold is the product, the scaffold must be composable. Different users need
different combinations of scaffold capabilities for different domains:

- A **coding agent** needs compilation gates, test harnesses, and diff-aware context assembly.
- A **research agent** needs citation verification, source credibility scoring, and knowledge
  synthesis.
- A **chain agent** needs transaction simulation, on-chain state grounding, and position
  monitoring.
- A **document agent** needs style consistency gates, version diffing, and corpus-aware
  retrieval.

No two domains share the same gate suite, context assembly strategy, or learning signal.
Roko achieves composability through the [Synapse Architecture](../reference/README.md): one
data medium (`Engram`) and six verb traits (`Substrate`, `Scorer`, `Gate`, `Router`,
`Composer`, `Policy`). A domain-specific agent assembles its scaffold by implementing the
traits relevant to its domain and composing them in the universal cognitive loop.

### 2.3 Self-Improvement Is the Terminal Goal

The scaffold thesis points toward a system that improves itself. If scaffolding determines
performance and scaffolding accumulates knowledge through use, then a scaffold that learns
from its own operation compounds its improvement over time. Roko's self-hosting loop makes
this concrete:

1. Roko reads its own product requirements (`roko prd`)
2. Roko generates implementation plans (`roko plan`)
3. Roko dispatches Claude agents to execute them (`roko plan run`)
4. The 11-gate verification pipeline validates outputs
5. The learning subsystems extract heuristics from outcomes
6. Improved heuristics feed into future plan generation

Every improvement to Roko makes the system that builds Roko better. This is not a thought
experiment — it is the active development workflow as of 2026-04-17.

---

## 3. The Synapse Architecture

Roko's internal architecture is called the **Synapse Architecture**. The name captures the
design philosophy: individual components (neurons) are simple, but their combination
(synaptic connections) produces complex cognition.

### 3.1 Two Mediums

All information in the system flows through exactly two mediums:

| Medium | Status | Description |
|---|---|---|
| `Engram` | Shipping | Durable, content-addressed record. Persisted to `Substrate`. BLAKE3 hash identity. 7-axis scored. Four decay models. Lineage DAG for chain-of-thought. |
| `Pulse` | Planned | Ephemeral event stream. Routed through `Bus`. Short-lived, not persisted by default. Replaces `EventBus<E>` in target state. |

This medium split — durable vs. ephemeral — is the foundational architectural decision. It
separates the concerns of *what the agent knows* (Engram/Substrate) from *what is happening
right now* (Pulse/Bus), enabling independent scaling, different decay semantics, and
different consistency guarantees for each.

### 3.2 Two Fabrics

The mediums move through two fabrics:

| Fabric | Status | Description |
|---|---|---|
| `Substrate` | Shipping | Storage fabric. CRUD + similarity search on `Engram` records. JSONL `FileSubstrate` ships today; pluggable backends for vector DBs, S3, and distributed stores. |
| `Bus` | Planned | Transport fabric. Routes `Pulse` events through `Topic` handles. Current implementation: `EventBus<E>`. |

### 3.3 Six Operators (the "Synapse" verbs)

Six composable trait objects coordinate the system. Together they are the *Synapse* — the
six verbs that operate on the two mediums across the two fabrics:

| Trait | Domain | Status | Role |
|---|---|---|---|
| `Substrate` | Storage | Shipping | Persist and retrieve `Engram` records |
| `Scorer` | Appraisal | Shipping | Compute 7-axis `Score` for any `Engram` |
| `Gate` | Verification | Shipping | Binary accept/reject verdict on agent output |
| `Router` | Dispatch | Shipping | Select model, agent, or execution path |
| `Composer` | Context | Shipping | Assemble system prompts and context windows |
| `Policy` | Control | Shipping | Encode behavioral rules and safety constraints |

Any agent behavior, from code generation to chain interaction, is an implementation of these
six traits. This forces composability: every new capability is a new `Gate` or `Composer`
or `Policy`, not a special-purpose code path.

### 3.4 The Universal Cognitive Loop

Every agent execution in Roko follows the same eight-step cognitive loop:

```
query → score → route → compose → act → verify → persist → react
```

This loop is not a convention — it is the structure that makes learning possible. By
following the same sequence for every task, the system accumulates comparable experience
across domains. A heuristic learned from a code-generation task ("route architecture
questions to Sonnet, not Haiku") generalizes across every task type that hits the router.

### 3.5 Three Cognitive Speeds

The cognitive loop runs at three distinct temporal scales, inspired by neural oscillation
bands:

| Speed | Name | Period | Role |
|---|---|---|---|
| Fast | Gamma (γ) | ~5–15 seconds | Reactive execution: a single LLM call, tool dispatch, immediate gate |
| Medium | Theta (θ) | ~75 seconds | Reflective synthesis: multi-step reasoning, context assembly, plan evaluation |
| Slow | Delta (δ) | Hours | Consolidation: offline learning, knowledge compression, playbook construction |

The three-speed design prevents slow consolidation work from blocking fast reactive work,
and prevents fast reactive noise from polluting slow consolidated knowledge.

### 3.6 Five Dependency Layers

Roko's codebase is organized into five layers with strictly downward dependencies:

| Layer | Name | Primary Crates |
|---|---|---|
| L0 | Runtime | `roko-runtime`, `roko-core` |
| L1 | Framework | `roko-agent`, `roko-gate`, `roko-fs`, `roko-neuro` |
| L2 | Scaffold | `roko-compose`, `roko-learn`, `roko-daimon`, `roko-dreams` |
| L3 | Harness | `roko-conductor`, `roko-chain`, `roko-std`, `roko-serve` |
| L4 | Orchestration | `roko-orchestrator`, `roko-cli` |

No crate depends on one above it in this stack. This constraint is enforced by the workspace
dependency graph and validated in CI.

### 3.7 Three Cross-Cuts

Three subsystems span multiple layers via trait injection:

| Cross-cut | Status | Role |
|---|---|---|
| `Neuro` | Built | Durable knowledge: 6 knowledge types × 4 validation tiers, encoded as HDC vectors for sub-millisecond similarity search. The agent's long-term memory. |
| `Daimon` | Built | Affect engine: PAD vectors modulate model tier selection, exploration rate, and compute allocation. Public alias: `AffectBias`. Implements somatic marker hypothesis. |
| `Dreams` | Scaffold | Delta-speed consolidation: NREM replay, REM imagination, and slow-consolidation learning that transforms execution episodes into persistent knowledge. |

---

## 4. Design Principles Overview

Seven principles govern every architectural decision in Roko. They are expanded in
[`reference/12-design-principles.md`](../reference/12-design-principles.md).

1. **The scaffold IS the product** — The harness, not the model, is the differentiated asset.
2. **Nothing is lost** — Every execution leaves a learning signal; no work is discarded.
3. **Composability over completeness** — Ship six general traits, not 60 domain-specific APIs.
4. **Gate failure is a verdict, not an error** — Verification is first-class, not defensive.
5. **Memory decays; knowledge compounds** — Short-term data expires; long-term knowledge grows.
6. **Speeds must not bleed** — Gamma, Theta, and Delta operate independently; no cross-band blocking.
7. **The system must be able to read its own requirements** — Self-hosting is not aspirational; it is a current operational capability.

---

## 5. Self-Improvement Loops

Roko's architecture enables multiple compounding self-improvement loops operating
simultaneously:

### 5.1 The Heuristic Loop (Theta-speed)

Every agent turn extracts a learning signal:
- Did the gate pipeline pass or fail?
- Which model tier produced the best outcome?
- Which context assembly strategy minimized hallucinations?

These signals feed the playbook (extractable rules), the cascade router (bandit update),
and the cost tracker (efficiency accounting). Each subsequent task benefits from all prior
experience.

### 5.2 The Consolidation Loop (Delta-speed)

Offline, `Dreams` processes the episode log:
- **NREM replay**: Replays high-utility episodes to reinforce effective strategies.
- **REM imagination**: Generates counterfactual scenarios to build robustness without
  additional real execution cost.
- **Slow consolidation**: Compresses working knowledge into persistent `Neuro` entries.

The result is that the agent's knowledge base improves even when the agent is not actively
executing tasks.

### 5.3 The Self-Hosting Loop (orchestration-speed)

Roko reads its own product requirements, generates plans for improving itself, and executes
those plans using its own agent infrastructure. A successful plan improvement feeds back as
a new Engram, contributing to the knowledge base that future plans query. This is not a
metaphor — as of 2026-04-17, the active development workflow for Roko codebase runs on Roko.

### 5.4 The Collective Loop (multi-agent)

When multiple Roko agents share a `Substrate`, their Engrams become accessible to each
other. A successful strategy learned by one agent propagates to others through the shared
knowledge base. The C-Factor metric (collective intelligence quotient) measures how much
faster the fleet solves problems together than any single agent solves them alone.

---

## 6. Current Implementation Status

As of 2026-04-17 (per [`status/status.md`](status.md)):

- **~322K Rust LOC** across 36 workspace members
- **3,761 test functions** across the workspace
- **Self-hosting loop**: operational — `roko prd` → `roko plan run` → gate → persist → resume
- **Shipping**: Core Engram/Synapse layer, Orchestrator, Agents (5 backends), Composition,
  Verification (11 gates), Learning, Safety, CLI/TUI/HTTP interfaces, Standard tools
- **Built (not yet wired)**: Neuro, Conductor, Chain/Korai, Daimon
- **Scaffold**: Dreams, Heartbeat, Coordination, Lifecycle
- **Deferred**: Chain deployment, Identity economy, advanced Deployment

---

## 7. The Pitch in One Paragraph

Every AI lab is racing to build better models. Roko bets that the model is not the moat.
The moat is the scaffold: the context engineering that knows what to put in front of the
model, the verification that knows whether what came out is correct, the learning loops that
accumulate judgment no single model run could develop on its own, and the cognitive
architecture that makes all of it composable across any domain. Roko is that scaffold,
written in Rust, running in production today, and improving with every task it completes.

---

## See Also

- [`reference/12-design-principles.md`](../reference/12-design-principles.md) — Full design principles with rationale
- [`reference/11-crate-map.md`](../reference/11-crate-map.md) — Which crate ships which concept
- [`research/frontier-summary.md`](../research/frontier-summary.md) — How Roko maps to modern agent-systems research
- [`status/executive-summary.md`](executive-summary.md) — One-page pitch for investors and technical buyers
- [`GLOSSARY.md`](../GLOSSARY.md) — Canonical A-Z vocabulary

## Open Questions

- How does the scaffold thesis hold as models begin to internalize context engineering
  (e.g., via long context windows and chain-of-thought training)?
- What is the right metric for "scaffold quality" comparable to SWE-bench for models?
- When does the collective loop saturate — at what fleet size does shared-Substrate
  knowledge stop producing marginal gains?
