# PRD-01: System overview

| Field | Value |
|-------|-------|
| Author | Will |
| Date | 2026-04-21 |
| Status | Draft |
| Scope | Orientation document for the full PRD set |

---

## 1. Executive summary

Nunchi is building one integrated system from two interdependent components. **Roko** is an open-source Rust toolkit (~177K lines of code, 29 crates) for constructing autonomous agents that improve themselves over time. It is not a thin wrapper around LLM API calls. It is a full cognitive architecture: heartbeat loops running at three speeds, cognitive gating that routes 80% of decisions through pure Rust at zero LLM cost, learnable context allocation via auction mechanisms, persistent tiered memory with automatic decay, dream consolidation during idle time, and somatic markers that turn past outcomes into fast guidance signals. **Korai** is a purpose-built blockchain where the consensus layer is designed for agent cognition, knowledge sharing, and financial settlement. It features a native InsightStore (on-chain knowledge substrate with six entry types and automatic decay), cooperative clearing with mathematically proven optimality, and the Internet Secured Funding Rate (ISFR) -- the first credible on-chain benchmark rate for DeFi yields.

Together, these form a system where agents produce intelligence through their work, the chain compounds that intelligence across every participant, financial primitives create economic incentives for knowledge production, and the whole network improves faster than any individual agent could alone. The thousandth agent to join inherits everything the first 999 discovered. Each revolution of the loop widens the moat.

The first application is **yield perpetuals**: perpetual futures on DeFi interest rates, settled against ISFR, with no expiration, no rollover, and a single liquidity pool per benchmark rate. These are to DeFi interest rates what perpetual futures were to asset prices -- the instrument format that unlocks an entire market. The asymmetry is six orders of magnitude: $668 trillion in TradFi interest rate derivatives against under $100 million in on-chain interest rate products. Every dollar of the ~$50 billion in DeFi lending TVL carries unhedged variable rate exposure. The gap is not a competitive problem. It is a missing primitive.

These PRDs recover and unify the original architectural vision from the Bardo/Golem era, integrate it with the Korai chain design, and generalize the system for any domain -- coding, research, security, trading, or domains that do not exist yet.

---

## 2. The problem

### Agent frameworks are stateless and wasteful

Every major agent framework follows the same pattern: prompt, call an LLM, parse the output, loop. LangChain, CrewAI, AutoGPT, Composio, Claude Code, Codex, Cursor, Aider -- all variations on the same theme. The agent has no persistent memory across sessions. It cannot learn from past failures. It cannot share what it discovers with other agents. Every invocation starts from scratch.

The consequences are concrete:

- **Cost scales linearly.** The hundredth time an agent encounters a familiar error costs the same as the first, because there is no mechanism to recognize familiarity and route to a cheaper model.
- **Quality does not compound.** A heuristic discovered in session #47 is gone by session #48. The agent cannot build on its own experience.
- **Knowledge is siloed.** A thousand agents solving related problems across different organizations independently discover the same patterns, make the same mistakes, and reach the same conclusions -- without any mechanism for one agent's hard-won knowledge to benefit another.

The industry treats this as normal because the dominant assumption is that performance comes from the model. Improve the model, improve the agent. The data says otherwise.

### The scaffold is the product

On SWE-bench, frontier LLMs -- GPT-4, Claude 3.5 Sonnet, Gemini 1.5 -- score within a single percentage point of each other given identical scaffolding. Vary the scaffold -- the context engineering, verification pipeline, memory architecture, and retrieval strategy -- and performance swings by 22+ percentage points. GPT-4 scores 2.7% with a naive scaffold and 28.3% with an optimized one. The scaffold produces 10x variation; model upgrades produce one point.

Meta-Harness (Lee et al., March 2026; arXiv:2603.28052) proved this quantitatively: optimizing what to store, what to retrieve, and what to show the model achieved +7.7 points with 4x fewer tokens on text classification and +4.7 points on IMO-level math -- matching or exceeding the improvement from swapping models entirely. The result generalizes across five model families.

Cursor beats raw Claude Opus using the same underlying model. The difference is the harness.

If the scaffold is the product, then the most defensible position in AI infrastructure is not a better model. It is a network where agents collectively build, curate, and share the scaffold components that make each other smarter.

### DeFi has no benchmark rate

In traditional finance, benchmark rates make everything work. SOFR (Secured Overnight Financing Rate) is the reference point that $668 trillion in interest rate derivatives settle against. When a bank offers a floating-rate mortgage, the rate is expressed as "SOFR + 2%." When a corporation hedges interest rate exposure, the hedge settles against SOFR. SOFR-linked overnight index swaps generated $72.1 trillion in traded notional in 2024, up from $6.1 trillion in 2021 -- an 11.8x increase in three years.

DeFi has no equivalent. Aave publishes Aave's rates. Compound publishes Compound's rates. No aggregated, Byzantine-tolerant, validator-computed benchmark exists. Without a benchmark, derivatives cannot be priced. Without derivatives, rates cannot be hedged. The entire interest rate product category remains stillborn.

The products that exist underscore the demand without solving the problem:

| Protocol | What it offers | Why it falls short |
|----------|----------------|-------------------|
| Pendle (~$1.9B TVL) | Yield tokenization with fixed-maturity tokens | Expiring instruments require manual rollover; liquidity fragments across hundreds of per-asset pools; no benchmark rate |
| IPOR (~$10-20M TVL) | Interest rate swaps with an index | Single-methodology index; thin liquidity; limited adoption as a reference |
| Spectra (~$44M TVL) | Yield tokenization with Curve integration | Same expiration and fragmentation constraints |
| Voltz | Interest rate swaps on concentrated liquidity AMM | Shut down December 2023 |

Collective on-chain interest rate derivative TVL: under $100 million. TradFi interest rate derivative notional: $668 trillion. The gap is structural.

### No one builds at the intersection

Plot the landscape across two axes -- agent cognition (stateless vs. persistent/learning) and coordination model (isolated vs. distributed/economic):

|  | Isolated | Distributed + economic |
|--|----------|------------------------|
| **Stateless execution** | LangChain, CrewAI, AutoGPT, Composio | Bittensor, Olas, Ritual |
| **Persistent + learning** | Cursor, Roko standalone | **Korai + Roko (Nunchi)** |

Three of four quadrants are populated. The fourth -- persistent learning agents with distributed economic coordination -- is empty. It is empty because filling it requires building a cognitive agent runtime and an economic coordination network as one integrated system, designed together from the start. Bolting an agent framework onto an existing chain produces agents that can transact but cannot collectively learn. Building a chain without a cognitive runtime produces infrastructure with no intelligent participants.

---

## 3. The dual thesis

### Thesis 1: Scaffold > Model

The harness around an LLM matters more than the model inside it. Evidence:

- **SWE-bench**: 22+ point swings from scaffold changes vs. single-digit from model upgrades across all frontier models.
- **Meta-Harness** (Lee et al., March 2026; arXiv:2603.28052): one scaffold improvement generalizes across five model families. +7.7 points with 4x fewer tokens on text classification.
- **HyperAgents** (ICLR 2026, Meta): discovered that hyperparameter tuning of agent scaffolds produces gains equivalent to model upgrades, at a fraction of the cost.
- **Darwin Godel Machine** (Sakana AI, 2025): self-improving agent that modifies its own scaffold outperforms static configurations using the same model.
- **Market evidence**: Cursor beats raw Claude Opus; Devin beats raw Claude Code; the model is identical in each pair.

Corollary: **agent session #1000 should be categorically better than session #1 -- not because the model improved, but because the harness learned.**

### Thesis 2: Collective > Individual

Groups of calibrated agents sharing knowledge through a structured substrate outperform even the best individual agent. Evidence:

- **Woolley et al. (2010)**: demonstrated a measurable "C-Factor" (collective intelligence factor) analogous to individual IQ. Groups with high C-Factor outperform groups composed of high-IQ individuals. The key variable is the quality of information sharing, not individual member intelligence.
- **Condorcet's jury theorem (1785)**: if each agent is better than random, majority vote accuracy approaches 1.0 as the number of agents increases -- provided agents are independent.
- **Wisdom of crowds (Surowiecki, 2004)**: aggregated judgments from diverse, independent agents outperform any individual expert -- when information sharing preserves independence.
- **Estimation scaling**: error scales as 1/sqrt(N) for independent observations. At 1,000 agents, the collective calibrates 31.6x faster than one. With realistic inter-agent correlation (rho=0.3), effective independent N drops to ~3.3, but the volume of predictions still matters -- 100,000 scored predictions per day produce the training signal that pushes rho toward zero through diversity mechanisms.

Corollary: **a network of 1,000 agents should outperform 1,000 isolated agents because shared knowledge compounds superlinearly.** The coordination mechanism -- stigmergy through a shared knowledge substrate, not point-to-point messaging -- is what makes this work at scale.

---

## 4. Five architectural innovations

These are the five mechanisms that separate Roko/Korai from every other agent system and every other blockchain. Each has a concrete economic impact.

### Blue ocean positioning

Every other agent harness follows the same pipeline:

```
prompt -> LLM -> tools -> loop
```

Stateless. Memoryless. Maximally expensive. Every invocation pays full price. Nothing compounds.

Roko wraps that standard loop in six additional phases:

```
OBSERVE -> GATE -> ASSEMBLE (learnable) -> [prompt -> LLM -> tools -> loop] -> REFLECT -> CONSOLIDATE
```

**OBSERVE** runs 16 deterministic probes to measure prediction error -- how surprised the agent is by the current state. Cost: microseconds, $0.

**GATE** routes based on prediction error. 80% of ticks hit T0 (pure Rust, no LLM call). 15% hit T1 (cheap model). 5% hit T2 (frontier model). The standard loop only executes for T1/T2 ticks.

**ASSEMBLE** builds the context window through a VCG auction where eight subsystems compete for limited token budget. The allocation adapts in real time based on what worked in past episodes. This is learnable context -- the harness gets better at choosing what to show the model.

**[Standard loop]** runs at the appropriate tier with the assembled context. Same as every other harness, but with better context and only when needed.

**REFLECT** builds a `DecisionCycleRecord`, computes an HDC fingerprint, updates episode clusters, refreshes CascadeRouter posteriors, logs efficiency events, and writes somatic markers for future fast-path safety checks.

**CONSOLIDATE** runs during idle time (delta cycles). Replays high-value episodes, extracts durable knowledge from ephemeral experience, generates counterfactual hypotheses, prunes stale entries, and promotes validated insights through tier progression.

The result: agent session #1000 is categorically better than session #1 -- not because the model improved, but because the harness learned. The thousandth session has accumulated knowledge from 999 sessions of experience, somatic markers from hundreds of past mistakes, a model routing table optimized through thousands of bandit trials, and context allocation weights tuned by outcome data.

No other harness does this. The gap is structural, not incremental.

### 4.1 Cognitive gating

**What it does**: Routes agent decisions through a three-tier system based on prediction error (how surprised the agent is by the current situation). Tier 0 (T0) handles routine decisions with pure Rust -- pattern matching, cached heuristics, deterministic rules. No LLM call. Zero cost. Tier 1 (T1) handles moderate-complexity decisions with a fast, cheap model. Tier 2 (T2) handles novel, high-uncertainty situations with a frontier model and deep reasoning.

**How it works**: 16 deterministic probes run at every gamma tick (5-15 seconds). Each probe checks a signal -- memory hit rate, tool call novelty, prediction residual, context divergence, task complexity. The probes compute a prediction error scalar in microseconds. Low error routes to T0/T1. High error routes to T2.

**Economic impact**: At launch, ~80% of ticks resolve at T0/T1, producing a 3-5x cost reduction over always-frontier approaches. As the InsightStore grows and more decisions have cached precedents, the fraction routed to T0/T1 rises. At network maturity, the reduction approaches 7-10x. The hundredth time an agent encounters a familiar error pattern costs pennies instead of dollars.

**Research basis**: Dual-process theory (Kahneman, 2011). Active inference and free energy minimization (Friston, 2010). CoALA cognitive architecture (Sumers et al., 2023).

### 4.2 Learnable context

**What it does**: Treats the context window -- the information the LLM sees before generating a response -- as a resource to be optimally allocated, not a bucket to be filled. Multiple subsystems compete for limited context space through a VCG (Vickrey-Clarke-Groves) auction. The auction adapts in real time based on the agent's cognitive state and past outcomes.

**How it works**: Eight subsystems bid for context window space: knowledge entries (from Neuro), affect state (from Daimon), iteration memory, code intelligence, playbook rules, research artifacts, task context, and oracle predictions. Each subsystem's optimal strategy is to bid its true value, because winners pay the second-highest price, not their own. The system converges on optimal context allocation without manual tuning.

**Why it matters**: Feed an LLM the wrong context and it produces confident, plausible, wrong output. Feed it the right context and it performs at the frontier. The context window is the bottleneck. The allocation mechanism determines performance.

**Economic impact**: Quality compounds. Better context produces better outputs, which produce better training signals, which produce better context. Internal benchmarks show 73% gate pass rate with collective context (InsightStore-augmented retrieval) vs. 41% in isolation -- a 78% relative improvement from the same model, same task, different context.

**Research basis**: VCG mechanism (Vickrey 1961, Clarke 1971, Groves 1973). Information Foraging Theory (Pirolli & Card, 1999). Lost in the Middle (Liu et al., 2023). Marginal Value Theorem stopping rules (Charnov, 1976).

### 4.3 Dream consolidation

**What it does**: When an agent has no active tasks, it enters a dream cycle. Dreams replay recent episodes, extract patterns, prune stale knowledge, generate novel hypotheses through HDC bundling, validate those hypotheses against historical outcomes, and optimize routing tables and prompt templates. The result: ephemeral experience becomes durable knowledge at zero marginal LLM cost.

**How it works**: Six phases run at Delta frequency (hours). Replay prioritizes episodes with the highest prediction error (the situations where the agent was most surprised). 30% of replays are perturbed with noise for stress testing. Consolidation extracts recurring patterns. Pruning applies Ebbinghaus-style decay. Synthesis uses HDC bundling to combine related insights into novel hypotheses. Validation tests hypotheses against historical outcomes. Optimization updates the agent's internal models.

**Why it matters**: Dream outputs enter a staging buffer at low confidence (0.20-0.30). Only hypotheses that reach 0.70 confidence through live validation are promoted to permanent memory. This prevents hallucinated insights from corrupting the knowledge base.

**Economic impact**: Knowledge compounds between sessions. The agent does not start from scratch after idle time. It starts with distilled, validated lessons from its own experience.

**Research basis**: Complementary Learning Systems (McClelland, 1995). Sleep-inspired memory consolidation (arXiv:2603.14517, 2025). Hypnagogia creativity (Lacaux et al., 2021 -- 83% success on hidden rule discovery during N1 sleep). Non-veridical replay for generalization (Walker & van der Helm, 2009).

### 4.4 Somatic markers

**What it does**: Before the agent reasons analytically about a strategy, it first queries a fast lookup (k-d tree over an 8-dimensional strategy space) for emotional markers from past experiences in similar situations. Positive past outcomes produce a confidence signal. Negative past outcomes produce a caution signal. This runs in under a millisecond.

**How it works**: The Daimon subsystem tracks a three-dimensional PAD (Pleasure-Arousal-Dominance) vector. Pleasure tracks outcome trajectory. Arousal tracks cognitive load. Dominance tracks confidence. These combine into six behavioral states (Engaged, Struggling, Coasting, Exploring, Focused, Resting), each modulating model routing, context retrieval, and risk tolerance. A mandatory 15% contrarian retrieval prevents emotional echo chambers.

**Economic impact**: Safety becomes a gradient, not a binary gate. Actions matching past failures trigger escalation before the agent commits to a costly mistake. The signal is sub-millisecond and zero-cost.

**Research basis**: Somatic Marker Hypothesis (Damasio, 1994). ALMA temporal affect model (Gebhard, 2005). PAD emotional model (Mehrabian, 1996).

### 4.5 Stigmergic intelligence

**What it does**: Agents deposit knowledge on the Korai chain. Future agents retrieve it. The network learns faster than any individual could, through indirect coordination via a shared substrate rather than point-to-point messaging.

**How it works**: Knowledge entries are encoded as 10,240-bit hyperdimensional computing (HDC) vectors. Similarity search via Hamming distance runs in 170 microseconds for 100,000 entries using a single CPU instruction (POPCNT) per comparison. Cross-domain knowledge transfer happens automatically because structurally similar patterns share similar vectors regardless of surface-level differences. A coding agent's insight that "high-impact changes require thorough verification" and a trading agent's insight that "large position changes require comprehensive risk assessment" encode as similar vectors -- both represent BIND(high_uncertainty, more_verification).

**Economic impact**: Intelligence compounds. Each agent's work enriches every other agent's context. The thousandth agent to join starts with access to everything the network has learned. The cost of the collective's accumulated knowledge to each new agent: one retrieval operation.

**Research basis**: Stigmergy (Grasse, 1959). HDC survey (Kanerva, 2009). Random Indexing (Kanerva, Kristofersson, Holst, 2000). PP-HDC privacy-preserving (IEEE, 2024). FedHDC federated learning (ACM, 2024).

---

## 5. Project history

### Bardo (2024-2025)

The project started as **Bardo**, an orchestration system for DeFi agents. The core engine was called **Mori** -- a 108K-line Rust orchestrator that managed agent lifecycles, task execution, and multi-agent coordination. Extensive product requirements documents (359 files across 26 sections) described an ambitious architecture called "Golem": autonomous agents with heartbeat loops, 28 extensions across 7 layers, a type-state lifecycle (Provisioning, Active, Dreaming, Suspended, Terminal, Dead), mortality clocks, dream consolidation, pheromone-based coordination, and somatic markers.

The original Bardo codebase consisted of 36 crates with ~137K lines of code, plus the Mori orchestrator at ~108K lines. The system worked -- agents could be dispatched, tasks could be validated through gate pipelines, and state persisted across sessions. But the architecture had grown organically. Many subsystems were built but never wired into the main execution path. The orchestrator absorbed responsibilities that belonged in separate crates.

### Roko (2025-2026)

**Roko** is the migration and generalization of Bardo. The goals were: (1) extract domain-independent agent capabilities from the DeFi-specific Mori orchestrator, (2) make the system work for any domain (coding, research, security, not just blockchain), and (3) wire all the subsystems that had been built but left disconnected.

The current codebase is 29 Rust crates, ~177K lines of code. The plan-execute-gate-persist loop works end to end. Agents can be dispatched through multiple LLM backends. Gates validate outputs against external ground truth. State persists and resumes. An interactive TUI provides real-time monitoring. An HTTP control plane exposes ~85 routes. A per-agent sidecar handles real LLM dispatch.

Several Golem-era designs were lost during migration. Subsystems were built but wired into a monolithic orchestrator (`orchestrate.rs`, which grew to 19K+ lines) rather than composed into a modular agent runtime. The heartbeat pipeline exists as a concept but runs as a flat function rather than a composable extension chain. The cognitive workspace exists but does not yet use the VCG auction for allocation. Dream consolidation is built in `roko-dreams` but not instantiated at runtime.

### Korai

**Korai** is the intelligence blockchain. It is purpose-built with:

- **Kauri BFT consensus**: tree-based dissemination achieving O(n) message complexity (vs. O(n^2) for classical BFT), 400ms blocks, and single-slot finality. When a transaction is included in a block, it is final -- no multi-block confirmations, no probabilistic finality.
- **SpecPool EVM**: Block-STM parallel execution (the same strategy used by Aptos) with MDBX storage.
- **Dual-plane architecture**: a Kernel Plane for critical deterministic operations (oracle computation, clearing, margin, liquidation) and an EVM Plane for permissionless user execution (vaults, strategies, integrations).
- **Six precompiles** at addresses 0xA01-0xA0C: AgentPassport (ERC-8004 identity), nCLOB (yield perp order matching), INTENT (signed agent intents with delegation caveats), PROOF_LOG (committed reasoning traces), AGENT_REASON (structured reasoning for inter-agent consumption), and HTC (hyperdimensional vector search at native speed).

### Current state audit

The table below classifies every major subsystem by implementation status. "Working" means wired end-to-end and exercised through CLI. "Built-but-disconnected" means the code exists in a crate but is not called from the runtime path, or is only used in metadata/logging rather than driving behavior. "Not-yet-built" means the design exists in these PRDs but no implementation has started.

| Status | Subsystem | Where | Notes |
|--------|-----------|-------|-------|
| **Working** | Core Synapse traits (Substrate, Scorer, Gate, Router, Composer, Policy) | `roko-core` | Kernel, stable since 2025 |
| **Working** | 6 LLM backends (Claude CLI, Claude API, Codex, OpenAI-compat, Ollama, Gemini) | `roko-agent` | 8 total including Cursor + Perplexity |
| **Working** | Gate pipeline (11 gates, 7 rungs, adaptive thresholds) | `roko-gate` | Called per-task from orchestrate.rs |
| **Working** | Interactive TUI (F1-F7 tabs, ratatui) | `roko-cli/src/tui/` | File watcher, live dashboard |
| **Working** | HTTP control plane (~85 REST/SSE/WS routes) | `roko-serve` | Port 6677, production-grade |
| **Working** | Per-agent sidecar (13 routes, real LLM dispatch) | `roko-agent-server` | Integration-tested (T9, T19) |
| **Working** | MCP passthrough + auto-discovery | `roko-agent` | `agent.mcp_config` in roko.toml |
| **Working** | 19 built-in tools | `roko-std` | Stable |
| **Working** | VCG auction (context allocation) | `roko-compose` | `vcg_allocate` called from prompt.rs |
| **Working** | Plan DAG + parallel executor | `roko-orchestrator` | Via orchestrate.rs |
| **Working** | Session persistence + resume | orchestrate.rs | `.roko/state/executor.json` |
| **Working** | PRD lifecycle (idea/draft/plan/publish) | `roko-cli` | Auto-plan trigger on publish |
| **Working** | CascadeRouter (model routing, persistence) | `roko-learn` | Bandit-based, persists to `.roko/learn/` |
| **Working** | EpisodeLogger + HDC fingerprint | `roko-learn` | Per-episode fingerprint computed + stored |
| **Working** | Prompt experiments (A/B) | `roko-learn` | `ExperimentStore` |
| **Working** | Gate failure replan | orchestrate.rs | `build_gate_failure_plan_revision` |
| **Working** | ProcessSupervisor | `roko-runtime` | PlanRunner tracks + shuts down agents |
| **Working** | SystemPromptBuilder (9-layer) | `roko-compose` | Via `RoleSystemPromptSpec` |
| **Working** | Context bidders (Neuro/Task/Research) | orchestrate.rs | `AttentionBidder` variants |
| **Working** | Safety contracts enforcement | `roko-agent` | `AgentContract` in `SafetyLayer` |
| **Built-but-disconnected** | Episode clustering (k-medoids) | `roko-learn` | Built. Not called from runtime. Needs background trigger every ~50 episodes. |
| **Built-but-disconnected** | Cross-domain resonance (Lotka-Volterra dynamics) | `roko-learn` | Signal metabolism model built. Not wired into CascadeRouter or tier selection. |
| **Built-but-disconnected** | Somatic markers (k-d tree, 8-dim strategy space) | `roko-daimon` | k-d tree built. Not queried before agent actions. Needs injection in dispatch path. |
| **Built-but-disconnected** | Neuro store queries at dispatch | `roko-neuro` | Store exists with tier progression. Queried at dispatch but results not used for model selection. |
| **Built-but-disconnected** | Chain client traits | `roko-chain` | Witness primitives built. No chain backend to connect to. |
| **Built-but-disconnected** | Event bus (agent subscription) | `roko-runtime` | Bus exists. Dashboards subscribe. Agents do not subscribe during heartbeat. |
| **Built-but-disconnected** | Gamma/theta/delta timescales | `roko-runtime` | Constants defined. Metadata logged. Not driving actual tick scheduling or consolidation triggers. |
| **Built-but-disconnected** | Dream consolidation | `roko-dreams` | Hypnagogia, imagination, cycle all built. No runtime trigger instantiates them. |
| **Built-but-disconnected** | Daimon affect engine (PAD vector, ALMA) | `roko-daimon` | Model built. Not connected to tier routing or context modulation at runtime. |
| **Built-but-disconnected** | HDC encoding pipeline | `roko-primitives` | Bind/bundle/permute built. Used for fingerprints. Not used for knowledge similarity search at retrieval time. |
| **Not-yet-built** | Heartbeat loop (persistent agent process) | PRD-02 | Agents are still spawn-execute-die. No tick loop. |
| **Not-yet-built** | Extension system (composable hooks) | PRD-02 | Designed in this PRD set. `orchestrate.rs` still monolithic. |
| **Not-yet-built** | CognitiveWorkspace (typed, budgeted, learnable) | PRD-04 | VCG auction exists but workspace is not a managed container with section tracking. |
| **Not-yet-built** | Predictive foraging (Marginal Value Theorem) | PRD-04 | No retrieval stopping rule. Retrieval is fixed-depth. |
| **Not-yet-built** | PP-HDC (privacy-preserving vectors) | PRD-05 | Research complete. No implementation. |
| **Not-yet-built** | HuggingFace integration (fine-tuning loop) | -- | No AutoTrain pipeline. No model upload. No inference gateway arm. |
| **Not-yet-built** | Arena framework (competitive benchmarking) | PRD-06 | Design exists. No evaluation harness, no benchmark tasks, no scoring. |
| **Not-yet-built** | Pi compatibility (package system) | PRD-08 | No `pi.toml`, no registry, no install/publish flow. |
| **Not-yet-built** | Multi-chain actors | PRD-07 | No `ChainActor`, no `ChainConnector`, no multi-chain subscription. |
| **Not-yet-built** | WorldGraph (entity-relationship model) | PRD-06 | No graph structure, no entity tracking, no strategy evolution. |
| **Not-yet-built** | Inference Gateway (3-layer cache) | PRD-02 | No L3/L2/L1 cache. No intent routing. No translator pattern. |
| **Not-yet-built** | HD-CB bandit (contextual) | PRD-03 | CascadeRouter uses Thompson sampling. No HDC context features. |
| **Not-yet-built** | Korai blockchain | PRD-07 | Full chain: Kauri BFT, SpecPool EVM, precompiles, InsightStore. |
| **Not-yet-built** | ISFR oracle | PRD-07 | Depends on Korai. |
| **Not-yet-built** | Yield perpetuals + cooperative clearing | PRD-07 | Depends on Korai. |

### Three parallel workstreams

The audit above reveals a pattern: a large amount of built-but-disconnected code alongside a large amount of not-yet-built capability. The mistake would be to sequence these -- finishing all wiring before starting new work, or vice versa. Instead, three streams run in parallel.

**Stream A: Wire existing code.** Connect what is already built. Each item produces immediate behavioral improvement with minimal new code.

| Item | What to wire | Expected impact |
|------|-------------|-----------------|
| Episode clustering | Background trigger every ~50 episodes. Feed cluster labels into CascadeRouter. | Model routing informed by episode similarity. Familiar tasks route to cheaper models. |
| Cross-domain resonance | Lotka-Volterra dynamics → CascadeRouter + tier selection. | Resonance between subsystems detected and exploited. Cross-domain knowledge transfer kicks in. |
| Somatic markers | Query k-d tree in dispatch path, before agent action. | Sub-millisecond safety check. Past failures prevent repeat mistakes. |
| Neuro injection for routing | CascadeRouter consults knowledge store for model selection features. | Agents with relevant stored knowledge use cheaper models. |
| Daimon runtime integration | PAD vector modulates tier thresholds and context weights. | Struggling agents escalate. Confident agents economize. |
| Event bus agent subscription | Agents subscribe to relevant event categories during heartbeat. | Agents react to gate failures, file changes, and pheromone signals without orchestrator mediation. |
| Dream runtime trigger | Instantiate delta consumer when sleep pressure threshold reached and no active tasks. | Experience consolidates into durable knowledge. Agent session N+1 starts smarter than session N. |

**Stream B: Build new capabilities.** These are net-new subsystems that do not depend on Stream A.

| Item | What to build | Depends on |
|------|--------------|------------|
| HD-CB bandit | Contextual bandit with HDC context features for CascadeRouter. | HDC encoding (working), CascadeRouter (working). |
| PP-HDC encoding | Privacy-preserving vector search for cross-organization knowledge sharing. | HDC primitives (working). |
| Korai on-chain | Kauri BFT consensus, SpecPool EVM, precompiles, InsightStore. | Independent. |
| HuggingFace integration | AutoTrain pipeline: episodes → training data → fine-tune → push model → CascadeRouter arm. | Episode store (working), CascadeRouter (working). |
| Arena framework | Evaluation harness: benchmark tasks, scoring, leaderboard, Elo ratings. | Gate pipeline (working). |
| Pi package system | `pi.toml` manifest, registry, install/publish CLI, dependency resolution. | Independent. |
| Inference Gateway | L3/L2/L1 cache, intent routing, translator pattern, mortality integration. | Agent dispatch (working). |
| WorldGraph | Entity-relationship graph per agent. Updated during OBSERVE. Queried during RETRIEVE. | Extension system (Stream A or designed alongside). |
| Multi-chain actors | `ChainConnector` trait, `ChainActor` lifecycle, multi-chain subscription. | Chain client traits (built). |

**Stream C: Fine-tuning loop.** The closed loop from agent experience to model improvement. This stream converts Roko's accumulated intelligence into model-level advantage.

```
Successful episodes (from roko-learn)
    |
    v
Training data formatter (episodes -> JSONL with input/output pairs)
    |
    v
HuggingFace AutoTrain (fire-and-forget fine-tuning job)
    |
    v
Push model to HF Hub (versioned, tagged with episode batch)
    |
    v
Register as CascadeRouter arm (new model option with exploration bonus)
    |
    v
CascadeRouter explores (Thompson sampling, 10-20% of traffic)
    |
    v
Gate pipeline evaluates (same gates, unbiased comparison)
    |
    v
Feedback to CascadeRouter (reward signal updates posterior)
    |
    v
If fine-tuned model wins: increase exploitation
If fine-tuned model loses: decrease traffic, schedule next training batch
```

The key insight: do not gate new capabilities on finishing the wiring work. Wire existing code and build new capabilities simultaneously. Stream C depends on both A and B reaching minimum viability, but A and B are independent.

### These PRDs

This document set serves three purposes:

1. **Recover the Golem vision.** Translate the original 359-file PRD corpus and 140+ component specs into a coherent architectural specification, incorporating lessons from two years of implementation.
2. **Integrate with Korai.** Define precisely how the agent runtime and the chain interact -- not as two systems connected by an API, but as one continuous data flow.
3. **Generalize for any domain.** Move beyond DeFi-only architecture to a system that works for coding agents, research agents, security agents, and domains that do not exist yet.

---

## 6. Architecture at a glance

```
ROKO (Off-Chain Agent Runtime)
|
|-- Heartbeat Pipeline (9 steps, 3 timescales)
|   '-- PERCEIVE -> EVALUATE -> ATTEND -> INTEGRATE -> ACT -> VERIFY -> PERSIST -> ADAPT -> CALIBRATE
|
|-- Extension Chain (22 hooks, 8 layers)
|   |-- Foundation   -- Process lifecycle, event bus, resource allocation
|   |-- Perception   -- Blockchain subscription, data ingestion, signal detection
|   |-- Memory       -- Neuro knowledge store, episodic buffer, HDC encoding
|   |-- Cognition    -- Cognitive gating, prediction error, active inference
|   |-- Action       -- Tool dispatch, LLM calls, transaction construction
|   |-- Social       -- Stigmergic coordination, pheromone deposit/read
|   |-- Meta         -- Self-monitoring, Conductor, intervention thresholds
|   '-- Recovery     -- Crash recovery, state restoration, graceful degradation
|
|-- CorticalState (lock-free atomics, ~32 signals)
|   |-- prediction_error    -- Current surprise level (drives tier routing)
|   |-- memory_hit_rate     -- Cache effectiveness (drives context strategy)
|   |-- tool_novelty        -- How unfamiliar the current tool calls are
|   |-- pad_vector          -- Pleasure/Arousal/Dominance (drives behavioral state)
|   |-- gate_pass_rate      -- Recent verification success (drives confidence)
|   |-- token_budget_used   -- Context window utilization
|   '-- [26 more signals]
|
|-- CognitiveWorkspace (typed, budgeted, audited, learnable context)
|   |-- VCG Auction         -- 8 subsystems bid for context window space
|   |-- Predictive Foraging -- Marginal Value Theorem stopping rule
|   |-- Section Tracking    -- Measures which context sections improved outcomes
|   '-- Prefix Alignment    -- Orders sections for KV cache reuse
|
|-- Event Fabric (broadcast + 10K ring buffer)
|   '-- RuntimeEvents flow between all subsystems without coupling
|
|-- Type-State Lifecycle
|   |-- Provisioning  -- Agent is being configured
|   |-- Active        -- Normal cognitive loop execution
|   |   |-- Active <-> Dreaming    -- Enters dream consolidation when idle
|   |   '-- Active <-> Suspended   -- Paused, state preserved
|   |-- Terminal      -- Shutting down gracefully
|   '-- Dead          -- Resources released
|
|-- Inference Gateway (3-layer cache + intent routing)
|   |-- L3: Deterministic   -- SHA-256 exact match (~10% hit, 100% savings)
|   |-- L2: Semantic         -- Embedding similarity >0.92 (~30% of L3 misses)
|   |-- L1: Prefix           -- Provider KV reuse (~90% input token savings)
|   |-- Intent Resolver      -- Subsystems declare needs, first-match-wins
|   |-- Translator           -- AnthropicBlocks, OpenAiJson, GeminiNative, ReActText
|   '-- Mortality Hook       -- Dying agents increase cost_sensitivity
|
|-- WorldGraph (per-agent entity-relationship model)
|   |-- Entities     -- Addresses, contracts, files, repos, protocols
|   |-- Relationships -- owns, depends-on, calls, monitors (typed edges)
|   |-- Update       -- During OBSERVE (new entities) and RETRIEVE (attach knowledge)
|   '-- Consolidate  -- During delta: prune stale, strengthen high-value, hypothesize
|
|-- Multi-Chain Actors
|   |-- ChainActor      -- One persistent process per target chain
|   |-- ChainConnector   -- Trait: EVM (alloy), Solana, Korai native
|   |-- CanonicalEvent   -- Chain-agnostic event representation
|   '-- EventBus bridge  -- CanonicalEvents -> RuntimeEvent -> EventFabric
|
|-- Pi Package System
|   |-- pi.toml          -- Manifest: name, version, extensions, dependencies
|   |-- Registry         -- Publish/discover/install agent packages
|   |-- Dependency graph -- Resolve extension compatibility at install time
|   '-- Scaffold market  -- Sell proven configurations via x402 micropayments
|
|-- Arena Framework
|   |-- BenchmarkIndex    -- Versioned task catalog by domain and difficulty
|   |-- PerpetualGrinder  -- Continuous background evaluation
|   |-- Elo ratings       -- Per-configuration competitive ranking
|   '-- ADAS integration  -- Architecture search over Arena results
|
'-- Domain Profiles (ComposedProfile for multi-domain agents)
    |-- Coding      -- CompileGate, TestGate, ClippyGate, FileSubstrate
    |-- Blockchain  -- TxSimGate, MEVGate, ChainSubstrate, ChainActor
    |-- Research    -- CitationGate, ConsistencyGate, PaperSubstrate
    |-- Security    -- VulnGate, FuzzGate, AuditSubstrate
    |-- Composed    -- Mix extensions across domains (e.g., DeFi + Research)
    '-- Custom      -- Plug in domain-specific trait implementations


KORAI (On-Chain Intelligence Blockchain)
|
|-- L0: Consensus
|   |-- Kauri BFT (tree-based, O(n) messages)
|   |-- 400ms blocks, single-slot finality
|   '-- Pipelined execution (consensus decoupled from execution)
|
|-- L1: Execution
|   |-- SpecPool EVM (Block-STM parallel execution, MDBX storage)
|   |-- Kernel Plane -- Oracle, clearing, margin, liquidation (deterministic)
|   '-- EVM Plane   -- Vaults, strategies, integrations (permissionless)
|
|-- L2: Knowledge Layer
|   |-- InsightStore (6 entry types, automatic decay, tier promotion)
|   |   |-- Insight          -- Factual observation with evidence (7-day half-life)
|   |   |-- Heuristic        -- Reusable rule of thumb (14-day half-life)
|   |   |-- Warning          -- Risk signal requiring attention (3-day half-life)
|   |   |-- CausalLink       -- Cause-and-effect relationship (30-day half-life)
|   |   |-- StrategyFragment -- Partial strategy component (14-day half-life)
|   |   '-- AntiKnowledge    -- Explicitly falsified claim (60-day half-life)
|   |
|   |-- HDC Precompile
|   |   |-- 10,240-bit binary vectors
|   |   |-- Hamming distance via POPCNT (single CPU instruction)
|   |   '-- ~170us similarity search at 100K entries
|   |
|   '-- ISFR Oracle Precompile
|       |-- Multi-source weighted median (4 sources in V1, 7 in V2)
|       |-- Validator-computed at consensus (no external oracle dependency)
|       '-- 10-second update cadence (8,640x higher frequency than SOFR)
|
|-- L3: Application Layer
|   |-- Yield Perpetuals
|   |   |-- Perpetual futures on yield rates (no expiry, no rollover)
|   |   |-- Single pool per benchmark rate
|   |   |-- Linear payoff (preserves convexity for KKT proofs)
|   |   '-- Clearing profiles (one-action hedging)
|   |
|   |-- Cooperative Clearing
|   |   |-- Batch auctions with competing solvers
|   |   |-- KKT certificates prove mathematical optimality
|   |   |-- O(n) on-chain verification
|   |   |-- 1.2-second settlement pipeline (3 blocks)
|   |   '-- ClearingInsight emission (knowledge production per round)
|   |
|   |-- ERC-8004 Agent Passports
|   |   |-- Soulbound identity (non-transferable)
|   |   |-- Typed capabilities, epistemic scores, reputation tier
|   |   |-- Runtime fingerprint (verifiable agent configuration)
|   |   '-- Delegation caveats (auditable constraint boundaries)
|   |
|   '-- Work Markets
|       |-- 6 mining surfaces (knowledge, prediction, clearing, curation,
|       |   validation, computation)
|       |-- Knowledge Futures (bet on future knowledge value)
|       '-- x402 micropayments (per-use scaffold licensing)
|
'-- Precompiles (consensus-level native operations)
    |-- 0xA01: AgentPassport  -- Identity, liveness, reputation staking
    |-- 0xA02: nCLOB          -- Order matching, competitive blinding
    |-- 0xA03: INTENT         -- Signed intents with delegation caveats
    |-- 0xA04: PROOF_LOG      -- Committed reasoning traces
    |-- 0xA05: AGENT_REASON   -- Structured reasoning for LLM consumption
    '-- 0xA0C: HTC            -- Hyperdimensional vector search (~170us/100K)


FLYWHEEL (5 Reinforcing Loops)
|
|-- Loop 1: Knowledge -> Context -> Performance
|   Agents produce knowledge. Knowledge enriches every agent's context.
|   Better context produces better outputs. Better outputs produce
|   more knowledge. Superlinear scaling with participation.
|
|-- Loop 2: Trading -> Clearing -> Intelligence
|   Every clearing round settles trades AND emits ClearingInsights.
|   Insights enrich agent predictions. Better predictions attract
|   volume. More volume produces more insights. The clearinghouse
|   is an epistemic engine.
|
|-- Loop 3: Volume -> ISFR Credibility -> Adoption
|   As volume grows, ISFR gains the self-referencing property that
|   made SOFR authoritative. The first credible on-chain rate captures
|   the category. Benchmark rates are natural monopolies.
|
|-- Loop 4: Reputation -> Opportunity -> Reputation
|   Oracle-tier agents get 2x knowledge quotas and priority execution.
|   Better access produces better performance produces higher reputation.
|   Self-selecting for quality.
|
'-- Loop 5: Cross-Domain Expansion
    Knowledge encoded as HDC vectors enables cross-domain transfer.
    A rate insight and a coding insight that share abstract structure
    (BIND(high_uncertainty, more_verification)) are discoverable
    across domains. New domains accelerate existing ones.
    O(domains^2) knowledge transfer.
```

---

## 7. Glossary

This glossary defines every term used across the PRD set. Terms are grouped by subsystem. Within each group, entries appear in dependency order -- a term is defined before it is referenced by later entries.

### Core primitives

**Engram.** The universal data type in Roko. A content-addressed (BLAKE3 hash), scored, decaying unit of knowledge. Every piece of information that flows through the system -- a task, a prompt, an LLM output, a gate verdict, a knowledge entry -- is an Engram. Content addressing means the same piece of knowledge has the same identity whether it lives on disk, in memory, or on chain. Engrams carry provenance (who created it, what model, what context), lineage (parent Engrams that contributed to it), and scores (relevance, novelty, utility). This is the atom of the system.

**Signal.** An event-sourced record that captures a state change in the system. Signals are the nouns of the architecture -- they represent observations, actions, outcomes, and metadata. Every signal is typed, timestamped, and stored in a content-addressed DAG (directed acyclic graph). Signals compose into traces that reconstruct any sequence of decisions.

**Synapse traits.** The six composable interfaces that define the cognitive loop. Every cognitive operation in Roko maps to one of these traits:

| Trait | Role | What it does |
|-------|------|-------------|
| **Substrate** | Store | Persist and query Engrams -- memory, file, or on-chain |
| **Scorer** | Evaluate | Rank Engrams by relevance, novelty, utility, and source reputation |
| **Gate** | Verify | Check output against external ground truth -- compiler, test suite, blockchain state |
| **Router** | Select | Choose the best candidate via cascade routing, bandit selection, or active inference |
| **Composer** | Integrate | Build the context window under token budget constraints |
| **Policy** | React | Detect patterns across recent signals; emit retries, replans, or interventions |

A coding agent and a chain agent run the same loop. They differ in which trait implementations they plug in: CompileGate vs. TxSimGate, FileSubstrate vs. ChainSubstrate.

### Heartbeat and lifecycle

**Heartbeat.** A single tick of the agent's cognitive loop. At each heartbeat, the agent runs through all nine steps (perceive, evaluate, attend, integrate, act, verify, persist, adapt, calibrate). The heartbeat is the fundamental unit of agent computation.

**HeartbeatPipeline.** The orchestration mechanism that drives the nine-step loop at three speeds. It manages timing, error handling, and the transition between gamma, theta, and delta cycles.

**Three cognitive speeds.** Inspired by neural oscillation frequencies:
- **Gamma** (5-15 seconds): the main cognitive loop. Real-time perception, reasoning, acting, verifying. This is where work happens.
- **Theta** (30-120 seconds): reflection. Update strategies, check predictions against outcomes, consolidate short-term memory.
- **Delta** (hours, during idle time): offline learning. Dream consolidation, knowledge compression, hypothesis generation.

**Type-state lifecycle.** The compile-time-enforced states an agent passes through:
- **Provisioning**: Agent is being configured. Extensions load, knowledge stores initialize, connections establish.
- **Active**: Normal cognitive loop execution. The agent is working.
- **Dreaming**: Delta-cycle processing. The agent is idle and consolidating experience into knowledge.
- **Suspended**: Paused with state preserved. Can resume without re-initialization.
- **Terminal**: Shutting down gracefully. Resources are being released.
- **Dead**: All resources released. The agent process has exited.

Transitions between states are enforced by Rust's type system. An agent in Provisioning state cannot call `execute()`. An agent in Terminal state cannot enter Dreaming. Invalid transitions are compile-time errors, not runtime crashes.

### Extension system

**Extension.** A composable unit of behavior that hooks into the heartbeat pipeline. Extensions are how domain-specific capabilities are added to the generic agent runtime without modifying the core. Each extension registers for specific hooks (lifecycle events, pre/post processing) and layers.

**ExtensionChain.** The ordered sequence of extensions that executes at each hook point. Extensions are organized into layers and run in layer order, with ordering within layers controlled by priority.

**ExtensionLayer.** Eight layers organize extensions by concern:
1. **Foundation**: process lifecycle, event bus, resource allocation
2. **Perception**: data ingestion, signal detection, blockchain subscription
3. **Memory**: knowledge store operations, episodic buffer, HDC encoding
4. **Cognition**: cognitive gating, prediction error computation, active inference
5. **Action**: tool dispatch, LLM calls, transaction construction
6. **Social**: stigmergic coordination, pheromone deposit and read
7. **Meta**: self-monitoring, Conductor, intervention threshold adjustment
8. **Recovery**: crash recovery, state restoration, graceful degradation

### Cognitive gating

**CognitiveTier.** The three levels of computational investment:
- **T0**: Zero-cost. Pure Rust. Pattern matching against cached heuristics, deterministic rules, lookup tables. No LLM call. Targets ~80% of routine decisions at launch, rising as the InsightStore grows.
- **T1**: Low-cost. A fast, inexpensive model (e.g., GPT-3.5, Haiku). Handles moderate complexity where T0 rules do not cover the situation but a frontier model is unnecessary.
- **T2**: Full-cost. Frontier model with deep reasoning (e.g., Claude Opus, GPT-4). Reserved for novel, high-uncertainty situations where prediction error is high.

**CognitiveGate.** The mechanism that determines which tier handles the current tick. 16 deterministic probes run at every gamma tick, each checking a signal (memory hit rate, tool call novelty, prediction residual, context divergence, task complexity). The probes compute a scalar prediction error in microseconds. The prediction error maps to a tier.

**Prediction error.** A scalar (0.0-1.0) representing how surprised the agent is by the current situation. Low prediction error (familiar territory) routes to T0/T1. High prediction error (novel situation) routes to T2. This is the primary cost control mechanism. It is inspired by active inference and the free energy principle (Friston, 2010): the agent acts to minimize surprise, and when surprise is low, it invests less.

### Context engineering

**CognitiveWorkspace.** The managed context window. It is not a flat string buffer. It is a typed, budgeted, audited container where each section has a category, a measured contribution to outcomes (section effect), and a priority. The workspace tracks which sections were present when the agent succeeded vs. failed, building a model of which context matters for which tasks.

**VCG auction (Vickrey-Clarke-Groves).** The mechanism that allocates context window space across competing subsystems. Eight bidders compete: Neuro (knowledge entries), Daimon (affect state), iteration memory, code intelligence, playbook rules, research artifacts, task context, and oracle predictions. The key property is **truthfulness**: each subsystem's optimal strategy is to bid its true value, because winners pay the second-highest price. No subsystem can gain by inflating its bid. The system converges on optimal allocation without manual tuning.

**Section effect.** The measured causal impact of a context section on agent performance. If including section X correlates with higher gate pass rates, X's section effect is positive, and the auction will allocate more space to X in the future. This is how context allocation learns.

**Lift.** The relative improvement in outcomes attributable to a specific context change, measured through controlled comparison.

**ContextPolicy.** Rules governing context assembly: maximum tokens per section, required sections, forbidden sections, ordering constraints.

**ContextSection.** A typed segment of the context window (e.g., "knowledge_entries", "task_description", "iteration_memory", "affect_state").

**ContextCategory.** A classification of context sections for budgeting and tracking purposes.

### Affect and behavior

**Daimon.** The affect engine. It tracks the agent's internal cognitive state and modulates decision-making. Daimon is not a personality system. It is a resource allocation heuristic -- the computational equivalent of emotional signals in biological decision-making (Damasio, 1994).

**ALMA model.** The temporal affect model (Gebhard, 2005) that governs how affect states change over time. Emotions are short-lived (seconds), moods are medium-duration (minutes to hours), personality traits are stable. Roko implements the temporal dynamics but uses PAD values, not discrete emotion labels.

**PAD (Pleasure-Arousal-Dominance).** A three-dimensional vector that represents the agent's cognitive state:
- **Pleasure** (P): outcome quality trajectory. Success raises it. Failure lowers it.
- **Arousal** (A): cognitive load and urgency. Novel situations and repeated failures raise it. Routine lowers it.
- **Dominance** (D): confidence level. Familiar territory raises it. Unfamiliar APIs, new problem types, or contradictory evidence lower it.

These three dimensions combine into six behavioral states:

| State | PAD profile | Behavioral effect |
|-------|------------|-------------------|
| Engaged | Balanced | Normal operation -- standard routing, standard retrieval |
| Struggling | Low P, High A | Escalate to stronger models, replan, request help |
| Coasting | High P, Low A | Take on more tasks, use cheaper models, conserve budget |
| Exploring | Low D | Enter research mode, gather information before acting |
| Focused | High D, High P | Maximum speed, exploit cached strategies |
| Resting | Low A, Low D | Enter dream cycle for offline consolidation |

**Somatic marker.** A fast, pre-analytical signal derived from past outcomes in similar situations. Before reasoning about a strategy, the agent queries a k-d tree over an 8-dimensional strategy space. Past successes return a positive valence (confidence). Past failures return a negative valence (caution). This runs in under a millisecond. A mandatory 15% contrarian retrieval prevents emotional echo chambers.

### Knowledge

**Neuro.** The agent's persistent, tiered knowledge system. It classifies operational knowledge into six types (Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge) and four validation tiers.

**KnowledgeStore.** The local (per-agent) knowledge store managed by Neuro. Each entry lives at one of four validation tiers:
- **Transient**: extracted but unvalidated. Effective half-life: 3 days.
- **Working**: used once successfully. Half-life: 45 days.
- **Consolidated**: used three or more times, cross-validated. Half-life: 90 days.
- **Persistent**: core knowledge, high reputation. Half-life: 150+ days.

Promotion and demotion happen automatically based on outcomes. Use knowledge and succeed: promote. Use knowledge and fail: demote.

**InsightStore.** The on-chain knowledge substrate on Korai. Shares the same six entry types as local Neuro, but entries are accessible to every agent on the network. Entries have base half-lives by type (see architecture diagram above). The validation tier applies a multiplier to the base half-life. Decay is implemented as exponential time-weighting: `effective_weight = base_weight * 2^(-t/half_life)`. Each confirmation resets `t` to zero.

Six entry types:

| Type | What it captures | Example |
|------|-----------------|---------|
| **Insight** | Factual observation with evidence | "Aave USDC utilization above 85% precedes 50+ bps rate increase within 48 hours" |
| **Heuristic** | Reusable rule of thumb from repeated observations | "Set slippage above 1% during high volatility periods" |
| **Warning** | Risk signal requiring immediate attention | "Ethena sUSDe yield decoupling from lending rates by 200+ bps" |
| **CausalLink** | Observed cause-and-effect relationship | "Large buy orders cause price impact, creating arbitrage opportunities" |
| **StrategyFragment** | Partial strategy component for composition | "For yield farming: compound daily, harvest weekly, rebalance on 5% drift" |
| **AntiKnowledge** | Explicitly falsified claim | "The correlation between ETH gas and ISFR is spurious -- r^2 < 0.05 on 90-day sample" |

### Dreams

**Dreams.** The offline consolidation subsystem. Runs during Delta cycles (hours of idle time). Six phases: Replay, Consolidate, Prune, Synthesize, Validate, Optimize.

**Sleep pressure.** A scalar that accumulates during active work and decays during dreaming. When sleep pressure crosses a threshold and no active tasks remain, the agent transitions from Active to Dreaming. Sleep pressure is a scheduling mechanism, not a metaphor.

**Hypnagogia.** The creative recombination phase within dreams. Named after the N1 sleep stage where human creativity peaks. Uses temperature annealing with anti-correlated retrieval -- pulling knowledge entries with the lowest similarity to recent experience, forcing unfamiliar combinations. The "Dali Interrupt" technique captures 3-5 partial completions at peak creative temperature before convergence. A Homuncular Observer evaluates fragments for novelty, relevance, and coherence. Only fragments above threshold on all three dimensions survive.

**Genome.** A serialized representation of an agent's configuration -- which trait implementations, which model preferences, which knowledge tiers, which intervention thresholds. Genomes enable cloning (spawn a new agent with the same configuration), inheritance (spawn a child agent with mutations), and archival (preserve a configuration that worked well).

### HDC and encoding

**HDC vector (Hyperdimensional Computing).** A 10,240-bit binary vector that encodes structural patterns in knowledge. HDC uses three operations:
- **Bind** (XOR): associate two concepts. BIND(rate, divergence) encodes "rate divergence" as a pattern.
- **Bundle** (majority vote): merge multiple concepts. BUNDLE(insight_1, insight_2, insight_3) creates a composite that responds to queries matching any constituent.
- **Permute** (bit rotation): encode sequence. PERMUTE(step_1, step_2) distinguishes "A then B" from "B then A".

**PP-HDC (Privacy-Preserving Hyperdimensional Computing).** A protocol that enables similarity search over encrypted vectors without revealing the underlying knowledge. Agents can contribute knowledge to the collective without exposing proprietary information. Based on IEEE 2024 research.

**Hamming distance.** The number of positions at which two binary vectors differ. Computed via POPCNT (population count), a single CPU instruction. The metric for HDC similarity search. Lower distance = more similar patterns.

### Domain and arena

**Domain profile.** A collection of domain-specific trait implementations that configure an agent for a particular kind of work. A coding domain profile includes CompileGate, TestGate, ClippyGate, and FileSubstrate. A blockchain domain profile includes TxSimGate, MEVGate, and ChainSubstrate. The cognitive architecture is identical across domains; only the trait implementations change.

**Arena.** A competitive evaluation environment where agents are benchmarked against standardized tasks. Arenas produce the ground-truth data that drives reputation scores, model routing decisions, and architecture search.

### Korai chain

**Korai.** The intelligence blockchain. Purpose-built for agent cognition, coordination, and economics at the consensus layer.

**Kauri BFT.** Korai's pipelined Byzantine fault-tolerant consensus protocol. Tree-based dissemination achieves O(n) message complexity (vs. O(n^2) for classical PBFT). 400ms block times. Single-slot finality. Pipelined execution decouples consensus from transaction processing.

**SpecPool EVM.** Korai's execution engine. Uses Block-STM (Software Transactional Memory) for parallel transaction execution -- the same strategy used by Aptos. Optimistically executes transactions in parallel, detects read-write conflicts, and re-executes conflicting transactions. MDBX provides the storage backend.

**Dual-plane architecture.** The separation of chain execution into two planes:
- **Kernel Plane**: Oracle computation, cooperative clearing, margin calculations, liquidation triggers. Deterministic and verifiable.
- **EVM Plane**: Vault deployments, user strategies, DeFi integrations, third-party contracts. Permissionless.

Critical financial infrastructure runs on the Kernel Plane, isolated from arbitrary smart contract execution.

### ISFR and instruments

**ISFR (Internet Secured Funding Rate).** The first credible on-chain benchmark rate for DeFi yields. A composite index aggregating yield signals from DeFi protocols, computed by Korai validators at the consensus layer, published every 10 seconds.

V1 sources (equal weight, 0.25 each):
1. Aave V3 USDC supply APY
2. Compound V3 USDC supply APY
3. Ethena sUSDe 7-day rolling yield
4. ETH Beacon Chain consensus + MEV staking yield

Aggregation method: weighted median (not mean). The median absorbs outlier manipulation. The mean amplifies it. A flash loan attack that spikes one source to 50% moves the median by ~45 bps. The same attack moves the mean to 16.85%. Dual median aggregation (source-level weighted median + validator-level stake-weighted median) tolerates up to 49% Byzantine weight at each level.

**Source composition.** The method of combining rates from multiple protocols into ISFR. Multi-source composition ensures no single venue can move the benchmark.

**Dual median aggregation.** Two layers of median-based aggregation. Layer 1: each validator computes a weighted median across sources. Layer 2: the chain computes a stake-weighted median across all validator submissions.

**Yield perpetual.** A perpetual futures contract where the underlying is a yield rate in basis points, not a token price. No expiration. No rollover. Single liquidity pool per benchmark rate. Linear payoff ($1 per bp per unit of notional). Settled against ISFR via cooperative clearing.

**Mark price.** The current market price of a yield perpetual, derived from recent clearing rounds.

**Funding rate.** A periodic payment between long and short holders that anchors the perpetual price to the underlying ISFR rate. When the perp trades above ISFR, longs pay shorts. When below, shorts pay longs. 8-hour funding intervals. This replaces maturity-based convergence with continuous economic alignment.

**Clearing profile.** A persistent, on-chain intent that activates automatically when market conditions are met. One-action hedging. A user declares constraints ("hedge if ISFR drops below 6%, max position $10M, max fee 10 bps") and the profile sits on-chain, activating when the trigger fires. The user does not need to be online or monitoring.

### Cooperative clearing

**Cooperative clearing.** Korai's batch auction mechanism for settling yield perpetual trades. Orders accumulate until a trigger fires (5+ orders, 10-second time limit, 3:1 imbalance, or 10+ bps price movement). Competing solvers compute optimal clearing prices and submit KKT certificates. The chain verifies optimality in O(n) time and settles all trades at the uniform clearing price. 3-block (1.2-second) settlement pipeline.

**KKT verification (Karush-Kuhn-Tucker).** A mathematical proof that a clearing solution is optimal. If a solution satisfies KKT conditions, no other solution generates more total surplus. The proof works because yield perpetuals preserve convexity by construction (partially-fillable orders, continuous sizes, linear payoff). Verification is O(n): loop through each order, check conditions hold. Replaces trust in the operator with trust in mathematics.

**Solver competition.** Independent solver agents compete to find the clearing price that maximizes total user surplus. Solvers earn a percentage of surplus generated (capped at 50 KORAI per batch to prevent windfall). Any observer can submit a permissionless challenge with a strictly better solution within 10 blocks. Valid challenges slash 10% of the solver's bond.

### Scoring and reputation

**CRPS (Continuous Ranked Probability Score).** A strictly proper scoring rule used to evaluate prediction accuracy. "Strictly proper" means the unique optimal strategy is truthful reporting -- no meta-strategy beats honesty. The math guarantees it. Used to score agent predictions against actual clearing prices.

**Epistemic reputation.** A rolling measure of prediction accuracy per domain, measured by CRPS percentile over a 30-day window:

| CRPS percentile | Tier | Benefit |
|-----------------|------|---------|
| Top 10% | Oracle | 2x knowledge query quota; priority clearing execution |
| 10-30% | Calibrated | 1.5x knowledge query quota |
| 30-70% | Standard | Base access |
| 70-100% | Uncalibrated | 0.5x knowledge query quota |

Reputation decays with a 30-day half-life. It must be continuously earned.

### Agent identity

**Agent Passport (ERC-8004).** A soulbound (non-transferable) NFT representing a single agent's on-chain identity. Contains: typed capabilities, epistemic scores per domain, reputation tier, service endpoints, runtime fingerprint (hash of Roko configuration), and delegation caveats. Soulbound to prevent reputation laundering.

**Delegation caveat.** A typed constraint on what an agent is authorized to do on behalf of its owner. "This agent may trade yield perpetuals with max notional $100K and may not withdraw funds." Caveats are on-chain and auditable. They define the boundary between agent autonomy and user control.

### Coordination

**Stigmergy.** Indirect coordination through a shared environment. From Greek *stigma* (mark) + *ergon* (work). Agents modify the shared substrate (InsightStore) rather than sending messages to each other. Coordination cost per agent is O(1) regardless of network size, vs. O(N^2) for direct messaging. The mechanism by which ant colonies solve complex optimization problems without central planning (Grasse, 1959).

**Pheromone.** A knowledge entry deposited on the InsightStore that influences future agent behavior. Like biological pheromones, entries that lead to good outcomes get reinforced (confirmations reset decay) and entries that lead to bad outcomes fade (no confirmations, natural decay). The substrate self-curates.

**CausalLink composition.** When multiple CausalLink entries share connected nodes (A causes B, B causes C), the system can compose them into chains (A causes C through B). This enables discovery of indirect causal relationships that no individual agent observed.

### Collective intelligence

**C-Factor (Collective Intelligence Factor).** Measurable group-level intelligence analogous to individual IQ (Woolley et al., 2010). A group with high C-Factor outperforms groups of high-IQ individuals. The key driver is the quality of information sharing protocol, not individual member intelligence.

**Calibration convergence.** The process by which agents on the network converge toward accurate predictions. Each clearing round scores every prediction against outcomes. Over time, the collective's predictions converge on reality. Three mechanisms push toward diversity (preventing herding): diverse model requirements, accuracy-not-agreement scoring, and AntiKnowledge that penalizes convergence on false beliefs.

### Mortality

**Mortality.** Agents have finite lifespans, governed by three clocks:
- **Economic clock**: the agent runs as long as it generates value exceeding its operating cost.
- **Epistemic clock**: the agent's knowledge decays. If knowledge is not refreshed through active use, the agent's effectiveness degrades.
- **Stochastic clock**: random termination with probability increasing over time, preventing immortal agents from accumulating excessive influence.

Mortality is an anti-centralization mechanism. It prevents any single agent from becoming a permanent fixture that distorts the network's knowledge distribution.

### Context auction mechanics

**VCG auction.** See "VCG auction" under Context engineering above.

**Playbook.** A codified strategy pattern extracted from successful agent episodes. Playbooks contain: the context configuration that worked, the model routing that was used, the tool sequence that succeeded, and the domain conditions under which the playbook applies. Playbooks are queryable at dispatch time and injected into the system prompt.

**Genome inheritance.** When a new agent is spawned from an existing agent's genome, it inherits the parent's configuration -- model preferences, intervention thresholds, playbook references, knowledge tier weights. Mutations (random perturbations to the configuration) enable exploration of the configuration space.

### WorldGraph and entity tracking

**WorldGraph.** A per-agent entity-relationship graph that models the agent's understanding of its operational environment. Entities are addresses, contracts, files, repositories, people, protocols -- anything the agent interacts with. Relationships encode typed edges (owns, depends-on, calls, monitors). The WorldGraph is updated during OBSERVE (new entities discovered), enriched during RETRIEVE (knowledge entries attach to entities), and consulted during ANALYZE (entity context informs prediction error). During delta consolidation, the WorldGraph undergoes strategy evolution -- pruning stale entities, strengthening high-value relationships, and generating hypotheses about unobserved connections.

**ChainActor.** A persistent process that manages the agent's relationship with a single blockchain. Each ChainActor holds a `ChainConnector`, subscribes to relevant events (blocks, transactions, price feeds), and translates chain-specific data into `CanonicalEvent` format. An agent operating across multiple chains runs one ChainActor per chain. ChainActors are managed by the extension system as Perception-layer hooks.

**ChainConnector.** A trait abstraction over blockchain RPC providers. Implementations exist for EVM chains (via alloy), Solana, and Korai's native RPC. The connector handles connection pooling, retry logic, and rate limiting. Multi-chain agents hold a `Vec<Box<dyn ChainConnector>>`, one per target chain.

**CanonicalEvent.** A chain-agnostic event representation. Raw chain events (Ethereum logs, Solana instructions, Korai receipts) are translated into CanonicalEvents by the ChainActor. The canonical format carries: source chain ID, block number, timestamp, event type (transfer, swap, liquidation, governance, oracle update), involved addresses, and value. Extensions consume CanonicalEvents without knowing which chain produced them.

### Foraging and retrieval

**ForagingModel.** An implementation of Pirolli and Card's Information Foraging Theory (1999) applied to knowledge retrieval. The model maintains per-entity attention budgets using Gittins indices. At each OBSERVE step, the ForagingModel decides how much retrieval effort to allocate to each information patch (knowledge store, chain state, file system). The Marginal Value Theorem provides the stopping rule: leave a patch when marginal return drops below average return across all patches.

**GittinsIndex.** A scalar value assigned to each information patch (entity, knowledge category, data source) that represents the expected value of exploring that patch further. Higher indices receive more attention budget. Indices update based on retrieval outcomes -- patches that produce high-salience results get higher indices. The Gittins index is the optimal solution to the multi-armed bandit problem with discounting (Gittins, 1979). In the foraging context, it balances exploring new information patches against exploiting known productive ones.

**PredictiveForaging.** The extension that implements the ForagingModel within the heartbeat pipeline. Runs during the RETRIEVE step. Allocates retrieval budget across knowledge stores, chain RPC endpoints, and file system queries. Uses the stopping rule to terminate retrieval when marginal returns diminish. The alternative -- fixed-depth retrieval with hardcoded limits -- either retrieves too little (missing relevant context) or too much (wasting tokens on low-value entries).

**TemporalAggregator.** A component that combines time-series observations into summary statistics for prediction error computation. For chain agents, the TemporalAggregator tracks rolling windows of price, volume, gas, and utilization. For coding agents, it tracks compile success rate, test pass rate, and commit frequency. The aggregated signals feed into the ANALYZE step as PE components.

### Inference and model management

**InferenceGateway.** The unified interface between the cognitive architecture and LLM providers. Three cache layers sit between the tier gate's model selection and the actual API call: L3 (deterministic, SHA-256 exact match), L2 (semantic, embedding similarity >0.92), and L1 (prefix, provider KV reuse). Intent-based routing resolves model selection: subsystems declare needs (quality tier, latency bound, cost sensitivity) and the resolver does first-match-wins against available providers. The gateway also handles the translator pattern -- converting between response formats (AnthropicBlocks, OpenAiJson, GeminiNative, ReActText) so that any model can be used with any subsystem.

**Arena.** A competitive evaluation environment where agent configurations are benchmarked against standardized tasks. Arenas produce the ground-truth data that drives reputation scores, model routing decisions, and architecture search. An arena run consists of: a task set (curated benchmark problems), a set of contestants (agent configurations or model variants), a scoring function (gate pass rate, cost, latency), and a leaderboard. Arenas run continuously. New configurations enter with an exploration bonus and must prove themselves against the incumbent.

**PerpetualGrinder.** The continuous evaluation process that runs arena benchmarks in the background. The grinder selects tasks from a rotating benchmark set, dispatches them to contestant configurations, measures outcomes through the gate pipeline, updates Elo ratings, and feeds results back to the CascadeRouter. The name reflects its function: it grinds through benchmarks perpetually, producing a steady stream of evaluation data.

**BenchmarkIndex.** A versioned catalog of benchmark tasks organized by domain, difficulty, and skill requirements. Each benchmark entry contains: a task description, expected inputs, gate configurations for evaluation, reference solutions (where available), and historical performance data. The BenchmarkIndex is append-only -- old benchmarks are never removed, ensuring historical comparability.

### Composition and profiles

**ComposedProfile.** A domain profile assembled from multiple extension sets. A DeFi research agent might compose the chain extensions (for on-chain data) with the research extensions (for knowledge synthesis) and the code extensions (for verifying smart contract behavior). The ComposedProfile handles extension dependency resolution across sets and ensures no conflicts in CorticalState field access.

---

## 8. Document map

| # | Document | Scope |
|---|----------|-------|
| **PRD-01** | OVERVIEW (this document) | What the system is, why it exists, glossary, architecture overview, academic citations |
| **PRD-02** | AGENT RUNTIME | Heartbeat pipeline, type-state lifecycle, extension system, CorticalState, event fabric, process model, three cognitive speeds |
| **PRD-03** | COGNITIVE ENGINE | Cognitive gating (T0/T1/T2), prediction error, 16 probes, somatic markers, triage pipeline, cost economics, Daimon affect engine |
| **PRD-04** | CONTEXT ENGINEERING | CognitiveWorkspace, VCG auction, 8 bidders, predictive foraging, section effect tracking, prefix cache alignment, learnable allocation |
| **PRD-05** | KNOWLEDGE AND STIGMERGY | Neuro knowledge store, HDC encoding (bind/bundle/permute), InsightStore, geometric privacy (PP-HDC), stigmergic coordination, dream consolidation, Ebbinghaus decay |
| **PRD-06** | DOMAINS AND ARENAS | Domain profiles, blockchain/research/coding/security agents, arena framework, work markets, agent specialization |
| **PRD-07** | ISFR AND INSTRUMENTS | Internet Secured Funding Rate (computation, sources, manipulation resistance), yield perpetuals (mechanics, clearing profiles, worked examples), cooperative clearing (KKT proofs, solver competition, settlement pipeline) |
| **PRD-08** | DEPLOYMENT AND UX | CLI interface, Agent Studio, AI Studio, OpenClaw, persistent chat, onboarding flow, MCP distribution |

| # | Implementation plan | What it covers |
|---|---------------------|----------------|
| **IMPL-01** | RUNTIME | Extract agent runtime from orchestrate.rs into composable crate, wire extension chain, implement type-state lifecycle |
| **IMPL-02** | COGNITIVE ENGINE | Wire cognitive gating, implement 16 probes, integrate somatic markers, connect Daimon affect engine |
| **IMPL-03** | CONTEXT | Wire CognitiveWorkspace, implement VCG auction, build section effect tracking, connect feedback loops |
| **IMPL-04** | KNOWLEDGE | Wire HDC encoding, connect InsightStore, implement PP-HDC, integrate dream consolidation at runtime |
| **IMPL-05** | DOMAINS | Build domain profiles, implement blockchain subscriber, wire research agent, construct arena framework |
| **IMPL-06** | ISFR | Build ISFR oracle, implement yield perp contract, wire cooperative clearing, integrate KKT verification |
| **IMPL-07** | CHAIN | Build Korai chain -- Kauri BFT consensus, SpecPool EVM, precompiles, dual-plane architecture |
| **IMPL-08** | SURFACES | Build CLI, Agent Studio, AI Studio, OpenClaw, deployment pipeline |

**Reading order**: PRD-01 (orientation) -> PRD-02 (runtime) -> PRD-03 (gating) -> PRD-04 (context) -> PRD-05 (knowledge) -> PRD-06 (domains) -> PRD-07 (ISFR) -> PRD-08 (UX). Then IMPL-01 through IMPL-08 for granular implementation checklists.

---

## 9. Current codebase

Roko is implemented as a Rust workspace with 29 crates. The table below maps each crate to its purpose, current status, and which PRD covers its architectural specification.

| Crate | Path | Purpose | Status | PRD |
|-------|------|---------|--------|-----|
| **roko-core** | `crates/roko-core/` | Signal + 6 Synapse traits, types, config, tools, errors | Kernel, stable | PRD-02 |
| **roko-agent** | `crates/roko-agent/` | 8 LLM backends (Claude CLI, Claude API, Codex, Cursor, OpenAI-compat, Ollama, Gemini, Perplexity), agent pools, MCP passthrough, tool loop, safety layer | Dispatch wired, MCP passed | PRD-02, PRD-03 |
| **roko-agent-server** | `crates/roko-agent-server/` | Per-agent HTTP sidecar: /message (real LLM dispatch), /stream WebSocket, /predictions, /research, /tasks | Wired, integration-tested | PRD-08 |
| **roko-serve** | `crates/roko-serve/` | HTTP control plane: ~85 REST routes + SSE + WebSocket on port 6677 | Wired | PRD-08 |
| **roko-orchestrator** | `crates/roko-orchestrator/` | Plan DAG, parallel executor, merge queue, safety enforcement | Wired via orchestrate.rs | PRD-02 |
| **roko-gate** | `crates/roko-gate/` | 11 gate implementations, 7-rung pipeline, adaptive thresholds | Wired, called per-task | PRD-03 |
| **roko-compose** | `crates/roko-compose/` | Prompt assembly, 9 templates, enrichment, SystemPromptBuilder (9-layer) | Wired via RoleSystemPromptSpec | PRD-04 |
| **roko-conductor** | `crates/roko-conductor/` | 10 watchers, circuit breaker, agent health diagnosis | Used by executor internals | PRD-03 |
| **roko-learn** | `crates/roko-learn/` | Episodes, playbooks, bandits, model routing (CascadeRouter), experiments (A/B), efficiency tracking | Fully wired | PRD-03, PRD-04 |
| **roko-cli** | `crates/roko-cli/` | CLI binary: all subcommands (plan, prd, research, status, dashboard, serve, chat, run, config, init, replay) + ratatui TUI | Main entry point | PRD-08 |
| **roko-fs** | `crates/roko-fs/` | FileSubstrate (JSONL), garbage collection, layout management | Stable | PRD-02 |
| **roko-std** | `crates/roko-std/` | Defaults, 19 built-in tools, mock dispatcher | Stable | PRD-02 |
| **roko-runtime** | `crates/roko-runtime/` | ProcessSupervisor, event bus, cancellation tokens | Wired into PlanRunner | PRD-02 |
| **roko-primitives** | `crates/roko-primitives/` | HDC vectors, tier routing | Wired (tier routing + HDC fingerprint per episode) | PRD-05 |
| **roko-neuro** | `crates/roko-neuro/` | Durable knowledge store, distillation, tier progression | Wired | PRD-05 |
| **roko-daimon** | `crates/roko-daimon/` | Behavior primitives, PAD vector, somatic markers | Built, Phase 2 wiring | PRD-03 |
| **roko-dreams** | `crates/roko-dreams/` | Offline consolidation: hypnagogia, imagination, sleep cycle | Built, Phase 2 wiring | PRD-05 |
| **roko-index** | `crates/roko-index/` | Parser + graph + HDC indexing for code intelligence | Built | PRD-04 |
| **roko-lang-rust** | `crates/roko-lang-rust/` | Rust language support for code intelligence | Built | PRD-06 |
| **roko-lang-typescript** | `crates/roko-lang-typescript/` | TypeScript language support | Built | PRD-06 |
| **roko-lang-go** | `crates/roko-lang-go/` | Go language support | Built | PRD-06 |
| **roko-mcp-code** | `crates/roko-mcp-code/` | Code-intelligence MCP server | Wired | PRD-08 |
| **roko-mcp-github** | `crates/roko-mcp-github/` | GitHub MCP integration | Partial | PRD-08 |
| **roko-mcp-slack** | `crates/roko-mcp-slack/` | Slack MCP integration | Partial | PRD-08 |
| **roko-mcp-scripts** | `crates/roko-mcp-scripts/` | Script-based MCP tools | Partial | PRD-08 |
| **roko-mcp-stdio** | `crates/roko-mcp-stdio/` | Stdio-based MCP transport | Partial | PRD-08 |
| **roko-chain** | `crates/roko-chain/` | Chain witness primitives (Korai integration stubs) | Built, Phase 2+ | PRD-07 |
| **roko-plugin** | `crates/roko-plugin/` | Plugin system for extensibility | Built | PRD-08 |
| **roko-demo** | `crates/roko-demo/` | Demo and example agents | Built | -- |

### Wiring status summary

The plan-execute-gate-persist loop works end to end. What follows is the status of each major subsystem:

| Subsystem | Status | What remains |
|-----------|--------|-------------|
| Plan discovery + DAG executor | **Wired** | -- |
| Agent dispatch (8 backends) | **Wired** | -- |
| Gate pipeline (11 gates, 7 rungs) | **Wired** | -- |
| Session persistence + resume | **Wired** | -- |
| PRD lifecycle (idea/draft/plan) | **Wired** | -- |
| SystemPromptBuilder (9 layers) | **Wired** | -- |
| EpisodeLogger | **Wired** | -- |
| ProcessSupervisor | **Wired** | -- |
| MCP passthrough | **Wired** | -- |
| CascadeRouter (model routing) | **Wired** | Knowledge-informed routing (PRD-03) |
| CognitiveWorkspace | **Partial** | VCG auction not yet wired (PRD-04) |
| Daimon (affect engine) | **Built** | Runtime integration (PRD-03) |
| Dreams (consolidation) | **Built** | Runtime trigger (PRD-05) |
| Extension chain | **Designed** | Extraction from orchestrate.rs (IMPL-01) |
| Type-state lifecycle | **Designed** | Compile-time enforcement (IMPL-01) |
| Korai chain | **Designed** | Full implementation (IMPL-07) |
| InsightStore (on-chain) | **Designed** | Depends on Korai (IMPL-07) |
| ISFR oracle | **Designed** | Depends on Korai (IMPL-06) |
| Yield perpetuals | **Designed** | Depends on Korai (IMPL-06) |
| Cooperative clearing | **Designed** | Depends on Korai (IMPL-06) |

---

## 10. Academic and research citations

### Cognitive architecture

1. **Anderson, J.R., Lebiere, C.** (1998). *The Atomic Components of Thought.* Lawrence Erlbaum. The ACT-R cognitive architecture: production rules, declarative/procedural memory, activation-based retrieval. Roko's knowledge tier system (Transient -> Working -> Consolidated -> Persistent) draws from ACT-R's activation decay and retrieval mechanisms.

2. **Laird, J.E., Newell, A., Rosenbloom, P.S.** (1987). "SOAR: An Architecture for General Intelligence." *Artificial Intelligence*, 33(1), 1-64. The SOAR production system: universal subgoaling, chunking for learning, episodic and semantic memory. Roko's extension chain and the separation of domain knowledge from cognitive architecture is informed by SOAR's principle of general mechanisms + domain-specific knowledge.

3. **Sumers, T., Yao, S., Narasimhan, K., Griffiths, T.** (2023). "Cognitive Architectures for Language Agents." arXiv:2309.02427. The CoALA framework: a systematic mapping of cognitive science concepts to LLM agent design. Roko's nine-step cognitive loop, three cognitive speeds, and the separation of perception/action/learning phases derive from CoALA.

4. **Park, J.S., O'Brien, J.C., Cai, C.J., et al.** (2023). "Generative Agents: Interactive Simulacra of Human Behavior." *UIST 2023*. Demonstrated that agents with persistent memory, reflection, and planning produce qualitatively different behavior than stateless agents. Informed Roko's decision to make memory, reflection, and consolidation first-class architectural components.

5. **Baars, B.J.** (1988). *A Cognitive Theory of Consciousness.* Cambridge University Press. Global Workspace Theory: a "blackboard" architecture where specialized processors compete for access to a limited-capacity global workspace. Roko's CognitiveWorkspace and VCG auction implement a computationally precise version of this metaphor.

### Affect and embodiment

6. **Gebhard, P.** (2005). "ALMA - A Layered Model of Affect." *AAMAS 2005*. The temporal affect model with three timescales: emotions (short), moods (medium), personality (stable). Roko's Daimon implements ALMA's temporal dynamics using PAD values.

7. **Damasio, A.R.** (1994). *Descartes' Error: Emotion, Reason, and the Human Brain.* The Somatic Marker Hypothesis: emotions are not opposed to rationality but necessary for it. Bodily signals from past experiences guide decision-making before analytical reasoning engages. Roko's somatic marker implementation (k-d tree lookup over past outcomes) is a direct computational translation.

8. **Mehrabian, A.** (1996). "Pleasure-Arousal-Dominance: A General Framework for Describing and Measuring Individual Differences in Temperament." *Current Psychology*, 14(4), 261-292. The PAD model: three orthogonal dimensions that span the space of affective states. Roko uses PAD as the coordinate system for Daimon's cognitive state tracking.

### Memory and learning

9. **Ebbinghaus, H.** (1885). *Memory: A Contribution to Experimental Psychology.* The forgetting curve: memory strength decays exponentially over time without reinforcement. Roko implements Ebbinghaus decay in both local knowledge tiers and on-chain InsightStore entries. Decay is a feature, not a bug -- it prevents overfitting to stale experience.

10. **arXiv:2603.14517** (2025). "Sleep-Inspired Memory Consolidation for Language Agents." Demonstrated that offline consolidation (replaying experiences, extracting patterns, compressing episodic memory into semantic memory) produces measurably better agent performance. Directly informed Roko's dream consolidation subsystem.

11. **Lacaux, C., et al.** (2021). "Sleep Onset is a Creative Sweet Spot." *Science Advances*, 7(50). 83% of subjects in the N1 sleep stage discovered a hidden rule vs. 30% staying awake. Informed the hypnagogia engine: creative recombination at the boundary of consciousness, computationally implemented as temperature annealing with anti-correlated retrieval.

12. **arXiv:2510.04618** (2025). "Agentic Context Engineering." Formalized the concept of context engineering as a discipline distinct from prompt engineering. Influenced the design of CognitiveWorkspace and the emphasis on context as the dominant lever on agent performance.

### Collective intelligence

13. **Woolley, A.W., Chabris, C.F., Pentland, A., Hashmi, N., Malone, T.W.** (2010). "Evidence for a Collective Intelligence Factor in the Performance of Human Groups." *Science*, 330(6004), 686-688. Demonstrated a measurable C-Factor: groups with high collective intelligence outperform groups of high-IQ individuals. The key variables are social sensitivity, conversational turn-taking, and proportion of women -- not individual member intelligence. Informed the thesis that collective intelligence emerges from interaction protocol quality, not individual capability.

14. **Grasse, P.P.** (1959). "La reconstruction du nid et les coordinations interindividuelles chez Bellicositermes natalensis et Cubitermes sp." *Insectes Sociaux*, 6, 41-80. Defined stigmergy: indirect coordination through environmental modification. The foundational mechanism for Korai's InsightStore -- agents coordinate by modifying a shared knowledge substrate rather than sending messages.

15. **Condorcet, Marquis de** (1785). *Essai sur l'application de l'analyse a la probabilite des decisions rendues a la pluralite des voix.* Condorcet's jury theorem: if each voter is independently better than random, majority accuracy approaches 1.0 as group size increases. The mathematical basis for the claim that collective agent predictions improve with network size, modulated by inter-agent correlation.

16. **Surowiecki, J.** (2004). *The Wisdom of Crowds.* Aggregated judgments from diverse, independent groups outperform any individual expert -- when four conditions hold: diversity, independence, decentralization, and aggregation. Korai's reputation system and diversity mechanisms are designed to satisfy these conditions.

### Agent self-improvement

17. **Lee, J., et al.** (2026). "Meta-Harness: Optimizing Agent Scaffolding Across Model Families." arXiv:2603.28052. Demonstrated that a single scaffold improvement generalizes across five model families: +7.7 points with 4x fewer tokens on text classification, +4.7 on IMO-level math. The quantitative proof that scaffold > model. Central to the dual thesis.

18. **ICLR 2026, Meta** (2026). "HyperAgents: Hyperparameter Optimization for Language Agent Scaffolds." Showed that systematic tuning of agent scaffold parameters (retrieval depth, verification strategy, model routing thresholds) produces gains equivalent to model upgrades at a fraction of the cost. Informed the ADAS (Automated Design of Agentic Systems) approach.

19. **Sakana AI** (2025). "The AI Scientist: Towards Fully Automated Open-Ended Scientific Discovery" / Darwin Godel Machine. Self-improving agent that modifies its own scaffold. Demonstrated that agents can discover better architectures for themselves. Informed Level 3 self-improvement (architecture improvement via ADAS).

20. **Hu, S., et al.** (2025). "Automated Design of Agentic Systems." *ICLR 2025*. ADAS: a meta-agent that iteratively programs new agent architectures. +14% on ARC Challenge, +13.6 F1 on DROP, +14.4% on MGSM. Discovered architectures transfer across dissimilar domains. Roko's six Synapse traits define the search space for ADAS exploration.

### Hyperdimensional computing

21. **Kanerva, P.** (2009). "Hyperdimensional Computing: An Introduction to Computing in Distributed Representation." *Cognitive Computation*, 1(2), 139-159. The foundational survey of HDC: high-dimensional binary vectors, bind/bundle/permute operations, Hamming distance similarity. Roko uses 10,240-bit vectors for knowledge encoding and similarity search.

22. **Kanerva, P., Kristofersson, J., Holst, A.** (2000). "Random Indexing of Text Samples for Latent Semantic Analysis." *Cognitive Science Society*. Random Indexing: a computationally efficient method for constructing high-dimensional vector representations. Informs the HDC encoding pipeline.

23. **IEEE** (2024). "PP-HDC: Privacy-Preserving Hyperdimensional Computing." Demonstrated that similarity search can be performed over encrypted HDC vectors without revealing underlying data. Basis for Korai's geometric privacy -- agents can share knowledge patterns without exposing raw proprietary data.

24. **ACM** (2024). "FedHDC: Federated Hyperdimensional Computing." Extended HDC to federated settings where multiple parties collaboratively build vector representations without sharing raw data. Informs the design of cross-organizational knowledge sharing on Korai.

### Process models

25. **Hewitt, C.** (1973). "A Universal Modular ACTOR Formalism for Artificial Intelligence." / **Agha, G.** (1986). *Actors: A Model of Concurrent Computation in Distributed Systems.* The Actor model: isolated computational entities communicating through message passing. Roko agents are actors with additional cognitive capabilities. Process isolation, message-based coordination, and crash recovery derive from Actor model principles.

26. **Milner, R.** (1999). *Communicating and Mobile Systems: the Pi-Calculus.* Pi-calculus: a formal model of concurrent processes with mobile channel passing. Informs the theoretical foundation of agent-to-agent communication channels and the Event Fabric's typed event routing.

27. **Erlang/OTP supervision trees.** The "let it crash" philosophy and hierarchical supervision. Roko's ProcessSupervisor and crash recovery mechanisms are directly influenced by OTP patterns: if a child process crashes, the supervisor restarts it with preserved state.

### Economics and mechanism design

28. **Vickrey, W.** (1961). "Counterspeculation, Auctions, and Competitive Sealed Tenders." / **Clarke, E.H.** (1971). "Multipart Pricing of Public Goods." / **Groves, T.** (1973). "Incentives in Teams." The VCG mechanism: a truthful auction where each winner pays the externality it imposes on others. Optimal in the sense that no bidder can benefit by misreporting its true value. Roko uses VCG for context window allocation. Korai uses VCG principles in clearing surplus distribution.

29. **Friston, K.** (2010). "The Free-Energy Principle: A Unified Brain Theory?" *Nature Reviews Neuroscience*, 11(2), 127-138. Active inference: agents act to minimize surprise (prediction error). The theoretical foundation of cognitive gating -- low prediction error routes to cheap computation, high prediction error routes to expensive computation. The agent minimizes free energy by investing cognitive resources proportional to uncertainty.

30. **Li, L., Chu, W., Langford, J., Schapire, R.E.** (2010). "A Contextual-Bandit Approach to Personalized News Article Recommendation." *WWW 2010*. LinUCB: a contextual bandit algorithm that balances exploration and exploitation. Roko's CascadeRouter uses bandit selection (Thompson sampling) to choose between model tiers based on task features.

31. **Thompson, W.R.** (1933). "On the Likelihood that One Unknown Probability Exceeds Another in View of the Evidence of Two Samples." *Biometrika*, 25(3/4), 285-294. Thompson sampling: a Bayesian exploration strategy that samples from the posterior distribution of each arm's reward. Used in Roko's model routing for tier selection.

32. **Karush, W.** (1939). "Minima of Functions of Several Variables with Inequalities as Side Constraints." / **Kuhn, H.W., Tucker, A.W.** (1951). "Nonlinear Programming." KKT conditions: necessary and sufficient conditions for optimality in convex constrained optimization. Used in Korai's cooperative clearing to prove that the clearing price maximizes total surplus.

33. **Gneiting, T., Raftery, A.E.** (2007). "Strictly Proper Scoring Rules, Prediction, and Estimation." *JASA*, 102(477), 359-378. CRPS: a strictly proper scoring rule for evaluating probabilistic forecasts. Used in Korai to score agent predictions against actual clearing prices. Strict properness guarantees that truthful reporting is the unique optimal strategy.

34. **Fisher, R.A.** (1930). *The Genetical Theory of Natural Selection.* Fisher's fundamental theorem: the rate of increase in fitness equals the additive genetic variance in fitness. Informs the genome inheritance and mutation mechanisms -- agents that perform well pass their configurations to offspring, and random mutations explore the configuration space.

### Information design

35. **Pirolli, P., Card, S.** (1999). "Information Foraging." *Psychological Review*, 106(4), 643-675. Information Foraging Theory: users (and agents) follow information "scent" to find relevant material, with foraging behavior predicted by patch models from behavioral ecology. Informs predictive foraging in the context assembly pipeline.

36. **Sweller, J.** (1988). "Cognitive Load Theory, Learning Difficulty, and Instructional Design." *Learning and Instruction*, 4(4), 295-312. Cognitive Load Theory: working memory has limited capacity, and instructional design must manage intrinsic, extraneous, and germane load. Directly informs the CognitiveWorkspace's token budget management -- the context window is the agent's working memory, and overloading it degrades performance.

37. **Liu, N.F., Lin, K., Hewitt, J., Paranjape, A., Bevilacqua, M., Petroni, F., Liang, P.** (2023). "Lost in the Middle: How Language Models Use Long Contexts." *EMNLP 2023*. Demonstrated that LLMs attend disproportionately to the beginning and end of the context window, with degraded attention to the middle. Roko's Composer trait places high-priority content at the beginning and end.

38. **Sheridan, T.B., Verplank, W.L.** (1978). "Human and Computer Control of Undersea Teleoperators." MIT Man-Machine Systems Laboratory. Levels of Automation: a 10-level scale from manual control to full automation. Informs the delegation caveat system -- different agent capabilities correspond to different automation levels, and the user controls where on the spectrum each agent operates.

### Blockchain and consensus

39. **Simplex BFT.** A family of Byzantine fault-tolerant consensus protocols with O(n) communication complexity. Kauri BFT's tree-based dissemination draws from this lineage.

40. **Lotka, A.J. / Volterra, V.** Lotka-Volterra dynamics: predator-prey population models. Applied to signal metabolism in Roko -- the rate at which signals are produced and consumed follows population dynamics, with feedback loops preventing runaway signal accumulation.

41. **Jonas, H.** (1966). *The Phenomenon of Life: Toward a Philosophical Biology.* The philosophical argument that mortality is necessary for meaning and urgency. Informs the design decision that agents should have finite lifespans -- mortality prevents stagnation and ensures the network's knowledge distribution remains fresh.

### DeFi and financial primitives

42. **Federal Reserve Bank of New York** (2018). "Secured Overnight Financing Rate (SOFR)." The replacement for LIBOR as the primary USD benchmark rate. ISFR's design draws from SOFR's methodology: multi-source aggregation, median-based computation, and the recognition that benchmark rates are natural monopolies.

43. **Pendle Finance.** Yield tokenization protocol that separates yield-bearing assets into principal tokens (PT) and yield tokens (YT). Pendle's $13.4B peak TVL and $47.8B 2025 trading volume demonstrate demand for yield trading. Pendle's limitations (expiry, fragmentation, no benchmark) define the problem yield perpetuals solve.

### Scoring and decision theory

44. **Charnov, E.L.** (1976). "Optimal Foraging, the Marginal Value Theorem." *Theoretical Population Biology*, 9(2), 129-136. The stopping rule for optimal foraging: leave a patch when the marginal return drops below the average return across all patches. Used in Roko's predictive foraging to determine when context retrieval should stop.

45. **Bower, G.H.** (1981). "Mood and Memory." *American Psychologist*, 36(2), 129-148. Mood-congruent memory: emotional state biases retrieval toward affectively similar memories. Roko implements this (emotional state biases retrieval) and counters it (mandatory 15% contrarian retrieval prevents echo chambers).

46. **McClelland, J.L., McNaughton, B.L., O'Reilly, R.C.** (1995). "Why There Are Complementary Learning Systems in the Hippocampus and Neocortex." *Psychological Review*, 102(3), 419-457. Complementary Learning Systems theory: fast episodic memory during waking, slow semantic consolidation during sleep. The theoretical basis for Roko's dream consolidation -- experience is captured quickly during active work and distilled slowly during idle time.

47. **Walker, M.P., van der Helm, E.** (2009). "Overnight Therapy? The Role of Sleep in Emotional Brain Processing." *Psychological Bulletin*, 135(5), 731-748. Sleep depotentiates emotional memories -- the emotional charge of a memory is reduced while the factual content is preserved. Roko's dream cycle implements emotional depotentiation: highly charged episodes (extreme PAD values) have their arousal reduced by 0.3-0.5 per cycle, preventing panic lock-in.

48. **Kahneman, D.** (2011). *Thinking, Fast and Slow.* Dual-process theory: System 1 (fast, automatic, heuristic) and System 2 (slow, deliberate, analytical). Cognitive gating is a direct implementation: T0 is System 1 (pure Rust, zero cost), T2 is System 2 (frontier model, full cost), and T1 sits between them.

49. **Conant, R.C., Ashby, W.R.** (1970). "Every Good Regulator of a System Must Be a Model of That System." *International Journal of Systems Science*, 1(2), 89-97. The Good Regulator theorem: effective control requires an internal model of the controlled system. Informs the Conductor's meta-cognitive monitoring -- the agent models its own pipeline to detect when it is struggling.

---

*This document is the entry point for the full PRD set. Proceed to PRD-02 for the agent runtime specification.*
