# Architectural Theory: Layer Model, Co-Evolution, Developmental Stages, and Generalization

> **Audience**: Systems architects, researchers, platform engineers
> **Scope**: The theoretical foundations that make roko more than a tool — a cognitive architecture grounded in 40+ years of AI and cognitive science research
> **Source**: mori-refactor docs (00-26), 27 documents totaling 700+ KB

---

## 1. The Five-Layer Architecture

### Layer Stack (Dependencies Flow Downward Only)

```
Layer 4: ORCHESTRATION   (domain-specific)
  ├── Plan DAG executor, 14-phase state machine
  ├── Merge queue, crash recovery, wave scheduling
  └── Multi-agent coordination, task assignment

Layer 3: HARNESS          (quality verification)
  ├── 7-rung gate pipeline with adaptive thresholds
  ├── 10 conductor watchers, circuit breaker
  └── Pattern detection, intervention policies

Layer 2: SCAFFOLD          (context engineering)
  ├── 9-step enrichment pipeline (83% token reduction)
  ├── 6-layer prompt assembly with cache alignment
  └── Dynamic budget allocation per role

Layer 1: FRAMEWORK         (model-agnostic primitives)
  ├── 28 agent roles with tool permissions
  ├── 3 connection backends (Claude/Cursor/Codex)
  └── Model routing, capability tokens, tool registry

Layer 0: RUNTIME           (process lifecycle)
  ├── ProcessSupervisor, hierarchical CancelToken
  ├── EventBus (typed broadcast + replay ring)
  └── ResourceAccount (token/cost/time budgets)
```

### Design Principles

**P1: Dependencies flow downward only.** Layer 4 may depend on Layers 0-3. Layer 0 depends on nothing. Cross-cutting concerns (inference, memory, safety, observability, learning) are injected via traits, never via upward imports.

**P2: Trait-based API boundaries.** Every layer exposes capabilities through Rust traits. Concrete implementations live behind `Arc<dyn Trait>`. This enables: testing each layer in isolation with mocks, swapping implementations without changing consumers, and clear documentation of what each layer provides.

**P3: Cross-cutting concerns are injected.** Memory, safety, observability, and inference optimization are not owned by any single layer — they span multiple layers and are implemented as components injected into each.

**P4: Domain-specific logic stays at application layer.** The framework layers (0-4) must NOT contain roko-specific domain knowledge. Prompt templates, crate names, coding conventions — all loaded from configuration at the application layer.

**P5: Everything is observable.** Every layer emits structured events. Every cross-cutting operation emits metrics. Every multi-step operation carries trace context.

### Cross-Cutting Concerns (Horizontal)

| Concern | What It Does | Spans Layers |
|---|---|---|
| **Inference Optimization** | Caching, cascades, cost routing, prompt normalization | 1-2 |
| **Memory & Knowledge** | Episodic/semantic retrieval, decay, distillation | 0-3 |
| **Code Intelligence** | AST, symbols, embeddings, graphs, fingerprints | 1-2 |
| **Safety & Alignment** | Capabilities, audit chain, taint labels | 0-4 |
| **Observability** | Events, metrics, tracing, logging | 0-4 |
| **Self-Improvement** | Online learning, playbook rules, bandit selection | 2-4 |

### Evidence: The Scaffold IS the Product

**Lee et al. (Stanford, 2026)**: 6× performance gap between best and worst scaffold using the same model. The harness matters more than the foundation model.

| Agent | Model | SWE-bench |
|---|---|---|
| SWE-agent | Claude Sonnet 4 | 57.5% |
| Agentless | Claude Sonnet 4 | 41.8% |
| Vanilla | Claude Sonnet 4 | ~20% |

Same model. Different scaffold. 2.9× performance range. This is why roko invests in layers 2-3 (scaffold + harness) rather than chasing the latest model.

---

## 2. Cognitive Architecture Mapping

Roko's architecture maps to four classical cognitive architectures, each contributing a specific design insight:

### ACT-R (Anderson et al., 2004) — 40 Years of Validation

| ACT-R Component | Roko Implementation |
|---|---|
| Declarative memory (activation-based retrieval) | Playbook rule confidence scoring |
| Procedural memory (production rules) | Gate pipeline + conductor watchers |
| Goal module (task stack) | Plan DAG with task dependencies |
| Activation spreading | PageRank on symbol graph |

**Key insight from ACT-R**: Activation-based retrieval (more frequently/recently accessed items have higher activation) maps directly to roko's playbook confidence scoring and Ebbinghaus decay.

### SOAR (Laird, 2012) — Impasse-Driven Learning

| SOAR Component | Roko Implementation |
|---|---|
| Impasse detection | Gate failure triggering re-planning |
| Chunking (learning from impasse resolution) | Episode → pattern → playbook rule promotion |
| Subgoaling | Task decomposition in enrichment pipeline |
| Universal subgoaling | Any failed gate creates subgoal to fix it |

**Key insight from SOAR**: When an agent reaches an impasse (gate failure), the resolution process (reflection + retry) is itself a learning opportunity. The solution becomes a new "chunk" (playbook rule) that prevents future impasses.

### CLARION (Sun, 2006) — Dual-Process Theory

| CLARION Component | Roko Implementation |
|---|---|
| Implicit (bottom-up) processing | T0 tier: deterministic probes, cached decisions |
| Explicit (top-down) processing | T2 tier: full LLM reasoning |
| Bottom-up learning | Pattern extraction from successful episodes |
| Top-down learning | Prompt optimization from explicit feedback |

**Key insight from CLARION**: The two processing modes (implicit/explicit) are not alternatives — they interact. In roko, T0 (implicit) and T2 (explicit) work together: T0 handles routine ticks cheaply, T2 handles novel situations expensively, and the boundary between them is learned via the prediction error signal.

### Global Workspace Theory (Baars, 1988) — Broadcast Competition

| GWT Component | Roko Implementation |
|---|---|
| Global workspace (limited capacity broadcast) | CorticalState (32 atomic signals, 256 bytes) |
| Unconscious specialists | 10 conductor watchers running in parallel |
| Attention competition | VCG auction allocating cognitive budget |
| Conscious broadcast | EventBus distributing winning signals to all subsystems |

**Key insight from GWT**: Intelligence emerges from competition for a limited-capacity broadcast channel. In roko, the CorticalState IS the global workspace — 32 signals that all subsystems can read, written by whichever subsystem has the most urgent update.

### The 8-Step Unified Cognitive Cycle

```
PERCEIVE     → What is happening? (observations, probes, chain events)
REMEMBER     → What do I know? (Grimoire retrieval, 4-factor scoring)
REASON       → What should I do? (LLM deliberation, if T1/T2)
GATE         → Should I act? At what depth? (prediction error → tier selection)
ACT          → Do it (tool execution, code writing, tx signing)
EVALUATE     → Did it work? (gate pipeline, external verification)
REFLECT      → What did I learn? (episode logging, pattern extraction)
META-COGNIZE → Am I doing this well? (conductor monitoring, mortality check)
```

**Research**: CoALA (Sumers et al., 2023) — the first formal cognitive architecture for language agents.

---

## 3. Agent-Environment Co-Evolution

### The Niche Construction Thesis

Roko agents don't just operate IN a codebase — they CONSTRUCT it. Every modification changes the environment for future agents. This creates a bidirectional feedback loop:

```
Agent produces code → Codebase changes → Future agents inherit changed codebase
    → Quality of inherited code affects future agent performance
    → Performance affects quality of produced code → (loop)
```

**Research**: Odling-Smee et al. (2003) — niche construction theory. Organisms modify their environment, changing selection pressures on themselves and descendants.

### Affordance Scoring

Each file/module has an **affordance score** — how easy it is for an agent to work with:

```
affordance = w₁ × extensibility + w₂ × test_coverage + w₃ × documentation
           + w₄ × (1 - coupling) + w₅ × recent_stability + w₆ × (1 - size/max_size)
```

Computable from tree-sitter AST + git log. High-affordance code → agents succeed more often → code quality improves further. Low-affordance code → agents fail → rework degrades quality further.

**The exponential**: 1% affordance improvement per plan × 100 plans = **170% cumulative improvement**. The inverse (negative niche construction) creates a "death spiral" where degraded affordances → more failures → lower-quality code → further degradation.

### Information Scent

**Research**: Pirolli & Card (1999) — information foraging theory. Agents navigate codebases by following "scent" cues: function names, doc comments, test names, import statements.

Strong scent → efficient navigation → more correct code. Weak scent → wasted tokens exploring dead ends → higher cost, lower quality.

Roko amplifies scent by:
1. Generating doc comments on agent-written code
2. Writing descriptive test names
3. Using meaningful variable names
4. Tracking scent quality over time

### Stigmergy: Git IS Coordination

**Research**: Grassé (1959) — stigmergy. Coordination through shared environment modification, not direct communication.

In roko: `discovered-patterns.json`, `CONTEXT.md`, playbook rules, and commit messages are **stigmergic marks** — information deposited in the environment that other agents read without direct communication. Cost: O(1) per agent (no N² messaging). Scales to any number of agents.

---

## 4. Developmental Trajectory (Four Stages)

### Stage Model (Inspired by Piaget's Cognitive Development)

System capabilities change over time. Plan 1 ≠ Plan 500 — accumulated playbook rules, episodes, learned routing, calibrated gates, and codebase familiarity all evolve.

| Stage | Plans | Description | Parallel Agents | Review Depth |
|---|---|---|---|---|
| **Bootstrap** | 1-5 | Blind execution, no model, conservative | 2 | Full |
| **Learning** | 5-30 | Pattern recognition, playbook accumulation | 4 | Targeted |
| **Competent** | 30-100 | Generalization, learned routing, confidence | 8 | Diff-only |
| **Expert** | 100+ | Full autonomy for routine, minimal human | 12 | Anomaly-only |

### Stage-Dependent Parameters

| Parameter | Bootstrap | Learning | Competent | Expert |
|---|---|---|---|---|
| Max parallel agents | 2 | 4 | 8 | 12 |
| Review depth | Full pipeline | Targeted | Diff-only | Anomaly detection |
| Model routing | Conservative (always Opus) | Empirical (try Sonnet) | Aggressive (Haiku for simple) | Minimal (cached decisions) |
| Conductor threshold | Low (intervene often) | Medium | High | Very high |
| Enrichment verbosity | Maximum (full context) | High | Medium | Minimal (only deltas) |
| Gate retry limit | 5 | 4 | 3 | 2 |

### Theoretical Foundations

- **Piaget**: Discrete qualitative transitions between stages, not just quantitative improvement
- **Vygotsky (ZPD)**: Optimal learning at tasks WITHIN the Zone of Proximal Development; scaffolding fades as competence grows
- **Dreyfus (Skill Acquisition)**: Novice (rule-based) → Expert (intuitive recognition)
- **Curriculum Learning**: Easy tasks first build the playbook for hard tasks (superlinear effect)

### Session Continuity

Stage progress persists across sessions. Without persistence, every restart is a cold start:

- Routing statistics → model selection quality
- Scaffold weights → context assembly quality
- Complexity calibration → pipeline selection quality
- Gate calibration → retry budget quality
- Codebase model → context relevance quality

---

## 5. The Information Architecture (7-Boundary Pipeline)

Every signal passes through 7 major boundaries from PRD to execution:

```
PRD → Plan → tasks.toml → Enrichment → Prompt → Agent Reasoning → Gate → Iteration
  ↑___________________________________(feedback loop)_________________________________↓
```

Each boundary is a communication channel with finite capacity and noise. **Signal degradation across boundaries determines output quality.** Perfect signal preservation at every boundary = first-pass success.

### Boundary Analysis

| Boundary | Signal Loss Risk | Mitigation |
|---|---|---|
| PRD → Plan | Requirements omitted/misinterpreted | Research agent + human review |
| Plan → tasks.toml | Acceptance criteria underspecified | Generated verification artifacts |
| tasks.toml → Enrichment | Context irrelevant/missing | HDC fingerprinting + PageRank |
| Enrichment → Prompt | Budget truncation drops critical info | Priority-based section ordering |
| Prompt → Reasoning | Lost-in-the-middle attention degradation | U-shaped placement (critical at start/end) |
| Reasoning → Gate | Agent produces wrong code | 7-rung verification pipeline |
| Gate → Iteration | Error feedback too noisy/vague | Structured error digest + Haiku reflection |

**The feedback loop** is the key mechanism: gate failures feed back into enrichment (what context was missing?) and prompt assembly (which sections correlate with success?). This closes the signal degradation loop.

---

## 6. Generalization: From Rust Tool to Agent Platform

### Current Coupling (16 Points)

Roko is currently Rust-specific in 16 places across 12 files (~4,500 lines of Rust/Cargo-specific logic). Generalization separates WHAT (orchestrate agents) from HOW (language-specific tools).

### Key Abstractions

```rust
pub trait ProjectDetector {
    fn detect(root: &Path) -> Option<ProjectProfile>;
}

pub trait BuildSystem: Send + Sync {
    fn compile_check(&self, root: &Path, scope: &[String]) -> Result<GateResult>;
    fn run_tests(&self, root: &Path, scope: &[String]) -> Result<GateResult>;
    fn lint(&self, root: &Path, scope: &[String]) -> Result<GateResult>;
}

pub trait LanguageProvider: Send + Sync {
    fn parse(&self, source: &str) -> Result<SymbolTable>;
    fn language_name(&self) -> &str;
    fn file_extensions(&self) -> &[&str];
}
```

### Five Implementations

| Build System | Detection File | Compile | Test | Lint |
|---|---|---|---|---|
| **Cargo** | Cargo.toml | `cargo check` | `cargo test` | `cargo clippy` |
| **Npm** | package.json | `npm run build` | `npm test` | `eslint` |
| **Go** | go.mod | `go build ./...` | `go test ./...` | `golangci-lint` |
| **Python** | pyproject.toml/setup.py | `python -m py_compile` | `pytest` | `ruff` |
| **Forge** | foundry.toml | `forge build` | `forge test` | `slither` |

### Plugin Architecture

```rust
pub trait MoriPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn on_load(&self, registry: &mut PluginRegistry);
    // Register: new language, backend, gate type, enrichment step
}
```

Third parties can add: new languages, new model backends, custom gate types, enrichment steps, and non-coding use cases (research, documentation, DevOps).

### Seven Deployment Modes

| Mode | Interface | Use Case |
|---|---|---|
| Local TUI | Terminal UI (ratatui) | Developer workstation |
| Headless local | JSON logs, exit codes | CI/CD pipelines |
| Remote server | HTTP + WebSocket | Team shared instance |
| Standalone agent | CLI one-shot/REPL | Quick tasks |
| Agent-as-service | REST API, multiple sessions | Platform integration |
| GitHub bot | Webhooks | Auto-triage, auto-review |
| Daemon | Background with subscriptions | Event-driven workflows |

---

## 7. Cost Optimization: Five Multiplicative Levers

### The Cost Equation

```
Total = Σ(input_tokens × price + output_tokens × price) × iterations × tasks × plans − cache_savings
```

### Five Levers (Multiplicative — Reducing ANY One Reduces Total)

| Lever | Current | Target | Mechanism |
|---|---|---|---|
| **Input tokens** | 25K avg | 4-10K | Dynamic budgets by complexity band |
| **Iterations** | 1.86 avg | 1.2 | Better context → first-pass success |
| **Model routing** | 0.1% Haiku | 15%+ Haiku | Complexity classifier → cheap model for simple tasks |
| **Cache hit rate** | 81% | 90%+ | BTreeMap deterministic serialization |
| **Batch API** | 0% | 30%+ | Non-urgent enrichment via 50% discount batch |

### Token Budget by Complexity

| Complexity | Budget | Target Cost |
|---|---|---|
| Fast (trivial) | 4K tokens | $0.05 |
| Standard | 10K tokens | $0.50 |
| Complex | 25K tokens | $2.00 |

**Biggest single waste**: Fast tasks currently receive the same 25K-token prompts as complex tasks. Narrowing the complexity band → $0.40/task savings (~40%).

### Architecture

```
Complexity Classifier → Token Budget → Context Strategy Selector
  → Section Bandit → Prompt Assembly → Cache Check → Inference
  → Gate → Learning Update (bandit feedback)
```

Each component is independently optimizable. The section bandit learns which prompt sections are worth including per task type. The complexity classifier determines the budget. The cache check skips inference entirely for repeated contexts.

---

## 8. The Unified Theory: Same Pattern, Four Parameters

Both the coding agent (Mori/roko) and the DeFi agent (Golem) are instances of the **same cognitive architecture** differing only in four parameters:

| Parameter | Coding Agent | DeFi Agent |
|---|---|---|
| **Domain** | Software development (files, tests, PRs) | DeFi trading (pools, positions, transactions) |
| **Timescale** | Hours per plan cycle | Seconds per heartbeat tick |
| **Multiplicity** | Multi-agent swarm (28 roles) | Single agent (1 golem) |
| **Substrate** | Code files on disk | Blockchain state on-chain |

Everything else is shared: the 5-layer architecture, the cognitive cycle, the memory substrates, the gate pipeline, the learning loops, the safety model. This is why the codebase shares a foundation (`bardo-runtime`, `bardo-primitives`) across both applications.

### Dependency Constraint

```
roko-* crates NEVER depend on golem-* crates
golem-* crates NEVER depend on roko-* crates
Communication only through shared crates (bardo-*) or network protocols (Korai)
```

### What This Means

A self-improving coding agent and a self-improving DeFi agent are the same system operating in different domains. Improvements to the foundation (better routing, better gates, better learning) benefit BOTH applications simultaneously.

---

## 9. The Human-Agent Interface (Trust and Autonomy)

### TUI as Cognitive Tool

The interface is not a dashboard — it's a **cockpit** for operator perception. Design principles:

- **Dual-Reading**: Glance (emotional state via creature brightness) + sustained reading (specific numbers). One screen works at both distances.
- **Trust calibration**: Operator assesses reliability per-plan and per-phase. Trust builds through successful outcomes.
- **Autonomy delegation**: Choosing which plans run autonomously vs. pause for approval.
- **Mixed-initiative**: Protocol for who initiates action and when initiative transfers.

### The 7-Boundary Information Pipeline

Signal flows through 7 major boundaries from PRD to execution:

```
PRD → Plan → tasks.toml → Enrichment → Prompt → Agent Reasoning → Gate → Iteration
  ↑________________________________(feedback loop)________________________________↓
```

Each boundary is a communication channel with **finite capacity and noise**. Signal degradation across boundaries determines output quality. Perfect preservation at every boundary = first-pass success.

| Boundary | Risk | Mitigation |
|---|---|---|
| PRD → Plan | Requirements omitted | Research agent + human review |
| Plan → tasks.toml | Criteria underspecified | Generated verification artifacts |
| tasks.toml → Enrichment | Context irrelevant | HDC fingerprinting + PageRank |
| Enrichment → Prompt | Budget truncation | Priority-based section ordering |
| Prompt → Reasoning | Attention degradation | U-shaped placement (critical at start/end) |
| Reasoning → Gate | Wrong code produced | 7-rung verification pipeline |
| Gate → Iteration | Feedback too noisy | Structured error digest + Haiku reflection |

**Research**: Shannon (channel capacity), Liu et al. (2023 — Lost in the Middle).

---

## 10. The Substrate Layer (Below Runtime)

Everything beneath the agent runtime: hardware, OS, network, build infrastructure.

### The Substrate Invisibility Problem

Substrate failures are invisible until they cascade:
- **Disk pressure** triggers GC during agent execution → latency spike
- **CPU saturation** from 12 parallel agents starves TUI → interface freeze
- **API latency spike** from provider → all agents timeout simultaneously
- **Build cache eviction** → 30s compile delays at every gate

### ResourceAccount (Budget Tracking)

```rust
pub struct ResourceAccount {
    tokens: BudgetEntry<u64>,      // input + output
    cost: BudgetEntry<f64>,        // USD
    time_limit: Duration,
    started_at: Option<Instant>,
}

// Tier presets:
ResourceAccount::trivial()   // 2K tokens, $0.05, 30s
ResourceAccount::simple()    // 8K tokens, $0.50, 120s
ResourceAccount::standard()  // 20K tokens, $2.00, 300s
ResourceAccount::complex()   // 50K tokens, $5.00, 600s
```

Utilization metrics: `token_utilisation()`, `cost_utilisation()`, `time_utilisation()` → f64 [0.0, 1.0]. Any `>= 1.0` → budget exhausted → agent stops.

---

## 11. The 35 Catalogued Gaps (From Gap Analysis)

Organized by layer, with severity:

### Layer 0 (Runtime)
- **HIGH**: No structured concurrency (supervision trees)
- MED: No process migration (live handoff on crash)

### Layer 1 (Framework)
- MED: No dynamic tool synthesis
- MED: Hardcoded to 3 backends

### Layer 2 (Scaffold)
- **HIGH**: No automated prompt optimization (DSPy-style)
- MED: No HyDE retrieval
- **HIGH**: Hardcoded to Rust/Cargo

### Layer 3 (Harness)
- **HIGH**: No process reward models (only final-output gates)
- **HIGH**: No property-based gate generation
- MED: No learned intervention policies

### Cross-Cutting (Inference)
- **HIGH**: No cascade routing with learned thresholds
- MED: No cross-request batching

### Cross-Cutting (Memory)
- **HIGH**: No memory graphs (flat JSONL/SQLite)
- **HIGH**: No cross-agent memory for coding agents

### Cross-Cutting (Code Intelligence)
- MED: No embedding-based code search
- MED: No program slicing
- MED: No change impact analysis

**What this means for roadmap**: HIGH gaps are the priority. Automated prompt optimization, cascade routing, and memory graphs would each independently produce 15-30% improvement. Together they compound multiplicatively.

---

## Research Citations (This Document)

| Paper | Year | Contribution |
|---|---|---|
| Anderson et al. (ACT-R) | 2004 | Activation-based memory retrieval |
| Laird (SOAR) | 2012 | Impasse-driven learning, chunking |
| Sun (CLARION) | 2006 | Dual-process theory for agents |
| Baars (Global Workspace) | 1988 | Broadcast competition for attention |
| Sumers et al. (CoALA) | 2023 | Cognitive architecture for language agents |
| Odling-Smee et al. (Niche Construction) | 2003 | Agent-environment co-evolution |
| Gibson (Affordances) | 1979 | Action possibilities from environment |
| Pirolli & Card (Info Foraging) | 1999 | Information scent navigation |
| Grassé (Stigmergy) | 1959 | Indirect coordination via environment |
| Piaget (Cognitive Development) | 1952 | Stage-based development theory |
| Vygotsky (ZPD) | 1978 | Scaffolding in zone of proximal development |
| Dreyfus (Skill Acquisition) | 1980 | Novice → Expert progression |
| Kahneman (Dual Process) | 2011 | System 1/System 2 thinking |
| Clark (Predictive Processing) | 2013 | Brain as prediction machine |
| Friston (Active Inference) | 2010 | Free energy minimization |
| Conant & Ashby (Good Regulator) | 1970 | Self-models required for control |
| Kauffman (NK Landscapes) | 1993 | Clean layer boundaries reduce fitness landscape ruggedness |
| Lee et al. (Meta-Harness) | 2026 | 6× performance gap from scaffold alone |
| Bengio et al. (Curriculum Learning) | 2009 | Training on easier examples first improves generalization |
| Shannon (Channel Capacity) | 1948 | Maximum information throughput per communication channel |
| Liu et al. (Lost in the Middle) | 2023 | Attention degradation in long-context LLMs |

---

## 12. The Developmental Trajectory: How the System Grows Over Time

Plan 1 is not Plan 500. A roko instance that has executed five plans is fundamentally different from one that has executed five hundred. The difference is not just quantitative (more data) but qualitative (different reasoning strategies, different confidence profiles, different relationships with the operator). This section describes how roko's capabilities evolve across its lifetime and what theoretical models explain that evolution.

### Four Developmental Stages

| Stage | Plan Range | Description | Internal State |
|---|---|---|---|
| **Bootstrap** | 1-5 | Blind execution, no internal model | Empty playbook, uncalibrated gates, conservative routing, no episode history |
| **Learning** | 5-30 | Recognizes patterns, applies literally | Growing playbook (~20-50 rules), initial gate calibration, first routing experiments |
| **Competent** | 30-100 | Generalizes within domain | Mature playbook (~100-200 rules), calibrated gates, empirical routing, codebase familiarity |
| **Expert** | 100+ | Full autonomy for routine tasks | Dense playbook (~300+ rules), fine-tuned gates, aggressive routing, deep codebase model |

The transitions between stages are not smooth — they are **qualitative shifts** in how the system operates. A Bootstrap-stage instance executes every task with maximum context, full review, and conservative model routing. An Expert-stage instance recognizes that a routine `cargo fmt` fix needs 4K tokens, no review, and a Haiku call. The Expert is not just faster — it reasons differently about the same task.

### Theoretical Foundations: Three Models of Growth

**Piaget's Cognitive Development (1952)**: Piaget demonstrated that children don't learn gradually — they pass through discrete stages (sensorimotor, preoperational, concrete operational, formal operational) where each stage involves qualitatively different cognitive operations. The roko stages mirror this: Bootstrap is sensorimotor (reactive, no model), Learning is preoperational (has rules but applies them rigidly), Competent is concrete operational (generalizes within known domains), Expert is formal operational (abstracts across domains).

The critical Piagetian insight is **assimilation vs. accommodation**. When the system encounters a new task type:
- **Assimilation**: The task fits existing playbook rules. Apply them. (Cheap, fast, usually correct.)
- **Accommodation**: The task doesn't fit. Existing rules must be modified or new rules created. (Expensive, slow, learning happens here.)

Early stages are dominated by accommodation (everything is new). Later stages are dominated by assimilation (most tasks fit existing patterns). The ratio of assimilation to accommodation IS the developmental stage.

**Dreyfus Skill Acquisition Model (1980)**: Dreyfus identified five stages of skill acquisition: Novice (follows rules), Advanced Beginner (recognizes situational elements), Competent (prioritizes and plans), Proficient (sees the whole picture), Expert (acts from intuition). Roko's four stages compress this into a computationally tractable model where the key transition is from rule-following (Bootstrap/Learning) to pattern recognition (Competent/Expert).

The Dreyfus model's most important claim: experts don't follow rules faster — they don't follow rules at all. They recognize situations and respond from accumulated experience. In roko terms, an Expert-stage instance routes 60%+ of tasks through T0 (cached decisions) because it recognizes the task pattern and already knows the correct approach. The playbook rule IS the expert intuition.

**Vygotsky's Zone of Proximal Development (1978)**: The ZPD is the range of tasks a learner cannot do alone but can accomplish with guidance. Below the ZPD: already mastered, no learning value. Above the ZPD: too hard, learning fails. Within the ZPD: optimal learning territory.

For roko, the operator is the "more capable partner" who:
1. Reviews outputs during Bootstrap (full scaffolding)
2. Reviews only failures during Learning (partial scaffolding)
3. Reviews only anomalies during Competent (scaffolding fading)
4. Reviews only novel domains during Expert (scaffolding withdrawn)

This maps directly to the **scaffolding fading** principle: as the system demonstrates competence at a task class, the operator withdraws support for that class and focuses their attention on the next frontier.

### Stage-Dependent Parameters (Detailed)

| Parameter | Bootstrap (1-5) | Learning (5-30) | Competent (30-100) | Expert (100+) |
|---|---|---|---|---|
| Max parallel agents | 2 | 4 | 8 | 12 |
| Review depth | Full pipeline | Targeted (failures + novel) | Diff-only (changed lines) | Anomaly-only (statistical outliers) |
| Model routing | Conservative (always T2/Opus) | Empirical (try T1/Sonnet) | Aggressive (T0/Haiku for simple) | Minimal (60%+ cached decisions) |
| Conductor threshold | Low (intervene at 3 signals) | Medium (intervene at 5) | High (intervene at 8) | Very high (intervene at 12) |
| Gate retry limit | 5 | 4 | 3 | 2 |
| Enrichment verbosity | Maximum (full context, all sections) | High (prioritized sections) | Medium (role-specific budget) | Minimal (only deltas from last success) |
| Playbook rule weight | 0.0 (no rules exist) | 0.5 (rules used but not trusted) | 0.8 (rules strongly weighted) | 0.95 (rules are default behavior) |
| Autonomy level | Human approves each task | Human approves each plan | Human approves anomalies | Human approves new domains |

The parameter transitions are not instant. Each parameter has a **transition function** that interpolates between stages based on accumulated evidence (number of successful plans, gate pass rates, routing accuracy). A system at plan 28 with high gate pass rates may operate at Learning parameters for most dimensions but Competent parameters for model routing if it has enough routing data.

### Curriculum Learning: Easy Tasks First

**Research**: Bengio et al. (2009) demonstrated that training on easier examples first, then progressively harder ones, improves both convergence speed and final performance compared to random ordering. This is curriculum learning.

Roko applies curriculum learning to plan execution. Among equally-prioritized free plans (no blocking dependencies), the executor prefers simpler ones first:

```
plan_complexity = Σ(task_complexity) + dependency_depth × depth_weight
task_complexity = file_count × 0.3 + estimated_tokens × 0.3 + new_module × 0.4
```

Simpler plans execute first. Their success:
1. Generates playbook rules applicable to harder plans
2. Calibrates gate thresholds with easier-to-verify outputs
3. Builds routing statistics with lower-risk tasks
4. Accumulates codebase familiarity from smaller changes

The compounding effect is superlinear: each easy plan completed makes the next harder plan cheaper and more likely to succeed on the first attempt. This is not just efficiency — it's the system building its own training curriculum.

### The ZPD and Scaffolding Fading

The operator-system relationship follows a predictable trajectory:

**Bootstrap (Plans 1-5)**: Operator is deeply involved. Reviews every output. Corrects every mistake. The system has no internal model — the operator IS the model. This is Vygotsky's "other-regulation" — the learner depends entirely on the more capable partner.

**Learning (Plans 5-30)**: Operator reviews failures and novel situations. The system has begun to internalize patterns (playbook rules emerge). But it applies rules literally, without context sensitivity. The operator provides the contextual judgment. This is the ZPD — the system can succeed with help but not alone.

**Competent (Plans 30-100)**: Operator reviews anomalies only. The system generalizes within known domains. Playbook rules have context conditions. Gate thresholds are calibrated. The operator's role shifts from "doing the work" to "monitoring the work." Scaffolding is fading.

**Expert (Plans 100+)**: Operator reviews new domains and edge cases. Routine tasks execute autonomously. The operator's role is strategic: deciding WHAT to build, not HOW to build it. This is Vygotsky's "self-regulation" — the learner has internalized the partner's capabilities.

The key insight: **scaffolding should fade at different rates for different task types.** The system may be Expert-stage for "add a new field to an existing struct" but Bootstrap-stage for "design a new subsystem." The operator's attention should track the system's per-task-type developmental stage, not a global stage.

### Session Continuity: Nothing Is Lost Between Runs

The developmental trajectory only works if learning state persists across sessions. Without persistence, every restart is a cold start — Plan 501 is no better than Plan 1. Roko persists:

| State | File | What It Preserves |
|---|---|---|
| Routing statistics | `.roko/learn/cascade-router.json` | Per-(role, task_type, complexity) model selection accuracy |
| Gate calibration | `.roko/learn/gate-thresholds.json` | EMA pass rates per rung, adaptive retry limits |
| Scaffold weights | `.roko/learn/section-effects.json` | Which prompt sections correlate with task success |
| Playbook rules | `.roko/learn/playbook.json` | Validated behavioral rules with confidence scores |
| Skill library | `.roko/learn/skills.json` | Reusable tool-use patterns extracted from episodes |
| Episode history | `.roko/episodes.jsonl` | Full agent turn recordings (append-only, max 90 days) |
| Efficiency metrics | `.roko/learn/efficiency.jsonl` | Per-turn cost, latency, token usage, pass rates |
| Codebase model | `.roko/learn/patterns.json` | HDC-clustered episode patterns for codebase familiarity |
| Developmental stage | Derived from above | Computed from accumulated evidence, not stored directly |

The developmental stage is not stored as a single number — it's **derived** from the accumulated evidence across all persisted state. This prevents gaming (you can't just set `stage = Expert`) and ensures the stage reflects actual capability.

---

## 13. Information Architecture: Signal Flow Through the Pipeline

Every agent system is a chain of communication channels. Information enters as a human-written PRD and exits as agent-written code. Between those endpoints, the signal passes through seven major boundaries, each of which transforms the information — compressing, expanding, filtering, or reformatting it. Every transformation is lossy. The cumulative loss across all seven boundaries determines whether the agent's output satisfies the original PRD.

### The Seven Boundaries

```
[PRD]                     Human intent, natural language
  ↓ Boundary 1: Decomposition
[Plan]                    Structured task graph with dependencies
  ↓ Boundary 2: Specification
[tasks.toml]              Machine-readable task descriptions with acceptance criteria
  ↓ Boundary 3: Enrichment
[Enriched Context]        Task + codebase context + playbook + history
  ↓ Boundary 4: Assembly
[Prompt]                  6-layer system prompt + user message
  ↓ Boundary 5: Inference
[Agent Reasoning]         LLM internal computation → tool calls + code
  ↓ Boundary 6: Verification
[Gate Verdict]            Pass/fail + structured error feedback
  ↓ Boundary 7: Iteration
[Feedback → Boundary 3]  Error digest feeds back into enrichment
```

### Boundary-by-Boundary Analysis

**Boundary 1: PRD to Plan (Decomposition)**

The human writes "Wire SystemPromptBuilder into orchestrate.rs" and the system must decompose this into a task graph with dependencies, file targets, and acceptance criteria.

Signal loss risk: Requirements omitted or misinterpreted. A PRD that says "wire the system prompt builder" doesn't specify which builder methods to call, which template to use, or how to handle errors. The plan generator must infer these — and inference introduces noise.

Mitigation: Research agent enriches the PRD with codebase analysis before plan generation. Human reviews the generated plan before execution. The research step adds signal that the PRD author assumed but didn't write.

**Boundary 2: Plan to tasks.toml (Specification)**

The plan's prose descriptions must be converted into machine-readable TOML with specific fields: `description`, `acceptance_criteria`, `files`, `dependencies`.

Signal loss risk: Acceptance criteria underspecified. A task description saying "implement the builder" doesn't tell the gate pipeline what to verify. Without explicit criteria, the gate can only check compilation and existing tests — necessary but not sufficient.

Mitigation: Generated verification artifacts. The enrichment pipeline (step 4) produces explicit verification tasks with concrete assertions: "the system prompt must contain exactly 6 layers" rather than "implement the builder."

**Boundary 3: tasks.toml to Enrichment (Context Selection)**

The enrichment pipeline must select which codebase context is relevant to this specific task. From potentially millions of lines of code, it must surface the 5-25K tokens that the agent actually needs.

Signal loss risk: Context irrelevant (wasted tokens on unrelated code) or missing (critical code not included, agent works blind). Both are expensive: irrelevant context wastes budget and dilutes attention; missing context causes incorrect implementations.

Mitigation: HDC fingerprinting maps task descriptions to relevant code sections via binary spatter code similarity. PageRank on the symbol graph surfaces high-connectivity modules. Together they produce targeted context with minimal waste.

**Boundary 4: Enrichment to Prompt (Assembly and Budget)**

The enriched context must be assembled into a prompt that fits within the token budget, with cache-aligned ordering and role-specific section weights.

Signal loss risk: Budget truncation drops critical information. When the enriched context exceeds the token budget, sections must be cut. If the cut section contained the one function signature the agent needed, the task fails.

Mitigation: Priority-based section ordering. The section bandit learns which sections correlate with success per (role, task_type) combination. High-priority sections survive truncation. Low-priority sections are cut first. The learning loop continuously improves section ordering.

**Boundary 5: Prompt to Agent Reasoning (Inference)**

The LLM must read the prompt, understand the task, and produce correct tool calls and code. This is the boundary with the least human control — the LLM's internal computation is opaque.

Signal loss risk: Lost-in-the-middle attention degradation (Liu et al., 2023). Information placed in the middle of long contexts receives less attention than information at the beginning or end. Critical context placed at position 15K in a 25K prompt may be effectively invisible.

Mitigation: U-shaped placement. Critical information (task description, acceptance criteria, error feedback) placed at the start and end of the prompt. Stable, less-critical information (conventions, workspace map) placed in the middle. Cache alignment naturally achieves this: Layer 1 (stable) at the start, Layer 6 (task-specific) at the end.

**Boundary 6: Agent Reasoning to Gate (Verification)**

The agent produces code. The gate pipeline verifies it. The gate verdict must accurately represent whether the code satisfies the task requirements.

Signal loss risk: False positives (gate passes but code is wrong) or false negatives (gate fails but code is correct). False positives are more dangerous — they produce incorrect code that enters the codebase.

Mitigation: 7-rung pipeline with escalating coverage. Rung 0 (compile) catches syntax errors. Rung 1 (test) catches behavioral errors. Rung 3 (generated tests) catches specification errors. Each rung catches a different failure mode. The pipeline's false positive rate is the product of individual rung false positive rates — multiplicative reduction.

**Boundary 7: Gate to Iteration (Feedback)**

When a gate fails, the error feedback must be clear enough for the agent to fix the problem on the next attempt. Vague or noisy feedback wastes retry budget.

Signal loss risk: Error feedback too noisy (full compiler output with irrelevant warnings) or too vague ("test failed" without specifying which assertion).

Mitigation: Structured error digest. A Haiku-tier reflection call extracts the specific error, the likely cause, and a suggested fix from the raw gate output. This compressed digest replaces the raw output in the retry prompt. The feedback loop closes: gate failure → structured digest → enrichment (Boundary 3) → prompt (Boundary 4) → agent (Boundary 5) → gate (Boundary 6).

### Shannon's Channel Capacity and the Context Window

**Research**: Shannon (1948) proved that every communication channel has a maximum throughput of meaningful information, determined by the channel's bandwidth and noise characteristics.

The LLM context window IS a communication channel. Its capacity is bounded by:
- **Bandwidth**: The token limit (e.g., 200K tokens for Claude)
- **Noise**: Irrelevant context that dilutes attention (the lost-in-the-middle effect)
- **Effective capacity**: Much less than the raw token limit because attention degradation reduces the information extraction rate for tokens in the middle of the context

The practical implication: doubling the context window does NOT double the effective information throughput. A 200K-token context window may have an effective capacity of ~30K tokens of high-fidelity information, with the remaining 170K tokens subject to progressively worse attention degradation.

This is why roko invests heavily in the enrichment pipeline (Boundary 3) and prompt assembly (Boundary 4). The goal is not to fill the context window — it's to maximize the information density of the tokens that DO receive attention.

### Signal Preservation Strategies

Every mitigation strategy maps to preserving signal at one or more boundaries:

| Strategy | Target Boundary | Mechanism |
|---|---|---|
| Research agent | 1 (PRD → Plan) | Adds context the PRD author assumed |
| Generated verification artifacts | 2 (Plan → tasks.toml) | Makes acceptance criteria explicit and machine-checkable |
| HDC fingerprinting + PageRank | 3 (tasks.toml → Enrichment) | Selects relevant context, reduces irrelevant noise |
| Section bandit | 4 (Enrichment → Prompt) | Learns which sections help per task type |
| Cache-aligned U-shaped placement | 5 (Prompt → Reasoning) | Maximizes attention on critical information |
| 7-rung gate pipeline | 6 (Reasoning → Gate) | Multiple independent verification layers |
| Structured error digest | 7 (Gate → Iteration) | Compresses noisy errors into actionable feedback |

### Cumulative Loss and the First-Pass Success Rate

If each boundary preserves 90% of the signal (a generous estimate), cumulative preservation across 7 boundaries is:

```
0.90^7 = 0.478 (47.8% of original signal survives)
```

This means that even with 90% preservation at each boundary, the agent receives less than half of the information in the original PRD. At 80% per boundary:

```
0.80^7 = 0.210 (21.0% of original signal survives)
```

This explains why naive agent systems (no enrichment, no cache alignment, no structured feedback) have low first-pass success rates. The signal simply doesn't survive the pipeline.

Roko's mitigation strategies aim to push per-boundary preservation above 95%:

```
0.95^7 = 0.698 (69.8% of original signal survives)
```

The difference between 21% and 70% signal preservation is the difference between a system that retries 4 times per task and one that succeeds on the first attempt. Every percentage point of per-boundary improvement compounds multiplicatively across all seven boundaries.

### Every Failure Is a Symptom

The most important insight from the information architecture perspective: **every failure, retry, and human intervention is a symptom of signal loss at one or more boundaries.** When debugging a failed task, the question is not "what went wrong?" but "where did the signal degrade?"

- Agent wrote incorrect code? Check Boundary 5 (prompt → reasoning). Was the relevant context included? Was it placed where the model would attend to it?
- Agent wrote code that doesn't match the task? Check Boundary 2-3 (tasks.toml → enrichment). Were the acceptance criteria specific enough? Was the relevant codebase context selected?
- Agent succeeded but the wrong task was specified? Check Boundary 1 (PRD → plan). Was the decomposition correct? Were requirements omitted?

The feedback loop (Boundary 7 → Boundary 3) exists precisely to correct signal loss. Each retry is an opportunity to add signal that was missing on the previous attempt. The structured error digest from the gate provides the specific information about WHAT was lost, enabling targeted correction rather than blind retry.
