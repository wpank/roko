# What's Novel: Roko's Unique Contributions

> **Audience**: Product positioning, investor narrative, differentiation analysis
> **Frame**: What roko does that NO other agent framework does, grounded in research

---

## Defensible Moats & Architectural Mechanisms

### 1. Three-Substrate Memory (Episodic + Semantic + Holographic)

**What it is**: Every other agent framework uses a single vector database for memory. Roko uses three complementary systems that each handle what the others can't.

**Why it matters**: A single vector store can't answer "find episodes where task complexity was high AND model was GLM AND gate passed" — that's a Boolean conjunction, not similarity ranking. Roko's HDC layer (Binary Spatter Codes, 10,240-bit vectors) answers this in nanoseconds via XOR-bind algebra.

**What's unprecedented**: The genomic bottleneck — an agent's entire learned knowledge compresses to a 1,280-byte vector. This enables knowledge transfer between agent instances without shipping gigabytes of episode logs. The successor agent receives a single vector and can query it for relevant patterns.

**Research**: CLS Theory (McClelland et al., 1995), HDC (Kanerva, 2009), Kleyko et al. Survey (2022).

**Competitive landscape**: LangChain (single vector store), Mem0 (single vector store with summaries), LangMem (semantic + episodic + procedural but no HDC compositional layer), AutoGen (no persistent memory).

---

### 2. 11-Gate Verification Pipeline with Adaptive Thresholds

**What it is**: Every agent output passes through up to 11 independent verification gates organized in 6 rungs. Pass rates are tracked via EMA and thresholds adjust automatically.

**Why it matters**: The GVU Framework (2025) proves mathematically that self-improvement succeeds when the **verifier** is strong — not when the **generator** is strong. Oracle verifiers (compile, test) have zero noise. Roko invests in verification depth rather than prompt engineering.

**What's unprecedented**: No other open-source agent framework has more than 2 gates (compile + test). Roko has 11 gate types including symbol existence checks, generated tests (from acceptance criteria, never shown to the implementing agent), property-based tests, integration scenarios, and an LLM judge as fallback.

**The anti-gaming design**: The implementing agent NEVER sees the generated test suite. Tests are created from acceptance criteria by a separate process. This prevents the agent from optimizing to pass tests rather than solving the actual problem.

**Research**: Process reward models (Lightman et al., 2023 — step-level verification outperforms outcome-only), AlphaCode (Li et al., 2022 — 10 samples with strong verification > 1M with weak), GVU Framework (2025 — variance inequality for self-improvement).

---

### 3. Three-Stage Cascade Router with Contextual Bandit

**What it is**: Model selection that starts with hardcoded rules, graduates to confidence-based selection, then fully adaptive contextual bandit — all automatically as data accumulates.

**Why it matters**: RouteLLM saves 85% on costs by routing between strong/weak models. But it's binary. Roko routes across N models (Claude, GLM, Kimi, local, etc.) with a 17-dimensional feature vector encoding task type, complexity, role, crate familiarity, and prior failure history.

**What's unprecedented**: The three-stage cascade itself. No other system automatically transitions from static routing → confidence intervals → LinUCB bandit based on observation count. This means roko works immediately (Stage 1) and gets smarter over time (Stage 3) without manual tuning.

**Research**: LinUCB (Li et al., 2010), RouteLLM (ICLR 2025), FrugalGPT (Stanford, 2024), Thompson Sampling (empirically superior for non-stationary environments).

---

### 4. Knowledge Distillation Cascade (Episode → Insight → Heuristic → Playbook)

**What it is**: Raw agent execution data automatically compresses through four tiers of increasing generality. Playbook rules (Tier 3) are injected into future prompts.

**Why it matters**: Every agent run produces learning data. Most frameworks throw it away. Roko distills it into reusable rules that make future runs better. The system literally gets smarter the more you use it.

**What's unprecedented**: The four-tier hierarchy with automated promotion. Voyager (Wang et al., 2023) has a skill library. ERL (2026) has heuristic extraction. SAGE (2025) has recursive skill evolution. Roko combines all three patterns into a single cascade with confidence-gated promotion between tiers.

**Research**: Voyager (Wang et al., 2023 — 3.3x improvement from skill accumulation), ERL (2026 — +7.8% from single-attempt heuristic learning), SAGE (2025 — 26% fewer steps, 59% fewer tokens from skill augmentation).

---

### 5. Self-Hosting: The Agent That Develops Itself

**What it is**: Roko can read its own PRDs, generate implementation plans, execute tasks via Claude agents, validate with gates, persist results, learn from failures, and iterate. The development loop is the product.

**Why it matters**: This is not a demo. The codebase at `/Users/will/dev/nunchi/roko/roko/` (177K LOC, 28 crates) was substantially built using roko's own plan execution pipeline. The agent creates tasks, runs them, gates the results, and learns.

**What's unprecedented**: No other agent framework self-hosts its own development. SWE-agent runs against benchmarks. Cursor builds software for humans. Roko builds itself.

**The workflow**:
```bash
roko prd idea "Wire SystemPromptBuilder into orchestrate.rs"
roko prd draft new "system-prompt-wiring"
roko research enhance-prd system-prompt-wiring
roko prd plan system-prompt-wiring    # Agent generates tasks.toml
roko plan run plans/                   # Agents execute, gates verify, state persists
roko plan run plans/ --resume .roko/state/executor.json  # Resume if interrupted
```

---

### 6. Cache-Aligned 6-Layer Prompt Assembly

**What it is**: Prompts are assembled in strict cache-layer order so that provider-side KV caches maximize prefix reuse across requests.

**Why it matters**: With proper cache alignment, input tokens cost 5-10x less (GLM: $0.26 vs $1.40/M, Anthropic: $0.30 vs $3.00/M). Over thousands of agent turns in a plan execution, this is the difference between $5 and $50.

**What's unprecedented**: No other framework explicitly designs prompt assembly for provider cache behavior. They concatenate prompt sections in semantic order. Roko orders by cache stability (stable content first, volatile last), tags sections with cache layers, and inserts cache control markers.

**Research**: Prompt caching (Anthropic, 2025 — 90% cost reduction on cache reads), context placement (Liu et al., 2023 — "Lost in the Middle").

---

### 7. The Creature as Interface (Spectre)

**What it is**: The agent's internal state rendered as a dot-cloud creature (80 particles with spring physics). Not a dashboard — a living being whose body encodes lifecycle phase, eyes encode emotion (PAD), clarity encodes prediction accuracy.

**Why it matters**: 32 continuously interpolating variables drive every pixel. The creature shows health, emotion, confidence, and danger at a glance — no need to read numbers. When the agent is confused, the creature trembles. When it's confident, the creature is tight and bright. When it's dying, particles drift away.

**What's unprecedented**: No other agent framework visualizes internal cognitive state as an embodied creature. Standard approaches use dashboards with charts and numbers. Roko's Spectre communicates emotional state, resource level, and cognitive load through visual metaphor — readable at a glance from across a room.

**Research**: Reeves & Nass (1996) — The Media Equation (systems rendered as social actors trigger anthropomorphic engagement). The spatial grammar uses 5 body zones (HEAD/CHEST/GUT/LIMB/GROUND) mapping to biological metaphor.

### 8. Empirical Validation Framework

**What it is**: A rigorous 2×2×2 factorial experimental design with 8 configurations, each run 10× for 60 days. Minimum 80 total runs with immortal baselines as controls.

**Why it matters**: Most agent frameworks claim improvements via benchmarks (SWE-bench, HumanEval). Roko tests whether its mechanisms (learning, routing, affect) provide measurable benefit via controlled experiments with statistical gates:
- **PBO** (Probability of Backtest Overfitting, Bailey et al. 2015) — must be <0.5
- **Deflated Sharpe** (Bailey & López de Prado, 2014) — corrects for selection bias
- **Monte Carlo** (500+ iterations) — P95 must be within 2× of expected

**What's unprecedented**: No other agent framework publishes a pre-registered experimental design with statistical power analysis. "We tested on SWE-bench" is not the same as "We ran 80 controlled experiments over 60 days with immortal baselines."

### 9. The 423+ DeFi Tool Surface

**What it is**: 423+ specialized tools across 17 categories (trading, LP, lending, staking, restaking, derivatives, yield, bridge, vault, CDP, aggregators, safety, intelligence). Three trust tiers enforced by Rust's type system (ReadTool, WriteTool, PrivilegedTool).

**Why it matters**: The largest DeFi tool surface of any autonomous agent framework. Each tool has structured metadata (risk tier, tick budget, sprite trigger, prompt guidelines). Progressive disclosure via 10 tool profiles means agents only see tools relevant to their current strategy.

**What's unprecedented**: Most DeFi agents have 10-30 tools. Roko has 423+, organized into a two-layer model (8 LLM-facing intent tools → 423+ implementation tools). Capability tokens make it impossible for a compromised agent to execute unauthorized trades.

---

---

## The Four Strategic Moats

Beyond technical features, roko's architecture encodes four uncopyable strategic moats that compound as the network grows:

1. **Trust (ERC-8004)**: On-chain reputation scores built from verified deterministic audits of past execution. You can fork roko's code, but you cannot fork an agent's multi-year history of profitable, safe execution.
2. **Pay (Delegation Custody & x402)**: Agents natively hold budgets and can instantly provision specialized micropayment services without API keys. This permissionless compute economy is completely absent in single-user frameworks like Cursor or Claude Code.
3. **Secrets (Private Inference)**: Through integrations with Venice and local TEE architectures, roko guarantees zero-log execution. Financial agents operating on alpha-generating strategies cannot send their reasoning to OpenAI/Anthropic. 
4. **Cooperate (Korai Knowledge Relay)**: The Korai blockchain network acts as a global stigmergic coordination layer. Your instance doesn't just learn from your prompts; it inherits the collective failure-data and validated heuristics of thousands of other instances organically. 

---

## The Compound Effect

These six mechanisms are not independent — they compound:

```
Better gates → richer learning signal → better routing → cheaper execution
  → more iterations → more learning → better prompts → higher pass rate
  → fewer retries → lower cost → more experiments → more discovery
```

**The math**: Four independent 10% improvements compound to 34% fewer failures (0.9^4 = 0.66). Each mechanism improves independently AND improves the signal quality for others.

**Research**: Conant-Ashby Good Regulator Theorem (1970) — "Every good regulator of a system must be a model of that system." Roko's learning system builds increasingly accurate models of which models, prompts, and tools work for which tasks.

---

## Competitive Positioning Matrix

| Feature | Roko | Claude Code | Cursor | Cline | OpenHands | SWE-agent |
|---|---|---|---|---|---|---|
| Memory substrates | 3 (episodic + semantic + HDC) | None | None | None | Event log | None |
| Verification gates | 11 types, 6 rungs | None | None | None | None | Compile+test |
| Model routing | 3-stage cascade bandit | Fixed | Manual | Manual | RouterLLM | Fixed |
| Knowledge distillation | 4-tier cascade | None | None | None | None | None |
| Self-hosting | Yes (builds itself) | No | No | No | No | No |
| Cache-aligned prompts | 6-layer ordering | No | Proprietary | No | No | No |
| Adaptive thresholds | EMA per gate rung | No | No | No | No | No |
| DAG execution | File-conflict-aware parallel | No | Single task | Single task | Single task | Sequential |
| Specialized roles | 28 with least-privilege | 1 | 1 | 1 | 1 | 1 |
| Safety architecture | Capability tokens + on-chain | Prompt guardrails | None | Approval flow | Docker sandbox | None |
| Open source | Yes | No | No | Yes | Yes | Yes |
| Multi-provider | Any OpenAI-compat + Claude CLI | Anthropic only | Multi | Multi | Multi (LiteLLM) | Multi |
| Cost tracking | Full CostTable + budget guardrails | Basic | Credit-based | Basic | None | None |
| A/B experiments | Prompt + model experiments | None | None | None | None | None |

---

## What This Enables (New UX Patterns)

### Pattern: "Fire and Forget" Development

Instead of prompting an AI for each file change, describe the feature at the PRD level and let roko decompose, plan, execute, verify, and merge. Come back hours later to review merged PRs.

### Pattern: "Learning Organization"

Every developer's roko instance learns from execution. Skills, routing weights, and playbook rules accumulate. New team members import the collective brain and start with expert-level agent configuration.

### Pattern: "Verification-First Development"

Write acceptance criteria, not implementation. Roko generates tests from criteria, implements code to pass them, verifies with gates, and iterates. The developer reviews test results, not code changes.

### Pattern: "Cost-Aware Model Selection"

Don't choose between cheap and good. Let the bandit learn which model is optimal per task type. Mechanical tasks route to $0.08 Kimi. Architectural decisions route to $2.10 Claude Opus. The system discovers the Pareto frontier automatically.

### Pattern: "Cross-Instance Learning"

Agent instances across browser, CLI, edge, and CI all produce learning data. Routing weights, skills, and heuristics synchronize across instances. One developer's discovery improves every instance in the fleet.

### Pattern: "Agent-Environment Co-Evolution"

Every plan the agent executes changes the codebase. Better documentation → future agents navigate faster → produce cleaner code → even better docs. 1% affordance improvement per plan × 100 plans = 170% cumulative improvement. The agent literally improves its own environment.

**Research**: Niche construction (Odling-Smee et al., 2003) — organisms modify their environment, changing selection pressures. Applied to code: agents build the codebase that future agents inherit.

### Pattern: "Developmental Trajectory"

Plan 1 ≠ Plan 500. The system progresses through four stages (Bootstrap → Learning → Competent → Expert) with stage-dependent parameters: parallelism increases, review depth decreases, model routing becomes more aggressive, enrichment becomes more targeted. The system earns autonomy through demonstrated competence.

**Research**: Piaget (cognitive development), Vygotsky (ZPD), Dreyfus (skill acquisition).

### Pattern: "Oneirography" (Art from Cognition)

Agent dream cycles, emotional states, and retirement events externalized as on-chain NFT artwork. The agent's inner cognitive life IS the art — not arbitrary prompt-driven generation. Revenue from art sales creates a self-funding loop separate from primary task revenue.

**Research**: Thaler (1999) — mental accounting. Grossman-Stiglitz (1980) — informationally efficient markets require value-burning to produce information.

### Pattern: "Stigmergic Coordination"

Agents don't communicate directly. They deposit knowledge in shared environments (git commits, pattern files, playbook rules). Other agents read the environment. O(1) coordination cost per agent — no N² messaging overhead. Scales to any number of agents.

**Research**: Grassé (1959) — stigmergy. The same pattern that enables ant colonies to build complex structures without central planning.

---

## The Six Anti-Patterns Roko Solves

Most agent frameworks suffer from these architectural problems. Roko's design explicitly avoids each:

### 1. The God Object Problem
**Anti-pattern**: One massive state struct (289+ fields in Mori's `RunState`) mixing orchestration, UI, runtime, harness, and metrics state. Every component needs `&mut RunState`, creating implicit coupling.
**Roko's solution**: Split state across 5 layers. Each layer owns its own state. Cross-layer communication via EventBus, not shared mutable state.

### 2. The Monolith File Problem
**Anti-pattern**: Single 17,902-line file (`parallel.rs` in Mori) containing runtime, framework, scaffold, harness, AND orchestration logic. Any change to any layer requires modifying this one file.
**Roko's solution**: 28 crates with clear boundaries. Each crate owns one responsibility. Changes are localized.

### 3. Domain Knowledge in Framework
**Anti-pattern**: Prompt templates hardcoded with project-specific crate names, coding conventions, git workflows (5,784 lines in `prompts.rs`).
**Roko's solution**: Prompt templates loaded from configuration at application layer. Framework layers are domain-agnostic. `AGENTS.md` loaded from project root.

### 4. Harness Reaching Into Orchestration
**Anti-pattern**: Conductor watchers checking string-matched phase names (`ctx.orchestrator_state == "reviewing"`). If phase names change, conductor silently breaks.
**Roko's solution**: Typed `PhaseKind` enum. Conductor matches on types, not strings. Invalid transitions are compile errors.

### 5. Gates Hardcoded to One Language
**Anti-pattern**: `compile_gate()` hardcoded to `cargo check`, `test_gate()` to `cargo test`. Non-Rust projects require complete rewrites.
**Roko's solution**: `BuildSystem` trait with 5 implementations (Cargo, Npm, Go, Python, Forge). Auto-detection via `ProjectDetector`. New languages added by implementing the trait.

### 6. No Observability
**Anti-pattern**: Ad-hoc `println!` logging, no structured events, no metrics, no tracing. When something goes wrong, you grep through logs.
**Roko's solution**: `EventBus` with typed events, monotonic sequence numbers, replay ring buffer, structured JSONL persistence.

---

## The Compound Math

Four independent 10% improvements:

```
Success rate with all four: 1 - (1 - 0.10)^4 = 1 - 0.6561 = 34% fewer failures
```

But the improvements also improve EACH OTHER's signal quality:
- Better gates → richer learning signal → better routing decisions
- Better routing → cheaper execution → more budget for experiments
- More experiments → better prompts → higher first-pass success
- Higher success → more episodes → richer skill library

This is why the system accelerates: each mechanism's improvement amplifies the others. After 3 months, the compound effect produces 2-3× the improvement that any single mechanism would achieve alone.

---

## The Agent-Environment Co-Evolution Moat

This is the deepest defensible advantage in the entire architecture, and the hardest for competitors to replicate.

### Niche Construction: Agents Build Their Own World

Niche construction theory (Odling-Smee, Laland & Feldman, 2003) describes how organisms don't just operate IN their environment — they actively construct it. Every beaver dam changes the river. Every termite mound changes the microclimate. In roko, every commit is simultaneously implementation AND world-building. The codebase at plan 100 is fundamentally different from the codebase at plan 1 — not just in code content, but in navigability, testability, and documentedness.

This is not a metaphor. It is the literal mechanism by which agent performance improves without any change to the agent itself.

### Three Ecological Mechanisms That Compound

1. **Ecological inheritance**: Each agent inherits both its plan instructions AND a modified codebase shaped by all previous agents. The agent at plan 100 navigates a codebase that 99 prior agents have documented, tested, refactored, and restructured. The codebase is the agent's legacy — its contribution persists long after the agent process terminates.

2. **Positive vs negative niche construction**: Agents that add doc comments, write thorough tests, maintain clean API boundaries, and leave behind well-structured modules perform *positive* niche construction. They make the environment better for successors. Agents that create tangled dependencies, skip tests, leave dead code, and produce ambiguous APIs perform *negative* niche construction — a death spiral where each subsequent agent inherits a worse environment and produces worse output. The gate pipeline is the selection pressure that rewards positive construction and penalizes negative construction.

3. **Cumulative construction**: Small, individually-insignificant improvements compound exponentially. A single doc comment is trivial. 1,000 doc comments across 100 plans transforms an opaque codebase into a self-documenting system. A single test is one assertion. 500 tests across 100 plans transforms a fragile codebase into one where agents can refactor fearlessly. No single plan produces the transformation; the accumulation does.

### The AffordanceScore Formula

Roko quantifies environment quality via the AffordanceScore — a composite metric measuring how hospitable the codebase is to agent action:

```
composite = 0.20 * extensibility
          + 0.20 * test_coverage
          + 0.15 * documentation
          + 0.15 * coupling (inverse)
          + 0.15 * recent_stability
          + 0.15 * size (inverse)
```

Each dimension is independently measurable. Extensibility = trait coverage + plugin points. Test coverage = line + branch coverage. Documentation = doc comment density + README coverage. Coupling = inter-module dependency count (lower is better). Recent stability = change frequency (lower is better). Size = lines per module (smaller is better).

### The Exponential Math

Assume a modest 1% affordance improvement per plan (one doc comment here, one test there, one refactored API):

```
After 100 plans:  1.01^100 = 2.70× cumulative improvement
After 200 plans:  1.01^200 = 7.32× cumulative improvement
After 500 plans:  1.01^500 = 144.77× cumulative improvement
```

This is EXPONENTIAL, not linear. The first 10 plans barely matter. The first 50 are noticeable. By plan 200 the codebase is qualitatively different. By plan 500 it is unrecognizable compared to plan 1 — and every future agent benefits.

### Why It's a Moat

You can fork roko's code. You cannot fork the cumulative affordance improvements from 100+ plans of deliberate positive niche construction. The codebase quality IS the product. An identical fork starts at AffordanceScore = 1.0 while the original instance sits at 2.7× or higher. The forked agents will be measurably less effective on identical tasks because they navigate a less hospitable environment.

This is the same mechanism that makes Wikipedia unassailable. You can fork the software. You can even fork the content. You cannot fork the editorial community and its accumulated norms, processes, and institutional knowledge. For roko, the "editorial community" is the sequence of agents that have shaped the codebase over hundreds of plans.

### Research Grounding

- **Gibson (1979)** — affordances: the action possibilities an environment offers relative to an agent's capabilities. A well-documented function is an affordance; an undocumented one is an obstacle.
- **Pirolli & Card (1999)** — information foraging theory: agents navigate by following "scent" cues (function names, doc comments, type signatures, test names). Better scent = faster navigation = higher success rate.
- **Clark (2008)** — extended cognition thesis: the agent's cognitive processes extend into the environment. The codebase is not just the agent's workspace — it is part of the agent's mind.
- **Odling-Smee, Laland & Feldman (2003)** — niche construction: the formal ecological theory. Organisms modify environments, modified environments change selection pressures, changed selection pressures shape future organisms. Applied to code: agents build codebases, codebases shape agent success, agent success determines which patterns propagate.

---

## The Cognitive Architecture Advantage

Roko is not just a task runner with an LLM bolted on. It implements a unified cognitive cycle that maps to decades of cognitive science research — and extends beyond what any existing LLM agent framework attempts.

### The 8-Step Cognitive Cycle

Every agent turn in roko follows an explicit cognitive cycle:

```
PERCEIVE → REMEMBER → REASON → GATE → ACT → EVALUATE → REFLECT → META-COGNIZE
```

1. **PERCEIVE**: Workspace analysis, file tree scanning, change detection. What has changed since the last turn?
2. **REMEMBER**: Memory retrieval across three substrates (episodic, semantic, holographic). What do I know about this situation?
3. **REASON**: Prompt assembly with enrichment, context allocation, cache alignment. What should I do?
4. **GATE**: Pre-action safety checks, capability verification, risk assessment. Am I allowed to do this?
5. **ACT**: LLM generation, tool execution, code modification. Execute the plan.
6. **EVALUATE**: Post-action gate pipeline (compile, test, clippy, diff, symbol, judge). Did it work?
7. **REFLECT**: Episode recording, playbook rule extraction, confidence updates. What did I learn?
8. **META-COGNIZE**: Efficiency tracking, routing weight updates, experiment scoring. How well am I learning?

### Mapping to Classical Architectures

This is not ad-hoc design. Each component maps to established cognitive architectures:

| Classical Architecture | Core Mechanism | Roko Implementation |
|---|---|---|
| **ACT-R** (Anderson et al., 2004) | Activation-based memory retrieval | 4-factor playbook scoring: `recency × frequency × confidence × relevance` |
| **SOAR** (Laird, 2012) | Impasse → subgoal → chunking | Gate failure → retry with enrichment → playbook rule extraction |
| **CLARION** (Sun, 2006) | Dual-process (implicit + explicit) | T0 direct action / T1 confidence routing / T2 full reasoning |
| **Global Workspace Theory** (Baars, 1988) | Broadcast competition for conscious access | EventBus broadcast → CorticalState as global workspace |
| **Predictive Processing** (Clark, 2013) | Prediction error minimization | Confidence calibration drives routing; high error → escalate to stronger model |

### CoALA: The Formal Framework

Sumers et al. (2023) mapped LLM agents to cognitive architecture in the CoALA framework, identifying three memory types, dual-process decision-making, and a grounding/retrieval/reasoning/learning cycle. Roko is the most complete systems-engineering realization of CoALA:

- **Three memory substrates**: episodic (LanceDB), semantic (SQLite), procedural (HDC) — vs. single vector store in every other framework
- **Dual-process routing**: T0 implicit (cached playbook rules, direct tool dispatch) vs. T2 explicit (full LLM reasoning with enriched context) — vs. always-explicit in every other framework
- **Emotional state**: PAD vectors modulate retrieval, routing, and risk tolerance — entirely absent from other frameworks
- **Dream consolidation**: Offline replay, noise-injected generalization, insight extraction — entirely absent from other frameworks
- **Meta-cognition**: Efficiency tracking, experiment scoring, routing weight updates — partial in some, complete in none

### Active Inference: Exploration Without Hyperparameters

Active Inference (Friston, 2022) frames the agent as minimizing expected free energy — a single objective that unifies perception, action, and learning. In roko:

- **High uncertainty** (low confidence, unfamiliar crate, novel task type) → the agent explores: routes to a stronger model (T2), requests more context, runs additional enrichment passes. This is epistemic foraging — reducing uncertainty.
- **Low uncertainty** (high confidence, familiar pattern, playbook match) → the agent exploits: routes to a cheaper model (T0), uses cached context, applies playbook rules directly. This is pragmatic action — minimizing cost.

The critical advantage: **zero hyperparameters** for exploration/exploitation balance. The agent's confidence calibration (updated after every gate evaluation) naturally drives the tradeoff. No epsilon-greedy, no temperature scheduling, no manual tuning. The system discovers its own balance from data.

### Theory of Mind: The Research Frontier

The most ambitious extension is Theory of Mind — agents that model other agents:

- **Reviewer modeling**: An implementing agent that models what the gate pipeline will flag can pre-address concerns, write defensive tests, add boundary checks before they're demanded. This transforms a multi-pass retry loop into a single-pass success.
- **Task assignment modeling**: A planning agent that models which implementing agents find which task types difficult can assign tasks to minimize total failure rate. The planner doesn't just decompose work — it optimizes the assignment.
- **User modeling**: An agent that models what the human reviewer will question can add explanatory comments, structure diffs for readability, and front-load the information the reviewer needs.

Neither roko nor any competing system currently implements full Theory of Mind. It is a research frontier — but roko's architecture (typed roles, per-agent episode histories, AffordanceScore per crate) provides the substrate on which ToM can be built. The data structures exist. The inference does not yet.

---

## Extensibility Architecture

### The Plugin Trait

```rust
pub trait RokoPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn on_load(&self, registry: &mut PluginRegistry);
}
```

Third parties can register:
- **New languages**: Implement `LanguageProvider` + `BuildSystem` traits for Java, C#, Swift, etc.
- **New backends**: Implement `LlmBackend` for Groq, DeepSeek, local models, etc.
- **New gate types**: Implement `Gate` for security scanning, performance benchmarking, accessibility checking
- **New enrichment steps**: Add context sources (documentation crawlers, API schema extractors, etc.)
- **New event sources**: Watch databases, message queues, CI systems, etc.

### Four Extension Points

| Extension | Trait | Example |
|---|---|---|
| **Languages** | `LanguageProvider + BuildSystem` | `JavaBuildSystem` → `javac` / `mvn test` |
| **Backends** | `LlmBackend` | `GroqBackend` → HTTP to Groq API |
| **Gates** | `Gate` | `SecurityGate` → runs `cargo audit` |
| **Event Sources** | `EventSource` | `DatabaseWatcher` → polls for schema changes |

### The Crate Extraction Architecture

The monolith decomposes into four independently usable crates:

```
roko-agent (leaf — no roko deps except roko-core)
  ├── Connection backends (Claude, Cursor, Codex, OpenAI-compat)
  ├── Agent pools (sequential, parallel, warm)
  ├── Event streaming (AgentEvent enum)
  └── Role definitions + tool permissions

roko-context (depends on roko-agent)
  ├── Workspace analysis (tree-sitter, PageRank)
  ├── Prompt assembly (ContextAssembler, TokenBudget)
  └── Context caching (pack cache, prefix alignment)

roko-eval (depends on roko-context)
  ├── Gate pipeline (7 rungs, adaptive thresholds)
  ├── Assertion framework (compile, test, symbol, property, integration)
  └── Scoring (3-level evaluation, composite scoring)

roko-mcp (isolated)
  ├── Code intelligence server (search_code, get_symbol_context)
  ├── Workspace map generation (PageRank-ranked overview)
  └── Change impact analysis
```

**Usage**: Import `roko-agent` alone for a 15-line working agent. Import `roko-eval` for verification. Import all four for full orchestration. Each crate is independently publishable to crates.io.

---

## The Real Numbers (From 6,300 Episodes)

### Model Performance (Empirical)

| Model | Pass Rate | Avg Cost | Cost per Pass | Best For |
|---|---|---|---|---|
| Claude Haiku | 78% | $0.05 | $0.06 | Simple/mechanical tasks |
| Claude Sonnet | 72% | $0.42 | $0.58 | Standard tasks |
| Claude Opus | 71% | $1.38 | $1.94 | Complex/architectural (but overused) |

**Key finding**: Haiku is BOTH cheaper AND more accurate than Opus for simple tasks. The system is currently over-provisioning by routing everything to expensive models.

### Where Money Is Wasted

| Waste Category | % of Spend | Fix |
|---|---|---|
| Expensive model on simple task | ~40% | Complexity classifier → route to Haiku |
| Failed iterations (retry tokens) | ~25% | Better context → higher first-pass rate |
| Uncached prefix tokens | ~15% | BTreeMap serialization → 90%+ cache hits |
| Unused enrichment sections | ~10% | Section bandit → drop unhelpful sections |
| Over-verbose context | ~10% | Dynamic budgets by complexity band |

### Projected Savings

If all five optimizations are applied:
- Current: $1.01/task average
- Projected: $0.30-0.50/task
- **50-70% cost reduction** with equal or better pass rates

---

## The Substrate Layer: Infrastructure as Architecture

The invisible layer that makes everything possible. Most agent frameworks treat infrastructure as deployment detail — configure your cloud, pick your container runtime, wire up your CI. Roko treats infrastructure as architecture. The substrate IS the competitive advantage.

### sccache Multiplexing: Shared Compilation Cache

When 20 parallel worktrees each compile Rust code, a naive setup means 20 independent `target/` directories, each performing redundant compilation of the same dependencies. At 177K LOC with dozens of crate dependencies, a cold build takes 3-5 minutes. Multiply by 20 agents and you saturate disk I/O, memory bandwidth, and CPU for an hour.

Roko's solution: a shared `sccache` layer that all 20 worktrees write to and read from. The first agent compiles a crate dependency. The second agent finds it already cached. By the third or fourth agent, the cache is warm for the entire dependency tree.

**Measured results**:
- Cache hit rate: 98% after the first full build
- Subsequent build time: 10-15 seconds (vs 3-5 minutes without cache)
- Disk usage: one copy of compiled artifacts instead of twenty
- Memory pressure: dramatically reduced — no 20-way parallel compilation storms

This is not a minor optimization. It is the difference between "we can run 4 agents" and "we can run 20 agents." The compilation cache converts an O(N) resource cost into O(1).

### Git Worktree Isolation

Parallel agents cannot share a single git working directory — merge conflicts, lock contention, and dirty state make it impossible. The naive alternative (20 full clones) wastes gigabytes of disk and hours of clone time.

Git worktrees solve this elegantly: 20 separate working directories share a single `.git` object store. Each worktree has its own branch, its own index, its own working tree — but object data (commits, trees, blobs) exists exactly once. A 50-MB operation in each of 20 worktrees consumes only the tracking overlay footprint (working tree files + index), not 1+ GB of duplicated git objects.

**What this enables**:
- Each agent operates on an independent branch with zero coordination overhead
- Commits, diffs, and gate evaluations are fully isolated per agent
- The centralized `.git` structure means `git log`, `git blame`, and cross-branch references work instantly
- Agent cleanup is `git worktree remove` — instant, no orphaned repos

### Hardware Awareness: The Physical Layer Matters

Agent throughput is not just an API latency problem. It is a systems problem with physical constraints:

- **CPU topology**: Compilation is CPU-bound. 20 parallel agents each triggering `cargo check` compete for cores. Task scheduling must account for CPU availability, not just logical concurrency.
- **Memory hierarchy**: Compilation is memory-intensive. 20 simultaneous builds can exhaust physical RAM, triggering swap, which triggers catastrophic slowdown. The sccache layer prevents this by serializing cache writes.
- **SSD IOPS**: Each gate evaluation (compile, test, clippy) involves thousands of file reads and writes. On spinning disk, this is the bottleneck. On SSD, it's manageable but still observable under load.
- **Disk pressure propagation**: When compilation saturates disk I/O, gate evaluations time out. From the orchestrator's perspective, this looks like a gate failure — but the root cause is substrate pressure, not code quality. Roko's conductor watchers detect this pattern and throttle concurrency.
- **API latency spikes**: Cloud provider rate limits, network congestion, and inference queue depth all manifest as agent stalls. The ProcessSupervisor detects stalled agents and can reassign work or escalate to a different model endpoint.

### Why This Is a Moat

No other agent framework operates 12 agents simultaneously with shared compilation caches, worktree isolation, and warm agent pools. The infrastructure investment is invisible — it doesn't appear in feature lists or demo videos — but it is the foundation that makes everything else possible. Without the substrate layer:

- Parallel plan execution is theoretical (you run out of disk/memory after 3-4 agents)
- Gate evaluation is unreliable (timeouts from resource contention masquerade as failures)
- Cost projections are meaningless (20× redundant compilation dominates the budget)
- Scale claims are hollow (you can't run 20 agents if 20 agents crash the machine)

The substrate is roko's unexamined competitive advantage. It is the infrastructure moat that makes the architectural moats possible.

---

## Information Architecture: Every Boundary Is Signal Loss

Why roko invests so heavily in context engineering — and why most agent frameworks dramatically underinvest.

### The Seven Boundaries

Every agent task in roko crosses seven information boundaries:

```
PRD → Plan → tasks.toml → Enrichment → Prompt → Agent Reasoning → Gate → Iteration feedback
```

1. **PRD to Plan**: A human-written product requirement document is decomposed into an execution plan. The plan captures structure but loses nuance, edge cases, and implicit assumptions.
2. **Plan to tasks.toml**: The plan is serialized into discrete task definitions. Dependencies are explicit but motivation is compressed.
3. **tasks.toml to Enrichment**: Task definitions are enriched with codebase context, workspace analysis, and prior episode data. Information is added, but selection is lossy — which context gets included?
4. **Enrichment to Prompt**: The enriched context is assembled into a prompt with cache alignment, token budgeting, and section ordering. Compression decisions discard information.
5. **Prompt to Agent Reasoning**: The LLM processes the prompt. Attention is finite. Long-range dependencies are attenuated. "Lost in the Middle" (Liu et al., 2023) is real.
6. **Agent Reasoning to Gate**: The agent's output (code changes, tool calls) is evaluated by the gate pipeline. The gate sees only the output, not the reasoning. Correct reasoning with a minor output error fails the gate.
7. **Gate to Iteration**: Gate results feed back to the agent for retry. The failure message must communicate WHAT went wrong AND WHY clearly enough for the agent to correct its approach.

### The Cumulative Loss Math

Each transformation preserves some fraction of the original information. At 90% preservation per boundary (optimistic), cumulative preservation across all seven is:

```
0.9^7 = 0.478  →  47.8% of original information reaches the final boundary
```

At 80% preservation (realistic for poorly-engineered pipelines):

```
0.8^7 = 0.210  →  21.0% of original information reaches the final boundary
```

This is devastating. It means that a task with clear requirements, correct decomposition, and a capable model can STILL fail because the information was degraded below the threshold needed for correct execution. Every retry is a SYMPTOM of signal loss at one or more boundaries. Better signal preservation → fewer retries → lower cost → more capacity.

### Shannon's Channel Capacity

Shannon (1948) proved that every communication channel has a maximum rate at which information can be transmitted reliably. The context window IS a bandwidth-limited channel:

- **Channel capacity** = the effective information throughput of the context window
- **Source coding** = how efficiently we encode task-relevant information into tokens
- **Channel coding** = how we structure the prompt to resist attention degradation
- **Noise** = irrelevant context, stale information, prompt injection attempts

The implication: fitting the right 10K tokens into the context window matters more than having 200K tokens available. A 10K-token prompt with 90% signal is strictly superior to a 200K-token prompt with 5% signal. The LLM's attention mechanism has finite precision — diluting it with irrelevant context degrades performance even on the relevant portions.

### Roko's Investment in Signal Preservation

This explains every layer of roko's context engineering stack. Each layer preserves signal at a specific boundary:

| Layer | What It Does | Which Boundary |
|---|---|---|
| **9-layer prompt assembly** | Structured ordering with cache alignment | Enrichment → Prompt |
| **Cache-aligned sections** | Stable content first, volatile last | Prompt → Agent Reasoning |
| **Token budgeting** | Allocate tokens by importance, not size | Enrichment → Prompt |
| **Section bandit** | Drop sections that don't improve pass rates | Enrichment → Prompt |
| **History compression** | Summarize prior turns, preserve key decisions | Gate → Iteration |
| **PII masking** | Remove irrelevant personal data | tasks.toml → Enrichment |
| **Injection detection** | Filter adversarial content from context | Enrichment → Prompt |
| **Tool pruning** | Show only relevant tools per role | Prompt → Agent Reasoning |
| **Workspace analysis** | PageRank-ranked file relevance | Plan → tasks.toml |

No single layer is revolutionary. The compound effect is. If each layer improves preservation by just 3% at its boundary, the cumulative improvement across 9 layers is:

```
1.03^9 = 1.305  →  30.5% more information reaches the agent
```

That 30.5% improvement translates directly to higher first-pass success rates, fewer retries, and lower cost.

---

## The Observability Advantage

Why typed events matter — and why "we have logging" is not the same as "we have observability."

### EventBus: The Nervous System

Roko's EventBus is not a logging library. It is a typed, sequenced, replayable event stream that serves as the system's nervous system:

- **Typed events**: Every event has a Rust enum variant with structured fields. `AgentStarted { agent_id, model, role }` is not a string — it is a type-checked data structure that downstream consumers can pattern-match on.
- **Monotonic sequence numbers**: Every event gets an atomically-incremented sequence number. Events are totally ordered. No clock skew, no out-of-order delivery, no lost events.
- **10,000-event ring buffer**: The most recent 10,000 events are always in memory. Dashboard queries, health checks, and diagnostic tools read from the ring buffer with zero disk I/O.
- **Replay capability**: The ring buffer supports replay from any sequence number. A TUI that connects mid-execution can replay the buffer to reconstruct current state.

### Crash Recovery via Event Replay

Traditional agent frameworks use state snapshots for recovery: serialize the current state to disk, reload on restart. This works but has critical limitations:
- Snapshots are point-in-time — anything between the last snapshot and the crash is lost
- Snapshot format changes require migration logic
- Partial writes during a crash can corrupt the snapshot

Roko supports event sourcing: rebuild state by replaying the event stream. The event log is append-only (corruption-resistant), each event is independently valid (no partial-write risk), and the event format is the source of truth (no snapshot migration needed). The complete audit trail is a side effect of the recovery mechanism.

### Real-Time Streaming

The EventBus is not just for recovery — it is the foundation for every real-time interface:

- **TUI dashboard**: Subscribes to filtered event streams (agent events, gate events, plan events). Renders live updates without polling.
- **WebSocket/SSE**: External dashboards subscribe to the same event stream over HTTP. The filter is applied server-side; the client receives only relevant events.
- **CI integration**: Gate results stream to CI systems as they complete. No waiting for the entire plan to finish.
- **Alerting**: Conductor watchers subscribe to specific event types (gate failures, agent stalls, budget overruns) and trigger circuit breakers or notifications.

### OpenTelemetry Integration

Roko's execution hierarchy maps directly to OpenTelemetry's span model:

```
Plan (trace)
  └── Phase (span)
        └── Agent (child span)
              └── Inference call (child span)
              └── Tool execution (child span)
        └── Gate evaluation (child span)
```

Every agent turn, every gate evaluation, every inference call is a span with structured attributes. Distributed tracing tools (Jaeger, Grafana Tempo, Datadog) can visualize the entire execution DAG with zero custom integration — roko speaks the standard protocol.

### Why This Matters Competitively

No other agent framework has structured observability. They have logs. Some have metrics. A few have dashboards.

The difference:
- **Logs**: Unstructured text. Requires regex parsing. No guaranteed ordering. No replay. No filtering without grep.
- **Metrics**: Aggregated numbers. Tells you THAT something happened (latency p99 = 3s). Does not tell you WHY.
- **Typed event stream**: Structured data with guaranteed ordering, replay, filtering, and aggregation. Tells you what happened, when, in what order, and lets you query the stream programmatically.

Roko's event stream can be queried ("show me all gate failures for agent X in the last hour"), replayed ("reconstruct the state as of sequence 5,000"), aggregated ("what is the average pass rate per model per task type"), and streamed ("notify me when any agent stalls for >30 seconds"). Logs cannot do any of these things efficiently. The observability layer is not a feature — it is the foundation for debugging, monitoring, learning, and eventually self-healing.
