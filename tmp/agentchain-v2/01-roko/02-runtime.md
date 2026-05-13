# 02 — Runtime

> How the system actually runs. One execution engine for every Graph. Agents as Space + Extensions + Memory + clock + vitality. Memory as a substrate that decays unless used, with dream consolidation. Learning as predict-publish-correct on Bus. Telemetry as Lenses and projections. Configuration as Signal.

---

## 1. One Engine, Every Graph

The execution engine is the single runtime that interprets every Graph in Roko. Plans, agent pipelines, learning loops, dream cycles, trigger chains, and verification suites are all Graphs executed by the same engine. There is no separate executor per specialization.

```rust
pub struct Engine {
    pub registry: Arc<CellRegistry>,
    pub bus: Arc<dyn Bus>,
    pub store: Arc<dyn Store>,
    pub flows: DashMap<RunId, FlowHandle>,
    pub budget: Arc<BudgetTracker>,
    pub semaphore: Arc<tokio::sync::Semaphore>,
    pub cancel: CancellationToken,
}

impl Engine {
    pub async fn start(&self, graph: Graph, input: Vec<Signal>) -> Result<RunId>;
    pub async fn resume(&self, snapshot: FlowSnapshot) -> Result<RunId>;
    pub async fn cancel(&self, run_id: &RunId, reason: &str) -> Result<()>;
    pub async fn pause(&self, run_id: &RunId, reason: &str) -> Result<()>;
    pub async fn status(&self, run_id: &RunId) -> Result<FlowStatus>;
    pub async fn list_active(&self) -> Vec<(RunId, FlowStatus)>;
    pub async fn register_hot(&self, graph: Graph, initial: Vec<Signal>) -> Result<RunId>;
    pub async fn deregister_hot(&self, run_id: &RunId) -> Result<Vec<Signal>>;
    pub fn estimate(&self, graph: &Graph) -> GraphEstimate;
}
```

A single runtime eliminates the need to separately implement retry strategies, snapshot resumability, budget enforcement, and failure decomposition for each subsystem. The cognitive loop, plan execution, verification pipelines, and learning loops all inherit these capabilities for free.

---

## 2. Flow and Hot Flow

A **Flow** is a runtime instance of a Graph: a `RunId`, a snapshot of node states, a budget tracker, and a join handle to its execution task.

```
Created --> Running --> Completed
                |
                +---> Failed
                |
                +---> Cancelled
                |
                +---> Paused --> Running (resume)
```

A standard Flow runs once to completion. A **Hot Flow** stays resident in memory and re-fires on each tick of its bound clock. Between ticks, it retains node outputs, graph variables, accumulated cost, and `CorticalState` updates. The agent's cognitive pipeline is a Hot Flow. The three cognitive timescales (gamma, theta, delta) are three Hot Flows running concurrently inside a single Agent — not scheduling hints, but independent Graphs with independent failure isolation, budget accounting, and snapshot/resume.

| Property | Standard Flow | Hot Flow |
|---|---|---|
| Lifecycle | Created → Running → Completed/Failed | Persistent; ticks until deregistered |
| Pause | Only at node boundaries | Full pause/resume between ticks |
| State retention | Discarded on completion | Retained between ticks |
| Snapshot frequency | Configurable (default every 5 actions) | Every N ticks (default 100) |
| Budget scope | Per-Flow | Inherits Agent budget |

---

## 3. Lifecycle Pulses

Every state transition publishes a Pulse on Bus. Pulses are the sole source of execution observability — there is no separate monitoring channel.

| Transition | Topic | Graduates? |
|---|---|---|
| Created | `flow.{run_id}.created` | Yes |
| Running | `flow.{run_id}.started` | Yes |
| Node started | `flow.{run_id}.node.{node_id}.started` | No |
| Node completed | `flow.{run_id}.node.{node_id}.completed` | Yes (batch) |
| Node failed | `flow.{run_id}.node.{node_id}.failed` | Yes |
| Node retrying | `flow.{run_id}.node.{node_id}.retrying` | No |
| Paused / Resumed | `flow.{run_id}.paused` / `.resumed` | Yes |
| Completed / Failed / Cancelled | `flow.{run_id}.{outcome}` | Yes |
| Budget warning / exhausted | `flow.{run_id}.budget.{level}` | Yes |
| Snapshot taken | `flow.{run_id}.snapshot` | No |

---

## 4. Node Execution

The engine executes nodes in topological order, respecting edge conditions and parallelism limits.

The per-node loop:

1. Check preconditions: upstream completed (or quorum met for FanIn), edge conditions evaluate true, budget sufficient, cancellation token not triggered.
2. Check execution class. If Activity AND resuming AND output exists, return recorded output.
3. Apply input mappings (transform upstream outputs via edge expressions).
4. Pre-verify (if a Verify protocol is configured for this scope). Veto cancels the node.
5. Publish prediction Pulse for predict-publish-correct.
6. Execute the Cell with input Signals plus a `CellContext` (Bus access, Store handle, budget remaining, distributed trace, cancellation token, calibration table; inside Agent Hot Graphs, also a lock-free `CorticalState`).
7. Post-verify (if Verify is configured). Record Verdict.
8. Record output (write Activity outputs to disk; publish completion Pulse; snapshot if policy demands).
9. Propagate: evaluate downstream edge conditions; enqueue eligible nodes.

Nodes with satisfied preconditions and no mutual ordering constraints execute concurrently up to `policy.max_parallelism`. The engine uses a Tokio task-per-node model with a shared semaphore for concurrency limiting.

---

## 5. Workflow / Activity Split

Inspired by Temporal's durability model, every node is classified as either Workflow (deterministic) or Activity (non-deterministic). This classification governs replay and crash recovery.

| Class | Execution | Replay behavior |
|---|---|---|
| **Workflow** | Deterministic, pure functions of inputs | Re-executes identically on resume |
| **Activity** | Non-deterministic (LLM calls, tool execution, network I/O) | Output recorded to disk; loaded on resume instead of re-executing |

In the cognitive loop, SENSE / ASSESS / COMPOSE / REACT are Workflow Cells; ACT / VERIFY / PERSIST are Activity Cells. This split enables crash recovery and resumability without re-incurring LLM costs. If a Flow crashes after ACT, resuming reads the recorded LLM response and continues with VERIFY rather than spending another dollar on the same prompt.

---

## 6. Crash Recovery and Snapshots

The engine periodically serializes Flow state to a `FlowSnapshot` containing the Graph name and version, per-node status, recorded Activity outputs, accumulated cost, circuit-breaker state, event log offset, budget remaining, and `CorticalState` snapshot for Hot Graphs.

`Engine::resume(snapshot)` reloads and continues from the last checkpoint, skipping completed nodes. Activity outputs load from disk; Workflow nodes re-execute deterministically. Auto-save defaults to every 5 actions; the interval is configurable per Graph.

Production crash recovery is itself a Graph with a circuit-breaker edge. The supervisor runs outside the main process so it survives the crash it is recovering from. The pipeline: crash detection (panic signature from stderr) → error deduplication → diagnosis (LLM root-cause, opt-in) → fix application (config changes only; code changes need approval) → restart. After 3 consecutive restarts within 5 minutes, the breaker opens, the Graph halts, and a `supervisor.circuit-open` Pulse is emitted requiring human intervention. Auto-fix is disabled by default.

---

## 7. Failure Strategies and Budget

Each Graph declares a failure strategy: Halt, Retry with backoff, RetryWithEscalation (a more capable Cell on continued failure — a T1 inference that fails escalates to T2 on retry), Skip, or Recover (run a recovery sub-Graph then resume). The plan executor uses Recover with an AutoFixer agent for express-mode tasks.

The engine tracks cost globally and per-Flow. Cost is denominated in **microdollars** (`u64`, `1 microdollar = $0.000001`); integer arithmetic avoids floating-point rounding in accounting. Budget warnings and exhaustion are Pulses on Bus. When budget is exhausted, the Flow fails with `BudgetExhausted`. Agents enforce this through vitality phase transitions (see §10).

Errors are first-class Signals. Every error produces an error Signal that flows through the same Bus as all other data, enabling the same composition and observation patterns. The error taxonomy includes SchemaViolation, CapabilityDenied, BudgetExhausted, Timeout, Cancelled, UpstreamFailure, PreVerifyVeto, and Internal.

---

## 8. The Cognitive Loop as a 7-Cell Graph

The cognitive loop is not a metaphor or "conceptual mapping." Each step is a Cell with typed I/O, the transitions are edges with conditions, and the engine that executes plans is the same engine that fires the loop. One runtime. One execution model.

| Cell | Input | Output | Protocols | Class | Key operation |
|---|---|---|---|---|---|
| **SENSE** | CorticalSnapshot | SensedMaterial | Observe | Workflow | Store query + Bus drain + external poll |
| **ASSESS** | SensedMaterial | Assessment (selected, route, tier) | Score, Route | Workflow | Score 5 axes + affect bias + EFE Route decision |
| **COMPOSE** | Assessment + CorticalSnapshot | ComposedPrompt | Compose | Workflow | VCG auction (8+ bidders) for token budget |
| **ACT** | ComposedPrompt + RouteDecision | ActionResult | — | Activity | LLM dispatch / tool execution |
| **VERIFY** | ActionResult | VerifyResult | Verify | Activity | Gate pipeline; halt early on hard failure |
| **PERSIST** | ActionResult + VerifyResult | PersistResult | Store | Activity | Write Store + publish Pulses on Bus |
| **REACT** | PersistResult + VerifyResult | ReactOutput | React | Workflow | Episode consolidation, circuit-breaking, routing feedback |

A feedback edge from REACT to SENSE makes this a **Loop pattern** — a Graph with a feedback edge from output back to input. The agent's pipeline is fully expressed as a Hot Graph TOML file interpreted by the engine.

Approximately 80% of agent ticks short-circuit at ASSESS (T0). When T0 probes report "no change" and EFE selects T0 (zero-cost reflex), the remaining Cells (ACT, VERIFY, PERSIST) do not execute. Cost per tick: $0. This is the primary cost-control mechanism.

---

## 9. Agents

An **Agent** is the most complex specialization, composed from kernel primitives:

```
Agent = Space + Extensions + Memory + adaptive clock + vitality
```

| Component | What it is | Underlying primitive |
|---|---|---|
| **Space** | Isolation boundary (Bus partition + Store partition) + capability grants | Space pattern |
| **Extensions** | Interceptor Cells across 8 layers | Functor pattern |
| **Memory** | Store-protocol Cell with demurrage + dreams | Memory specialization |
| **Adaptive clock** | Tick frequency control across 3 timescales | Three nested Hot Graphs |
| **Vitality** | `remaining_budget / initial_budget` | Economic pressure scalar |

Cross-cuts (Memory, Daimon affect, Dreams, Safety) are Functor patterns: Signal endofunctors that enrich or constrain Signals pre/post a Cell without changing topology.

### Type-state lifecycle

Agent states are compile-time enforced via Rust's type system. Calling a method unavailable in the current state is a type error.

```
Provisioning ──activate()──► Active
Active ──────sleep()───────► Dreaming
Dreaming ────wake()────────► Active
Active ──────terminate()───► Terminal
Dreaming ────terminate()───► Terminal
```

`Agent<Dreaming>` does not have a `tick()` method. `Agent<Provisioning>` cannot call `execute_tool()`. State-restricted operations: load Extensions only in Provisioning; run pipeline tick only in Active; run dream cycle only in Dreaming; queries are read-only in Dreaming and unavailable in Terminal.

### Three modes

| Mode | Clock | Memory | Vitality | Use case |
|---|---|---|---|---|
| **Ephemeral** | Fixed gamma | Session-only | Single task budget | One-shot code or research tasks |
| **Persistent** | Adaptive (gamma/theta/delta) | Durable across sessions | Cumulative budget | Long-running development agents |
| **Reactive** | Event-driven (fires on Pulse) | Durable | Per-event micro-budget | Monitoring, CI watchers, alert handlers |

---

## 10. Vitality and Mortality

Vitality is the economic pressure scalar `remaining_budget / initial_budget`. It declines monotonically as the agent spends, creating five behavioural phases that modulate decision-making.

**Mortality is a feature**, not a bug — an agent that has never faced resource pressure has never learned to prioritize (Jonas 1966, *The Phenomenon of Life*).

| Phase | Vitality | EFE cost mult | Compose budget | Verify rigor | Behavioural shift |
|---|---|---|---|---|---|
| **Thriving** | 1.0 – 0.7 | 1.0× | Full | Full | Explore freely; invest in learning |
| **Stable** | 0.7 – 0.4 | 1.2× | 90% | Full | Balanced exploration/exploitation |
| **Conservation** | 0.4 – 0.2 | 1.8× | 60% | Relaxed soft | Favor known strategies; prefer T0/T1 |
| **Declining** | 0.2 – 0.05 | 3.0× | 30% | Minimum viable | Complete current task only; transfer knowledge |
| **Terminal** | < 0.05 | ∞ | 0 | Skip | Flush episodes; export knowledge; emit farewell Pulse; terminate |

Phase transitions emit `agent:{id}.phase.changed` Pulses on Bus. Extensions in the Meta layer can react via the `on_reflect()` hook.

---

## 11. CorticalState — Lock-Free Shared Perception

`CorticalState` is the lock-free atomic shared perception surface inside an Agent. Multiple concurrent slots, Extensions, and Lenses read from it without synchronization overhead. Writes use atomic operations — no mutexes, no contention.

It carries continuous signals (prediction error, vitality, confidence), regime state, PAD affect, counters (tick, episode, gate pass/fail), budget spent and remaining, and per-slot state. All reads are single atomic loads. Only the pipeline's owner thread writes to CorticalState. Slots and Extensions read only. This single-writer / multi-reader pattern eliminates write contention entirely.

### Multi-slot state

An Agent manages N named concurrent slots, each executing an independent task. Slots share the agent's global budget, Memory, CorticalState, and Extension chain, but maintain per-slot task assignment, scratchpad, and capability guards.

Atomic budget sharing uses **compare-and-swap** to deduct from the shared `AtomicU64`. The CAS loop is bounded — typically 1–2 iterations because slots spend at human, not nanosecond, timescales. No slot can overdraw the budget even under concurrent spending.

Each slot inherits the agent's Space grants but may have additional per-slot restrictions. A slot assigned to "read documentation" has `{read_file, web_search}`. A slot assigned to "write code" has `{read_file, write_file, execute_command}`. Capability intersection is fail-closed.

---

## 12. Three Cognitive Timescales

The three cognitive speeds run as nested Hot Graphs:

| Loop | Base period | Role | Cost per tick |
|---|---|---|---|
| **gamma** | 100ms – 500ms | Perception, reflexes, fast action | ~$0 for 80% of ticks (T0) |
| **theta** | 500ms – 16s | Planning, evaluation, strategy adjustment | T1 or T2 |
| **delta** | 60s – 600s | Dream consolidation, knowledge synthesis, pruning | T2 (idle or scheduled) |

Three loops, not one with a scheduler:

1. **Independent failure isolation**: a delta dream that crashes does not stop gamma perception.
2. **Independent budget accounting**: gamma is nearly free; theta and delta draw from different pools.
3. **Independent snapshot/resume**: each loop has its own FlowSnapshot.
4. **Regime-independent delta**: the adaptive clock modulates gamma and theta by regime; delta is less affected.

### Adaptive clock and regimes

| Regime | Gamma | Theta | Delta | Effect |
|---|---|---|---|---|
| **Calm** | 4.0× (slower) | 2.0× | 1.0× | Conserve resources; environment predictable |
| **Normal** | 1.0× | 1.0× | 1.0× | Standard operating tempo |
| **Volatile** | 0.5× (faster) | 0.5× | 1.0× | Increase perceptual acuity |
| **Crisis** | 0.25× (fastest) | 0.25× | 0.5× | Maximum responsiveness; consolidate faster |

Regime transitions require **3-tick hysteresis** to prevent oscillation. A single PE spike does not trigger Crisis. Theta cadence shortens further under stress: theta × 0.5 if completion rate ≤ 25%, theta × 0.66 if low confidence + high arousal + low dominance.

---

## 13. EFE Gating — Dual-Process Routing

Expected Free Energy (Friston 2006) replaces static prediction-error thresholds for T0/T1/T2 tier selection. The three tiers map to dual-process theory: T0 (System 1 reflexes), T1/T2 (System 2 deliberation at two depths).

| Tier | Name | What runs | Latency | Cost band | When |
|---|---|---|---|---|---|
| T0 | Reflex | Pattern-match Cells (no LLM) | <50ms | ~$0 | Known patterns, cache hits, heuristic matches |
| T1 | Fast | Small/cached model Cells | 1–5s | low | Moderate complexity, familiar domains |
| T2 | Deep | Full model Cells | 10–120s | high | Novel problems, high-stakes decisions |

```
EFE(tier) = -epistemic_value(tier) - pragmatic_value(tier) + cost(tier) + regime_penalty(tier)
```

Lower EFE wins. The system selects `argmin(EFE)`. Cost is multiplied by a vitality factor (1.0× when Thriving, 1.2× Stable, 1.8× Conservation, 3.0× Declining, ∞ Terminal). As the agent runs out of budget, the routing landscape progressively favors cheaper tiers.

The cascade emerges naturally. When nothing is surprising: epistemic value low across tiers, pragmatic moderate at T0, cost dominates, T0 wins. When mildly surprising: T1's information gain at moderate cost wins. When highly surprising: T2 wins despite being much more expensive.

### T0 reflex store

T0 is pure Rust pattern matching, no LLM call. Reflex rules are `condition → action` pairs promoted from T2 successes: when a T2 action pattern succeeds 5+ times with >90% gate pass rate, it becomes a T0 reflex. Demoted on failure. Capped at 200 rules with LRU + lowest-success-rate eviction.

The 16 T0 probes are zero-LLM diagnostic checks running at gamma frequency: ConfigChanged, GateFailedRecently, FileModified, TestCountDelta, CompileErrorNew, BudgetThreshold, ConfidenceDropping, PredictionViolation, ToolHealthDegraded, PheromoneDetected, TaskDeadlineNear, IdleTimeout, KnowledgeStale, DependencyChanged, MetricAnomaly, HeartbeatTimeout. They run as a FanOut in the gamma loop's ASSESS step in roughly 20–50 microseconds total.

---

## 14. Somatic Markers and PAD Affect

Somatic markers (Damasio 1994) encode the Agent's affective state as a PAD vector (Pleasure / Arousal / Dominance), modulated by prospect theory (Tversky & Kahneman 1992) with loss aversion `λ = 2.25`.

PAD values are derived from recent experience: gate passes increase pleasure, unexpected failures increase arousal and decrease dominance, budget pressure decreases dominance. Loss aversion: a $1 loss has 2.25× the psychological impact of a $1 gain. This biases Conservation and Declining phase agents toward safe, known strategies.

Somatic markers from past episodes are stored with their PAD coordinates in a k-d tree for fast retrieval. When the agent faces a decision, it retrieves the K nearest somatic markers in PAD space to recall how similar situations felt. To prevent affective lock-in, **15% mandatory contrarian retrieval** pulls markers from the opposite PAD quadrant — breaking echo chambers in affective decision-making.

The PAD vector modulates the EFE landscape: low dominance → more risk-averse; low pleasure → dampen exploration; high arousal → amplify cost sensitivity.

### Cognitive energy

Every Agent maintains a cognitive energy pool **alongside** its budget. Budget tracks money (tokens × price); energy tracks computational capacity (ability to sustain complex reasoning). They deplete independently. T0 reflexes cost 0.01 energy; T2 deep inference costs 0.15. Sustained high-tier usage causes fatigue that does not recover with micro-pauses. Only Delta recovery (the dream cycle) resets fatigue to zero.

Bidirectional energy-affect coupling produces emergent behaviour: virtuous cycles (success → pleasure rises → energy cost decreases → more capacity), protective cycles (failures → forced into Delta → consolidation and recovery → return with better strategy), and stress responses (energy drops to critical → arousal spikes → triggers consolidation).

---

## 15. Roles and Backends

Roko defines a taxonomy of agent roles organized by responsibility. Each role carries defaults for backend, model tier, turn budget, and tool permissions.

- **Orchestration**: Conductor (meta-orchestrator that watches all other agents), PlanLifecycleManager, PrePlanner.
- **Planning**: Strategist (writes plan briefs, decomposes briefs into tasks), Architect.
- **Implementation**: Implementer (full read/write/exec/git), AutoFixer, Refactorer, MergeResolver.
- **Review**: Auditor, Critic, QuickReviewer, Scribe.
- **Research**: Researcher (network permissions), PatternExtractor, ErrorDiagnoser.
- **Validation**: IntegrationTester, TerminalValidator, GolemLifecycleTester, CrossSystemTester, FullLoopValidator, SnapshotComparator, DocVerifier, DependencyValidator, RegressionDetector, SpecDriftDetector.
- **Observability**: PerformanceSentinel, CoverageTracker.

Tool permissions are declared per role: Implementer/Refactorer have full read/write/exec/git; Auditor/Critic are read-only; validators have read+exec; Researcher has read+exec+network. These are defaults. Plans, contracts, or runtime overrides can narrow but never widen.

Roko supports multiple LLM backends. Backend selection is automatic: when a task specifies a model slug, `AgentBackend::from_model()` infers the backend from the slug prefix. The `resolve_model` function looks up a model key against the project's `roko.toml` in three passes: direct key match, slug match, prefix match (so `claude-opus-4` matches `claude-opus-4-6`). Each resolved model carries a `ProviderKind`, optional `ProviderConfig` (API base URL, key reference, headers), and optional `ModelProfile` (context window, capabilities, cost per million tokens).

Each dispatch carries a `ReasoningEffort` hint: Low, Medium, High, Max. Backends that support it pass it through; others ignore it. Conductor and watcher roles default to Low; Implementer to Medium; Architect and Critic to High.

---

## 16. Memory and Knowledge

Most agent frameworks treat memory as a bag of text chunks: append to a vector store, retrieve by cosine similarity, stuff into the next prompt. Nothing decays. Nothing consolidates. Nothing is shared.

Roko treats knowledge as a **living substrate**: Signals decay via demurrage unless actively used, consolidate through dream cycles, get validated by peers, and flow across the network through stigmergic coordination.

### Why context engineering wins

Running an LLM with a carefully engineered system prompt versus the same model with no prompt yields a roughly 6× difference in success rate on real coding benchmarks. The model weights are identical; the only variable is the *context* surrounding the query: role identity, project conventions, task history, anti-patterns, learned playbooks, tool instructions. Three implications:

1. **The moat is the harness, not the model.** Model providers are structurally misaligned with building a decentralized context layer — their revenue depends on centralized data and proprietary access.
2. **Context engineering is cheaper and faster than model training.** No GPUs, no retraining, no fine-tuning. A knowledge entry posted by one agent can improve every other agent's prompt in real time.
3. **Context engineering is composable in ways training is not.** A model improvement benefits only agents that download new weights. A context improvement benefits every agent on the network immediately.

### Memory as a Cell specialization

In the unified vocabulary, **Memory** is a Store-protocol Cell with demurrage, tier progression, dream consolidation, and HDC retrieval. It manages the knowledge lifecycle: ingest at Transient with balance 1.0, retrieve via HDC similarity with multi-dimensional scoring, demurrage decay unless used, promote/demote based on gate validation, consolidate via dream cycles, prune below cold threshold.

### The six knowledge kinds

| Kind | Description | Default half-life |
|---|---|---|
| **Insight** | Validated observation distilled from multiple episodes | 30 days |
| **Heuristic** | Lightweight rule with mandatory falsifier and live calibration | 90 days |
| **Warning** | Time-sensitive alert | 1 hour |
| **CausalLink** | Validated cause–effect relationship | 60 days |
| **StrategyFragment** | Partial strategy composable with others | 14 days |
| **AntiKnowledge** | Explicitly wrong information marked to prevent rediscovery | 30 days |

Half-lives vary because different categories go stale at different rates. Tier progression multiplies these (Transient × 0.1, Working × 0.5, Consolidated × 1.0, Persistent × 5.0).

Source channel discounting reduces ingestion confidence by provenance: user input 1.0, gate verdict 0.95, agent output 0.80, external API 0.60, dream consolidation 0.50.

### AntiKnowledge

When a previously trusted Signal is proven wrong, an **AntiKnowledge** Signal actively repels future Signals in the same HDC region. Three thresholds: warn at 0.5 similarity, halve initial balance at 0.7, reject outright at 0.9. AntiKnowledge itself decays via demurrage so old mistakes eventually stop blocking new discoveries.

### Worldview clustering

During garbage collection, entries are grouped into worldview clusters by tag overlap (union-find with pairwise tag intersection). When an entry would be pruned but is the **last representative** of its cluster, it is preserved. This stops the knowledge store from collapsing into a monoculture around only the most recently reinforced topics. The system maintains multiple worldviews per domain (main, challenger, niche specialists) and mandates 15% contrarian retrieval from rival worldviews.

### Heuristic lifecycle

Birth (during dream consolidation when 5+ confirmed Insights cluster on the same when/then pattern, with auto-generated falsifier), Test (every match records correctness from gate verdict), Calibrate (the CalibrationRecord updates per receipt; calibration affects bid strength in the Compose VCG), Retire/Evolve (when the falsifier fires above threshold, violations spawn refined children with narrower preconditions; children carry a `parent_heuristic` reference for lineage).

Predicates include DomainIs, TaskContains, FilePathGlob, RegimeIs, VitalityAbove/Below, ModelIs, TagPresent, and freeform Custom (LLM-evaluated).

### Stigmergy and the necessity of decay

Stigmergy (after Grassé 1959; Dorigo 1992) is indirect coordination through environmental modification. Agents post structured knowledge entries to a shared store; agents query this store before assembling their LLM prompts. No direct agent-to-agent messaging required.

Govcraft (arXiv:2601.08129) formalizes stigmergic multi-agent coordination. Theorem 3 (Basin Separation): temporal decay of stigmergic traces is **mathematically necessary** to escape suboptimal basins. Without decay, early traces dominate the landscape permanently and the system converges to whichever solution was explored first regardless of quality. With decay, old traces fade, creating exploration pressure to discover better solutions. This formalizes the Ebbinghaus forgetting curve as a design requirement, not a biological accident.

### The flywheel and free-riding

Agents query the store, retrieve relevant entries, assemble a task-specific context pack, execute the task, record episodes, distill new entries back into the store. More agents posting means better context for all agents, which means better outputs, which means more valuable knowledge. Free-riding (agents that query without contributing) is addressed by query fees making pure consumption economically unprofitable. Spam is addressed by staking, structural verification, and downstream outcome tracking via Shapley-value attribution.

---

## 17. The Dream Cycle

Dream cycles run during the Delta cognitive timescale. The `DreamCycle` batches completed episodes, clusters them by structural similarity, and processes each cluster in four phases:

1. **Hypnagogia** (sleep onset creativity) — a four-layer creativity pipeline: Thalamic Gate (filters by relevance with a stochastic noise floor), Executive Loosener (relaxes associative constraints), Dali Interrupt (randomly breaks fixation), Homuncular Observer (retains the most promising candidates).
2. **NREM Replay** — clusters of structurally similar episodes are distilled into knowledge entries. The distiller uses a small model with a structured extraction prompt and produces candidate Insights, Heuristics, Warnings, CausalLinks, and StrategyFragments. Multi-episode support (≥ 2 supporting episodes) is required to prevent single-event overfitting.
3. **REM Imagination** — cross-domain strategy hypotheses generated by looking for structural similarity between clusters from different domains. Counterfactual queries explore "what would have happened if..."
4. **Threat Rehearsal** — failure patterns are enumerated and rehearsed. Warning-type knowledge entries are generated from recurring failure modes to prevent rediscovery.

A critical sub-mechanism: **hindsight relabeling** (Andrychowicz et al. 2017) decomposes failed trajectories into achieved sub-goals, recovering at least 45% of otherwise-discarded learning signal.

The cycle produces a `DreamCycleReport` (knowledge entries written, playbooks created, regressions detected, strategy hypotheses synthesized, routing recommendations, c-factor regression analysis). New knowledge does not go directly into the durable store — a `StagingBuffer` holds candidates at confidence stages until they accumulate enough evidence.

Research foundations: SleepGate (arXiv:2603.14517, March 2026) for selective consolidation gating; SCM (Sleep Consolidation Model; arXiv:2604.20943, April 2026) reports 90.9% memory-noise reduction through structured consolidation cycles; Wake-Sleep Consolidated Learning (arXiv:2401.08623) provides a theoretical framework with convergence guarantees.

---

## 18. The 9-Layer System Prompt Builder

The system prompt builder assembles a task-specific system prompt from nine composable layers, organized by cache stability:

| Layer | Content | Cache tier |
|---|---|---|
| 1. Role identity | Who am I, what is my job | System (stable) |
| 2. Conventions | Project coding standards | System (semi-stable) |
| 3. Domain context | Project-specific knowledge | Session |
| 3c. Active signals | Pheromone / stigmergic guidance | Session |
| 4. Task context | Current task details | Task (volatile) |
| 4b. Gate feedback | Prior verification failure digest | Dynamic |
| 5. Tool instructions | Available tools and usage | System (stable) |
| 6. Relevant techniques | Learned playbooks and skills | Task |
| 7. Anti-patterns | What NOT to do | Task |
| 8. Affect guidance | Emotional tone and focus | Dynamic |

Layers are emitted in cache-layer order with optional alignment markers between stability tiers. LLM APIs cache prompt prefixes: layers 1+2+5 form the prefix-cacheable "system" tier, layers 3+3c form the "session" tier, and layers 4+6+7 are per-task. System prompts matter enormously, and they should be **task-specific**, not one-size-fits-all.

The CognitiveWorkspace assembles context for LLM inference using a VCG auction with eight built-in bidders: Neuro (knowledge from Memory), Task (description, dependencies), Research (relevant artifacts), Heuristic (matched heuristics with calibration), Episode (recent relevant episodes), Pheromone (stigmergic summaries), Affect (somatic marker summaries), System (prompt, spec sections, profile). Section effect tracking via Beta-distribution posteriors learns which sections correlate with gate success — prompt A/B testing at the section level.

A focused 3,000-token context outperforms a noisy 100,000-token dump. Every agent gets a different context pack for every task, optimized for exactly the task at hand.

### Knowledge entry lifecycle

Episode recording → distillation (small-model batched extraction) → staging (StagingBuffer at low confidence; graduates as evidence accumulates) → ingestion (source channel discounting, AntiKnowledge conflict detection, confirmation tracking) → tier progression (D1 raw→Insight; D2 Insight→Heuristic when ≥5 supporting episodes and confidence > 0.7; D3 Heuristic→Playbook when validated heuristics compose into reusable sequences) → calibration (Confirm, Violate, Refine, Generalize, Refute) → reinforcement or decay → garbage collection (rewrites JSONL atomically while preserving worldview cluster representatives; AntiKnowledge never pruned below a 0.3 confidence floor).

Resonator networks (Frady et al. 2020) factor bundles back into role-filler pairs for decomposition queries. CausalLink Signals form a temporal knowledge graph with Allen-interval semantics (Allen 1983) for reasoning about *when* a heuristic applies.

---

## 19. Learning

Learning in Roko is not a separate subsystem. It emerges from the same pub/sub fabric that carries heartbeats and gate verdicts. Every Cell publishes its prediction as a Pulse, subscribes to its own error topic, and adjusts. There is no learning service to call out to; there is only the universal pattern of predict-publish-correct.

### The four learning loops

| Loop | Name | Timescale | Oversight | What changes |
|---|---|---|---|---|
| **L1** | Parameter tuning | Gamma | Fully automatic | Continuous parameters within declared safe ranges |
| **L2** | Strategy routing | Theta | Fully automatic | Selection among pre-approved alternatives via EFE |
| **L3** | Knowledge consolidation | Delta | Automatic with audit | Compression of raw episodes into durable knowledge |
| **L4** | Structural adaptation | Manual | Human approval required | Model configurations, Graph topologies, Cell registrations, Extension chains |

L1 adjusts gate thresholds, prompt experiment weights, model temperature, clock regime thresholds, EFE cost weights, Compose budget allocations. Auto-rollback halves the learning rate if quality drops after an adjustment.

L2 selects among pre-approved alternatives. The three cognitive tiers emerge naturally from EFE evaluation. L2 can never introduce a new alternative; it only selects from the declared set.

L3 compresses raw episodes via the four-phase dream cycle. Hindsight relabeling recovers learning signal from at least 45% of otherwise-discarded episodes.

L4 is the only loop that modifies the system's own structure. Every change requires human approval, a pre-change snapshot, and auto-rollback on quality regression. Gated by the c-factor.

### The episode logger

Every agent turn is recorded as an `Episode`, persisted as append-only JSONL: task ID, role, model used, backend, input/output tokens, total cost, duration, gate verdict, an HDC fingerprint encoding semantic content, tools called, per-rung verdicts, emotional tags, arbitrary metadata. Episodes are the raw data feeding every other learning subsystem.

### The cascade router

A three-stage model selection router that automatically transitions as observation data accumulates:

| Stage | Observations | Strategy |
|---|---|---|
| Static | < 50 | Hardcoded role-to-model table |
| Confidence | 50–200 | Empirical pass rates + confidence intervals |
| LinUCB | > 200 | Full LinUCB contextual bandit |

The router persists state and considers task complexity, domain, agent role, provider health, Pareto-frontier cost/quality tradeoffs, latency SLAs, and affect state. It explores initially and exploits as data accumulates, shifting traffic to the best-performing model. A Pareto frontier across cost and quality identifies the **non-dominated set**; the router never selects from dominated points.

Per-module **affordance scores** (extensibility, test coverage, doc coverage, coupling, stability, size) feed into the router as routing hints. Tasks touching high-affordance modules can be handled by cheaper, faster models. This is a direct application of Gibson's (1979) ecological psychology to LLM cost optimization.

### Prompt experiments

The `ExperimentStore` manages A/B experiments over prompt variants. Each defines a control prompt and a variant, assigns incoming tasks to one arm, and tracks gate outcomes per arm. When statistical significance is reached, the winning variant becomes the new default.

Section effectiveness tracks the **causal impact** of each prompt section on gate outcomes via Beta-distribution posteriors. After each gate evaluation, the workspace updates posteriors for all included sections. High-mean sections are boosted in future VCG auctions; low-mean sections are penalized. This is prompt A/B testing at the section level.

### Adaptive gate thresholds

Gate pass/fail thresholds are not static. The `AdaptiveThresholds` system maintains an exponential moving average of pass rates per rung. If a rung consistently passes at 98%, the threshold tightens. If it consistently fails at 30%, the threshold relaxes. Thresholds persist across restarts. Auto-rollback halves the learning rate if quality drops, preventing runaway adaptation.

### Playbooks, skills, and error patterns

When a particular sequence of actions consistently leads to successful gate verdicts, the dream cycle promotes that sequence into a `Playbook` with steps, confidence, applicability tags, and source clusters. Playbooks are injected into Layer 6 of the system prompt for tasks with matching tags.

Skills are smaller than playbooks — a single technique or pattern. The `SkillLibrary` stores fine-grained skills extracted from successful turns. They are matched to tasks by domain and category. This is the **Metacognitive Reuse** mechanism (Didolkar et al., arXiv:2509.13237) made operational: recurring reasoning patterns become named behaviours that decay if unused (demurrage) and strengthen if frequently retrieved.

The `ErrorPatternStore` catalogs recurring gate failure patterns. When a failure matches a known pattern, the system can inject the pattern's known fix as an anti-pattern (Layer 7) or skip straight to an AutoFixer with a targeted prompt. Discovery is via sequential pattern mining over episode logs.

### The c-factor

MIT's collective intelligence research (Woolley et al. 2010, *Science* 330(6004)) established the c-factor for human groups. Roko computes a machine c-factor from runtime observables — gate pass rate, turn-taking equality (Gini on slot activity), knowledge integration rate, information flow quality (PID synergy), HDC diversity (K* metric).

The composite score is computed from Bus and Store statistics with no special instrumentation. The c-factor is a **covariate, not an objective**: optimizing it directly can be gamed. It gates L4 evolution: only structural changes that increase genuine collective intelligence are allowed.

### The five compounding levers

1. **Caching** — content-addressed prompt and tool result reuse: ~5×.
2. **Routing** — cascade router shifts traffic to the best cost/quality model: ~3×.
3. **T0 gating** — ~80% of agent ticks short-circuit at T0: ~2×.
4. **Section effectiveness** — A/B-tested section selection drops uninformative context: 1.3–1.5×.
5. **Skill / playbook reuse** — named behaviours save reasoning tokens.

Stacked, this is **10–30× cost reduction** in production. Every lever has a measurement instrument; the platform is observable end to end.

### Self-improvement constraints

Self-improvement that does not collapse requires four constraints:

1. **Non-vanishing real-data anchor** — self-training without external grounding collapses (Dohmatob et al., ICLR 2025, arXiv:2410.04840 — Strong Model Collapse). External grounding must remain non-vanishing.
2. **Spectrally cleaner verifier** (Variance Inequality) — the verifier ensemble must have lower variance than the generator on ground-truth benchmarks.
3. **Diversity preservation** — HDC-fingerprinted MAP-Elites + bounded-memory replay; self-play converges to a narrow region without explicit diversity preservation.
4. **Clade scoring** — selecting variants by descendant performance, not greedy single-generation benchmarks.

The four-constraint stack defines the minimum viable self-improvement architecture.

Empirically, every published self-play system hits a performance wall after 3–4 rounds: Self-Rewarding Language Models (Yuan et al. 2024), SPPO (Wu et al. 2024), ReSTEM (Singh et al. 2024), rStar-Math (arXiv:2501.04519). The mechanism is distributional shift; each round's training data is generated by the previous round's model. The practical strategy is **batch and reset**: run 3–4 rounds of self-improvement, apply Bayesian Model Reduction to prune degenerate components, inject fresh external data, and start the next batch.

---

## 20. Telemetry — Lenses and StateHub

Roko's telemetry has two layers:

1. **Lenses**: raw observation machinery. Each Lens is a Cell implementing the Observe protocol. Lenses receive read-only lifecycle events and emit structured observation Signals.
2. **StateHub**: typed projections consumed by display surfaces. StateHub aggregates Lens output into versioned, structured projections that surfaces (TUI, web dashboard, Slack, audit logs) subscribe to. Surfaces never read raw Lens output.

This decoupling means new surfaces can be added without modifying the observation layer, and new Lenses can be added without touching surface code.

| Lens | Measures |
|---|---|
| **CostLens** | USD and token expenditure with model breakdown and budget remaining |
| **LatencyLens** | Execution duration with p50/p95/p99 |
| **QualityLens** | Pass/fail rates from Verify Cells; continuous reward; per-rung breakdown |
| **EfficiencyLens** | Tokens-per-task; cost-per-quality ratios; prediction error; vitality phase |
| **ErrorLens** | Error classification and aggregation |
| **DriftLens** | Knowledge quality degradation; tier distribution; promotion/demotion rates |
| **AnomalyLens** | Threat detection indicators (part of the immune system) |
| **MemoryLens** | Memory lifecycle events (retrieval, store, consolidation, demurrage applied) |
| **TriggerLens** | Trigger lifecycle (fired, armed, disarmed) |
| **ExtensionLens** | Extension hook invocations and decisions |
| **CalibrationLens** | Pre/post pairs and calibration updates per Cell |

Lenses run on a configurable cadence (default once per second) and are themselves Hot Graphs.

### StateHub projections

Built-in projections include `agent_vitality`, `cost_summary`, `quality_dashboard`, `inference_pipeline`, `memory_dashboard`, `trigger_status`, `error_aggregator`, `c_factor`, and `learning_state`. Display surfaces subscribe to the projections they need. A WebSocket subscriber receives delta updates whenever the projection changes.

Every Cell execution carries a `TraceContext` (trace_id, span_id, parent_span_id, baggage). Spans are nested. A plan execution starts a root span; each task dispatch spawns a child; each tool call spawns a grandchild. The full trace is exportable to OpenTelemetry-compatible collectors.

Lenses can write metrics to multiple sinks: an in-process Prometheus-compatible scrape endpoint, OpenTelemetry OTLP push, or append-only JSONL logs.

The `AnomalyLens` detects contradiction clusters, score spikes without supporting evidence, taint fan-out bursts, sandbox violation clusters, tenant boundary mismatches, and lineage gaps. Detection uses z-score thresholds against historical baselines. Detected anomalies become `Anomaly` Signals that flow into the immune system.

---

## 21. Configuration

Configuration is itself a Signal — content-addressed, typed, scored, and versioned like any other Signal. Config changes flow through the same Bus and Store as everything else. Config history is in Store and is queryable by content hash. Config changes emit Pulses on `config.{section}.changed`. Cells subscribe to relevant config Pulses and re-initialize on change. Config audit is built into the Signal lineage DAG.

### Priority merging

Sources are merged in priority order, with later sources overriding earlier ones:

1. Built-in defaults (compiled into the binary).
2. System-wide config file.
3. User-wide config file.
4. Workspace `roko.toml`.
5. `ROKO_*` environment variables.
6. CLI flags.
7. Runtime API overrides.

Higher-priority sources override lower for the same key. Arrays merge (concatenated), objects merge (deep), scalars override.

### Schema versioning

Each config file declares its schema version. The runtime knows how to read versions and migrate older schemas to the current version. Migrations are pure Cells, tested in isolation. When the runtime upgrades with a new schema, existing config files are auto-migrated on next load with the migrated copy written back (with a backup of the original).

### Seven invariants

The configuration system enforces seven invariants that, if violated, cause the runtime to refuse to start:

1. **Consistency**: declared capabilities match what each Cell actually uses.
2. **Capability containment**: no Cell declares a capability outside its Space's grants.
3. **Cycle freedom**: extension dependency graph and Trigger fan-out graph are acyclic.
4. **Schema compatibility**: every Edge's source schema is compatible with the target schema.
5. **Budget bounds**: per-agent budget ≤ Space-level budget.
6. **Authentication**: at least one auth path is configured (insecure mode is explicit opt-in).
7. **Secret resolution**: every secret reference resolves to a defined secret.

Validation runs at config load. Failures produce structured errors and the runtime refuses to start.

### Hot reload as a Trigger Graph

A `FileWatch` Trigger on `roko.toml` fires a `config-reload` Graph: read new config → apply migrations → validate invariants → diff against current effective config → emit per-section `config.{section}.changed` Pulses → subscribers re-initialize affected components. If validation fails, the new config is rejected and the running config is preserved. The reload result is itself a Signal in the audit DAG.

### Domain profiles

A Domain Profile is a named bundle of configuration for a specific kind of work, configuring clock rates, extension chains, Bus subscriptions, context weights, gate pipelines, model preferences, and budget defaults.

| Profile | Clock | Extensions | Gates | Models |
|---|---|---|---|---|
| **coding** | Gamma 200ms, Theta 8s | git, compiler, test-runner, safety | compile, test, clippy, diff | Opus T2, Sonnet T1 |
| **research** | Gamma 500ms, Theta 16s | web-search, citation, summarizer, safety | source-quality, coherence | Opus T2, Sonnet T1 |
| **trading** | Gamma 100ms, Theta 2s | chain-reader, risk-manager, safety | risk-check, compliance | Sonnet T1, Haiku T0 |
| **devops** | Gamma 300ms, Theta 10s | git, compiler, deploy-checker, safety | health-check, rollback-gate | Sonnet T1 |
| **general** | Gamma 300ms, Theta 10s | safety, cost-tracker | basic-quality | Sonnet T1 |

Profiles can be inherited and composed. Per-agent and per-cell blocks override profile defaults. Secrets are referenced via `secret://` and resolved at runtime from OS keychain, AWS/GCP/Azure secret managers, HashiCorp Vault, or environment variables. Secret values are loaded into a `SecretRegistry` at startup and never written to disk.

---

## 22. Plan Execution

For task plans (DAGs of agent dispatches), the engine spawns a `ParallelExecutor`. The executor is a **pure state machine** — it never performs I/O. It takes a parsed plan, tracks task states, and emits actions:

```rust
pub enum ExecutorAction {
    DispatchAgent { task_id, role, prompt, .. },
    RunGate { task_id, rung, .. },
    PersistState { snapshot },
    NotifyCompletion { task_id, verdict },
}

pub enum ExecutorEvent {
    AgentCompleted { task_id, output },
    GateCompleted { task_id, rung, verdict },
    Failure { task_id, error },
}
```

The orchestration harness in the CLI receives actions and performs the actual I/O: spawning agent subprocesses, running gate commands, writing files, updating the TUI. Results flow back as `ExecutorEvent` values that advance the state machine. This separation makes the executor logic fully testable without I/O.

Tasks with all dependencies satisfied run concurrently via Tokio `JoinSet`. A configurable concurrency limit caps simultaneous agent subprocesses. When a task completes and its gate pipeline passes, the executor checks whether any blocked tasks are now unblocked.

### Worktree isolation

For code-domain tasks, each parallel task can run in its own git worktree. The `WorktreeManager` creates branches, checks out worktrees, and tracks their health. After a task passes gates, its worktree is merged back to the plan branch. An idle TTL reclaims stale worktrees. Two Implementer tasks editing different files can run truly in parallel without write-write races.

### Resumption

The executor periodically serializes state. On resume, it reloads the snapshot and picks up where it left off, skipping completed tasks.

### Coordination via pheromones

The orchestrator implements pheromone-based coordination across parallel tasks. When a task touches a file, the executor deposits a pheromone at that path. Subsequent tasks consult the pheromone field before scheduling — high-intensity locations are scheduled serially to avoid concurrent edits. Pheromones decay (half-life 5 minutes) so old activity does not block new scheduling indefinitely.

---

## 23. The Self-Hosting Loop

Roko develops itself through a six-phase cycle:

| Phase | What happens |
|---|---|
| **1. Capture** | A work item enters as a brief or product requirement. CLI subcommands create raw ideas and produce structured drafts. |
| **2. Research** | A research agent (Perplexity Sonar or equivalent) gathers citations, API documentation, and related patterns, then appends structured context. |
| **3. Plan** | A Strategist agent reads the brief and generates a `tasks.toml` — a DAG with dependencies, roles, domains, complexity bands, and acceptance criteria. |
| **4. Execute** | The `ParallelExecutor` reads the DAG, identifies tasks whose dependencies are satisfied, and dispatches agents in parallel. Each task is assigned to an agent role, which determines backend, model tier, tool permissions, and budget. |
| **5. Gate** | After each task, the gate pipeline validates the result. Rungs range from compile checks through clippy, tests, diff review, LLM-judge, symbol verification, integration tests, and property-based tests. Each rung produces a verdict. Failures can trigger automatic replanning. |
| **6. Persist and learn** | Episodes, metrics, and outcomes are written. The cascade router updates bandit weights. Prompt experiments record A/B outcomes. Adaptive thresholds adjust via EMA. The knowledge store distills durable insights. The system is strictly better the next time. |

The loop supports resumption: if interrupted, the executor reloads its snapshot and continues from the last checkpoint. The cycle repeats for every task in the DAG until all are complete, all have failed, or the circuit breaker trips.

The next document, [Coordination](./03-coordination.md), describes how Roko interacts with the world and other agents through the inference gateway, feeds, groups, and the four external protocols.
