# Unified Specification — Index

> **Version**: 2.0
> **Date**: 2026-04-26
> **Scope**: Protocol specification for the agent economy. Defines the vocabulary, composition rules, and behavioral contracts for Roko — a system where agents build themselves.

---

## What This Specification Is

This specification defines **standards for an agent economy**: persistent identity, trustless coordination, and composable computation. The design peers are Stripe (payment protocol), Ethereum (execution protocol), and ERC-20 (token standard) — not LangGraph, CrewAI, or AutoGen.

Three technical bottlenecks stand between today's agent demos and a functioning agent economy (cf. Sequoia Capital, "The Agentic Web," 2026):

| Bottleneck | How This Spec Addresses It |
|---|---|
| **Persistent identity** | ERC-8004 agent identities + HDC fingerprints (Kanerva 2009) + ZK attestation (PP-HDC) |
| **Agent communication protocol** | MCP (tool discovery, Linux Foundation) + A2A (agent cards, `/.well-known/agent-card.json`) + Bus (ephemeral pub/sub) + stigmergic coordination (Dorigo 1992) |
| **Trust without face-to-face** | ZK proofs over HDC vectors + TraceRank reputation + demurrage-weighted knowledge (Gesell 1916) with on-chain provenance |

The four external protocols — **MCP**, **A2A**, **ERC-8004**, **x402** — form a fixed exoskeleton. This spec defines what flows through them.

---

## Vocabulary

Roko is built from **3 fundamentals**, **9 protocols**, and **10 specializations**. A developer learns 12 core concepts; everything else is a discoverable pattern composed from those 12.

### 3 Fundamentals

| Primitive | What It Is |
|---|---|
| **Signal** | The durable data unit. Content-addressed (SHA-256), typed (`Kind`), scored (5 axes), decayed via demurrage (Gesell 1916), lineage-tracked (`Vec<SignalRef>` provenance DAG), HDC-fingerprinted (10,240-bit, Kanerva 2009). Lives in Store. Its ephemeral sibling **Pulse** (sequence-numbered, ring-buffered) lives on Bus. **Graduation** converts Pulse → Signal — the only path from transport to audit DAG. |
| **Cell** | Atomic computation. Signals in, Signals out. Declares typed I/O (`TypeSchema`), capabilities (fail-closed intersection), and protocol conformance. Every Cell is a learner via **predict-publish-correct**: it publishes predictions as Pulses, subscribes to its own error topic, and updates (Friston 2006, active inference). |
| **Graph** | Composition of Cells wired by typed edges. TOML-defined, serializable, runtime-interpreted. **Hot Graphs** stay resident and re-fire per tick (agent pipelines). The **Workflow/Activity split** (cf. Temporal) separates deterministic orchestration from non-deterministic execution for replay. |

### 9 Protocols

Every protocol supports **predict-publish-correct** (§ Design Principles). A Cell publishes its prediction as a Pulse, reality publishes the outcome, a `CalibrationPolicy` joins by lineage and computes error, and the Cell subscribes to its error topic to update. Learning is structural.

| Protocol | Verb | Existing Trait | Key Extension |
|---|---|---|---|
| **Store** | put / get / query / query_similar / prune | `Substrate` | `query_similar` for native HDC search (<1 ms over 800K vectors) |
| **Score** | rate Signal along 5 dimensions | `Scorer` | Predicts quality; corrected by gate verdicts via calibration |
| **Verify** | check Signal → Verdict | `Gate` | Pre-action `verify_pre()` can veto. Continuous reward `Verdict.reward: f64`. Evidence typed separately from Criterion. Conjunctive hard (AND) + Pareto soft (multi-objective, never weighted-sum — Goodhart-resistant). Pairwise Bradley-Terry judges with disjoint-family panels |
| **Route** | select among candidates | `Router` | Expected Free Energy (EFE, Friston) replaces LinUCB. Regime conditioning (`Calm/Normal/Volatile/Crisis`). Each cognitive timescale = different free-energy lower bound |
| **Compose** | combine Signals under budget → one Signal | `Composer` | VCG auction (Vickrey-Clarke-Groves) with 8+ context bidders. Section effect tracking via beta-distribution posteriors. Novelty attenuation: `1/(1+ln(freq))` |
| **React** | watch Pulse stream, emit Signals | `Policy` | **Breaking change**: operates on Pulses (ephemeral), not Signals. `ReactOutput { pulses, signals }` |
| **Observe** | read-only view → observation Signals | *New* (Lens) | StateHub projections. `CollectiveIntelligenceLens` computes c-factor (Woolley et al. 2010) |
| **Connect** | connect / query / execute / disconnect | `Connector` | External I/O lifecycle |
| **Trigger** | listen for events, fire Graphs | *New* | Declarative event ingress |

### 10 Specializations

| Name | Built From | Key Innovation |
|---|---|---|
| **Flow** | Graph at runtime + RunId + snapshots | **Hot Flow**: stays resident, re-fires per tick |
| **Rack** | Graph + Macros (knobs) + Slots (jacks) | DAW-inspired parameterization |
| **Trigger** | Cell + Trigger protocol | Declarative event ingress |
| **Lens** | Cell + Observe protocol | Read-only; StateHub projections |
| **Loop** | Graph with feedback edge | All 4 learning loops are Loops |
| **Memory** | Store Cell + demurrage + dreams | Demurrage replaces Ebbinghaus; Heuristic kind with mandatory falsifier |
| **Space** | Isolation boundary + capability grants | Multi-workspace with cross-workspace knowledge sharing |
| **Extension** | Cell intercepting another Cell's pipeline | 8 layers, 22 hooks, CaMeL IFC (capability-tagged information flow) |
| **Agent** | Space + Extensions + Memory + clock + vitality | Type-state lifecycle (`Provisioning → Active ↔ Dreaming → Terminal`), vitality scalar, behavioral phases, CorticalState (lock-free atomics), multi-slot state, somatic markers (PAD + prospect theory, Kahneman-Tversky λ=2.2), CognitiveWorkspace (VCG + section effects) |
| **Connector** | Cell + Connect protocol + lifecycle | External system I/O |

---

## Design Principles

### 1. Two mediums, two fabrics

Durable **Signals** in **Store**, ephemeral **Pulses** on **Bus**. Both are kernel-level (L0). The system has two data shapes because reality has two timescales: things that persist and things that flow. **Graduation** converts Pulse → Signal (the only path into the audit DAG). **Projection** converts Signal → Pulse (lossy broadcast). The code already had both shapes — `Engram` and `Envelope<E>` in `roko-runtime::event_bus` — but the v1 spec claimed one noun.

### 2. Every operator is a learner

**Predict-publish-correct** via Bus (Friston 2006, active inference made structural). Every Cell publishes its prediction as a Pulse on `prediction.{operator}`, subscribes to `calibration.{operator}.updated`, and adjusts. A `CalibrationPolicy` in `roko-learn` joins predictions with outcomes by `lineage_hint`, computes error, and publishes updates. Learning is not a separate subsystem bolted onto execution — it emerges from the same pub/sub fabric that carries heartbeats and gate verdicts.

### 3. Demurrage is default

Signals decay via attention-weighted holding cost (Gesell 1916), not pure time-based Ebbinghaus. Balance starts at 1.0, decreases unless actively reinforced by retrieval, citation, surprise, or gate-pass. Novelty-weighted reinforcement: citing a rare Signal (low HDC similarity to neighbors) gives a larger bonus than citing a common one. Cold threshold archives to slower storage. Ebbinghaus is recovered as the special case where no interactions occur. Self-trimming knowledge: unique insights stay warm, duplicates fade.

### 4. Mortality is a feature

Agents have finite **vitality** (`remaining_budget / initial_budget`) creating behavioral phases: Thriving (1.0–0.7) → Stable (0.7–0.4) → Conservation (0.4–0.2) → Declining (0.2–0.05) → Terminal (<0.05). Economic pressure drives efficient resource use, knowledge transfer before death, and honest self-assessment. An Agent that has never faced resource pressure has never learned to prioritize. (Cf. Jonas 1966 on mortality as precondition for value.)

### 5. Verify is load-bearing

The Verify protocol serves four roles simultaneously:
1. **Reward function** — continuous `Verdict.reward: f64` alongside binary pass/fail
2. **Relabeling oracle** — hindsight relabeling of failed trajectories for achieved sub-goals (recovers ≥45% of discarded episodes)
3. **Safety boundary** — pre-action `verify_pre()` can veto execution
4. **Economic attestation** — reputation flows from verified work via ERC-8004

All four learning loops depend on Verify. Conjunctive hard criteria (AND) + Pareto soft criteria (multi-objective, never weighted-sum) resist Goodhart's Law. Evidence is typed separately from Criterion (19 evidence kinds). The **Variance Inequality** mandates that the verifier is spectrally cleaner than the generator — no LLM judging itself.

### 6. Collective intelligence is measurable

The **c-factor** (Woolley et al. 2010, *Science*: 40%+ of group performance variance loads onto a single factor driven by turn-taking equality and social perceptiveness, not mean IQ) is computed from Bus and Store statistics: turn-taking entropy, peer prediction accuracy, citation reciprocity, Bus delivery rate, HDC diversity. c-factor is a Lens output that gates L4 evolution: only evolve configurations that increase genuine collective intelligence. It is a **covariate, not an objective** — optimizing c directly can be gamed.

### 7. Elegance through composition

Everything composes from 3 fundamentals + 9 protocols. An Agent is Space + Extensions + Memory + clock. A dream cycle is a Loop. Adaptive thresholds are a Loop. The cascade router is a Loop. The system learns using Graphs, not bespoke infrastructure. No special machinery. 36 old concepts collapsed to 12 without losing expressiveness.

### 8. Cost falls mechanically with volume

Wright's-law on rails — cost-per-decision falls as a structural property of the primitives, not as an optimization afterthought:
- **Route** = EFE balances quality vs cost naturally
- **Compose** = budget-constrained VCG auction
- **Verify** = cost attribution per Cell per Graph per Agent
- **Observe** (`CostLens`) = real-time cost telemetry
- **Demurrage** = even memory has cost pressure
- **Semantic caching** = content-addressed Signals maximize reuse across Flows
- **T0 gating** = ~80% of agent ticks cost $0 (pure Rust pattern matching, no LLM)

Stacked: caching (5×) × routing (3×) × gating (2×) = **10–30× cost reduction** vs naïve always-call-Opus baseline. Savings convert to more usage (Jevons paradox), not status-quo savings.

### 9. Protocol, not framework

Signal/Cell/Graph are standards. Third parties build on these because not adopting is more expensive than adopting. Each new conforming Cell multiplies combinations with every existing Cell, Graph, and Signal channel (ERC-20 precedent: $11.4T DEX volume from a handful of standards). The spec defines what flows through MCP, A2A, ERC-8004, and x402. It does not replace them.

### 10. The spec is a runtime artifact

These documents are:
- **Readable by agents at startup** — injected into system prompts during L4 evolution
- **Queryable as MCP tools** — an MCP server serves spec sections, protocol definitions, vocabulary
- **Evolvable through L4** — the evolutionary archive can propose spec amendments (human-reviewed)
- **Signed under ERC-8004** — each spec version has verifiable provenance

The formal structure (protocols, type definitions, invariants) is machine-parseable.

### 11. Safety scales with autonomy

Six levels from observe-only to structural changes. **CaMeL** capability-tagged information flow control (IFC) on Extensions prevents capability laundering. **Nayebi 5-head lexicographic corrigibility**: deference > switch > truth > impact > task — each a separate Verify Cell, the ordering is lexicographic (not weighted), so deference ALWAYS trumps task performance. Verify gates sit **outside the modifiable surface** — the agent cannot modify its own verification pipeline.

---

## Anti-Principles

Patterns that have failed — do not repeat:

| Anti-Pattern | Evidence | Response |
|---|---|---|
| Standalone destination app | Sora D30 retention <8%; Humane/Rabbit failed vs smartphones | Embed in existing surfaces |
| Naïve multi-agent debate | Homogeneous debate = majority vote in expectation | Require heterogeneity + structured indirection (Surowiecki 2004 conditions) |
| Opaque marketplace economics | GPT Store median creator earned <$100/quarter | Publish all metrics, transparent take-rates, creators own customers |
| "We have the most data" moat | Pure data network effects ~98% mythical (Towson) | Protocol + workflow embedding + cross-side marketplace |
| Weighted-sum verification | Goodhart's Law | Conjunctive hard + Pareto soft |
| LLM-judging-itself | Variance Inequality: verifier must be spectrally cleaner than generator | Verify is external and heterogeneous |
| Token speculation narrative | Detracts from utility | ERC-8004 agent identity and utility, not token price |

---

## Five Compounding Mechanisms

These take the system from linear to exponential returns:

1. **Protocol composability** (ERC-20 precedent). Each new conforming Cell multiplies combinations with every existing Cell, Graph, and Signal channel. Value grows combinatorially with ecosystem size.

2. **Reed's-law group formation**. Stigmergic coordination lets agent coalitions form without central permission. Corrected (Briscoe-Odlyzko: real value ∝ N·log(N) due to Dunbar limits) still outpaces Metcalfe (N²) once groups form.

3. **Wright's-law cost curve**. LLM inference prices fell 9–900× per year by task. The primitives ensure cost-per-decision falls mechanically with volume and converts savings into more usage (Jevons paradox).

4. **Knowledge compounding with attribution**. Each interaction adds to a corpus all future agents can query. HDC fingerprinting + ERC-8004 identity + demurrage = compounding semantic memory with cryptographic provenance. Avoids Stack Overflow's failure mode (contributor incentives degraded → 76% user drop).

5. **Recursive self-improvement**. L4 makes the OS itself an agent in the evolutionary archive. The spec evolves through use. Bounded by Variance Inequality (verifier spectrally cleaner than generator) and c-factor gate (only evolve when collective intelligence improves).

---

## Vocabulary Table

| Concept | What It Is | Primary Doc |
|---|---|---|
| Signal (durable) | Content-addressed, lineage-bearing, scored, decayed via demurrage, persisted in Store | 01 |
| Pulse (ephemeral) | Sequence-numbered, ring-buffered, broadcast via Bus, lives ~seconds | 01 |
| Graduation | Pulse → Signal. The only path from transport to audit DAG | 01 |
| Bus | Transport fabric — ephemeral pub/sub. Kernel-level alongside Store | 01 |
| Demurrage | Attention-weighted retention replacing pure time decay (Gesell 1916) | 01, 11 |
| Heuristic | Signal kind: when/then clause + mandatory falsifier + calibration track record | 01, 11 |
| AntiKnowledge | Signal kind that actively repels future Signals in the same HDC region | 01, 11 |
| Predict-publish-correct | Every operator predicts → publishes → receives corrections via Bus (Friston 2006) | 02, 10 |
| Pre-action Verify | `verify_pre()` — check before execution, can veto | 02 |
| Continuous reward | `Verdict.reward: f64` — domain-specific learning signal | 02 |
| Evidence typing | `EvidenceCollector` separate from `Criterion`, 19 evidence kinds | 02 |
| Conjunctive/Pareto | Hard criteria (AND) vs soft criteria (multi-objective Pareto, never weighted-sum) | 02 |
| Bradley-Terry judges | Pairwise comparison aggregated via BT MLE, disjoint-family panels | 02 |
| EFE gating | Expected Free Energy (Friston) for T0/T1/T2 and L2 routing | 02, 07, 10 |
| Regime conditioning | Route receives `regime: Signal` (Calm/Normal/Volatile/Crisis) | 07, 10 |
| VCG auction | Vickrey-Clarke-Groves for budget-constrained context assembly | 02, 07 |
| Section effect | Beta-distribution tracking: which context sections correlate with gate success | 07 |
| Hot Graph | Tick-driven Flow that stays resident between firings | 03, 05 |
| Workflow/Activity split | Deterministic orchestration vs non-deterministic execution (cf. Temporal) | 05 |
| Vitality | remaining_budget / initial_budget — economic pressure scalar | 07 |
| Behavioral phases | Thriving / Stable / Conservation / Declining / Terminal | 07 |
| Type-state lifecycle | Compile-time enforced Agent state transitions | 07 |
| CorticalState | Lock-free atomic shared perception surface (sub-μs concurrent reads) | 07 |
| Multi-slot state | Agent manages N concurrent named slots with shared global limits | 07 |
| Somatic markers | PAD affect (Pleasure/Arousal/Dominance) + prospect theory (Kahneman-Tversky λ=2.2) + k-d tree queries (<100μs) | 07 |
| CognitiveWorkspace | Learnable context assembly via VCG + section effect tracking | 07 |
| Novelty attenuation | `1/(1+ln(freq))` — habituation that never reaches zero | 07 |
| Hindsight relabeling | Failed trajectories relabeled for achieved sub-goals (≥45% recovery) | 10 |
| Clade-Metaproductivity (HGM) | Score variants by descendant performance in evolutionary archive | 10 |
| Verify-as-reward | Verify protocol as reward function for self-play | 10 |
| Variance Inequality | Verifier must be spectrally cleaner than generator | 10 |
| c-factor | Collective intelligence as runtime observable (Woolley et al. 2010) | 09, 10 |
| Resonator Networks | HDC factorization — recover constituents from bundles | 11 |
| Worldview | Coherent cluster of co-citing heuristics with high calibration scores | 11 |
| CaMeL IFC | Capability-tagged information flow control on Extensions | 17 |
| 5-head corrigibility | Lexicographic: deference > switch > truth > impact > task (Nayebi) | 17 |
| StateHub projections | Universal typed projections for all surfaces (TUI/web/Slack/audit) | 09 |
| Exoskeleton protocols | MCP + A2A + ERC-8004 + x402 as fixed external protocol layer | 12 |
| Domain profile (full) | Complete cognitive posture: clock + extensions + events + context weights + gates + infra | 14 |
| Package tiers | 5-tier SPI: prompts → config → declarative tools → WASM → native Rust | 14 |
| Arena | Universal measurement surface + 7-step flywheel | 19 |
| Meta-arena | Roko developing itself as an Arena | 19 |
| Finality oracle | Per-transaction confidence for multi-chain (Final/QuasiFinalized/Reversible) | 12 |
| Brain export | Portable agent knowledge via Merkle-CRDT merge (~100KB–1MB) | 20 |
| Workbench | Structured task delegation surface (Linear/Notion pattern, not blank chat) | 16 |
| Agent Inbox | Ambient notify/question/review surface (calm technology) | 16 |
| Generative Canvas | Visual Graph editor with typed cables (Bitwig/n8n pattern) | 16 |
| Stigmergy Minimap | RTS-style coordination visualization (fog-of-war, group selection) | 16 |
| Autonomy Slider | Progressive trust control (5 levels, per-capability granularity) | 16 |
| Spec-as-artifact | Specification readable, queryable, and evolvable by agents at runtime | 00, 10 |

---

## Reading Order

### Essential — learn the type system
1. **[01-SIGNAL.md](01-SIGNAL.md)** — Two mediums: Signal and Pulse. Bus and Store. Demurrage. HDC.
2. **[02-CELL.md](02-CELL.md)** — Universal computation. 9 protocols. Predict-publish-correct.
3. **[03-GRAPH.md](03-GRAPH.md)** — Universal composition. Hot Graphs. Workflow/Activity split.
4. **[04-SPECIALIZATIONS.md](04-SPECIALIZATIONS.md)** — 10 patterns. Conventions, not new primitives.

### Core systems
5. **[05-EXECUTION-ENGINE.md](05-EXECUTION-ENGINE.md)** — Runtime, failure strategies, Hot Graph execution, resumability.
6. **[06-TRIGGER-SYSTEM.md](06-TRIGGER-SYSTEM.md)** — Event-driven Graph firing. Pulses on Bus.
7. **[07-AGENT-RUNTIME.md](07-AGENT-RUNTIME.md)** — Vitality, type-state, CorticalState, EFE, somatic markers, CognitiveWorkspace.
8. **[08-EXTENSION-SYSTEM.md](08-EXTENSION-SYSTEM.md)** — 8 layers, 22 hooks, CaMeL IFC.

### Observation and learning
9. **[09-TELEMETRY.md](09-TELEMETRY.md)** — Lens system, StateHub projections, c-factor.
10. **[10-LEARNING-LOOPS.md](10-LEARNING-LOOPS.md)** — 4 loops, predict-publish-correct, hindsight relabeling, L4 self-evolution.

### Knowledge and connectivity
11. **[11-MEMORY-AND-KNOWLEDGE.md](11-MEMORY-AND-KNOWLEDGE.md)** — Demurrage, Heuristics, Resonator Networks, dreams.
12. **[12-CONNECTIVITY.md](12-CONNECTIVITY.md)** — Exoskeleton (MCP/A2A/ERC-8004/x402), relay, multi-chain.

### Developer experience
13. **[13-BUILTIN-BLOCK-CATALOG.md](13-BUILTIN-BLOCK-CATALOG.md)** — 45+ shipped Cells by protocol.
14. **[14-CONFIG-AND-AUTHORING.md](14-CONFIG-AND-AUTHORING.md)** — 5-tier SPI, domain profiles, workspace scoping.
15. **[15-MARKETPLACE-AND-SHARING.md](15-MARKETPLACE-AND-SHARING.md)** — Economics, DAW composability, fork chains.
16. **[16-SURFACES.md](16-SURFACES.md)** — 5 named surfaces as protocol-level data contracts.
17. **[17-SECURITY-MODEL.md](17-SECURITY-MODEL.md)** — CaMeL IFC, 5-head corrigibility, capability intersection.

### Implementation
22. **[22-PLAN-RUNNER-V2.md](22-PLAN-RUNNER-V2.md)** — Plan runner rewrite spec. Replaces orchestrate.rs (21K lines) with ~2,400-line event-driven runner using existing crate APIs.

### Infrastructure
18. **[18-ON-CHAIN-REGISTRIES.md](18-ON-CHAIN-REGISTRIES.md)** — ERC-8004 agent identities, ZK-HDC proofs, InsightStore.
19. **[19-ARENAS-EVALS-BOUNTIES.md](19-ARENAS-EVALS-BOUNTIES.md)** — Arena flywheel, meta-arena.
20. **[20-DEPLOYMENT.md](20-DEPLOYMENT.md)** — WASM + native, brain export, Merkle-CRDT.
21. **[21-ROADMAP.md](21-ROADMAP.md)** — Phased implementation.

---

## Naming Decisions

| Decision | Rationale |
|---|---|
| **Signal** (not Engram) | Immediately meaningful. Rust struct remains `Engram`; `type Signal = Engram;` bridges. Spec name is Signal; code name is Engram. |
| **Pulse** (not Envelope/Event) | Names the ephemeral sibling. Code has `Envelope<E>` in `roko-runtime::event_bus`; "Pulse" is the canonical name. |
| **Bus** (not EventBus) | Transport fabric promoted to kernel trait alongside Store. Two fabrics, not one. |
| **Cell** (not Module) | Composable, small, pluggable — Eurorack module, Scratch block. |
| **Graph** (not Workflow) | Precise: nodes + edges. "Flow" is the runtime instance. "Workflow" implies BPM. |
| **Rack** (not Parameterized Workflow) | Modular synthesis: macro knobs and patch jacks for the performer. |
| **Lens** (not Monitor/Probe) | Read-only by definition. Stacking gives different views. |
| **Loop** (not Feedback/Cycle) | Direct and unambiguous. A Graph feeding output back to input. |
| **Space** (not Workspace) | "Workspace" is overloaded (IDE, OS). "Space" is clean: isolation + grants. |
| **Demurrage** (not Decay) | "Decay" implies passive time loss. "Demurrage" implies active economic mechanism — use restores value. Gesell 1916. |
| **Vitality** (not Budget/Lifetime) | Captures both economic pressure and behavioral modulation. |
| **Heuristic** (not Playbook/Rule) | Testable prediction with calibration track record + mandatory falsifier. Richer than a rule; more formal than a playbook. |

---

## Concept Migration

### From roko-core code

| Code | Spec | Notes |
|---|---|---|
| `Engram` | Signal (durable) | Direct map. Rust struct stays `Engram`. |
| `Envelope<E>` / EventBus | Pulse / Bus | Promoted from impl detail to kernel. |
| `Substrate` | Store protocol | Identical + `query_similar`. |
| `Scorer` | Score protocol | + calibration. |
| `Gate` | Verify protocol | + pre/post, continuous reward, evidence typing, conjunctive/Pareto. |
| `Router` | Route protocol | + EFE, regime conditioning. |
| `Composer` | Compose protocol | + VCG, section effects. |
| `Policy` | React protocol | **Breaking**: operates on Pulses. |

### From architecture docs

| Old | New | Notes |
|---|---|---|
| Extension (22 hooks, 8 layers) | Extension specialization | + CaMeL IFC. |
| Connector (5 methods) | Connector specialization | Connect protocol + lifecycle. |
| Feed | Pulse stream on Bus | No separate primitive. |
| Knowledge Entry | Signal (Memory) | + demurrage, + Heuristic kind. |
| Pheromone | Pulse (typed) | Ephemeral coordination Signal. |
| DreamCycle | Loop specialization | + hindsight relabeling phase. |
| CorticalState | Agent internal | Lock-free atomics. |
| DomainProfile | Domain profile (full) | Clock + extensions + events + context weights + gates + infra. |
| TickPipeline | Agent's 9-step Hot Graph | Is a Graph, executed by same engine. |

---

## Document Status

All 22 documents are version 2.0 drafts, written from scratch incorporating all source material (53 documents across refinements, research, visual-gate2, run-anywhere, DeFi gap analysis, 04-21-26 generalizations/PRDs, and workflow specs).
