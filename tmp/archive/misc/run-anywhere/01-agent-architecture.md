# Roko Agent Architecture: How It Works and Why It's Different

> **Audience**: Product thinking, market positioning, feature iteration, identifying novel use cases
> **Scope**: Updated to roko naming. Cites 60+ academic papers.

---

## The Core Pattern: 1 Noun + 6 Verbs

Roko agents operate on a single universal pattern. One data type (**Signal**) flows through six operations (**Substrate, Scorer, Gate, Router, Composer, Policy**). Every agent loop — from a one-shot coding task to a multi-plan autonomous build — is this same pattern.

```
Signal arrives (observation from environment)
  → Substrate stores it (persistent knowledge base)
  → Scorer evaluates it (quality, confidence, novelty, utility)
  → Gate validates it (compile, test, verify — external truth)
  → Router selects the model (cascade: static → confidence → bandit)
  → Composer assembles context (6-layer prompt with cache alignment)
  → Policy decides action (behavioral phase, budget, safety constraints)
  → Agent acts (tool calls: read, edit, bash, search)
  → Signal produced (result observation)
  → Loop
```

**Research grounding**: This maps to CoALA (Sumers et al., 2023) — the first formal cognitive architecture for language agents. CoALA identifies perception, memory, reasoning, action, and reflection as universal agent components. Roko's 6 traits are a systems-engineering realization of CoALA's cognitive modules.

**What makes it different**: Most agent frameworks (LangChain, CrewAI, AutoGen) are conversation-turn-oriented — agent receives message, calls LLM, returns response. Roko is **decision-cycle-oriented** — the agent ticks on a timer, observes, retrieves, analyzes, gates, executes, verifies, reflects, persists. 80% of ticks may have no human input. The heartbeat is the primitive, not the chat message.

---

## The 5-Layer Stack

Roko is built as a highly modular Rust workspace (comprising 18+ discrete crates), structured across five primary layers:

1. **Cognition**: The heartbeat, router, gate pipeline, affect engine, and conductor logic (`roko-core`, `roko-heartbeat`, `roko-inference`).
2. **Memory (neuro)**: The three-substrate persistence layer tracking episodic, semantic, and holographic facts (`roko-neuro`).
3. **Knowledge Services (Korai)**: The blockchain networking layer for cross-agent knowledge syncing and stigmergic coordination (`roko-chain`).
4. **Tools**: The capability-enforced dispatcher with MCP support and the TypeScript sidecar for DeFi math (`roko-tools`).
5. **Custody**: The execution safety layer supporting delegation and embedded wallet profiles (`roko-safety`, `roko-chain`).

---

## The Six Mechanisms

### 1. Signal: The Universal Data Type

Everything is a Signal — a git push, a test failure, a Slack reaction, a cost measurement, a model prediction. Signals carry:

- **ContentHash**: BLAKE3 256-bit identity (immutable, verifiable)
- **Kind**: What type (github:push, feedback:slack:reaction, agent:output). The system operates a central **Event Fabric** multiplexer that handles 50+ strongly typed `RokoEvent` variants across the DAG.
- **Body**: Payload (text, JSON, binary)
- **Score**: Multi-dimensional quality (confidence, novelty, utility, reputation)
- **Lineage**: Parent signal hashes (forms a DAG — who caused what)
- **Decay**: Time-decay function (signals lose relevance over time)
- **Provenance**: Author + trust level (tainted signals from untrusted sources tracked)

**Why this matters**: Every piece of data in the system — episodes, tool results, gate verdicts, external feedback — is the same type. This means the learning system, routing system, and gate system all operate on the same substrate. No impedance mismatch between subsystems.

**Research**: Content-addressed data structures (IPFS, Git) + decay functions from memory research (Ebbinghaus, 1885; Richards & Frankland, 2017).

---

### 2. Substrate: The Knowledge Base

Roko uses **three complementary memory substrates** — not one — because each handles something the others can't:

| Substrate | Technology | What It's For | Why Not Just One? |
|---|---|---|---|
| **Episodic** | LanceDB (768-dim embeddings) | Fast nearest-neighbor for specific past experiences | Can't do Boolean compositional queries |
| **Semantic** | SQLite (structured facts) | Schema-based stable knowledge, queryable | Can't do approximate similarity search |
| **Holographic** | HDC Binary Spatter Codes (10,240-bit) | Compositional queries, knowledge compression, controlled forgetting | Can't do structured SQL queries |

The HDC (Hyperdimensional Computing) layer is the most unusual. Binary Spatter Codes (Kanerva, 2009) enable:

- **Compositional queries**: "Find patterns where arousal was high AND complexity was architectural AND model was GLM" — expressed as XOR-bind algebra, not sequential filter scans
- **Knowledge compression**: An entire agent's learned knowledge (500+ patterns) compressed to a single 1,280-byte vector via majority-vote bundling
- **Controlled forgetting**: Vote decay per bit position provides smooth SNR degradation instead of cliff-edge entry deletion

**Research**: Complementary Learning Systems (McClelland, McNaughton, O'Reilly, 1995), HDC/VSA survey (Kleyko et al., 2022), spacing effect (Ebbinghaus, 1885; Cepeda et al., 2006).

**What's novel**: No other agent framework combines three memory substrates. Standard approaches (LangChain MemoryBuffer, Mem0, LangMem) use a single vector store. Roko's three-substrate design is grounded in neuroscience — the hippocampus (episodic), neocortex (semantic), and procedural memory are distinct systems in biological brains that work together, not alternatives.

---

### 3. Scorer: Multi-Dimensional Quality Assessment

Every signal gets a 4-dimensional quality score:

- **Confidence**: How certain is this? [0,1] — calibrated via isotonic regression against actual outcomes
- **Novelty**: How new is this information? [0,1] — based on distance from existing knowledge
- **Utility**: How useful is this for the current task? [0,1] — context-dependent
- **Reputation**: How trustworthy is the source? [0,1] — based on provenance chain

**Confidence calibration** is critical because LLMs are systematically overconfident (Xiong et al., 2023). A model stating "90% confidence" is actually correct ~60% of the time. Isotonic regression maps stated confidence to empirical accuracy per (model, task_category) pair.

**Research**: Expected Calibration Error (Guo et al., 2017), confidence calibration (Dabah et al., 2025), curiosity-driven exploration (Pathak et al., 2017).

---

### 4. Gate: External Truth, Not Self-Assessment

The gate pipeline is roko's most important subsystem. It provides **external verification** that the agent's output is correct — without relying on the LLM to evaluate its own work.

**11 gate types across 6 rungs** (escalating cost and coverage):

| Rung | Gate | What It Checks | Latency | LLM Required? |
|---|---|---|---|---|
| 0 | CompileGate | Code compiles | ~5s | No |
| 1 | TestGate | Existing tests pass | ~30s | No |
| 1 | ClippyGate | No lint violations | ~10s | No |
| 2 | SymbolGate | Expected exports exist | ~10ms | No |
| 3 | GeneratedTestGate | Agent-generated tests pass | ~30s | No |
| 4 | PropertyTestGate | Property-based invariants hold | ~60s | No |
| 5 | IntegrationGate | Cross-component scenarios pass | ~120s | No |
| 5 | DiffGate | Changes are minimal/focused | ~1s | No |
| 5 | VerifyChainGate | On-chain state matches expected | ~2s | No |
| - | LlmJudgeGate | Quality assessment for non-verifiable outputs | ~5s | Yes |

**Key principle**: Gates 0-5 use NO LLM — they are deterministic, external, unfoolable. The LlmJudgeGate is a last resort for outputs that can't be mechanically verified.

**Adaptive thresholds**: Each rung's pass rate is tracked via Exponential Moving Average. If rung 0 passes 20+ times consecutively, it's advisory-skipped. If rung 3 has a 30% pass rate, max retries increase. The system learns how strict to be per gate.

**Research**: Process reward models (Lightman et al., 2023) — verifying intermediate steps outperforms outcome-only verification. GVU Framework (2025) — "strengthen the verifier, not the generator." AlphaCode (Li et al., 2022) — 10 samples with strong verification > 1M samples with weak filtering.

**What's novel**: Most agent frameworks have no gates at all (LangChain, CrewAI). Those that do (SWE-agent) use only compile + test. Roko's 6-rung pipeline with adaptive thresholds and 11 gate types is the most comprehensive verification system in any open-source agent framework.

---

### 5. Router: Multi-Model Cascade with Learning

The CascadeRouter selects which model to use for each task. It transitions through three stages as it accumulates data:

| Stage | Observations | Strategy | Description |
|---|---|---|---|
| **Static** | 0-49 | Hardcoded table | Role → model mapping (haiku for fast, sonnet for standard, opus for premium) |
| **Confidence** | 50-199 | Wilson confidence intervals | Empirical pass rates with uncertainty bounds |
| **UCB** | 200+ | LinUCB contextual bandit | 17-dimensional feature vector with exploration decay |

**The 17-dimensional routing context** encodes: task category (8-dim one-hot), complexity (scalar), iteration count, role hash (4-dim), crate familiarity, prior failure flag, and bias term.

**Reward function**: `0.5 * pass_rate + 0.3 * (1 - normalized_cost) + 0.2 * (1 - normalized_latency)` — quality, cost, and speed balanced.

**Research**: LinUCB (Li et al., 2010) — contextual bandits for personalized recommendation. RouteLLM (ICLR 2025) — 85% cost reduction routing between strong/weak models. FrugalGPT (Stanford, 2024) — cascade architectures achieving 98% cost reduction.

**What's novel**: The three-stage cascade (static → confidence → UCB) is unique. RouteLLM and FrugalGPT use binary strong/weak routing. Roko routes across N models with a full contextual bandit that learns per (role, task_type, complexity) combination.

---

### 6. Composer: 6-Layer Prompt Assembly with Cache Alignment

The SystemPromptBuilder assembles prompts from six layers, ordered for maximum provider cache hit rates:

| Layer | Content | Cache Behavior |
|---|---|---|
| 1. Role Identity | "You are a Roko agent..." | Stable across ALL tasks (90% cache discount) |
| 2. Conventions | Coding standards, project structure | Stable across plan |
| 3. Tools | Tool definitions (Read, Edit, Bash, etc.) | Stable across role |
| 4. Domain | Workspace map, cross-plan context | Stable within plan |
| 5. Anti-patterns | "Don't do X" rules from playbook | Stable within session |
| 6. Task | Specific task TOML, review feedback, error output | Unique per turn |

**Cache layer ordering** is critical: Layer 1 content always appears first in the prompt, so the provider's KV cache can reuse it across all requests. Volatile content (Layer 6) always appears last, so cache prefix length is maximized.

**Cost impact**: With proper cache alignment and GLM-5.1's automatic caching, input tokens cost $0.26/M (cache hit) vs $1.40/M (cache miss) — 5.4x savings on every request after the first in a plan.

**Research**: Prompt caching economics (Anthropic, 2025 — 90% discount on cache reads). Lost in the Middle (Liu et al., 2023) — relevant information placement in context window matters as much as selection. DSPy (Khattab et al., 2024) — programmatic prompt optimization.

---

## The Learning Loops

### Five Machine-Speed Feedback Loops

Every agent turn produces five learning signals at near-zero cost:

| Loop | What It Measures | Metric | Cost |
|---|---|---|---|
| **Confidence Calibration** | LLM's stated confidence vs actual outcome | Expected Calibration Error | ~Zero |
| **Context Attribution** | Which prompt sections helped? | Section → pass rate lift | ~Zero |
| **Cost-Effectiveness** | Was expensive inference worth it? | Quality delta per dollar | ~Zero |
| **Tool Utilization** | Right tools selected? | Tools used / tools available | ~Zero |
| **Anomaly Detection** | Is the agent stuck or looping? | Prompt hash repetition, cost z-score | ~Zero |

### Knowledge Distillation Cascade

Raw execution data compresses through four tiers:

```
Tier 0: Raw Episodes (thousands per week)
  → Tier 1: Insights (distilled by analysis)
  → Tier 2: Heuristics (validated patterns)
  → Tier 3: Playbook Rules (proven behavioral rules injected into prompts)
```

Each tier is ~10x more applicable than the previous. A Tier 3 playbook rule like "always run `cargo check` before `cargo test` in this crate" is worth hundreds of raw episodes.

**Research**: Voyager (Wang et al., 2023) — skill library accumulation, 3.3x improvement. ERL (2026) — experiential reflective learning with single-attempt cross-task transfer. SAGE (2025) — skill-augmented RL, 26% fewer steps, 59% fewer tokens.

---

## The Orchestration Engine

### Plan DAG Executor

Roko doesn't execute tasks sequentially. It builds a **Directed Acyclic Graph** of all tasks across all plans, detects file conflicts, and parallelizes everything that can run concurrently:

```rust
fn next_runnable(completed, in_flight) -> Vec<Task> {
    let blocked_files = files_touched_by(in_flight);
    dag.tasks()
        .filter(|t| !completed.contains(t))
        .filter(|t| !in_flight.contains(t))
        .filter(|t| t.dependencies().all(|d| completed.contains(d)))
        .filter(|t| !t.files().any(|f| blocked_files.contains(f)))
        .collect()
}
```

**14-phase state machine** per task: Plan → Enrich → Strategize → Implement → Gate → Verify → Review → Scribe → Merge → Complete (with retry loops at each phase).

### Git Worktree Isolation

Each parallel agent gets a **physically separate git checkout** (not a branch in the same directory):

```
.worktrees/
  impl-wave-1/     ← Agent A's physical repo copy
  impl-wave-2/     ← Agent B's physical repo copy
  review-1/        ← Reviewer's physical copy
```

Agents can modify files freely without affecting each other. New code is visible within the worktree's search index but NOT in other agents' searches until merged.

**Cost**: ~100MB per worktree × 8 parallel agents = 800MB (acceptable). Shared `sccache` compilation cache achieves ~98% hit rate across worktrees.

### Merge Queue

When parallel agents finish, their branches must merge in dependency order:

1. **File-conflict detection**: Union-find partitioning by touched files → independent groups
2. **Dependency ordering**: Plans with cross-plan deps merge after their dependencies
3. **MergeResolver agent**: When git can't auto-merge, a specialized agent understands the intent of both changes and produces a semantically correct resolution
4. **Atomic merge via `git update-ref`**: No main-repo checkout required (avoids index locking)

### Crash Recovery

Executor state snapshots to `.roko/state/executor.json` after every phase transition. On restart with `--resume`:
- Completed tasks: skipped (marked done in snapshot)
- In-flight tasks: restarted (processes are dead, but work may be in worktree)
- Merge checkpoint: if crash during merge, `MergeCheckpoint` enables atomic rollback
- Worktrees preserved: never deleted (user may need them for inspection)
- Episode logs flushed: no data loss on turn-level recordings

### The Signal DAG

Every piece of data in the system is a `Signal` with content-addressed lineage:

```
Signal {
  id: ContentHash (BLAKE3),      ← Immutable identity
  kind: "github:push",           ← What type
  body: Body (text/json/binary), ← Payload
  lineage: [parent_hash_1, ...], ← WHO caused this signal (forms a DAG)
  score: Score {                  ← Multi-dimensional quality
    confidence, novelty, utility, reputation
  },
  decay: Decay::Exponential { half_life_ticks: 500 },
  provenance: Provenance { author: "agent-42", tainted: false },
}
```

The `lineage` field creates a DAG: `roko replay <hash>` walks the signal graph backward to understand causal chains. "Why did the agent make this decision?" → trace the signal lineage → find the originating observation.

**Research**: Content-addressed data structures (Git, IPFS). BLAKE3 for performance (3.3 GB/s on modern hardware).

**Research**: Graham's bound (1966) — greedy makespan ≤ W/P + D. HEFT (Topcuoglu et al., 2002) — priority scheduling by critical path. The UnifiedTaskDag infers cross-plan dependencies from file overlap — a novel contribution with no equivalent in published literature.

---

## The Safety Layer

### Defense-in-Depth (Three Architectural Layers)

| Layer | Mechanism | Can LLM Bypass? | What It Prevents |
|---|---|---|---|
| **Type System** | `Capability<T>` tokens — unforgeable, single-use, consumed by value | **No** — enforced by Rust compiler | Tool execution without authorization |
| **Smart Contract** | PolicyCage — on-chain caveats, spending limits, asset whitelists | **No** — enforced by EVM | Budget overruns, unauthorized trades |
| **Runtime** | Safety hooks, conductor watchers, loop guard, taint tracking | **Maybe** — depends on integrity | Stuck loops, cost spikes, prompt injection |

**Key insight**: The type system and smart contract layers are **outside the LLM's reach**. Even if the LLM is fully compromised by prompt injection, it cannot forge a `Capability<WriteTool>` token (Rust ownership prevents it) and cannot exceed the on-chain spending cap (the EVM reverts the transaction).

**Research**: Capability-based security (Dennis & Van Horn, 1966). Agent Behavioral Contracts (2026) — <10ms overhead, composable safety specifications. WASM Component Model (Haas et al., 2017) — sandboxed execution.

---

## The Enrichment Pipeline: 9 Steps from PRD to Execution-Ready

Before any agent executes a task, the context undergoes a 9-step enrichment pipeline that compresses 150K tokens of raw PRD/plan material into ~25K tokens of execution-ready context (83% reduction):

**Phase 1 (Sequential, Zero LLM Cost):**
1. PRD extraction (regex-based)
2. Brief generation (markdown parsing)
3. Task generation (TOML from headings)

**Phase 2 (Batchable via Batch API, 50% Discount):**
4. Verification tasks (Sonnet)
5. Review tasks (Sonnet)
6. Step-by-step decomposition (Sonnet)
7. Testing backlog (Sonnet)
8. Review rubric & invariants (Haiku)
9. Scribe task list (Sonnet)

**Output per plan**: 10 artifacts (plan.md, brief.md, tasks.toml, prd-extract.md, verify-tasks.toml, review-tasks.toml, decomposition.md, testing-backlog.md, rubric.md, scribe-tasks.toml).

**Cost**: ~$0.02-0.10 per plan enrichment. Typical 100-task build: $31.50 total vs $105 naive (67% savings before cache hits).

### Dynamic Prompt Budgeting by Role

Each role gets a different token budget distribution:

| Role | Plan | PRD | Workspace | Brief | Reviews | Code | Skills |
|---|---|---|---|---|---|---|---|
| Implementer | 25% | 20% | 10% | 10% | 10% | 10% | 5% |
| Strategist | 30% | 20% | 20% | 5% | 10% | 0% | 5% |
| Architect | 25% | 15% | 15% | 10% | 15% | 8% | 2% |

With 40-50% prefix cache hits, the remaining tokens cost ~81% less. The compound effect: 83% context reduction × 50% cache discount × 80% T0 suppression = **~230x total cost reduction**.

---

## 28 Specialized Agent Roles

Roko doesn't use a single general-purpose agent. It deploys **28 specialized roles**, each with:
- A specific backend (Claude CLI, OpenAI API, Cursor ACP)
- A specific model tier (Fast/Haiku, Standard/Sonnet, Premium/Opus)
- A specific tool permission set (read-only, write, exec, git, network)
- A specific budget ceiling ($0.10 for Conductor, $3.00 for Architect)

| Category | Roles | Default Backend | Tool Access |
|---|---|---|---|
| **Orchestration** | Conductor, Strategist | Claude CLI | Read-only |
| **Implementation** | Implementer, AutoFixer, Refactorer | Claude CLI | Read + Write + Exec |
| **Review** | Architect, Auditor, QuickReviewer, Scribe, Critic | Codex/Claude | Read + JSON Schema |
| **Testing** | IntegrationTester, CrossSystemTester, TerminalValidator, FullLoopValidator, RegressionDetector, PerformanceSentinel, CoverageTracker, SpecDriftDetector | Claude/Codex | Read + Exec |
| **Utility** | PrePlanner, ErrorDiagnoser, DependencyValidator, PatternExtractor, SnapshotComparator, MergeResolver, DocVerifier, PlanLifecycleManager | Various | Role-specific |
| **Research** | Researcher | Claude CLI | Read + Network |

**Research**: Principle of least privilege — each role gets exactly the tools it needs. Research shows that restricting tool availability improves accuracy AND reduces token usage (agents don't waste tokens considering tools they can't use).

---

## The PRD Lifecycle: From Idea to Merged Code

Roko implements a complete development lifecycle that agents can execute autonomously:

```bash
roko prd idea "Wire SystemPromptBuilder into orchestrate.rs"  # Capture work item
roko prd draft new "system-prompt-wiring"                      # Agent drafts PRD
roko research enhance-prd system-prompt-wiring                 # Research enriches PRD
roko prd plan system-prompt-wiring                             # Agent generates tasks.toml
roko plan run plans/                                           # Agents execute, gates verify
roko plan run plans/ --resume .roko/state/executor.json        # Resume if interrupted
```

Each step is a CLI command that exists today. The plan-execute-gate-persist loop is wired end-to-end. This is how roko develops itself — the 177K LOC codebase was substantially built using this pipeline.

---

## The Spectre: Creature as Interface

The agent's internal state is rendered as a **dot-cloud creature** (80 particles with spring physics). Not a dashboard — a living being:

### 32 Interpolating Variable Channels

Every frame, 32 variables lerp toward target values at different speeds:

**Fast channels** (~0.7s convergence, rate 0.035/frame at 60fps):
- PAD pleasure, arousal, dominance (eye glyph, cloud cohesion, posture)
- Plutchik emotion label + intensity (glyph selection, brightness)
- Surprise rate (eye micro-flicker)

**Medium channels** (~0.3s convergence):
- FSM phase (brightness pulse during DECIDING/ACTING)
- Inference tier (background radial glow: T0=none, T2=bright)
- Probe severity (jitter on all dots)
- Prediction accuracy (displacement clarity)
- Attention breadth (peripheral particle density)

**Slow channels** (~3.3s convergence):
- Mood P/A/D (ambient posture, shimmer, density)
- Economic vitality (dot count fading)
- Behavioral phase (overall creature degradation)
- Compounding momentum (glacial, rate 0.05/frame)

### Spring Physics

```
Each frame, every dot updates:
  1. Ambient orbit (per-dot elliptical, random phase)
  2. Emotion-driven expansion (joy=0.88, surprise=1.2, default=1.0)
  3. Sadness sink (cloud drifts down)
  4. Phi-driven cohesion (high Phi → tighter pull)
  5. Credit depletion (outer dots drift on low balance)
  6. Context compression (horizontal squeeze at 60% utilization)
  7. Epistemic loosening (cloud loosens as knowledge stales)
```

Spring constant: 0.04 default, reduced to 0.01 during dreaming (loose, drifting). Damping: 0.88 (dots overshoot and wobble naturally).

### Why a Creature, Not a Dashboard

**Research**: Reeves & Nass (1996) — The Media Equation. Systems rendered as social actors trigger anthropomorphic engagement. The Spectre communicates health, emotion, confidence, and danger **at a glance** — readable from across a room. No need to parse numbers.

When the agent is confused, the creature trembles. When confident, tight and bright. When dying, particles drift away. This is the **dual-reading principle**: glance (emotional state via brightness) + sustained reading (specific numbers in adjacent panels).

---

## The .roko/ Data Directory

All persistent state lives under `.roko/` in the project root:

```
.roko/
├── state/executor.json         # Crash-recovery checkpoint
├── episodes.jsonl              # Agent turn recordings (append-only, max 90 days)
├── signals.jsonl               # Signal DAG (content-addressed)
├── learn/
│   ├── efficiency.jsonl        # Per-turn metrics (20+ fields)
│   ├── cascade-router.json     # 3-stage model routing state
│   ├── gate-thresholds.json    # Adaptive EMA per gate rung
│   ├── experiments.json        # Prompt A/B experiment results
│   ├── model-experiments.json  # Model A/B experiment results
│   ├── costs.jsonl             # Cost records per model per provider
│   ├── playbook.json           # Validated behavioral rules
│   ├── skills.json             # Reusable tool-use patterns
│   ├── patterns.json           # HDC-clustered episode patterns
│   ├── provider-health.json    # Circuit breaker per provider
│   ├── latency-stats.json      # Per-model latency percentiles
│   ├── section-effects.json    # Prompt section effectiveness
│   └── routing.jsonl           # Routing decision log
├── prd/                        # PRD storage
├── research/                   # Research artifacts
└── memory/                     # Agent session persistence
```

---

## Why This Architecture Is Different

| Dimension | Standard Agent Frameworks | Roko |
|---|---|---|
| **Primitive** | Chat message | Decision cycle (heartbeat tick) |
| **Memory** | Single vector store | Three complementary substrates (episodic + semantic + holographic) |
| **Verification** | None, or compile-only | 11 gates across 6 rungs with adaptive thresholds |
| **Model selection** | Hardcoded or manual | 3-stage cascade with contextual bandit learning |
| **Prompt assembly** | String concatenation | 6-layer cache-aligned composition |
| **Learning** | None | 5 machine-speed feedback loops + knowledge distillation cascade |
| **Execution** | Sequential tasks | DAG with file-conflict-aware parallelization |
| **Safety** | Prompt-level guardrails | Compile-time capability tokens + on-chain policy enforcement |
| **Roles** | Single general agent | 28 specialized roles with least-privilege tool access |
| **Orchestration** | Human-initiated | Self-hosted: PRD → plan → execute → gate → learn → iterate |

**The compound effect**: Each mechanism reinforces the others. Better gates produce better learning signals. Better learning produces better model routing. Better routing produces cheaper execution. Cheaper execution allows more iterations. More iterations produce more learning data. The system accelerates itself.

---

## Academic Citations

| Paper | Year | How Roko Uses It |
|---|---|---|
| CoALA (Sumers et al.) | 2023 | Universal cognitive architecture — the 9-step heartbeat loop |
| HDC/VSA Survey (Kleyko et al.) | 2022 | Holographic memory substrate — compositional queries |
| Binary Spatter Codes (Kanerva) | 2009 | 10,240-bit vectors for knowledge compression |
| CLS Theory (McClelland et al.) | 1995 | Three-substrate memory design |
| Spacing Effect (Ebbinghaus) | 1885 | Knowledge decay and retrieval scheduling |
| Forgetting as Regularization (Richards & Frankland) | 2017 | Controlled forgetting prevents overfitting |
| Process Reward Models (Lightman et al.) | 2023 | Gate pipeline — step-level verification |
| GVU Framework | 2025 | "Strengthen the verifier, not the generator" |
| AlphaCode (Li et al.) | 2022 | Strong verification > many samples |
| LinUCB (Li et al.) | 2010 | Contextual bandit model routing |
| RouteLLM (Ong et al.) | 2025 | Cost-optimized model routing |
| FrugalGPT (Chen et al.) | 2024 | Cascade routing — 98% cost reduction |
| DSPy (Khattab et al.) | 2024 | Programmatic prompt optimization |
| Lost in the Middle (Liu et al.) | 2023 | Context placement matters |
| Somatic Markers (Damasio) | 1994 | Emotional signals for salience |
| Voyager (Wang et al.) | 2023 | Skill library accumulation |
| ERL | 2026 | Experiential reflective learning |
| SAGE | 2025 | Skill-augmented RL |
| Graham's Bound | 1966 | DAG scheduling optimality |
| Capability Security (Dennis & Van Horn) | 1966 | Type-system safety |
| Agent Behavioral Contracts | 2026 | Formal safety with drift detection |
| Confidence Calibration (Guo et al.) | 2017 | Isotonic regression for LLM confidence |
| Good Regulator (Conant & Ashby) | 1970 | Feedback loop design principle |
| Hayek (Price Signals) | 1945 | Distributed information aggregation |
| Event Sourcing (Fowler) | 2005 | State derived from event replay |

---

## The Substrate Layer (Layer -1)

The five-layer architecture (Runtime → Framework → Scaffold → Harness → Orchestration) sits on top of something it rarely acknowledges: the physical infrastructure. Hardware, operating system, network, and build tooling form an implicit Layer -1 that every higher layer depends on but none explicitly models. This invisibility is the problem.

### Hardware-Aware Compute Allocation

Agent execution is not a purely logical process. It runs on metal with specific constraints:

**CPU topology**: A 12-core machine running 8 parallel agents leaves 4 cores for the TUI, gate pipeline, and OS overhead. But cores are not equal — hyperthreaded pairs share execution units, and NUMA nodes have asymmetric memory access latencies. Scheduling agent processes onto the same NUMA node as their worktree's disk I/O path reduces memory latency by 20-40%.

**Memory hierarchy**: Each agent process consumes 200-500MB (Rust compiler + language server + file buffers). Eight parallel agents = 1.6-4GB. Add the TUI, gate compilation, and OS buffers and a 16GB machine is at 80%+ utilization. When memory pressure triggers swap, ALL agents slow down simultaneously because swap latency (10ms) is 1000x DRAM latency (10ns). The correct response is not to detect swap — it's to prevent it by budgeting agent count to available RAM.

**SSD IOPS**: Git worktree creation, cargo compilation, and file I/O all compete for disk bandwidth. A typical NVMe SSD sustains 500K random read IOPS, but cargo builds are sequential-write-heavy, and 8 parallel builds can saturate the write queue. The filesystem's journal and the SSD's garbage collection then compete, creating latency spikes that appear random from the application layer.

### OS-Level Resource Management

The operating system provides mechanisms to prevent substrate-level resource starvation:

**Process limits**: On Linux, cgroups v2 can isolate each agent process group with CPU, memory, and I/O bandwidth limits. On macOS, `setrlimit` provides per-process memory and file descriptor caps. Without these limits, a single runaway agent (stuck in a compilation loop) can starve all other agents of CPU.

**I/O scheduling**: Agent worktree operations, gate compilation, and TUI updates compete for disk I/O. Prioritizing gate compilation (it's on the critical path — agents block until gates complete) over TUI updates (cosmetic, can be delayed 100ms) reduces tail latency for the pipeline that matters.

**File descriptor management**: Each agent may hold open 50-100 file descriptors (source files, compiler processes, language server sockets). Eight agents = 400-800 descriptors. The default macOS `ulimit -n` is 256. Without explicit configuration, agent 3 or 4 fails with EMFILE and the operator sees a cryptic "too many open files" error from deep inside the Rust compiler.

### Network Topology

Every LLM inference call traverses the network. The substrate layer's network characteristics directly affect agent throughput:

**Round-trip time to inference providers**: A typical API call to Anthropic or OpenAI involves DNS resolution (~5ms, cached), TLS handshake (~30ms, resumed), HTTP/2 stream setup (~10ms), and the actual inference latency (500ms-30s depending on model and output length). The fixed overhead (45ms) is negligible for long inference calls but significant for T0/T1 calls where the total response time is 100-200ms.

**Bandwidth for streaming responses**: Streaming SSE responses at 50-100 tokens/second requires minimal bandwidth (~1KB/s) but does require a persistent connection per active agent. Eight agents = 8 persistent HTTPS connections. Behind a corporate proxy or on a congested WiFi network, connection multiplexing and keep-alive become critical.

**DNS/TLS overhead**: Cold starts (first request after a period of inactivity) incur full DNS + TLS costs. The provider health monitor tracks per-provider latency percentiles and pre-warms connections to avoid cold-start penalties on the critical path.

### The Invisibility Problem

The substrate is invisible to layers above — and that is its most dangerous property. When substrate failures occur, they manifest as symptoms in higher layers:

| Substrate Failure | Symptom in Higher Layers | Misdiagnosis |
|---|---|---|
| Disk pressure during compilation | Gate timeout, agent retry | "The gate is too slow" |
| CPU saturation from parallel agents | TUI freeze, missed heartbeats | "The TUI is broken" |
| API latency spike | All agents timeout simultaneously | "The provider is down" |
| Memory pressure triggering swap | Random agent slowdowns | "The agent is looping" |
| File descriptor exhaustion | Compiler crashes mid-gate | "The gate has a bug" |
| Build cache eviction | 30-second compile delays at every gate | "The gate pipeline needs optimization" |

Every one of these is a substrate problem with a substrate solution. But without substrate visibility, operators and the system itself chase symptoms in the wrong layer.

### sccache Multiplexing: Shared Compilation Cache

The single most impactful substrate optimization for roko's multi-worktree execution model is shared compilation caching via sccache.

**The problem**: Each git worktree is a physically separate checkout. Without a shared cache, each worktree compiles every dependency from scratch. The roko workspace has ~400 transitive dependencies. A clean build takes 3-5 minutes. Eight parallel worktrees compiling independently = 8 redundant builds of the same dependencies.

**The solution**: `sccache` with two configuration flags:

```bash
CARGO_INCREMENTAL=0    # Disable cargo's per-worktree incremental cache (conflicts with sccache)
SCCACHE_BASEDIRS=/Users/will/dev/nunchi/roko/roko  # Normalize paths so all worktrees share cache
```

`SCCACHE_BASEDIRS` is the critical setting. By normalizing the workspace root path, sccache recognizes that `/path/to/.worktrees/impl-wave-1/crates/roko-core/src/lib.rs` and `/path/to/.worktrees/impl-wave-2/crates/roko-core/src/lib.rs` are the same source file. The compilation result is cached once and shared across all worktrees.

**Result**: 98% cache hit rate across parallel worktrees. 20 worktrees share one cache layer instead of each duplicating 1GB+ of compiled artifacts. First worktree: 3-5 minute build. Subsequent worktrees: 10-15 second build (cache hits + linking). Gate compilation for unchanged dependencies: near-instant.

**The compound effect**: Faster gate compilation means faster feedback loops. Faster feedback loops mean fewer wasted agent tokens (agents don't sit idle waiting for gates). The substrate optimization cascades upward through every layer of the stack.

---

## The EventBus: Typed Observability

### How the System Sees Itself

A self-improving system must observe its own behavior. Roko's observability layer is built on a typed event bus that captures every significant action as a structured event with a monotonic sequence number.

### Event Bus Architecture

The EventBus is an in-process broadcast channel with four properties:

1. **Typed events**: Every event is a Rust enum variant with typed fields. `AgentSpawned { agent_id, role, model, worktree }` — not a stringly-typed log line. The compiler catches event schema changes.

2. **Monotonic sequence numbers**: Every event receives a u64 sequence number from an atomic counter. Sequence numbers are strictly increasing and never reused. This enables: total ordering of events (even across threads), gap detection (if sequence 5001 follows 4999, sequence 5000 was lost), and replay from any point.

3. **10,000-event ring buffer**: The bus retains the last 10,000 events in a lock-free ring buffer. This provides a rolling window for real-time queries without unbounded memory growth. Events older than the ring buffer are available only from persistent storage.

4. **Broadcast semantics**: Every subscriber receives every event. Subscribers are responsible for filtering to their events of interest. This is the Global Workspace Theory (Baars, 1988) implemented as infrastructure: all subsystems see all events, like consciousness broadcasting to all brain regions.

### Event Taxonomy

Every significant action publishes an event. The taxonomy covers the full agent lifecycle:

| Category | Events | Consumer |
|---|---|---|
| **Plan lifecycle** | PlanQueued, PlanStarted, PlanCompleted, PlanFailed | TUI, learning system |
| **Task lifecycle** | TaskStarted, TaskPhaseChanged, TaskCompleted, TaskRetried | Conductor, TUI |
| **Agent lifecycle** | AgentSpawned, AgentTurnStarted, AgentTurnCompleted, AgentTerminated | ProcessSupervisor, TUI |
| **Gate lifecycle** | GateExecuted, GatePassRateUpdated, GateThresholdAdapted | Learning system, conductor |
| **Routing** | ModelSelected, RoutingDecisionLogged, CascadeStageTransitioned | Learning system, cost tracker |
| **Conductor** | ConductorIntervention, WatcherTriggered, CircuitBreakerOpened | TUI, learning system |
| **Resource** | BudgetUpdated, CostRecorded, TokensConsumed | ResourceAccount, TUI |
| **Learning** | PlaybookRuleCreated, SkillExtracted, PatternClustered | TUI, conductor |

### Four Observability Layers

Events flow through four layers, each with different latency, durability, and query characteristics:

**Layer 1: Event Bus (in-process)**
- Latency: <1us (atomic write to ring buffer)
- Durability: Volatile (lost on crash)
- Query: Sequential scan of ring buffer
- Use: Real-time TUI updates, conductor monitoring

**Layer 2: Persistent Storage (JSONL files)**
- Latency: ~1ms (buffered file append)
- Durability: Durable (survives crash, fsync on phase transitions)
- Query: Sequential scan with grep/jq
- Use: Post-hoc analysis, efficiency tracking, learning feedback

**Layer 3: Streaming (TUI, WebSocket)**
- Latency: ~10ms (channel send + render)
- Durability: Ephemeral (connection-scoped)
- Query: Filter subscription (subscriber specifies event types)
- Use: Live dashboard, remote monitoring

**Layer 4: Distributed Tracing (OpenTelemetry)**
- Latency: ~100ms (batch export)
- Durability: External (stored in tracing backend)
- Query: Trace ID lookup, span search
- Use: Cross-agent correlation, performance profiling, production debugging

### Event Sourcing: State from Replay

The event bus enables a powerful architectural pattern: **event sourcing**. Instead of storing the current state and mutating it, store the sequence of events that produced the current state. The current state is derived by replaying the event log.

**Crash recovery**: When the executor restarts with `--resume`, it replays events from the persistent JSONL log to reconstruct the state of each plan, task, and agent. This is more robust than snapshot-based recovery because:
- Snapshots can be inconsistent (crash during write = corrupted snapshot)
- Event replay is idempotent (replaying the same event twice produces the same state)
- Partial replay is meaningful (replay to any point in time for debugging)

**Debugging**: "Why did the agent retry 3 times on task 7?" → filter events by task_id=7, read the sequence: `TaskStarted → AgentTurnCompleted → GateExecuted(fail) → TaskRetried → AgentTurnCompleted → GateExecuted(fail) → TaskRetried → AgentTurnCompleted → GateExecuted(pass) → TaskCompleted`. The full causal chain is visible in the event log.

**Learning**: The learning system subscribes to events and computes metrics in real time. `GateExecuted` events feed gate calibration. `ModelSelected` + `GateExecuted` events feed routing accuracy. `TokensConsumed` + `TaskCompleted` events feed efficiency metrics. All computed from the same event stream.

**Research**: Event sourcing (Fowler, 2005). CQRS (Command Query Responsibility Segregation) — separating write (event append) from read (state derivation) enables independent optimization. The EventBus is the write side; persistent storage and state derivation are the read side.
