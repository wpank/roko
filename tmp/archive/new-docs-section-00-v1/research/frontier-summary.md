# Frontier Summary

> How Roko maps to the modern agent-systems research frontier. This document is the
> research-facing complement to [`status/vision.md`](../status/vision.md) — where the
> vision doc presents the pitch, this document connects Roko's architecture to academic
> literature and open research problems.

**Status**: Written (research narrative — no implementation status)
**Last reviewed**: 2026-04-17

---

## TL;DR

Roko is not building from first principles. It is implementing, combining, and extending
ideas from five active research areas: compound AI systems, cognitive architectures,
multi-agent coordination, active inference / cybernetics, and hyperdimensional computing.
In each area, Roko makes concrete engineering decisions that go beyond the research and
constitute novel contributions.

---

## 1. Compound AI Systems — The Architecture Frontier

### The Research Position

Zaharia et al. (2024), "The Shift to Compound AI Systems" (Berkeley AI Research): the
frontier of AI capability has moved from individual models to systems of multiple components.
The best-performing AI products are compound systems, not monoliths.

The paper's key claim: "AI systems, not AI models, are the frontier." The implication is
that the engineering challenge is system design — how components interact, what information
flows between them, how failures propagate — not model training.

### How Roko Maps

Roko is an existence proof of the Zaharia thesis. Its architecture is:
- Six composable operator traits (not one monolithic model)
- Three cognitive speeds (not one execution mode)
- Two distinct mediums (not one information type)
- Multiple learning subsystems operating simultaneously

The `CascadeRouter` implements the FrugalGPT finding (Chen et al. 2023) that cascading
through models of increasing capability matches frontier quality at a fraction of the cost.
The gate pipeline implements the Meta-Harness finding (Lee et al. 2026) that harness
optimization is a first-class performance lever.

### Novel Contributions

Roko extends the compound AI literature in two ways:

1. **Composable scaffold primitives**: Most compound AI research describes specific systems.
   Roko provides a generative framework — a set of primitives from which any compound
   system can be assembled. The six Synapse traits are the vocabulary for describing
   compound AI systems in Roko's domain.

2. **Temporal decomposition**: Roko separates compound system operation into three speed
   tiers, each with different consistency guarantees and learning rates. This temporal
   decomposition is not addressed in the Zaharia et al. framing.

### Open Research Questions

- What is the right metric for compound AI system quality, comparable to benchmark scores
  for individual models?
- How should failures be attributed in compound systems where multiple components interact?

---

## 2. Cognitive Architectures — From ACT-R to Synapse

### The Research Position

Cognitive architectures — computational models of how minds work — have a 50-year history
(SOAR, ACT-R, CLARION, CoALA). They differ from agent frameworks in that they specify a
complete cognitive cycle: perception, memory, learning, action, and their interactions.

CoALA (Cognitive Architectures for Language Agents, Sumers et al. 2023) provides the most
direct mapping to LLM-based agents: it proposes a framework organizing agent cognition into
action, memory, learning, and decision-making, with explicit distinctions between working
memory, episodic memory, semantic memory, and procedural memory.

### How Roko Maps

Roko implements a CoALA-compatible cognitive architecture:

| CoALA Component | Roko Implementation |
|---|---|
| Working memory | Active `Engram` set in `Substrate` (high-score, low-decay) |
| Episodic memory | Episode log in `roko-learn` |
| Semantic memory | `Neuro` knowledge base (6 types, 4 tiers) |
| Procedural memory | Playbook rules in `roko-learn` |
| Perception | `QUERY` step of the universal loop |
| Decision-making | `ROUTE` + `COMPOSE` steps |
| Action | `ACT` step (LLM call or tool dispatch) |
| Learning | `REACT` step + offline `Dreams` consolidation |

The Synapse Architecture maps to the CoALA framework with one extension: Roko adds the
`Daimon` affect layer, which CoALA does not address. Affect modulates resource allocation
across all cognitive components — a contribution from the somatic marker hypothesis
(Damasio 1994) that is absent from most cognitive architecture formalisms.

### Novel Contributions

1. **Affect as a resource allocator**: `Daimon`'s PAD vectors determine which model tier
   to use, how aggressively to explore, and how much context to retrieve. This is affect
   as cognitive economics, not affect as simulation.

2. **Decay as attention management**: Roko's four decay variants (demurrage, reinforcement,
   novelty weighting, cold-tier freeze/thaw) implement a richer memory management policy
   than typical cognitive architectures, which use simpler recency-based forgetting.

3. **Content-addressed memory**: `Engram`'s BLAKE3 content-addressing enables deduplication
   and integrity verification that traditional cognitive architectures do not address.

### Open Research Questions

- How should the transition from working to episodic to semantic memory be formalized
  in Roko's `Neuro` tier progression?
- Does the `Daimon` PAD affect model produce measurably better task outcomes than a
  system without affect modulation?

---

## 3. Multi-Agent Coordination — C-Factor and Collective Intelligence

### The Research Position

Woolley et al. (2010), "Evidence for a Collective Intelligence Factor in the Performance
of Human Groups": groups have a general factor (*c*) of collective intelligence, analogous
to the individual *g* factor. The *c* factor is not correlated with average individual
intelligence; it predicts group performance across diverse tasks.

More recently, the study of collective intelligence in AI systems has drawn on stigmergy
(indirect coordination via environment modification, from Grassé 1959), multi-agent
reinforcement learning (MARL), and distributed cognition theory.

### How Roko Maps

Roko's `C-Factor` metric operationalizes the Woolley finding for AI agent fleets:

- **C-Factor** measures how much faster a fleet of agents solves problems together than
  any single agent solves them alone.
- A shared `Substrate` enables indirect coordination: one agent's successful strategy
  (recorded as an `Engram`) is available to other agents, acting like a pheromone trail.
- The `roko-chain` `Spore/Sparrow` marketplace enables explicit coordination: agents can
  post jobs (Spore) and accept them (Sparrow), creating a distributed task allocation market.

The `Mesh` layer (planned) will implement the agent-network infrastructure that enables
direct peer-to-peer `Pulse` routing between agents, enabling more sophisticated coordination
protocols.

### Novel Contributions

1. **C-Factor as a training signal**: Roko treats the collective intelligence quotient as
   a learnable metric — the system optimizes for collective performance, not just individual
   agent performance.

2. **Stigmergy via Substrate**: Using a shared content-addressed store as the coordination
   medium (rather than explicit message passing) enables asynchronous coordination that
   scales without central coordination bottlenecks.

### Open Research Questions

- At what fleet size does the shared-Substrate C-Factor compound start saturating?
- What is the relationship between individual agent `Score` improvement and collective
  C-Factor improvement?

---

## 4. Active Inference and Cybernetics

### The Research Position

Friston et al.'s active inference framework (Friston 2010, 2019) proposes that cognition
is prediction-error minimization: a system maintains a generative model of the world,
publishes predictions, and updates the model based on the error between predictions and
observations. The Good Regulator Theorem (Conant & Ashby 1970) states that every good
regulator of a system must contain a model of that system.

### How Roko Maps

**Active inference** maps to Roko's planned `prediction.*` / `outcome.*` / `prediction.error.*`
Pulse topic hierarchy. Operators publish predictions about their outputs; downstream
operators receive the outcome and publish the prediction error; the upstream operator
updates its model. This is the active inference loop implemented as Pulse routing.

**Good Regulator Theorem** maps to `roko-conductor`. The Conductor maintains a behavioral
model of the agent it regulates. It can only intervene effectively because it models what
the agent *should* be doing. Graduated interventions (Continue/Restart/Fail) minimize
prediction error in the agent's execution trajectory.

**Yerkes-Dodson dynamics** (Yerkes & Dodson 1908) — the inverted-U relationship between
arousal and performance — are implemented in `roko-conductor`'s pressure dynamics. The
Conductor detects when an agent is under- or over-pressured and adjusts its intervention
strategy accordingly.

### Novel Contributions

1. **Active inference via message passing**: Most active inference implementations operate
   at the neural level. Roko implements it at the agent-operator level via typed Pulse topics,
   making the free-energy minimization loop explicit and inspectable.

2. **Graduated intervention under Yerkes-Dodson**: The Conductor's three-level intervention
   policy (Continue/Restart/Fail) is calibrated to the agent's arousal state, not just
   to a fixed threshold.

### Open Research Questions

- Does implementing active inference at the operator level (via Pulses) produce the same
  convergence properties as free-energy minimization at the neural level?
- How should prediction error signals be weighted against gate failure signals in the
  learning update?

---

## 5. Hyperdimensional Computing — HDC as Cognitive Infrastructure

### The Research Position

Kanerva (2009), "Hyperdimensional Computing: An Introduction to Computing in Distributed
Representation with High-Dimensional Random Vectors": binary vectors of thousands of
dimensions encode structured information in a way that is robust to noise, supports
compositional operations (bind, bundle, permute), and enables similarity search via
Hamming distance — without neural network inference.

HDC has been applied to pattern recognition, associative memory, time-series classification,
and, more recently, efficient on-chain computation.

### How Roko Maps

`roko-neuro` uses 10,240-bit HDC vectors to encode knowledge in `Neuro`. Each knowledge
entry is encoded as a hyperdimensional vector; similarity search is a Hamming distance
computation — sub-millisecond, no GPU, no inference.

`roko-chain`'s Korai EVM includes an HDC precompile at ~400 gas, enabling on-chain
vector similarity operations. This is potentially the first production EVM precompile for
hyperdimensional computing.

### Novel Contributions

1. **HDC for agent knowledge encoding**: Most HDC applications are in signal processing.
   Roko applies HDC to encode structured agent knowledge (heuristics, causal links,
   strategy fragments), exploiting HDC's compositionality for knowledge combination.

2. **On-chain HDC precompile**: The Korai EVM HDC precompile enables decentralized
   similarity search at gas costs comparable to simple arithmetic operations. This opens
   the possibility of on-chain reputation systems that use semantic similarity rather
   than exact-match.

### Open Research Questions

- What is the optimal HDC vector dimensionality for agent knowledge encoding? 10,240 bits
  was chosen based on capacity estimates; empirical validation needed.
- How should HDC vectors be updated as knowledge migrates through Neuro validation tiers?

---

## 6. DSPy and Automated Prompt Optimization

### The Research Position

Khattab et al. (2024), "DSPy: Compiling Declarative Language Model Calls into Self-Improving
Pipelines": prompts are learnable programs, not hand-written configuration. A prompt compiler
can automatically optimize a pipeline of LLM calls by searching the space of prompt
formulations, few-shot examples, and chain-of-thought strategies.

### How Roko Maps

Roko's `SystemPromptBuilder` is a hand-engineered prompt compiler, optimized manually.
The DSPy finding motivates the target-state `Calibrator` component, which would apply
automated optimization to Roko's prompt assembly pipeline.

The `roko-learn` playbook system is a step toward DSPy-style optimization: it extracts
successful prompt patterns from execution history and applies them to future prompts.
But it does not yet do gradient-free search over the full prompt space.

### Novel Contributions

Roko does not yet implement automated prompt optimization at the DSPy level. The roadmap
item is to use the playbook and episode log as the training signal for a Calibrator that
does automated prompt search within Roko's existing multi-tier context assembly pipeline.

### Open Research Questions

- Can DSPy-style optimization be applied to multi-stage prompt pipelines (Roko's
  13-step enrichment) without degrading the manual U-shape placement optimization?
- How should prompt optimization interact with the `Daimon` affect state that modulates
  context assembly?

---

## 7. SWE-bench and Code Agent Harnesses

### The Research Position

Jimenez et al. (2024), "SWE-bench: Can Language Models Resolve Real-World GitHub Issues?":
the benchmark reveals that the same base model achieves vastly different performance
depending on the agent harness wrapping it. The harness is a measurable performance
variable.

This is the empirical cornerstone of the scaffold thesis. It establishes that harness
design is a quantifiable engineering concern with benchmark-scale effects.

### How Roko Maps

Roko's code intelligence infrastructure (`roko-index`, `roko-lang-*`, the code intelligence
section) is designed to implement the harness optimizations that produce the top SWE-bench
scores:

- Diff-aware context assembly (include only the relevant diff, not the full file)
- Repository-structure-aware retrieval (understand file relationships before querying)
- Symbol-resolution gates (verify that referenced symbols exist before submitting)
- Test-driven verification (the GeneratedTest and PropertyTest gate rungs)

Roko's code intelligence crates are `[built]` (roko-index) and planned (`roko-lang-*`).
The goal is to make the full SWE-bench harness stack composable and reusable across
any code intelligence agent.

---

## How This Maps to Roko's Folder Structure

The papers and frameworks cited here are organized in the `research/` folder:

| Topic | Destination folder |
|---|---|
| Compound AI, FrugalGPT, Meta-Harness | [`research/foundations/`](foundations/) |
| CoALA, cognitive architectures | [`research/foundations/`](foundations/) |
| Active inference, cybernetics | [`research/foundations/active-inference.md`](foundations/active-inference.md) |
| Collective intelligence, C-Factor | [`research/foundations/c-factor.md`](foundations/c-factor.md) |
| HDC | [`research/foundations/`](foundations/) |
| DSPy | [`research/references/`](references/) |
| SWE-bench | [`research/references/`](references/) |
| Attention as currency (perspective) | [`research/perspectives/attention-as-currency.md`](perspectives/attention-as-currency.md) |
| Cognitive immune system (perspective) | [`research/perspectives/cognitive-immune-system.md`](perspectives/cognitive-immune-system.md) |
| Temporal knowledge topology (perspective) | [`research/perspectives/temporal-topology.md`](perspectives/temporal-topology.md) |
| Emergent goal structures (perspective) | [`research/perspectives/emergent-goals.md`](perspectives/emergent-goals.md) |
| Cognitive energy model (perspective) | [`research/perspectives/energy-model.md`](perspectives/energy-model.md) |

---

## See Also

- [`status/vision.md`](../status/vision.md) — the pitch: scaffold thesis and empirical evidence
- [`reference/12-design-principles.md`](../reference/12-design-principles.md) — how the research informs architectural decisions
- [`research/README.md`](README.md) — folder index for all research content

## Open Questions

- Are there frontier research areas Roko should be mapping to that are not covered here?
  Candidates: mechanistic interpretability (for understanding what Engrams represent),
  constitutional AI (for Policy trait implementation), and causal inference (for lineage
  DAG analysis).
- Should this document be maintained as a living survey or replaced by individual per-paper
  summaries in `research/references/`?
