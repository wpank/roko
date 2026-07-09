# Unified Specification — Index

> **Version**: 2.0 draft
> **Date**: 2026-04-26
> **Scope**: Protocol specification for the agent economy. Defines the vocabulary, composition rules, and behavioral contracts that third parties adopt because not adopting is more expensive.

---

## Purpose

This specification defines **standards for the agent economy** — persistent identity, trust without face-to-face, and a TCP/IP-equivalent for agent communication. The peer set is Stripe-the-protocol, Ethereum-the-protocol, ERC-20-the-standard — not LangGraph or CrewAI.

Signal/Block/Graph + HDC + ERC-8004 address the three technical bottlenecks (Sequoia, 2026) for the agent economy:

1. **Persistent identity** — ERC-8004 passports + HDC fingerprints + ZK attestation.
2. **Agent communication equivalent to TCP/IP** — MCP (tools) + A2A (agent discovery) + Bus (ephemeral transport) + stigmergic coordination.
3. **Trust without face-to-face** — ZK proofs over HDC vectors + TraceRank reputation + demurrage-weighted knowledge with on-chain provenance.

---

## Vocabulary

Roko is built from **3 fundamentals**, **9 protocols**, and **10 specializations**. A developer learns 12 core concepts; everything else is a discoverable specialization.

### 3 Fundamentals

| Primitive | What It Is |
|---|---|
| **Signal** | The durable data unit. Content-addressed, typed (Kind), scored, decaying via demurrage, lineage-tracked, HDC-fingerprinted. Lives in Store. Its ephemeral sibling **Pulse** lives on Bus. Graduation converts Pulse → Signal (the only path from transport to audit DAG). |
| **Block** | Atomic computation. Takes Signals in, produces Signals out. Declares capabilities and protocol conformance. Every operator predicts → publishes → receives corrections via Bus. |
| **Graph** | Composition of Blocks wired by typed edges. TOML-defined, serializable. Runtime interprets it. Hot Graphs stay resident and re-fire per tick. |

### 9 Protocols (Block interfaces)

Every protocol supports the **predict-publish-correct** pattern: a Block publishes its prediction as a Pulse, reality publishes the outcome, a CalibrationPolicy joins by lineage and computes error, and the Block subscribes to its own error topic to update itself. Learning is structural, not a separate subsystem.

| Protocol | Verb | Existing Trait | Notes |
|---|---|---|---|
| `Store` | put / get / query / prune Signals | `Substrate` | Durable fabric. |
| `Score` | rate Signal along dimensions | `Scorer` | Predicts quality; corrected by gate verdicts. |
| `Verify` | check Signal against truth → Verdict | `Gate` | Pre-action (`verify_pre`) and post-action (`verify_post`). Continuous reward `Verdict.reward: f64`. Conjunctive hard + Pareto soft (never weighted-sum). Evidence typed separately from Criterion. |
| `Route` | select among candidates, learn from outcome | `Router` | EFE (Expected Free Energy) replaces LinUCB for T0/T1/T2 gating and L2 routing. Receives `regime: Signal` for context-aware selection. |
| `Compose` | combine Signals under budget → one Signal | `Composer` | VCG auction with 8+ bidders. Section effect tracking via beta-distribution posteriors. Budget-constrained. |
| `React` | watch Pulse stream, emit new Signals | `Policy` | Operates on Pulses (ephemeral), not Signals. |
| `Observe` | read-only view, emit observation Signals | **New** (Lens) | StateHub projections consumed by all surfaces. c-factor as Lens output. |
| `Connect` | connect / query / execute / disconnect | `Connector` | External I/O lifecycle. |
| `Trigger` | listen for events, fire Graphs | **New** | Declarative event ingress. |

### 10 Specializations (built on fundamentals)

| Name | Definition | Source |
|---|---|---|
| **Flow** | Graph at runtime (RunId, snapshot, events). Hot variant stays resident. | Workflow execution |
| **Rack** | Graph with exposed Macros (knobs) + Slots (jacks) | Parameterized Workflow |
| **Trigger** | Block with Trigger protocol | Trigger system |
| **Lens** | Block with Observe protocol | Telemetry |
| **Loop** | Graph that feeds output back to input | DreamCycle, adaptive gates, cascade router |
| **Memory** | Store-protocol Block with demurrage + dreams | Knowledge store, neuro |
| **Space** | Graph isolation boundary + capability grants | Workspace |
| **Extension** | Block that intercepts another Block's pipeline | Extension trait (8 layers) |
| **Agent** | Space + Extensions + Memory + adaptive clock + vitality | AgentRuntime |
| **Connector** | Block with Connect protocol + lifecycle | Connector trait |

---

## Design Principles

1. **Two mediums, two fabrics.** Durable Signals in Store, ephemeral Pulses on Bus. Both are kernel-level. Graduation converts Pulse → Signal; projection converts Signal → Pulse (lossy). The system has two data shapes because reality has two timescales: things that persist and things that flow.

2. **Every operator is a learner.** Predict-publish-correct via Bus. Every Block publishes its prediction, subscribes to its error topic, and updates. Learning is not a separate subsystem bolted onto execution — it is structural, emerging from the same pub/sub fabric that carries heartbeats and gate verdicts.

3. **Demurrage is default.** Signals decay via attention-weighted holding cost unless actively used. Retrieval, citation, surprise, and gate-pass restore balance. Cold threshold freezes and archives. Self-trimming knowledge where unique insights stay warm and duplicates fade. Ebbinghaus is recovered as the special case where no interactions occur.

4. **Mortality is a feature.** Agents have finite vitality (`remaining_budget / initial_budget`) that creates behavioral phases: Thriving → Stable → Conservation → Declining → Terminal. Economic pressure drives efficient resource use and knowledge transfer. An Agent that has never faced resource pressure has never learned to prioritize.

5. **Verify is load-bearing.** It is the reward function (continuous `Verdict.reward`), the relabeling oracle (hindsight on failed trajectories), the safety boundary (pre-action `verify_pre`), and the economic attestation (reputation flows from verified work). All four learning loops depend on it. Conjunctive hard criteria (AND) + Pareto soft criteria (multi-objective, never weighted-sum) resist Goodhart's Law. Evidence is typed separately from Criterion.

6. **Collective intelligence is measurable.** The c-factor — computed from turn-taking entropy, peer prediction accuracy, citation reciprocity, HDC diversity — is a runtime observable that gates evolutionary decisions. Only evolve configurations that increase genuine collective intelligence.

7. **Elegance through composition.** Everything composes from 3 fundamentals + 9 protocols. No special machinery. An Agent is Space + Extensions + Memory + clock. A dream cycle is a Loop. Adaptive thresholds are a Loop. The system learns using Graphs, not bespoke infrastructure.

8. **Cost falls mechanically with volume.** Wright's-law on rails — not an optimization, but a structural property. Route = cost-aware model selection (EFE balances quality vs cost). Compose = budget-constrained VCG auction. Verify = cost attribution per Block. Observe (CostLens) = real-time cost telemetry. Demurrage = memory has cost pressure. Semantic caching = content-addressed Signals maximize reuse. T0 gating = 80% of ticks cost $0 (pure Rust pattern matching). Stacked: caching (5×) × routing (3×) × gating (2×) = 10-30× cost reduction vs naïve baseline.

9. **Protocol, not framework.** Signal/Block/Graph are standards for the agent economy. Third parties build on these because not adopting is more expensive than adopting. Each new conforming Block multiplies combinations with every existing Block, Graph, and Signal channel. The spec defines what flows through MCP, A2A, ERC-8004, and x402 — it does not replace them.

10. **The spec is a runtime artifact.** These documents are readable by agents at startup (system prompt context), queryable as MCP tools, evolvable through L4 (the evolutionary archive can propose spec amendments with human review), and signed under ERC-8004. The specification's formal structure is machine-parseable.

11. **Safety scales with autonomy.** Six levels from observe-only to structural changes. CaMeL capability-tagged information flow control on Extensions. Nayebi 5-head lexicographic corrigibility: deference > switch > truth > impact > task. Verify gates sit outside the modifiable surface. Parameters have declared ranges. Structural changes require human review.

### Anti-Principles (patterns that have failed — do not repeat)

- **No standalone destination app** — embed in existing surfaces. Sora D30 retention <8%. Humane/Rabbit failed against smartphones.
- **No naïve multi-agent debate** — requires heterogeneity + structured indirection. Homogeneous debate = majority vote in expectation.
- **No opaque marketplace economics** — GPT Store median creator earned <$100/quarter. Publish all metrics, transparent take-rates, creators own customers.
- **No "we have the most data" moat claims** — pure data network effects are ~98% mythical (Towson). Lean on protocol + workflow embedding + cross-side marketplace.
- **No weighted-sum verification** — Goodhart's Law. Use conjunctive hard + Pareto soft.
- **No LLM-judging-itself** — Variance Inequality: verifier must be spectrally cleaner than generator. Verify must be external and heterogeneous.

---

## Five Compounding Mechanisms

These take the system from linear to exponential:

1. **Protocol composability** (ERC-20 precedent: $11.4T DEX volume from a few standards). Any conforming Block composes with every existing Block, every Graph, every Signal channel. Each new Block multiplies combinations, not adds them.

2. **Reed's-law group formation** (2^N from ad-hoc coalitions). Stigmergic coordination lets agent coalitions form without central permission. Corrected (Briscoe-Odlyzko: real value ∝ N·log(N) due to Dunbar limits) still outpaces Metcalfe once groups form.

3. **Wright's-law cost curve** (LLM inference prices fell 9-900× per year by task). Cost-per-decision falls mechanically with volume and converts savings into more usage (Jevons paradox), not status-quo savings.

4. **Knowledge compounding with attribution** (each interaction adds to corpus all future agents can query). HDC fingerprinting + ERC-8004 identity + demurrage = compounding semantic memory with cryptographic provenance. Avoids Stack Overflow's failure mode (contributor incentives degraded → 76% drop).

5. **Recursive self-improvement on the OS itself** (DGM + AlphaEvolve precedent). L4 makes the OS itself an agent in the evolutionary archive. The spec evolves through use. Variance Inequality ensures the verifier is spectrally cleaner than the generator.

---

## Vocabulary Table

| Concept | What It Is | Where |
|---|---|---|
| Signal (durable) | Content-addressed, lineage-bearing, scored, decayed via demurrage, persisted | Doc-01 |
| Pulse (ephemeral) | Sequence-numbered, ring-buffered, broadcast via Bus | Doc-01 |
| Graduation | Pulse → Signal (the only path from transport to audit DAG) | Doc-01 |
| Bus | Transport fabric — ephemeral pub/sub alongside Store | Doc-01 |
| Pre-action Verify | `verify_pre()` — check before execution, can veto | Doc-02 |
| Continuous reward | `Verdict.reward: f64` — domain-specific learning signal | Doc-02 |
| Evidence typing | EvidenceCollector separate from Criterion, typed kinds | Doc-02 |
| Conjunctive/Pareto | Hard criteria (AND) vs soft criteria (multi-objective Pareto) | Doc-02 |
| Predict-publish-correct | Every operator predicts → publishes → receives corrections via Bus | Doc-02 |
| Hot Graph | Tick-driven Flow that stays resident between firings | Doc-05 |
| Workflow/Activity split | Deterministic orchestration vs non-deterministic execution | Doc-05 |
| Vitality | remaining_budget / initial_budget — economic pressure scalar | Doc-07 |
| Behavioral phases | Thriving / Stable / Conservation / Declining / Terminal | Doc-07 |
| Type-state lifecycle | Compile-time enforced Agent state transitions | Doc-07 |
| CorticalState | Lock-free atomic shared perception surface | Doc-07 |
| Multi-slot state | Agent manages N concurrent slots with shared limits | Doc-07 |
| EFE gating | Expected Free Energy for T0/T1/T2 and L2 routing | Doc-07, Doc-10 |
| Regime conditioning | Route receives regime Signal for context-aware selection | Doc-07, Doc-10 |
| Somatic markers | PAD affect + prospect theory + k-d tree queries (<100μs) | Doc-07 |
| CognitiveWorkspace | Learnable context assembly via VCG + section effect tracking | Doc-07 |
| Section effect | Beta-distribution tracking context → gate success correlation | Doc-07 |
| Novelty attenuation | `1/(1+ln(freq))` — habituation that never reaches zero | Doc-07 |
| Demurrage | Attention-weighted retention replacing pure time decay | Doc-11 |
| Heuristic | First-class Signal kind with when/then + mandatory falsifier | Doc-11 |
| Resonator Networks | HDC factorization — recover constituents from bundles | Doc-11 |
| Hindsight relabeling | Failed trajectories relabeled for achieved sub-goals | Doc-10 |
| Clade-Metaproductivity | Score variants by descendant performance | Doc-10 |
| Verify-as-reward | Verify protocol as reward function for self-play | Doc-10 |
| Variance Inequality | Verifier spectrally cleaner than generator | Doc-10 |
| c-factor | Collective intelligence as runtime observable via PID | Doc-09 |
| CaMeL IFC | Capability-tagged information flow control on Extensions | Doc-17 |
| 5-head corrigibility | Lexicographic safety: deference > switch > truth > impact > task | Doc-17 |
| Domain profile (full) | Complete cognitive posture: clock + extensions + events + gates + infra | Doc-14 |
| Package tiers | 5-tier extensibility with progressive capability/isolation | Doc-14 |
| Arena | Universal measurement surface + 7-step flywheel | Doc-19 |
| Meta-arena | Roko developing itself as an Arena | Doc-19 |
| Finality oracle | Per-transaction confidence for multi-chain operation | Doc-12 |
| Workspace scope | Multi-workspace isolation with cross-workspace knowledge sharing | Doc-14 |
| Brain export | Portable agent knowledge via Merkle-CRDT merge (~100KB-1MB) | Doc-20 |
| StateHub projections | Universal typed projections for all surfaces (TUI/web/audit) | Doc-09 |
| Exoskeleton protocols | MCP + A2A + ERC-8004 + x402 as the fixed external protocol layer | Doc-12 |
| Workbench surface | Structured task delegation (not blank chat) | Doc-16 |
| Agent Inbox surface | Ambient notify/question/review (calm technology) | Doc-16 |
| Generative Canvas | Visual Graph editor with typed cables | Doc-16 |
| Stigmergy Minimap | RTS-style coordination visualization | Doc-16 |
| Autonomy Slider | Progressive trust control with per-capability granularity | Doc-16 |
| Spec-as-artifact | The specification is readable, queryable, and evolvable by agents | Doc-00, Doc-10 |
| Protocol composability | Each new Block multiplies combinations (ERC-20 precedent) | Doc-00 |
| Wright's-law cost curve | Cost-per-decision falls mechanically with volume | Doc-00 |
| Knowledge compounding | Each interaction adds to corpus all future agents query | Doc-00 |

---

## Concept Migration Table

Every concept from the previous specs maps to the unified vocabulary. Nothing is lost.

### From Workflow Specs (tmp/workflow/)

| Old Concept | Unified Equivalent | Notes |
|---|---|---|
| Module | Block | Block is the generalization. |
| Workflow | Graph | Static definition; Flow is the runtime instance. |
| Artifact | Signal (persisted) | Content-addressed, versioned, lineage-tracked. |
| Macro | Macro (on Rack) | Scoped to the Rack specialization. |
| Slot | Slot (on Rack) | Scoped to the Rack specialization. |
| TypeSchema | TypeSchema (on Block) | Unchanged. |
| Capability | Capability (on Block) | Three-layer intersection model preserved. |
| Expr | Expr (on Graph edges) | Small, total, deterministic expression language. |
| Evidence | Signal (kind: Evidence) | Evidence is a Signal kind, not a separate primitive. |
| EvidenceBag | Signal set | A collection of evidence Signals. |
| Profile | Rack (verification) | A visual-gate2 Profile is a Rack computing a Verdict. |
| Finding | Signal (kind: Finding) | Findings emitted by verification Blocks. |
| WorkflowEvent | Pulse | Lifecycle events become Pulses on the Bus. |

### From Architecture Specs (tmp/architecture/)

| Old Concept | Unified Equivalent | Notes |
|---|---|---|
| Extension (22 hooks, 8 layers) | Extension specialization | 8 layers preserved. |
| Connector (5 methods) | Connector specialization | Connect protocol with lifecycle. |
| Feed | Pulse stream on Bus | A Feed is Pulses on a Bus topic. No separate primitive. |
| Recipe | Graph | A Recipe is a Graph of pure-transform Blocks. |
| Knowledge Entry | Signal (persisted, Memory) | Signals with demurrage, tiers, HDC fingerprints. |
| Pheromone | Pulse (ephemeral, typed) | Pheromones are Pulses with location hash and intensity. |
| DreamCycle | Loop specialization | Graph feeding output back to input on delta timescale. |
| MetaAgent | Agent specialization | Agent whose Space grants structural-change capabilities. |
| Generator | Block + Trigger | Triggered Block producing Signals on schedule. |
| AgentRuntime | Agent specialization | Agent = Space + Extensions + Memory + clock + vitality. |
| AgentMode | Agent config field | Ephemeral / Persistent / Reactive. |
| CorticalState | Agent internal state | Lock-free atomic perception surface. |
| DomainProfile | Domain profile (full) | Clock + extensions + events + context weights + gates + infra. |
| TickPipeline | Agent's 9-step Graph | The 9-step pipeline as a Graph. |
| Relay | Connector (relay) | Built-in Connector for presence and messaging. |

### From roko-core (actual code)

| Old Concept | Unified Equivalent | Notes |
|---|---|---|
| Engram | Signal (durable) | Direct identity. Rust struct remains `Engram`. |
| Pulse (event bus) | Pulse (ephemeral) | Promoted from `Envelope<E>` to first-class. |
| Substrate | Store protocol | Identical semantics. |
| Scorer | Score protocol | Identical semantics. |
| Gate | Verify protocol | Extended: pre/post, continuous reward, evidence typing. |
| Router | Route protocol | Extended: EFE, regime conditioning. |
| Composer | Compose protocol | Extended: VCG, section effect tracking. |
| Policy | React protocol | **Breaking**: operates on Pulses, not Signals. |

---

## Reading Order

### Essential (learn the type system)

1. **[01-SIGNAL.md](01-SIGNAL.md)** — Two mediums: Signal (durable) and Pulse (ephemeral)
2. **[02-BLOCK.md](02-BLOCK.md)** — The universal computation + 9 protocols
3. **[03-GRAPH.md](03-GRAPH.md)** — The universal composition
4. **[04-SPECIALIZATIONS.md](04-SPECIALIZATIONS.md)** — 10 patterns built on fundamentals

### Core systems

5. **[05-EXECUTION-ENGINE.md](05-EXECUTION-ENGINE.md)** — State graph runtime, hot Graphs, failure strategies, resumability
6. **[06-TRIGGER-SYSTEM.md](06-TRIGGER-SYSTEM.md)** — Event-driven Graph firing
7. **[07-AGENT-RUNTIME.md](07-AGENT-RUNTIME.md)** — Vitality, type-state lifecycle, CorticalState, EFE gating, somatic markers
8. **[08-EXTENSION-SYSTEM.md](08-EXTENSION-SYSTEM.md)** — Interceptor Blocks across 8 layers, CaMeL IFC

### Observation and learning

9. **[09-TELEMETRY.md](09-TELEMETRY.md)** — Lens system, StateHub projections, c-factor
10. **[10-LEARNING-LOOPS.md](10-LEARNING-LOOPS.md)** — 4 cybernetic loops, predict-publish-correct, hindsight relabeling, L4 self-evolution

### Knowledge and connectivity

11. **[11-MEMORY-AND-KNOWLEDGE.md](11-MEMORY-AND-KNOWLEDGE.md)** — Demurrage + Heuristics + HDC resonator networks + dreams
12. **[12-CONNECTIVITY.md](12-CONNECTIVITY.md)** — Exoskeleton protocols (MCP + A2A + ERC-8004 + x402), relay, cross-agent comms

### Developer experience

13. **[13-BUILTIN-BLOCK-CATALOG.md](13-BUILTIN-BLOCK-CATALOG.md)** — All shipped Blocks by protocol
14. **[14-CONFIG-AND-AUTHORING.md](14-CONFIG-AND-AUTHORING.md)** — 5-tier package SPI, domain profiles, workspace scoping
15. **[15-MARKETPLACE-AND-SHARING.md](15-MARKETPLACE-AND-SHARING.md)** — DAW composability, fork chains, marketplace economics
16. **[16-SURFACES.md](16-SURFACES.md)** — Five named UX surfaces as protocol-level primitives
17. **[17-SECURITY-MODEL.md](17-SECURITY-MODEL.md)** — CaMeL IFC, 5-head corrigibility, capability intersection

### Infrastructure

18. **[18-ON-CHAIN-REGISTRIES.md](18-ON-CHAIN-REGISTRIES.md)** — ERC-8004 passports, ZK-HDC fingerprints, InsightStore
19. **[19-ARENAS-EVALS-BOUNTIES.md](19-ARENAS-EVALS-BOUNTIES.md)** — Arena as 7-step flywheel, meta-arena
20. **[20-DEPLOYMENT.md](20-DEPLOYMENT.md)** — WASM + native, brain export, Merkle-CRDT sync
21. **[21-ROADMAP.md](21-ROADMAP.md)** — Phased implementation, crate mapping

---

## Naming Decisions

| Decision | Rationale |
|---|---|
| Signal (not Engram) | "Signal" is immediately meaningful. "Engram" is domain-specific jargon. The Rust struct remains `Engram`; "Signal" is the spec-level name. |
| Pulse (not Envelope/Event) | "Pulse" names the ephemeral sibling. The code has `Envelope<E>` in `roko-runtime::event_bus`; "Pulse" is the canonical name. |
| Bus (not EventBus) | "Bus" is the transport fabric. Promoted from implementation detail to kernel trait alongside Store. |
| Block (not Module) | "Block" implies composable, small, pluggable — like a Eurorack module or a Scratch block. |
| Graph (not Workflow) | "Graph" is precise: nodes + edges. "Flow" is the runtime instance. |
| Rack (not Parameterized Workflow) | From modular synthesis: macro knobs and patch jacks. |
| Lens (not Monitor/Probe/Watcher) | Read-only by definition. Stacking Lenses gives different views. |
| Loop (not Feedback/Cycle) | Direct and unambiguous. |
| Space (not Workspace/Environment) | "Workspace" is overloaded (IDE, OS). "Space" is clean. |
| Demurrage (not Decay) | "Decay" implies passive time-based loss. "Demurrage" implies active economic mechanism — use restores value. |
| Vitality (not Budget/Lifetime) | "Vitality" captures both economic pressure and behavioral modulation. |

---

## Document Status

| # | Title | Status | Source |
|---|---|---|---|
| 00 | Index (this document) | v2 draft | Redesigned from v1 + all source material |
| 01 | Signal | Pending v2 | Two mediums, demurrage, graduation, Heuristic kind |
| 02 | Block | Pending v2 | Predict-publish-correct, Verify redesign, EFE routing |
| 03 | Graph | Pending v2 | Hot Graphs, Workflow/Activity split |
| 04 | Specializations | Pending v2 | Agent vitality, type-state lifecycle |
| 05 | Execution Engine | Pending v2 | Hot Graph execution, deterministic replay |
| 06 | Trigger System | Pending v2 | (minor updates) |
| 07 | Agent Runtime | Pending v2 | Vitality, type-state, CorticalState, EFE, somatic markers, multi-slot |
| 08 | Extension System | Pending v2 | CaMeL IFC integration |
| 09 | Telemetry | Pending v2 | StateHub projections, c-factor, CollectiveIntelligenceLens |
| 10 | Learning Loops | Pending v2 | Predict-publish-correct, hindsight relabeling, L4 self-evolution, spec-as-artifact |
| 11 | Memory and Knowledge | Pending v2 | Demurrage, Heuristic with falsifier, Resonator Networks |
| 12 | Connectivity | Pending v2 | Exoskeleton protocols, finality oracle, multi-chain |
| 13 | Builtin Block Catalog | Pending v2 | (additions for new capabilities) |
| 14 | Config and Authoring | Pending v2 | 5-tier SPI, domain profiles (full), workspace scoping |
| 15 | Marketplace | Pending v2 | Marketplace economics, DAW composability, fork chains |
| 16 | Surfaces | Pending v2 | Five named surfaces as protocol-level primitives |
| 17 | Security Model | Pending v2 | CaMeL IFC, 5-head corrigibility |
| 18 | On-Chain Registries | Pending v2 | ZK-HDC passports |
| 19 | Arenas | Pending v2 | Arena as 7-step flywheel, meta-arena |
| 20 | Deployment | Pending v2 | WASM compilation, brain export, Merkle-CRDT |
| 21 | Roadmap | Pending v2 | Updated phases |
