# Vision and Core Thesis

> **v3 depth file** -- `/docs/v3/depth/00-architecture/vision-and-thesis.md`
> Canonical source: v1 `docs/v1/00-architecture/00-vision-and-thesis.md`
> Status: **Current** (48/48 epics accepted, Graph sole engine, 124/124 executable tasks)

---

## 1. The Problem: LLMs Are Necessary but Not Sufficient

Large language models can generate code, reason about strategy, and produce natural-language
plans. But the frontier of agent capability is not determined by the LLM alone. Empirical
evidence across multiple benchmarks demonstrates that the harness -- the system wrapping the
LLM -- accounts for a large and growing share of agent performance:

- **SWE-bench Verified** (Jimenez et al. 2024): The same base model (Claude Sonnet 3.5)
  achieves between 30% and 65% solve rates depending on the harness wrapping it. The
  difference is not the model -- it is the scaffolding.

- **AlphaCode** (Li et al. 2022): DeepMind achieved competitive programming results not
  through a single enormous model but through a compound system of generation, filtering,
  clustering, and selection. The harness architecture was the innovation.

- **Meta-Harness** (Lee et al. 2026, arXiv:2603.28052): Demonstrates +7.7 points on text
  classification and +4.7 points on IMO-level math from harness optimization alone, at 4x
  fewer tokens. This is the most direct empirical evidence that scaffolding is a first-class
  performance lever.

- **FrugalGPT** (Chen et al. 2023, arXiv:2305.05176): Shows that cascading through models
  of increasing capability -- routing easy queries to cheap models and hard queries to
  expensive ones -- can match GPT-4 quality at 2% of the cost. The routing harness is the
  product.

- **DSPy** (Khattab et al. 2024): Replaces hand-written prompts with a compiler that
  optimizes prompt pipelines automatically. Performance improvements come not from bigger
  models but from better prompt assembly -- a scaffold concern.

- **Compound AI Systems** (Zaharia et al. 2024): The Berkeley AI Research thesis that
  state-of-the-art results increasingly come from systems of multiple components rather
  than from individual models. The system architecture is the differentiator.

The pattern is clear: the scaffold IS the product. Given any fixed LLM, the quality of the
surrounding context engineering, verification pipeline, learning loops, model routing, and
cognitive architecture determines agent performance. Roko exists to be that scaffold --
composable, domain-agnostic, and self-improving.

---

## 2. The Scaffold Thesis

Roko's core thesis can be stated precisely:

> **Given the same LLM, agent performance varies dramatically based on the surrounding
> harness (context engineering, verification, learning loops, cognitive architecture).
> Roko is that scaffold, made composable.**

This thesis has three implications:

### 2.1 Scaffolding Is a Durable Competitive Moat

LLM capabilities improve rapidly. Any advantage based on model access is temporary -- today's
frontier model is tomorrow's commodity. But the scaffold wrapping that model compounds its
value over time:

- **Context engineering** improves as the agent learns which information to include in prompts
  and which to omit. This is a data advantage that deepens with usage.

- **Verification pipelines** become more calibrated as gate thresholds adapt to observed
  pass rates. A harness that has verified 10,000 tasks has better-calibrated gates than one
  that has verified 10.

- **Learning loops** extract heuristics, playbook rules, and skill libraries from execution
  episodes. These accumulate and transfer across agent lifetimes.

- **Model routing** improves as contextual bandits learn which models work best for which
  task types. Routing quality is a function of experience, not of model capability.

Each of these scaffold components improves independently of the underlying LLM. When the LLM
is upgraded, the scaffold's accumulated knowledge amplifies the new model's capabilities.
This is the compound advantage that Roko provides.

### 2.2 Composability Is the Design Requirement

If the scaffold is the product, then the scaffold must be composable -- different users need
different combinations of scaffold capabilities for different domains. A coding agent needs
compilation gates, test harnesses, and diff-aware context assembly. A research agent needs
citation verification, source credibility scoring, and knowledge synthesis. A chain agent
needs transaction simulation, on-chain state grounding, and position monitoring.

Roko achieves composability through its kernel architecture: one universal data type
(the Signal, backed by the `Engram` struct) and 12 composable kernel traits that express
every capability in the system. Any combination of trait implementations can be assembled to
create a domain-specific agent. The architecture is detailed in the sibling depth files
(see `synapse-traits-12.md` and `scorer-gate-router-composer-policy.md`).

### 2.3 Self-Improvement Is the Goal, Not the Assumption

Roko's design enables agents that improve themselves, but it does not assume that
self-improvement happens automatically. The architecture provides the mechanisms:

- **Episodic learning** -- agents record what they did, what worked, and what failed
- **Playbook extraction** -- patterns are distilled from episodes into reusable rules
- **Prediction tracking** -- agents make falsifiable predictions and are calibrated against
  outcomes
- **Knowledge consolidation** -- offline learning (Dreams) synthesizes episodes into durable
  knowledge

Whether an agent actually improves depends on the quality of its feedback loops, the
richness of its experience, and the appropriateness of its domain configuration. The
architecture provides the rails; the agent must travel them.

---

## 3. Theoretical Foundations

Roko's architecture draws from multiple theoretical traditions, each contributing a specific
design principle.

### 3.1 CoALA: Cognitive Architectures for Language Agents

Sumers et al. (2023, arXiv:2309.02427) propose CoALA as a framework for understanding
language agent architectures. CoALA identifies the core components of a cognitive
architecture: memory modules, action spaces, and a decision-making procedure that operates
over them.

Roko implements CoALA's framework directly:

| CoALA Component | Roko Implementation |
|---|---|
| Working memory | The `Context` struct + active Signals in the Store |
| Episodic memory | Episodes stored via `Store.put()` with `Kind::Episode` |
| Procedural memory | Playbook rules and skills in `roko-learn` |
| Semantic memory | Knowledge entries in `roko-neuro` (NeuroStore) |
| Action space | Tool registry (`ToolRegistry` in `roko-core`) |
| Decision procedure | The universal cognitive loop (`loop_tick`) |

The mapping is not superficial -- each CoALA component corresponds to a specific Rust type
or trait implementation in the Roko codebase.

### 3.2 Active Inference and the Free Energy Principle

Karl Friston's Free Energy Principle (Friston 2010, Nature Reviews Neuroscience 11(2))
provides the theoretical foundation for Roko's approach to attention and action selection.
The core insight: self-organizing systems minimize the divergence between their predictions
and their observations. Prediction error -- the surprise signal -- drives learning, attention,
and action.

The engineering mechanism: operators keep expectation/outcome records, then update calibration
when outcomes diverge from expectations. Operators publish prediction Pulses, later Pulses
resolve those predictions with outcomes, and learning policies publish calibration updates
plus `prediction.error.*` signals back onto the Bus.

- **Prediction error as a learning signal**: Every observation resolves a prediction. The
  residual feeds automatic calibration, per-operator updates, and topic-level
  `prediction.error.*` streams.

- **Prediction error as an attention signal**: Items with high prediction error receive more
  computational resources. Items with low error receive less. This approximates the
  exploration/exploitation tradeoff that Expected Free Energy formalizes (de Vries et al.
  2025, arXiv:2504.14898).

- **Prediction error as an action gate**: The agent may act when its action predictions are
  more accurate than its inaction predictions. This structurally prevents over-action.

The Expected Free Energy (EFE) formula that drives Roko's tier routing is:

```
G(pi) = E_q[ log q(s|pi) - log p(o,s|pi) ]
      = - Pragmatic Value - Epistemic Value
```

Where:
- **Pragmatic value** = expected reward from acting on policy pi
- **Epistemic value** = expected information gain from observing under policy pi

This decomposes naturally into Roko's dual-process cognition: high-certainty situations
(low epistemic value) route to fast processing (T0/T1), while high-uncertainty situations
(high epistemic value) route to deep reasoning (T2).

### 3.3 The Autocatalytic Thesis

Stuart Kauffman's work on autocatalytic sets (Kauffman 1993, *The Origins of Order*, Oxford
University Press) provides the mathematical foundation for Roko's self-improvement claim.
An autocatalytic set is a collection of entities where each entity's production is catalyzed
by other entities in the set. Once the set reaches a critical diversity threshold, it
becomes self-sustaining.

Roko's five layers form an autocatalytic improvement cycle:

```
Scaffold (L2) improves prompts --> Agent (L1) produces better outputs -->
Gates (L3) catch more errors --> Learning (Cognitive) extracts better heuristics -->
Scaffold (L2) has better context to work with --> cycle continues
```

The compound improvement math: if each layer contributes a 10% improvement independently,
the compound effect is not 10% but:

```
1 - 0.9^4 = 1 - 0.656 = 0.344 = 34.4% compound improvement
```

This is why the scaffold thesis matters practically: small improvements in each component
multiply through the loop, producing outsized aggregate gains. Meta-Harness (Lee et al. 2026)
provides empirical evidence of exactly this effect -- harness optimization alone produces
larger gains than expected from any single component.

Kauffman's theory predicts a phase transition: below a critical connectivity threshold,
catalytic closure does not emerge; above it, self-sustaining cycles become inevitable. Roko's
architecture is designed so that the 12 kernel traits provide sufficient connectivity for
this closure. The Gate pipeline validates Scaffold outputs, the Learning pipeline feeds
back into the Scaffold, and the Scaffold feeds into the Agent dispatch -- each component
catalyzes the production of inputs for the others.

### 3.4 Cybernetic Foundations

Three cybernetic principles shape Roko's architecture:

**Ashby's Law of Requisite Variety** (Ashby 1956, *An Introduction to Cybernetics*): A
system's regulatory capacity must match the variety of its environment. Roko addresses this
through compositional variety -- the 12 kernel traits can be combined in exponentially many
configurations, matching the variety of domains and tasks the system encounters.

**The Good Regulator Theorem** (Conant & Ashby 1970, *International Journal of Systems
Science* 1(2)): Every good regulator of a system must be (or contain) a model of that system.
Roko implements this through the Daimon (`roko-daimon`), which maintains an internal model of
the agent's own cognitive state (PAD vector, behavioral state, prediction calibration). The
agent literally models itself.

**Beer's Viable System Model** (Beer 1972, *Brain of the Firm*): Five recursive subsystems
are required for any viable organization. Roko's five layers map directly to Beer's VSM:

| Beer VSM | Roko Layer | Function |
|---|---|---|
| System 1: Operations | L0 Runtime | Primary activities -- process lifecycle, I/O |
| System 2: Coordination | L1 Framework | Anti-oscillation -- model routing, tool dispatch |
| System 3: Control | L2 Scaffold + L3 Harness | Resource allocation + auditing |
| System 4: Intelligence | L4 Orchestration + Cognitive | Environmental scanning, adaptation |
| System 5: Policy | Cognitive cross-cuts (Daimon) | Identity, purpose, self-model |

The VSM mapping is not decorative -- it explains why five layers are necessary and sufficient,
and why the cognitive cross-cuts must be injected across layers rather than living at any
single level.

---

## 4. What Roko Is

Roko is a cognitive agent operating system: a domain-agnostic, modular, composable framework
that provides five architectural layers plus cognitive cross-cuts that make agents
self-improving across any domain.

### 4.1 The Kernel Architecture

The system is built from **one noun** and **12 verbs**:

- **One noun**: The **Signal** (backed by the `Engram` struct) -- a content-addressed, scored,
  decaying, lineage-tracked unit of cognition. Every piece of information in Roko -- a task,
  a prompt, an LLM output, a gate verdict, a knowledge entry, a prediction, a tool trace --
  is a Signal.

- **12 verbs**: The kernel traits -- Store, ColdStore, Score, Verify, Route, Compose, React,
  Bus, Observe, Connect, Trigger, and Substrate (legacy alias). Every capability in Roko is
  an implementation of one of these traits operating on Signals.

### 4.2 Five Layers

Roko's crates are organized into five architectural layers with strictly downward
dependencies (see `five-layer-taxonomy.md` for the full specification):

| Layer | Name | Purpose | Key Crates |
|---|---|---|---|
| L0 | Runtime | Process lifecycle, events, supervision, I/O, adaptive clock | `roko-runtime`, `roko-primitives`, `roko-core` |
| L1 | Framework | Connections, roles, tools, model routing, safety | `roko-agent`, `roko-std` |
| L2 | Scaffold | Context engineering, prompts, enrichment, memory | `roko-compose` |
| L3 | Harness | Gates, conductor, monitoring, interventions, eval | `roko-gate`, `roko-fs` |
| L4 | Orchestration | DAGs, scheduling, state machines, multi-agent coordination | `roko-conductor`, `roko-graph` |

### 4.3 Three Cognitive Cross-Cuts

Three subsystems are injected across multiple layers rather than living at any single level:

- **Neuro** (`roko-neuro`) -- Knowledge management. Persists insights, heuristics, and
  warnings across tasks with tier-based decay and HDC encoding for similarity search.

- **Daimon** (`roko-daimon`) -- Motivation and focus. A PAD (Pleasure-Arousal-Dominance)
  vector that modulates tier routing, context bidding, and risk tolerance. The agent's
  self-model.

- **Dreams** (`roko-dreams`) -- Offline learning. Consolidates episodes during idle time
  through three phases: NREM replay, REM imagination, and integration staging.

### 4.4 Domain Agnosticism

Roko is domain-agnostic by design. The framework knows about Signals, traits, layers, and
cognitive loops. It does not know about any specific domain. Domain knowledge enters through
trait implementations.

---

## 5. What Roko Is Not

### 5.1 Not a Wrapper Around a Single LLM

Roko is not an "LLM wrapper" or a "prompt template library." It is a complete cognitive
architecture with storage (Store), evaluation (Score, Verify), decision-making (Route),
assembly (Compose), and reaction (React). LLMs are called at specific points within this
architecture -- they are inference engines, not the architecture itself.

### 5.2 Not a Framework for Immortal Agents

Roko agents operate under resource constraints -- budget limits, time deadlines, and
confidence tracking. These constraints drive prioritization and urgency. When an agent's
budget is exhausted, its confidence drops below useful thresholds, or its time allocation
expires, the agent's run concludes and its accumulated knowledge is available for backup,
transfer, or restoration.

### 5.3 Not Domain-Specific

Although Roko's early development focused on coding and chain domains, the architecture is
domain-agnostic. Every component -- from the Signal data type to the gate pipeline to the
learning loops -- operates on generic Signals with domain-specific behavior injected through
trait implementations. The framework does not privilege any domain.

---

## 6. The Self-Hosting Proof

Roko's strongest architectural argument is that it develops itself. The self-hosting workflow
demonstrates the scaffold thesis in practice:

```
1. Capture a work item           --> roko prd idea "Wire SystemPromptBuilder"
2. Draft a PRD                   --> roko prd draft new "system-prompt-wiring"
3. Research the topic            --> roko research enhance-prd system-prompt-wiring
4. Generate an implementation plan --> roko prd plan system-prompt-wiring
5. Execute the plan              --> roko plan run plans/
6. Resume if interrupted         --> roko plan run plans/ --resume-plan
7. Watch progress                --> roko dashboard
8. Check status                  --> roko status
```

Each step uses Roko's own infrastructure. The scaffold improves the scaffold. This is the
autocatalytic cycle made concrete. As of September 2026, all 48 epics are accepted and
124/124 executable tasks are complete.

---

## 7. Design Principles

Seven principles guide Roko's architecture:

### P1: Signals Over Interfaces

All data is Signals (the `Engram` struct, aliased as `Signal` in the codebase). Services
communicate by reading and writing Signals, not by calling each other's methods. This enables
loose coupling, audit trails via lineage chains, and the ability to replay any decision by
following the Signal DAG.

### P2: Composition Over Inheritance

Roko uses trait composition, not class hierarchies. A coding agent is not a subclass of
"BaseAgent" -- it is a specific combination of Store + Score + Verify + Route + Compose +
React implementations. New capabilities are added by implementing one of the 12 traits;
no core changes are needed.

### P3: Learning Is Infrastructure

Episode logging, playbook extraction, and knowledge consolidation are not optional add-ons --
they are built into the universal cognitive loop. Every tick produces Signals that feed the
learning pipeline. Learning is as fundamental as verification.

### P4: Verify Everything

Every agent output passes through the Gate pipeline before being persisted. Verification is
not an afterthought -- it is a core step of the cognitive loop. Gates are the bridge to
external reality: compilation, test execution, transaction simulation, schema validation.

### P5: Budget-Aware by Design

Every Composer operates under a Budget constraint (tokens, bytes, signal count, wall time).
Context windows are finite. Inference has costs. The architecture respects these constraints
at every level, not as an afterthought but as a core design parameter.

### P6: Downward Dependencies Only

Layer N may depend on Layer N-1, never the reverse. Cross-cutting concerns are injected via
trait objects. This ensures that any layer can be replaced or upgraded independently.

### P7: Honest About Uncertainty

Where the design is proven, the documentation says so. Where it is plausible but unproven,
the documentation says that too. Where it is aspirational, it is labeled as such. Roko does
not pretend to solve problems it has not solved.

---

## 8. Current State (September 2026)

| Metric | Value |
|---|---|
| Epics accepted | 48/48 |
| Executable tasks complete | 124/124 |
| Execution engine | Graph is sole engine (#260 default, #276 retired WorkflowEngine) |
| Workspace members | 39 crates |
| Lines of code | ~1M LOC |
| Tests | 10,300+ |
| LLM provider kinds | 12 (Anthropic, Claude CLI, Codex CLI, OpenAI-compat, Cursor ACP/CLI, Perplexity, Gemini API/CLI, Cerebras, Hermes, OpenClaw) |
| HTTP routes | ~376 canonical (~421 incl. aliases) |
| Gate rungs | 7-rung pipeline with 19 gates |
| Kernel traits | 12 |

---

## Academic Foundations

| Citation | Contribution to Roko |
|---|---|
| Lee et al. 2026 (arXiv:2603.28052) | Meta-Harness: +7.7 pts from harness optimization alone, 4x fewer tokens. Direct evidence for the scaffold thesis. |
| Chen et al. 2023 (arXiv:2305.05176) | FrugalGPT: cascade routing matches GPT-4 at 2% cost. Foundation for CascadeRouter. |
| Sumers et al. 2023 (arXiv:2309.02427) | CoALA: cognitive architecture framework for language agents. Structural blueprint for the universal cognitive loop. |
| Friston 2010, Nature Reviews Neuroscience 11(2) | Free Energy Principle: prediction error drives learning and attention. Foundation for Roko's Bus-level prediction/outcome/error loops. |
| Clark 2013, Behavioral and Brain Sciences 36(3) | Predictive Processing: brain as prediction machine. Supports prediction-error-based attention allocation. |
| Kauffman 1993, *The Origins of Order*, OUP | Autocatalytic sets: self-sustaining improvement cycles. Mathematical foundation for compound improvement. |
| Zaharia et al. 2024 | Compound AI Systems thesis: SOTA from systems, not models. Strategic framing for Roko's approach. |
| Khattab et al. 2024 | DSPy: compiler-optimized prompt pipelines. Evidence that scaffold optimization outperforms prompt engineering. |
| Jimenez et al. 2024 | SWE-bench: same model, 30-65% solve rate depending on harness. Empirical scaffold thesis. |
| Li et al. 2022 | AlphaCode: competitive programming through compound system design. Harness as innovation. |
| Ashby 1956, *An Introduction to Cybernetics* | Law of Requisite Variety: regulatory capacity must match environment. Motivates compositional design. |
| Conant & Ashby 1970, Intl. J. Systems Science 1(2) | Good Regulator Theorem: agent must model itself. Motivates Daimon self-model. |
| Beer 1972, *Brain of the Firm* | Viable System Model: 5 recursive subsystems. Maps to Roko's 5 layers. |
| Vovk et al. 2005, *Algorithmic Learning in a Random World* | Conformal prediction: distribution-free calibration. Mathematical basis for prediction quality tracking. |
| de Vries et al. 2025 (arXiv:2504.14898) | EFE as variational inference. Theoretical basis for attention allocation. |

---

## Cross-References

- `naming-and-glossary.md` -- Authoritative naming map and glossary
- `synapse-traits-12.md` -- The 12 kernel traits
- `five-layer-taxonomy.md` -- Layer architecture
- `scorer-gate-router-composer-policy.md` -- Five operational traits in detail
- `score-7-axis-appraisal.md` -- Scoring system
