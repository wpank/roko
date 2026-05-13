# 00 — Index

> Entry point. Read this first if you have never seen the runtime before.

---

## What Roko Is

**Roko** is an open agent runtime: a Rust toolkit for building AI agents that orchestrate LLM-backed work, learn from outcomes, and compose into larger systems. It is a kernel of five primitives, nine protocols, four universal patterns, and a small set of specializations on top, executed by a single engine that interprets every Graph the system runs. The defining property is composition: every subsystem — agent cognition, verification, learning, memory, marketplace, DeFi adjacency — is expressed as a Graph of the same Cells reading from a shared Store and broadcasting on a shared Bus, so any conforming new Cell composes with every existing Cell by construction.

Roko sits inside a larger stack called **agentchain**: an open agent runtime (Roko, this folder), a purpose-built blockchain (the Nunchi blockchain, with Daeji as its testnet), and a coordination layer for regulated finance. This folder describes only the agent runtime side. Where the runtime touches the chain — for posting attested knowledge, settling micropayments, proving identity — those touchpoints are described from the runtime's perspective only.

---

## Names

- **Nunchi** — the brand. The umbrella name across docs, surfaces, and the chain.
- **Nunchi blockchain** — the purpose-built chain that the runtime can post to and read from.
- **Daeji** — the Nunchi blockchain's testnet.
- **Roko** — the agent runtime described here. The Rust toolkit, the CLI, the control plane, the dashboard, the agent specialization, the executor.
- **agentchain** — the umbrella that contains Roko, the chain, and the coordination layer.
- **Korai (legacy)** — an earlier internal name. Not used anywhere in current docs or surfaces. Mentioned only here so that a reader who encounters the term in old artifacts knows it does not refer to anything live.

---

## Glossary

Every term used in any doc, defined in one place. Cross-link with the relevant doc for fuller treatment.

- **Activity Cell** — a non-deterministic Cell (LLM call, tool execution, network I/O); on resume its recorded output loads from disk rather than re-executing. See [Runtime](./02-runtime.md).
- **Adaptive Threshold** — gate pass/fail thresholds that move with EMA of pass rates per rung. L1 learning. See [Runtime](./02-runtime.md).
- **Affordance** — per-module score (extensibility, test coverage, doc coverage, coupling, stability, size) used as a routing hint.
- **Agent** — the most complex specialization: Space + Extensions + Memory + adaptive clock + vitality. See [Runtime](./02-runtime.md).
- **A2A** — Agent-to-Agent protocol; agent capability cards and discovery. See [Coordination](./03-coordination.md).
- **AntiKnowledge** — Signal kind that actively repels future Signals in the same HDC region after a previously trusted Signal was proven wrong.
- **Arena** — universal measurement surface for agent capability; a Compose-Verify-Score Graph with a leaderboard. See [Trust and Economy](./04-trust-and-economy.md).
- **Attestation** — orthogonal to taint; tracks who signed, not where data came from. Three levels: LocalAgent, OrgRole, ChainWitness.
- **Autocatalytic score** — out-degree of a Signal in the reversed lineage DAG; how many descendants used it.
- **BLAKE3** — content-hash function used for Signal identity, prompt cache keys, and config hashes.
- **Brain export** — portable knowledge bundle (Merkle-CRDT) for an Agent. See [Frontiers](./05-frontiers.md).
- **Bus** — ephemeral pub/sub fabric carrying Pulses. Sub-microsecond in-process; ~5ms across the relay; longer across NATS/Kafka/Chain backends.
- **Calibration** — the predict-publish-correct loop's accumulated state (Brier, Wilson CI, mean error, error variance).
- **CaMeL** — Capability-aware Memory Lattice; capability-tagged information flow control on Extensions and tool dispatch.
- **Capability** — fail-closed permission. Effective capability is the strict intersection of Cell-declared, Graph-allowed, and Space-permitted.
- **Cascade Router** — three-stage model selection router (Static → Confidence → LinUCB). See [Runtime](./02-runtime.md).
- **C-factor** — collective intelligence metric computed from runtime observables. Covariate, not objective.
- **Cell** — atomic computation unit with typed I/O, declared capabilities, protocol conformance. The kernel computation primitive.
- **Clade scoring** — selecting variants by descendant performance, not greedy single-generation benchmarks (HGM).
- **Cold storage** — Signal whose balance dropped below the cold threshold; body archived; lineage preserved; HDC fingerprint kept warm for thaw discovery.
- **Compose** — protocol that assembles Signals under a budget constraint via VCG auction.
- **CompoundKind** — `Compound([Kind])` lattice join in the Kind system.
- **Conductor** — meta-orchestrator agent role with ten cross-cutting watchers.
- **Connect** — protocol for lifecycle-bound external connections (chain RPC, MCP, database, webhook, API).
- **Connector** — Extension that conforms to the Connect protocol.
- **CorticalState** — lock-free atomic shared perception surface inside an Agent.
- **Custody** — dependent type recording who created, modified, reviewed, and approved every piece of data; required for privileged Signal kinds.
- **Daimon** — affect engine; PAD vector + somatic markers + prospect-theory loss aversion.
- **DAW pattern** — Digital Audio Workstation: nodes are plugins, connections are patch cables, parameters are knobs. Used in the Generative Canvas and the marketplace composability hierarchy.
- **Delta** — slowest cognitive timescale (60s–600s); dream consolidation, knowledge synthesis, pruning.
- **Demurrage** — attention-weighted holding cost on Signal balance (Gesell 1916). Replaces TTL.
- **Dream cycle** — four-phase consolidation (Hypnagogia, NREM Replay, REM Imagination, Threat Rehearsal) operating on episode batches during Delta.
- **DPO rewriting** — Double-Pushout hypergraph rewriting; preserves type-correctness by the pushout-complement theorem.
- **EFE** — Expected Free Energy (Friston 2006); the routing objective combining pragmatic value, epistemic value, cost, and a regime-conditioned penalty.
- **Engram** — internal Rust struct name for Signal; the public alias is `Signal`.
- **Episode** — a record of one agent turn; raw data feeding every learning loop.
- **ERC-8004** — on-chain agent identity and reputation registries.
- **Extension** — Functor-pattern Cell intercepting another Cell's pipeline at one of 22 hooks across 8 layers.
- **Falsifier** — mandatory field on every Heuristic; the concrete condition under which it should be considered wrong.
- **Feed** — Cell combining Connect + Trigger + Store; continuous data stream.
- **Flow** — runtime instance of a Graph; standard Flow runs once, Hot Flow re-fires on a clock.
- **Functor pattern** — cross-cut enriching Signals pre/post a Cell without changing Graph topology. One of the four universal patterns.
- **Gamma** — fast cognitive timescale (100–500ms); perception, reflexes, fast action.
- **Gate pipeline** — verification rungs (compile, test, clippy, diff, llm-judge, symbol, integration, property, benchmark, format, security, fact-check) producing a Verdict.
- **Gateway** — the inference gateway, a 9-stage Pipeline Graph mediating every LLM call.
- **Generative Canvas** — visual editor for Graphs of Cells, using twelve primitive node types in the DAW pattern.
- **Graduation** — `Pulse → Signal`; the only path from transport into the audit DAG.
- **Graph** — typed DAG (or cyclic graph) of Cells with edges, conditions, mappings. Implements Cell, so Graphs nest fractally.
- **Group** — Space specialization with membership and a coordination mode (Stigmergic, Pipeline, Broadcast, Leader-Follower).
- **HDC** — Hyperdimensional Computing; 10,240-bit binary vectors per Signal for similarity search and compositional encoding.
- **Heuristic** — Signal kind: when/then rule with mandatory falsifier and live calibration.
- **Hot Flow** — Graph that stays resident and re-fires on a clock; the cognitive loop is a Hot Flow.
- **Inference handle** — channel sender held by an agent into the gateway. Agents never see API keys.
- **InsightStore / KnowledgeStore** — substrate for durable knowledge entries with demurrage, tier progression, dream consolidation.
- **K\*** — entropy effective rank of the Gram matrix of agent output embeddings; label-free diversity metric.
- **Kind lattice** — join-semilattice over Signal kinds; `Compound([Kind])` is the join.
- **Lens** — Observe-protocol Cell producing observation Signals without mutating state.
- **Lineage** — content-hash DAG forming a free category; Signals are append-only and edges never removed.
- **Loop pattern** — Graph with a feedback edge. One of the four universal patterns.
- **Marketplace** — protocol (not platform) for trading Cells, Graphs, Racks, and Knowledge Bundles.
- **MAST** — the 14 multi-agent failure modes catalog (Cemri et al., NeurIPS 2025).
- **MCP** — Model Context Protocol; tool discovery and invocation between agents and external tools.
- **Memory** — Store-protocol Cell with demurrage, tier progression, dream consolidation, HDC retrieval.
- **METR** — Model Evaluation & Threat Research; the 50% time horizon benchmark.
- **MPP** — Metered Payment Protocol; session-based streaming payments.
- **Nunchi** — the brand and the chain; see Names.
- **Pareto front** — set of non-dominated Signals or Verdicts; multi-objective default in Verify and Score.
- **Passport** — the soulbound ERC-8004 NFT that represents an agent's on-chain identity.
- **PAD** — Pleasure / Arousal / Dominance vector representing agent affect state.
- **Pheromone** — Pulse on a `pheromone:{group_id}.{location_hash}` topic; stigmergic coordination signal that decays.
- **Pipeline pattern** — linear Graph with reject/transform/redirect edges. One of the four universal patterns.
- **Plan executor** — pure state machine that emits `ExecutorAction` values; the harness performs the I/O.
- **Playbook** — reusable step sequence extracted from successful episode clusters; injected into Layer 6 of the system prompt.
- **Predict-publish-correct** — universal learning mechanism: every Cell publishes its prediction, subscribes to its own error topic, adjusts. No separate ML pipeline.
- **Projection** — `Signal → Pulse`; lossy broadcast of stored Signals (forgets hash, fingerprint, score).
- **Protocol** — behavioural contract a Cell conforms to. Nine protocols: Store, Score, Verify, Route, Compose, React, Observe, Connect, Trigger.
- **Pulse** — ephemeral event carried by Bus. Sequence-numbered, ring-buffered, no lineage.
- **Q-LLM / T-LLM** — split where the query-LLM has zero direct tool access; the tool-LLM receives validated invocations through a capability-controlled pipeline (CaMeL).
- **Rack** — parameterized Graph with explicit Macros (knobs) and Slots (jacks).
- **React** — protocol for Pulse-driven response. Never changes Graph topology.
- **Recipe** — pure data Graph of Score Cells with no LLM calls and no agent involvement.
- **Regime** — ambient state (Calm, Normal, Volatile, Crisis) that modulates clocks, EFE weights, and verification rigor.
- **Relay** — internal cross-process Bus; carries Pulses between separate processes with low latency.
- **Route** — protocol for selecting among candidates; Roko uses EFE.
- **Score** — protocol for five-axis rating (relevance, quality, confidence, novelty, utility).
- **Signal** — durable datum: content-addressed (BLAKE3), typed, scored, decayed via demurrage, lineage-tracked, HDC-fingerprinted. Lives in Store.
- **Skill Library** — fine-grained reusable techniques, smaller than playbooks.
- **Space** — Graph that owns a Bus partition + a Store partition + capability grants. Isolation + collaboration boundary.
- **StateHub** — typed projections that surfaces subscribe to. Decouples observation from display.
- **Stigmergy** — indirect coordination through environmental modification (Grassé 1959; Dorigo 1992). Decay is mathematically necessary (Govcraft Theorem 3).
- **Store** — durable persistence fabric carrying Signals. JSONL on disk, in-memory, or chain-backed.
- **System prompt builder** — 9-layer prompt assembly with cache-stability ordering (system → session → task).
- **T0 / T1 / T2** — three model tiers (Reflex, Fast, Deep); EFE selects naturally.
- **Taint** — information-flow lattice classification (Clean → UserInput → ExternalFetch / LlmGenerated → Propagated). Only increases.
- **Theta** — middle cognitive timescale (500ms–16s); planning, evaluation, strategy adjustment.
- **Tier** — Signal retention level (Transient, Working, Consolidated, Persistent) with demurrage multipliers.
- **TraceRank** — multi-dimensional reputation across seven domain tracks.
- **Trigger** — protocol for event ingress. Seven kinds (Cron, Webhook, FileWatch, Bus, ChainEvent, Manual, SignalPattern).
- **Variance Inequality** — `Var[verifier - truth] < Var[generator - truth]`; verifier ensemble must be spectrally cleaner than generator.
- **VCG** — Vickrey-Clarke-Groves auction; truthful bidding is the dominant strategy. Used by Compose and arbitration.
- **Vitality** — `remaining_budget / initial_budget`; economic pressure scalar with five behavioural phases.
- **Worldview cluster** — co-citation cluster of heuristics; the system maintains multiple worldviews per domain and mandates 15% contrarian retrieval.
- **x402** — HTTP-native micropayments; Coinbase, May 2025; Base L2 sub-second finality.

---

## The Five Content Documents

| # | Doc | One-line summary |
|---|---|---|
| 1 | [01-foundations.md](./01-foundations.md) | The vocabulary and shape of the system: 5 primitives, 9 protocols, 4 universal patterns, 13 specializations, predict-publish-correct, fractal composition, and the categorical/compositional foundations that make composition work. |
| 2 | [02-runtime.md](./02-runtime.md) | How the system actually runs: one engine for every Graph, Flow / Hot Flow lifecycle, Workflow/Activity split, the Agent specialization with vitality and three cognitive timescales, memory and dreams, learning loops, telemetry, and configuration. |
| 3 | [03-coordination.md](./03-coordination.md) | How Roko interacts with the world: the inference gateway, feeds and recipes, groups and emergent communication, the four external protocols (MCP, A2A, ERC-8004, x402), extensions and triggers, tools, surfaces, and design principles for collective intelligence. |
| 4 | [04-trust-and-economy.md](./04-trust-and-economy.md) | Trust, identity, and value flows: capability intersection, taint lattice, lexicographic corrigibility, the cognitive immune system, authentication, x402 and MPP payments, marketplace mechanics, ERC-8004 identity, TraceRank reputation, arenas, and runtime-side DeFi adjacency. |
| 5 | [05-frontiers.md](./05-frontiers.md) | Operational concerns and research frontiers: deployment topology, cross-cuts as endofunctors, the orchestrator, long-horizon planning, self-improvement that does not collapse, MCTS over graph rewrites, hierarchical RL, multi-modal perception, biological inspiration, the Gaia2 inverse-scaling warning, metacognition, and the three-pillar anti-collapse stack. |

---

## Reading Paths

### Engineer onboarding (first time touching the runtime)

1. [01-foundations](./01-foundations.md) §§1–11 — primitives, protocols, predict-publish-correct.
2. [02-runtime](./02-runtime.md) §§1–8 — engine, Flow, lifecycle, the cognitive loop.
3. [03-coordination](./03-coordination.md) §§1–6 — gateway, connectivity, extensions, tools.
4. [02-runtime](./02-runtime.md) §§9–23 — agents, memory, learning, configuration, plan execution.
5. [05-frontiers](./05-frontiers.md) §§1–11 — deployment, orchestrator.

### Architecture reviewer (looking for the structural claims)

1. [01-foundations](./01-foundations.md) — the whole document, especially §§16–18 (algebra summary, anti-patterns, compositional foundations).
2. [02-runtime](./02-runtime.md) §§5–8, §16 — Workflow/Activity split, the cognitive loop, the why of memory.
3. [03-coordination](./03-coordination.md) §§9–13 — failure modes, PID, sheaf consensus, design principles.
4. [05-frontiers](./05-frontiers.md) §§10–22 — cross-cuts, frontiers, three-pillar anti-collapse stack.

### Safety reviewer (looking for the trust model)

1. [04-trust-and-economy](./04-trust-and-economy.md) §§1–6 — capability intersection, taint, CaMeL, lexicographic corrigibility, immune system.
2. [01-foundations](./01-foundations.md) §9 (Verify) and §§14–15 (provenance, lineage).
3. [03-coordination](./03-coordination.md) §6 (tool dispatch funnel), §§11–13 (deception detection, design principles).
4. [05-frontiers](./05-frontiers.md) §§20–21 — threat anchors, three-pillar stack.

### Integrator (using Roko from outside)

1. [03-coordination](./03-coordination.md) §§1–3 — exoskeleton, inference gateway, connectivity.
2. [03-coordination](./03-coordination.md) §§7–8, §14 — feeds and recipes, surfaces.
3. [04-trust-and-economy](./04-trust-and-economy.md) §§7–11 — auth, payments, marketplace, identity, reputation.
4. [05-frontiers](./05-frontiers.md) §§1–9 — deployment, daemon lifecycle, brain export, health endpoints.

---

## What Roko Is Not

To prevent confusion:

- Roko is **not** a chatbot wrapper, a single LLM, or a model-training framework. It is a runtime that orchestrates LLM-backed agents and learns from their outcomes.
- Roko is **not** the Nunchi blockchain. The chain is a separate folder. Roko uses the chain for posting attested knowledge, settling micropayments, and proving identity, but it can run entirely off-chain.
- Roko is **not** a frontend framework. It ships a TUI, an HTTP control plane, and an optional web dashboard, but the kernel knows nothing about UI. UI is a Surface.
- Roko is **not** a closed framework. Every concept is a protocol with a typed contract. Conforming Cells from third parties compose with built-ins by construction.

---

## Where to Go Next

For the kernel: [01-foundations](./01-foundations.md). For the running system: [02-runtime](./02-runtime.md). For external interaction: [03-coordination](./03-coordination.md). For trust, identity, and value: [04-trust-and-economy](./04-trust-and-economy.md). For deployment and research frontiers: [05-frontiers](./05-frontiers.md).
