# Architecture Paradigms and Implementation Patterns

This document synthesizes the architectural paradigms and implementation patterns defined in the Roko v2 specification. It is intended as a self-contained reference for someone with no prior context on the system.

---

## 1. The Core Paradigm: Everything Is a Graph of Cells Processing Signals

Roko's architecture is organized around a single governing rule: **everything is a Graph of Cells processing Signals through Bus and Store.** Every subsystem -- agent cognition, verification pipelines, learning loops, dream consolidation, data feeds, marketplace workflows, DeFi trading pipelines -- is expressed as a composition of the same five kernel primitives. There are no special cases.

### The Five Primitives

**Signal** is the durable data medium. A Signal is a content-addressed (SHA-256) datum carrying a typed payload, a 5-axis quality score, an economic balance that decays via demurrage, a full lineage DAG for provenance tracking, and a 10,240-bit HDC (Hyperdimensional Computing) fingerprint for similarity search. Signals live in **Store**, the persistent data fabric. Every piece of durable state in the system -- knowledge entries, episode logs, gate verdicts, configuration snapshots, trade positions -- is a Signal.

**Pulse** is the ephemeral data medium. A Pulse is a sequence-numbered, ring-buffered event broadcast via **Bus**, the ephemeral pub/sub transport fabric. Pulses carry lifecycle events (agent heartbeats, gate verdicts, flow transitions), streaming output, predictions, and coordination signals. Unlike Signals, Pulses have no lineage, no scoring, and no HDC fingerprint -- they are intentionally transient.

The system bridges these two mediums through two explicit operations: **Graduation** (Pulse to Signal, the only path from transport into the audit DAG) and **Projection** (Signal to Pulse, a lossy broadcast of stored data).

**Cell** is the universal computation unit. A Cell consumes Signals, produces Signals, declares typed I/O schemas, requires specific capabilities, and conforms to one or more of nine protocols. Every first-class computation in Roko -- scoring, verification, routing, composition, storage, reaction, observation, connection, triggering -- is a Cell. Cells also carry cost and duration estimates, enabling budget-aware scheduling before execution begins.

**Graph** is the universal composition primitive. A Graph is a typed DAG (or cyclic graph) of Cells connected by edges with optional conditions and data mappings. Graphs are defined in TOML, loaded at runtime, and interpreted by a single execution engine. The critical property is **fractal composition**: a Graph implements the Cell trait, so any Graph can be embedded as a node inside another Graph. A Pipeline of Pipelines is just a Pipeline. A Loop containing a Graph is just a Loop. This eliminates the need for special glue code between subsystems.

**Protocol** defines behavioral contracts. The nine protocols (Store, Score, Verify, Route, Compose, React, Observe, Connect, Trigger) are async trait interfaces that Cells can conform to. Every protocol supports the predict-publish-correct learning mechanism.

### Two Fabrics, No Alternatives

The system has exactly two data transport mechanisms: **Store** for durable Signals and **Bus** for ephemeral Pulses. There are no ad-hoc state channels, no hidden caches, no side-band communication. Every piece of data flows through one of these two fabrics. This constraint is what makes subsystem composition work by construction -- anything that reads from Store or subscribes to Bus automatically interoperates with everything else.

---

## 2. The Four Universal Patterns

The spec identifies four recurring Graph topologies that appear across every subsystem. Every concrete system is an instance of one (or a composition of several) of these patterns.

### Pipeline

A linear chain of Cells where each can reject (Verify), transform (Compose), or redirect (Route). Pipelines process data sequentially with early exit. Instances include the inference gateway (a 9-stage Pipeline that handles loop detection, caching, tool pruning, output budgeting, provider calls, and cost tracking), the gate pipeline (verification rungs applied to task output), the immune system (a 5-layer security Pipeline), and the authentication chain (a Pipeline of Verify Cells).

### Loop

A Graph with a feedback edge from output back to input. A React Cell watches outcomes and adjusts parameters or strategies. Loops are the structural mechanism for self-improvement. Instances include the cognitive loop (the 7-Cell agent pipeline that feeds REACT output back to SENSE), the four learning loops (parameter tuning, strategy routing, knowledge consolidation, structural adaptation), the dream cycle (offline episode compression), adaptive gate thresholds (EMA feedback on pass rates), and the cascade router (Bayesian posterior updates on model performance).

### Functor

A cross-cut that enriches Signals before or after a Cell without changing the Graph's topology. Formally, an endofunctor F: Signal to Signal. Functors implement orthogonal concerns that apply across many Cells without requiring each Cell to know about them. Instances include memory enrichment (injecting relevant knowledge into agent context), Daimon affect bias (modulating scores based on emotional state), dream consolidation (compressing episodes during sleep), and safety constraints (applying capability restrictions).

### Space

A Graph that owns a Bus partition and a Store partition. Members share these resources under access control, creating isolation and collaboration boundaries. Instances include individual agents (each agent is a Space with its own Bus/Store partitions), groups (persistent agent collectives), workspaces, namespaces, and sandboxes.

### Why These Four Matter

These patterns eliminate architectural proliferation. Instead of designing bespoke infrastructure for each new subsystem, implementers compose existing patterns. A new verification pipeline is "just a Pipeline." A new learning mechanism is "just a Loop." A new agent capability is "just a Functor." This makes the architecture learnable (a developer learns 4 patterns, not 40 subsystems) and composable (patterns compose with each other by construction).

---

## 3. Agent Architecture

An Agent is the most complex specialization, composed from kernel primitives:

```
Agent = Space + Extensions + Memory + adaptive clock + vitality
```

### Cognitive Runtime

The agent's cognitive pipeline is a Hot Graph -- a resident Graph that re-fires on each tick of its bound clock, interpreted by the same execution engine that runs all other Graphs. The pipeline has seven Cells: SENSE (observe environment via Store query, Bus drain, external I/O), ASSESS (score candidates, apply affect bias, make routing decision), COMPOSE (assemble prompt context via VCG auction), ACT (dispatch to LLM or tool), VERIFY (run gate pipeline), PERSIST/BROADCAST (write to Store, publish to Bus), and REACT (consolidate episode, update routing feedback, check circuit breakers).

The ACT Cell is the only point where external provider calls happen. Agents never hold API keys directly; they interact through an InferenceHandle channel to a gateway.

### Type-State Lifecycle

Agent states are enforced at compile time via Rust's type system. An `Agent<Provisioning>` can load extensions but cannot tick. An `Agent<Active>` can tick and execute tools. An `Agent<Dreaming>` runs consolidation but cannot tick. An `Agent<Terminal>` can only flush and export. Calling a method unavailable in the current state is a type error, not a runtime error.

### Three Cognitive Timescales

Agents run three nested Hot Graphs concurrently at different speeds: gamma (1-5s, fast perception and reflex processing, ~95% T0 short-circuit rate), theta (5-60s, working memory updates and replanning, ~70% T0 rate), and delta (120s+, consolidation, dream cycles, long-term learning, ~50% T0 rate). These are independent Graphs with independent failure isolation, budget accounting, and snapshot/resume -- not scheduling hints.

### T0 Short-Circuit

Approximately 80% of ticks short-circuit at the ASSESS Cell. When all 16 T0 probes report "no change" and EFE (Expected Free Energy) selects T0 (zero-cost reflex), the remaining Cells (ACT, VERIFY, PERSIST) do not execute. Cost per tick: $0. This is the primary cost control mechanism: most of the time, the agent confirms nothing interesting is happening and does nothing.

### Vitality and Mortality

Every agent has finite vitality (`remaining_budget / initial_budget`), creating five behavioral phases: Thriving (explore freely), Stable (balanced exploration/exploitation), Conservation (favor known strategies, reduce exploration), Declining (complete current task only), Terminal (flush episodes, export knowledge, terminate). Mortality is a feature: an agent that has never faced resource pressure has never learned to prioritize.

### Multi-Slot Concurrency

An agent manages N named concurrent slots, each executing an independent task. Slots share the agent's global budget (via lock-free CAS-based atomic deduction), memory, CorticalState (a lock-free atomic shared perception surface), and extension chain, but maintain per-slot task assignments and capability guards.

---

## 4. Memory and Learning Systems

### Memory as Living Substrate

Memory is not a vector store with append-only semantics. It is a Store-protocol Cell with demurrage economics, tier progression, dream consolidation, and HDC-based retrieval.

**Demurrage** (after Silvio Gesell, 1916) replaces passive time-based forgetting with an active economic mechanism. Every Signal carries a balance that decays via a Gesell-Shannon ODE combining flat tax and exponential decay. Active use -- retrieval, citation, gate-pass, surprise -- restores balance. The effect is economic selection pressure: unique, actively-useful insights stay warm; redundant or stale entries fade.

**Four-tier progression** governs knowledge durability: Transient (decays 10x faster, raw observations), Working (2x faster, partially validated), Consolidated (base rate, well-validated), Persistent (5x slower, human or consortium approved). Promotion requires gate-pass evidence; demotion happens on gate failures or balance decay.

**Heuristics** are first-class Signal kinds with structured when/then clauses, mandatory falsifiers (Popperian falsificationism applied to learned rules), and live calibration records grounded in episode outcomes. A heuristic without a falsifier cannot be created. When a heuristic's falsifier fires above its retirement threshold, it spawns refined children with narrower preconditions.

**Worldviews** emerge from co-citation clusters of heuristics with high calibration scores. They are not explicitly created; they are discovered patterns in how heuristics reinforce each other. The system deliberately maintains multiple worldviews per domain (main, challenger, niche specialists) and mandates 15% contrarian retrieval from rival worldviews to prevent cognitive monoculture.

**AntiKnowledge** is a Signal kind that actively repels future Signals in the same HDC region. When previously trusted knowledge is disproven, it becomes AntiKnowledge. New arrivals similar to existing AntiKnowledge are warned (similarity > 0.5), discounted (> 0.7), or outright rejected (> 0.9).

### Four Learning Loops

Learning operates at four timescales with increasing scope and oversight:

**L1 (Parameter Tuning, gamma timescale, fully automatic)**: Adjusts continuous parameters within declared safe ranges using EMA feedback. Gate thresholds, prompt experiment weights, model temperature, clock regime thresholds, EFE cost weights, and compose budget allocations are all L1-tunable. Auto-rollback fires if quality drops after an adjustment, halving the learning rate.

**L2 (Strategy Routing, theta timescale, fully automatic)**: Selects among pre-approved alternatives using Expected Free Energy (Friston 2006). EFE balances epistemic value (information gain), pragmatic value (goal advancement), and cost. The three cognitive tiers (T0/T1/T2) naturally emerge from EFE bound evaluation. L2 can never introduce a new alternative; it only selects from the declared set.

**L3 (Knowledge Consolidation, delta timescale, automatic with audit)**: Compresses raw episodes into durable knowledge via a four-phase dream cycle: NREM Replay (cluster high prediction-error episodes, extract patterns), Hindsight Relabeling (decompose failed trajectories into achieved sub-goals, relabel as successes -- recovering useful learning signal from at least 45% of otherwise-discarded episodes), REM Imagination (generate counterfactual scenarios and threat rehearsal), and Integration (promote validated knowledge through tiers).

**L4 (Structural Adaptation, manual timescale, requires human approval)**: The only loop that modifies the system's own structure. Can change model configurations, Graph topologies, Cell registrations, and extension chains. Every change requires human approval, a pre-change snapshot, and auto-rollback on quality regression. Gated by the c-factor (collective intelligence metric).

### Predict-Publish-Correct

The structural mechanism underlying all learning: every Cell publishes its prediction as a Pulse, reality publishes the outcome as another Pulse, a CalibrationPolicy joins them by lineage and computes error, and the Cell subscribes to its own error topic to update. This pattern uses the same Bus that carries heartbeats and lifecycle events. There is no learning-specific infrastructure -- the Bus IS the learning fabric.

---

## 5. The Execution Model

A single Engine interprets all Graphs. Plans, agent pipelines, learning loops, dream cycles, trigger chains, and verification suites are all Graphs executed by the same runtime.

### Flow Lifecycle

Every Graph execution is a Flow with a standard lifecycle: Created, Running, Paused, Completed, Failed, Cancelled. Every state transition publishes a Pulse on Bus -- there is no separate monitoring channel. Pulses are the sole source of execution observability.

### Node Execution

The engine executes nodes in topological order, respecting edge conditions and parallelism limits. Each node goes through: precondition check (upstream completion, edge conditions, budget sufficiency), execution class check (Workflow/Activity split), input mapping, pre-verify (can veto), prediction publishing, Cell execution, post-verify, output recording, and downstream propagation.

### Workflow/Activity Split

Inspired by Temporal's durability model: Workflow Cells (SENSE, ASSESS, COMPOSE, REACT) are deterministic and re-execute identically on replay. Activity Cells (ACT, VERIFY, PERSIST) are non-deterministic; their outputs are recorded to disk and loaded on resume instead of re-executing. This enables crash recovery and resumability without re-incurring LLM costs.

### Hot Graphs

Hot Graphs stay resident in memory and re-fire on each tick of their bound clock. Between ticks, they retain all node outputs, graph-level variables, accumulated cost, and CorticalState updates. The agent's cognitive pipeline is a Hot Graph. Standard Flows (plans, one-shot tasks) are not hot -- they execute once and complete.

### Budget Enforcement

The engine tracks cost globally and per-Flow. Budget warnings and exhaustion are Pulses. When budget is exhausted, the Flow fails with a BudgetExhausted error. Agents enforce this through vitality phase transitions.

---

## 6. Feeds and Data Surfaces

### Feeds as Cell Specializations

A Feed is a Cell specialization combining three protocols: Connect (external data source I/O), Trigger (event-driven activation), and Store (optional persistence). Feeds are the "always-on" complement to one-shot queries.

Four feed types form composable value chains: Raw feeds (direct ingestion from external sources -- blockchain RPCs, webhooks, exchange APIs), Derived feeds (computed from raw feeds by an agent), Composite feeds (aggregated from multiple derived feeds, often cross-domain), and Meta feeds (feeds about feeds -- health monitoring, accuracy tracking).

### Feed Economics

Each agent in the feed chain pays for its inputs and charges for its output. The economic model is straightforward: revenue per subscriber minus input cost equals margin. Feed access can be public (free) or paid (with x402 per-request or MPP session-based streaming payments).

### Recipes

Recipes are pure data Graphs of Score Cells -- composable data transformations with no LLM calls and no agent involvement. They are distinct from Plans (task DAGs) and Compose (prompt assembly).

---

## 7. Marketplace and Registries

### Marketplace as Protocol

The marketplace turns local Cells, Graphs, Racks (parameterized Graphs with DAW-style knobs and slots), and Knowledge Bundles into community artifacts. Design deliberately avoids the failures of the GPT Store: transparent take-rates (0% to $1M lifetime creator revenue, 12-15% above), creators own their customer relationships, all metrics are published publicly, and fork is a fundamental operation, not a failure mode.

### 5-Tier Package SPI

Progressive capability with progressive trust: Tier 1 (Prompts, pure Markdown, no execution), Tier 2 (Config Profiles, TOML bundles), Tier 3 (Declarative Tools, subprocess/HTTP/MCP wrappers with OS-level sandboxing), Tier 4 (WASM modules, fuel-metered with deterministic builds), Tier 5 (Native Rust, full trust, in-tree only). The visual editor only writes Tiers 1-3.

### DAW Composability

The marketplace is modeled on Digital Audio Workstation plugin ecosystems: Criteria (verification checks) are plugins, Profiles (verification presets) are plugin presets, fork is the remix. This creates a composability hierarchy: Criterion -> Profile -> Rack -> Graph -> Space Template, where each level composes the level below.

### On-Chain Registries

ERC-8004 defines agent identity as a transferable NFT carrying capabilities, reputation tier (Gray through Amber), advertised feeds, service endpoints, and delegation caveats. Reputation scores are per-agent, per-domain, derived from on-chain attestations (arena completions, bounty resolutions, clearing participation, knowledge validation) via EMA with decay.

**TraceRank** is the multi-dimensional reputation model computing composite scores from five dimensions: consistency (low variance in attestations), breadth (positive domains), depth (max single-domain score), recency (exponential decay without activity), and collaboration (diverse peer interactions). TraceRank is itself a Score-protocol Cell that participates in predict-publish-correct.

---

## 8. DeFi Integration Patterns

All DeFi types are domain-specific Cell specializations implementing standard protocols. No new kernel primitives are introduced.

### Kernel Mapping

The ISFR (Internet Secured Funding Rate) oracle is a Cell implementing Score + Verify protocols, aggregating weighted lending rates across DeFi protocols via a dual-median approach (TVL-weighted median per validator, then median across validators). Yield perpetual positions are Signals stored in Store. The ClearingHouse is a Compose-protocol Cell using VCG welfare-maximizing settlement. VenueAdapters are Act-protocol Cells normalizing venue-specific execution. The DeFiRiskEngine is a Verify-protocol Cell that every trade flows through before execution.

### Safety and Affect Modulation

Position sizing passes through the Daimon affect engine. Losses are weighted 2.25x per prospect theory (Tversky & Kahneman 1992), preventing agents from doubling down after drawdowns. Simulation before execution is mandatory: trades run through fork simulation before hitting live chains.

### Multi-Chain Data

ISFR components come from Ethereum, Base, and Arbitrum. The ChainDataAggregator (a Graph of ChainDataSource Cells) composes cross-chain data into unified state. Price feed ingestion runs as a Hot Flow on a gamma-tick clock.

---

## 9. Key Architectural Decisions and Their Rationale

**One runtime, not many.** The single Engine that interprets all Graphs eliminates the need to separately implement retry strategies, snapshot resumability, budget enforcement, and failure decomposition for each subsystem. The cognitive loop, plan execution, verification pipelines, and learning loops all inherit these capabilities for free.

**Demurrage over TTL.** Time-based expiration (TTL) is binary: alive or dead. Demurrage is continuous and economically grounded: knowledge pays a holding cost for occupying store space, and active use restores value. This creates selection pressure that is qualitatively different from expiration -- genuinely useful knowledge persists indefinitely while noise self-eliminates.

**Conjunctive hard + Pareto soft over weighted-sum.** Weighted-sum verification (combine all criteria into one score) is vulnerable to Goodhart's Law: once you define the weights, the system optimizes for the weighted sum rather than genuine quality. Separating hard criteria (all must pass, binary AND) from soft criteria (multi-objective Pareto front, never collapsed to a single scalar) resists this failure mode.

**EFE over bandits.** Expected Free Energy (Friston 2006) for model routing provides principled exploration (information gain, not confidence bounds), native cost awareness (cost enters the free energy directly), first-class regime conditioning (different priors per environmental regime), and natural emergence of T0/T1/T2 cognitive tiers.

**Content-addressed identity.** Signals (SHA-256 of payload), Cells (SHA-256 of name + version + author), and Graphs (SHA-256 of name + version + sorted nodes + sorted edges) are all content-addressed. This makes lineage tracking, deduplication, and integrity verification structural rather than bolted on.

**Mortality as mechanism.** Finite vitality creates behavioral phases that modulate decision-making. An agent in Conservation mode (20-40% budget remaining) makes different routing, composition, and exploration decisions than one in Thriving mode (70-100%). Without mortality, agents have no intrinsic pressure to be efficient.

**Mandatory falsifiers.** Every Heuristic requires a concrete falsifier -- a condition under which it should be considered wrong. This prevents unfalsifiable belief accumulation and enables automatic retirement when evidence shifts.

---

## 10. Novel Patterns vs. Borrowed Patterns

### Novel or Unusual

**Demurrage-based knowledge management** applies Silvio Gesell's 1916 monetary theory to agent memory. Knowledge pays a holding cost; use restores value. This is original in the agent framework space -- no other system treats knowledge retention as an economic mechanism with explicit rate laws and tier progression.

**Predict-publish-correct as structural learning** makes learning an emergent property of the pub/sub fabric rather than a separate subsystem. Every Cell automatically participates in calibration by virtue of publishing predictions and subscribing to error topics on the same Bus that carries all other events.

**AntiKnowledge with HDC repulsion** goes beyond "delete wrong entries." Known-bad knowledge actively repels future similar entries via HDC similarity matching, preventing rediscovery of known mistakes. The repulsion decays via demurrage, so old mistakes eventually stop blocking new discoveries.

**Hindsight relabeling** recovers learning signal from failed trajectories by decomposing them into sub-goals and relabeling achieved sub-goals as successes. The claimed 45% recovery rate means nearly half of otherwise-discarded episodes contribute to knowledge.

**Worldview emergence from co-citation** discovers belief clusters automatically rather than requiring explicit worldview definition. Maintaining rival worldviews with mandatory contrarian retrieval prevents cognitive monoculture.

**Vitality-modulated behavior** ties agent decision-making to resource pressure, creating qualitatively different behavior at different budget levels. This draws from Jonas's philosophy of biology (1966) -- organisms that face mortality develop genuine priorities.

**VCG auction for prompt assembly** applies mechanism design (Vickrey-Clarke-Groves auctions) to the problem of which context sections to include under a token budget. Section effects (Beta-distribution tracking which sections correlate with gate success) provide the bidding signal.

### Borrowed and Adapted

**Content-addressed data** (from Git, IPFS, blockchain consensus) applied to agent signals and knowledge entries.

**Workflow/Activity split** (from Temporal's durability model) for deterministic replay of agent pipelines.

**DAW composability** (from Bitwig, Ableton Live) for marketplace artifact composition -- Criteria as plugins, Profiles as presets, fork as remix.

**ERC-20 / ERC-721 composability precedent** applied to Cell/Graph composition: each new conforming Cell multiplies combinations with every existing Cell.

**Active inference / Free Energy Principle** (Friston 2006) for routing decisions and cognitive tier selection.

**Prospect theory** (Kahneman & Tversky 1979, 1992) for affect-modulated DeFi position sizing, with the canonical loss aversion coefficient of 2.25.

**Beta-Binomial conjugate calibration** (standard Bayesian inference) for Score Cell calibration, combined with temperature scaling (Guo et al. 2017) for cross-Cell score normalization.

**Reed's Law** (group-forming value) and **Wright's Law** (cost curves with volume) as economic mechanisms, not merely theoretical observations.

**Stigmergic coordination** (Dorigo 1992, from ant colony optimization) using pheromone Pulses on Bus for indirect agent-to-agent coordination.

---

## Summary

Roko's architecture achieves its composability through a deliberate reduction to five kernel primitives and four universal patterns. The governing constraint -- everything flows through Bus or Store, everything computes through Cells, everything composes through Graphs -- is what makes the system's many subsystems (agents, verification, learning, feeds, marketplace, DeFi) interoperate without special-purpose glue. The cost of this uniformity is learning the kernel vocabulary; the payoff is that every new Cell or Graph composes with every existing one combinatorially.
