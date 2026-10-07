# Design Decisions and Rationale

> **Verified against codebase: 2026-09-15.**
> Every major architectural decision in Roko, documented with its reasoning,
> alternatives considered, trade-offs accepted, and where the decision is
> implemented in code. This document answers "why" -- the architecture guide
> answers "what" and the data flow diagrams answer "how."

---

## Table of Contents

1. [Everything is a Signal](#1-everything-is-a-signal)
2. [Graph Engine as Sole Executor](#2-graph-engine-as-sole-executor)
3. [Nineteen Gates in a Seven-Rung Pipeline](#3-nineteen-gates-in-a-seven-rung-pipeline)
4. [Cascade Model Routing](#4-cascade-model-routing)
5. [Affect-Aware Dispatch (Daimon)](#5-affect-aware-dispatch-daimon)
6. [JSONL-Based Durable State](#6-jsonl-based-durable-state)
7. [Workspace-Local Data](#7-workspace-local-data)
8. [Twelve Kernel Traits](#8-twelve-kernel-traits)
9. [CoALA Cognitive Architecture](#9-coala-cognitive-architecture)
10. [Nine-Layer System Prompt Builder](#10-nine-layer-system-prompt-builder)
11. [Separation of Durable Signals and Ephemeral Pulses](#11-separation-of-durable-signals-and-ephemeral-pulses)
12. [Content-Addressed Identity (BLAKE3)](#12-content-addressed-identity-blake3)
13. [HDC Fingerprints for Semantic Similarity](#13-hdc-fingerprints-for-semantic-similarity)
14. [Self-Hosting as Architectural Driver](#14-self-hosting-as-architectural-driver)
15. [Separation of Composition and Execution](#15-separation-of-composition-and-execution)
16. [Four-Tier Signal Lifecycle](#16-four-tier-signal-lifecycle)
17. [Prospect Theory for Outcome Valuation](#17-prospect-theory-for-outcome-valuation)
18. [Twelve Provider Backends (Not One)](#18-twelve-provider-backends-not-one)
19. [Summary Table](#19-summary-table)

---

## 1. Everything is a Signal

**Decision**: A single data type (`Signal`) represents every piece of information
in the system: agent outputs, gate verdicts, knowledge entries, task definitions,
episode records, pheromone traces, research artifacts.

**Why**: The self-hosting loop requires uniform operations. When everything is the
same type, the same scoring, routing, composing, and verification operations apply
everywhere. You do not need separate "agent output management," "knowledge
management," and "task tracking" subsystems with different APIs -- they are all
Signals with different `Kind` values.

This is the single most important architectural decision in Roko. It is what makes
the system composable: any Signal can flow through any Store, be ranked by any Score,
selected by any Route, assembled by any Compose, and verified by any Verify. Without
this uniformity, the 12-trait protocol system would need N x M adapters instead of
N + M implementations.

**Alternatives considered**:
- **Separate types per domain** (AgentOutput, GateVerdict, KnowledgeEntry, etc.).
  This is the conventional approach and the first instinct of most engineers. Rejected
  because it requires N separate storage backends, N separate query systems, and N
  separate composition strategies. Every new capability requires plumbing through all N.
  In a system that grew to 38 crates, this would have been unsustainable.
- **Generic trait with associated types** (`trait Data { type Output; }`). More flexible
  than a single struct, but loses the ability to store heterogeneous collections and
  compose across domains. A `Vec<Signal>` can hold tasks and knowledge entries side by
  side; a `Vec<dyn Data>` cannot without type erasure.

**Trade-offs accepted**:
- The Signal struct carries fields that are irrelevant for many use cases. A gate
  verdict does not need an `emotional_tag`; a task definition does not need a
  `decay` curve. These fields are `Option` or have sensible defaults, so they add
  memory overhead (~50 bytes per unused field) but no behavioral cost.
- Pattern matching on `Kind` replaces static type safety for domain dispatch. This
  is a deliberate trade: composability across domains outweighs compile-time
  domain specificity. A `Verify` that receives a `Kind::Task` when it expected
  `Kind::AgentOutput` returns a failed verdict at runtime, not a compile error.

**Key properties of a Signal**:
- Content-addressed via BLAKE3 hash (identity = content).
- Hash covers: kind, body, author, taint, lineage, tags. Excludes: score, decay,
  timestamp, attestation, emotional tag (these can change without changing identity).
- Immutable once created.
- Traceable via parent content hashes (lineage DAG).
- Decaying via configurable half-life curves.
- Scored along multiple dimensions (confidence, novelty, utility, reputation).
- Optionally fingerprinted with a 10,240-bit HDC vector for semantic similarity.
- Tiered lifecycle: Transient -> Working -> Consolidated -> Persistent.

**Where implemented**: `crates/roko-core/src/signal.rs` (primary struct definition,
`Signal::builder()`, `Signal::content_hash()`), `crates/roko-core/src/engram.rs`
(re-export with `pub type Engram = Signal` backward-compatible alias),
`crates/roko-core/src/kind.rs` (the `Kind` enum, `#[non_exhaustive]` with 27+
variants and `Custom(String)` escape hatch).

---

## 2. Graph Engine as Sole Executor

**Decision**: All plan execution runs through a single DAG-based Graph engine.
The older WorkflowEngine was retired (PR #276) and Runner-v2 is retained only as
`--engine legacy` for one deprecation cycle.

**Why**: Multiple execution engines meant multiple code paths for the same operation.
Each engine had different bug profiles, different checkpoint formats, and different
feature coverage. Adding parallel execution required three implementations. Adding
cost enforcement required three implementations. Every feature had to be built and
tested three times.

The convergence to one engine was a hard-won lesson. Here is the timeline:

1. **Runner-v2** was the original executor (sequential, no parallelism). Simple but
   could not run independent tasks concurrently, which made large plans slow.
2. **WorkflowEngine** was added to support richer DAG semantics with conditional
   routing. But it duplicated much of Runner-v2's logic.
3. **GraphEngine** was added for Cell-based composition with parallel waves, cost
   enforcement, and immune verification. It was the most capable but the third
   implementation of the same thing.
4. **PR #260** made GraphEngine the default. Plans now execute on Graph unless you
   explicitly opt out.
5. **PR #276** retired WorkflowEngine entirely. Runner-v2 kept as `--engine legacy`
   for one release cycle to allow migration.

**Alternatives considered**:
- **Keep all three engines.** Rejected because the maintenance burden was unsustainable.
  Bug fixes had to land in all three engines. Features were inconsistently available.
  Checkpoint formats were incompatible. Testing surface tripled.
- **Keep Runner-v2 as primary.** Rejected because it lacks parallel wave execution,
  conditional routing, and cost enforcement -- all required by the self-hosting loop
  when running plans with 20+ tasks.
- **External workflow engine** (Temporal, Airflow, Prefect). Rejected because Roko must
  run on a developer's laptop with no external services. Also, the tight coupling
  between execution and the Signal/Cell model makes an external engine awkward --
  every cell would need serialization/deserialization adapters.
- **Event-sourced engine** (record all state transitions as events, replay to recover).
  Partially adopted: the Graph engine does checkpoint state as events, but it uses a
  simpler wave-based model rather than full event sourcing.

**Trade-offs accepted**:
- Plans that relied on Runner-v2-specific checkpoint format need migration or the
  `--engine legacy` flag. This affects very few users since the system is pre-release.
- The Graph engine is more complex than a simple sequential loop. Topological sort,
  wave scheduling, parallel dispatch, and the immune decision graph add code that
  would not exist in a simpler system. This complexity is justified by the capability
  it enables: parallel execution, conditional routing, cost enforcement, and durable
  resume.

**Key properties of the Graph engine**:
- Parallel waves: independent cells execute concurrently within a wave
- Conditional routing: cells can route to different paths based on output
- Cost enforcement: per-plan and per-task USD budgets, atomic reservations
- Checkpoint/resume: durable state after each task, resume from exact point
- Immune Graph: five-stage verification screens all outputs before propagation
- Graph fingerprinting: deterministic fingerprint for Activity resume matching

**Where implemented**: `crates/roko-graph/src/engine.rs` (GraphEngine),
`crates/roko-graph/src/topo.rs` (topological sort and wave scheduling),
`crates/roko-graph/src/budget.rs` (cost enforcement and atomic reservations),
`crates/roko-graph/src/snapshot.rs` (checkpoint/resume),
`crates/roko-graph/src/fingerprint.rs` (graph fingerprinting).

---

## 3. Nineteen Gates in a Seven-Rung Pipeline

**Decision**: Agent output is verified by up to 19 gate types organized into 7
rungs. Rungs execute in order of increasing cost; failure at any rung can halt the
pipeline. Gate thresholds adapt automatically using exponential moving averages.

**Why**: LLM output is unreliable. This is the central problem that Roko addresses,
and the gate pipeline is the mechanism that makes the system trustworthy. Without
gates, Roko is just an expensive random text generator. With gates, it is a software
engineering pipeline that produces verified code.

The multi-rung design reflects a key insight: different kinds of verification have
different costs and different signal value. Running them cheapest-first minimizes
waste:
- **Rung 0 (Compile)** is nearly free (seconds) and catches 40-60% of agent failures.
  Most broken code fails to compile. Running compile first means you never waste money
  on an LLM judge call for code that does not even parse.
- **Rung 6 (Integration + LLM Judge)** is expensive (minutes + API cost) but catches
  subtle semantic issues that cheaper gates miss.

**The 19 gates**:

| Gate | Rung | What it checks |
|------|------|---------------|
| CompileGate | 0 | Does it build? (cargo check, tsc, go build) |
| ClippyGate | 1 | Does it pass linting? (cargo clippy -D warnings) |
| TestGate | 2 | Do existing tests pass? (cargo test) |
| SymbolGate | 3 | Did public APIs break? (diff analysis) |
| GeneratedTestGate | 4 | Do agent-written behavioral tests pass? |
| VerifyChainGate | 4 | Does the verification chain hold? |
| PropertyTestGate | 5 | Do property-based tests pass? (proptest/quickcheck) |
| FactCheckGate | 5 | Are factual claims in the output correct? |
| IntegrationGate | 6 | Do full integration scenarios pass? |
| LlmJudgeGate | 6 | Does an LLM evaluator rate the code as good? |
| DiffGate | -- | Post-task diff analysis (standalone) |
| CodeExecutionGate | -- | Sandboxed code execution (standalone) |
| ShellGate | -- | Arbitrary shell command verification (standalone) |
| BenchmarkRegressionGate | -- | Performance regression check (standalone) |
| FormatCheckGate | -- | Code formatting (cargo fmt) check (standalone) |
| SecurityScanGate | -- | Security vulnerability scanning (standalone) |

**Why adaptive thresholds**: Fixed thresholds create a dilemma. Set them too high
and you reject too much (wasting money on retries of slightly imperfect code). Set
them too low and you accept broken code. EMA-based thresholds solve this by converging
to the natural pass rate of the system:
- When gates consistently pass, thresholds tighten (higher standards).
- When gates consistently fail, thresholds relax (avoid infinite retry loops).
- The learning rate (alpha) is configurable under `[learning]` in `roko.toml`.

**Alternatives considered**:
- **Single gate (compile-only).** Insufficient. Code that compiles but fails tests
  is still broken. Code that passes tests but has lint warnings will be rejected by CI.
- **Binary pass/fail without thresholds.** Too brittle. Some tasks legitimately
  produce warnings or minor test regressions (e.g., a refactor that changes test
  output format but is functionally correct).
- **Human-in-the-loop review for every output.** Defeats the purpose of automation.
  The self-hosting loop must run unattended for hours.
- **Statistical quality gate (pass if > X% of recent outputs compiled).** Too
  coarse. Each output must be individually verified.

**Trade-offs accepted**:
- The full 7-rung pipeline is slow (minutes per task). For quick iterations, lower
  rungs can be selected via `[gates] max_rung = 2` in config.
- Adaptive thresholds can drift if the input distribution shifts (e.g., suddenly doing
  much harder tasks). The conductor's 12 watchers monitor for threshold degradation.
- LlmJudgeGate (Rung 6) adds LLM API cost on top of the agent's cost. This is
  acceptable because it catches issues that syntactic gates cannot, but it means
  full-rung verification costs ~1.5x a single agent call.

**Where implemented**: `crates/roko-gate/src/lib.rs` (gate type definitions),
`crates/roko-gate/src/gate_pipeline.rs` (pipeline orchestration),
`crates/roko-gate/src/adaptive_threshold.rs` (EMA threshold logic),
`crates/roko-gate/src/rung_dispatch.rs` (rung execution),
`crates/roko-gate/src/rung_selector.rs` (which rungs to run),
`crates/roko-cli/src/runner/gate_dispatch.rs` (runner integration).

---

## 4. Cascade Model Routing

**Decision**: A three-stage cascade router selects which LLM model to use for each
task. Stages: Static (config lookup, < 50 observations), Confidence (empirical pass
rates + confidence intervals, 50-200 observations), UCB (full LinUCB contextual
bandit, 200+ observations).

**Why**: Different tasks need different models. Renaming a variable does not need
Claude Opus (the most expensive model); designing a complex API does. Hardcoding a
single model wastes money on easy tasks and risks failure on hard ones.

The three stages solve the **cold-start problem** -- the fundamental challenge of
any learning system: you cannot do optimization with zero data.

- **Static (0-49 observations)**: With no data, use a sensible default table that maps
  task role to a model. This table is hardcoded based on the developer's experience
  and can be overridden in config.
- **Confidence (50-199 observations)**: With some data, use empirical pass rates but
  respect uncertainty. If model A has a 90% pass rate +/- 15%, and model B has an 85%
  pass rate +/- 5%, prefer B because we are more certain about it.
- **UCB (200+ observations)**: With enough data, do full contextual bandit optimization.
  LinUCB (Linear Upper Confidence Bound) uses features (task role, complexity, domain,
  cost) to select the model with the highest expected reward adjusted for uncertainty.

**Key features beyond basic selection**:
- **Provider health filtering**: unhealthy providers (HTTP errors, timeouts) are
  excluded from selection using the persisted health registry.
- **Affect-based tier shifts**: after failures, DaimonState biases toward more
  capable (and more expensive) models. After successes, it allows cheaper models.
- **Cost pressure**: as budget runs low, bias toward cheaper models.
- **Hysteresis**: avoid flip-flopping between models on consecutive tasks. If model A
  was used for the previous task and model B is only marginally better for this one,
  stick with A.
- **Fallback chains**: if the primary model fails (API error, rate limit), try the
  fallback chain automatically.
- **Context overflow fallback**: if the prompt exceeds the primary model's context
  window, switch to a model with a larger window.

**Alternatives considered**:
- **Always use the best model.** Too expensive. Claude Opus costs 15x more than
  Claude Haiku. For a 100-task plan, this difference is hundreds of dollars.
- **Pure bandit from the start.** Bandits need exploration data to converge. Exploring
  with an expensive model on every task to build that data is wasteful and potentially
  harmful (exploring means accepting worse outcomes).
- **Manual model selection per task.** Defeats automation. The self-hosting loop must
  select models without human intervention.
- **Simple heuristic (complexity score -> model tier).** Works for obvious cases but
  misses subtleties. Some "simple" tasks (e.g., fixing a tricky borrow checker error)
  need a capable model, while some "complex" tasks (e.g., adding 10 similar endpoints)
  can be done cheaply.

**Trade-offs accepted**:
- The cascade has substantial complexity: tier shifts, hysteresis, Pareto frontier
  recomputation, cost pressure factors, provider health integration. This complexity
  is justified by the cost savings.
- The Static stage uses a hardcoded table that may not match all deployments. This
  is acceptable because it only lasts for 50 observations, after which empirical data
  takes over.
- The bandit explores, which means it sometimes deliberately chooses suboptimal models
  to gather data. The exploration rate is bounded and configurable.

**Where implemented**: `crates/roko-learn/src/cascade_router.rs` (main router logic,
stage selection, CascadeModel output), `crates/roko-learn/src/model_router.rs`
(LinUCB bandit implementation), `crates/roko-learn/src/cascade/` (helpers, types,
persistence).

---

## 5. Affect-Aware Dispatch (Daimon)

**Decision**: An affect engine tracks recent success/failure history across three
timescales (fast emotion, medium mood, slow temperament) and adjusts agent dispatch
parameters: temperature, turn budget, and exploration rate.

**Why**: After a string of failures, blindly retrying with the same parameters
wastes money and time. The affect engine notices patterns and adapts:
- **Frustration** (repeated gate failures): lower temperature (more deterministic
  output), reduce exploration (stick with known-good models), consider escalating to
  a more capable model.
- **Flow** (consecutive successes): maintain current parameters, allow more
  exploration (try cheaper models), optionally increase turn budget.
- **Fatigue** (long session, many tasks): reduce turn budget, increase caution.

This is inspired by the CoALA (Cognitive Architectures for Language Agents) framework,
which identifies affect as a key driver of agent behavior. The insight is that human
developers adjust their behavior after failures -- they become more careful, try
different approaches, or take a break. The Daimon gives Roko the same adaptive
capacity.

**The three timescales**:

| Timescale | Name | Decay | What it tracks | Example |
|-----------|------|-------|---------------|---------|
| Fast | Emotion | Minutes | Immediate reaction | "That last task failed" |
| Medium | Mood | Hours | Running session average | "This session is going badly" |
| Slow | Temperament | Days | Long-term tendency | "This project is hard" |

Each timescale uses a PAD (Pleasure/Arousal/Dominance) vector from psychology:
- **Pleasure**: positive outcomes increase it, negative outcomes decrease it.
- **Arousal**: surprise increases it, predictability decreases it.
- **Dominance**: agency (choosing actions) increases it, helplessness decreases it.

**How it modulates dispatch**:
- `temperature_delta`: added to the model's base temperature. Negative after failures
  (more deterministic), positive during flow (more creative). Range: -0.3 to +0.1.
- `max_turns`: caps the agent's tool loop iterations. Reduced during fatigue or
  frustration (avoid wasting turns on a stuck task). Range: 15 to 30.
- `exploration_rate`: multiplier on the cascade router's exploration parameter.
  Reduced when cautious (0.3x), increased when confident (1.5x).

**Outcome valuation uses prospect theory** (Kahneman & Tversky): losses are weighted
2.25x more than gains of the same magnitude, and both are curved (diminishing
sensitivity). This means a failure has a disproportionate impact on affect, which is
correct behavior -- you should be more cautious after a failure than bold after a
success.

**Alternatives considered**:
- **No affect engine.** Simpler but misses the adaptation signal. Fixed parameters
  retry failures identically, burning money on the same approach.
- **Simple retry counter.** Tracks failures but not their pattern. Does not distinguish
  "failed because the task is hard" from "failed because the model is wrong for this
  task" from "failed because of a flaky test."
- **Full cognitive architecture.** Too heavy for a developer tool. Roko is not a
  research platform. The PAD model is well-understood, lightweight (~100 bytes of
  state), and computationally trivial.

**Trade-offs accepted**:
- The affect engine adds state that must be persisted and loaded per session.
- The PAD model is a simplification of human emotion. It works well enough for
  dispatch modulation but does not capture all failure modes.
- Somatic markers (learned situation-specific associations stored in a KD-tree) add
  memory proportional to the number of distinct task/model combinations encountered.

**Where implemented**: `crates/roko-daimon/src/lib.rs` (DaimonState, PAD vectors,
prospect_value(), somatic marker storage), `crates/roko-daimon/src/policy.rs`
(AffectPolicy adapter for WorkflowEngine integration),
`crates/roko-daimon/src/somatic_ta.rs` (somatic TA integration, IIT Phi metric).

---

## 6. JSONL-Based Durable State

**Decision**: All runtime state is stored as append-only JSONL (JSON Lines) files.
Episodes, signals, efficiency events, and gate thresholds all use this format.

**Why**: JSONL has a set of properties that are uniquely suited for a developer tool:

| Property | Why it matters |
|----------|---------------|
| **Human-readable** | Open the file in any text editor, grep for patterns. Debug by reading. |
| **Append-only** | No in-place mutation means no corruption from partial writes. A crash mid-write loses at most one line. |
| **No server** | No database process to start, configure, monitor, or crash. `roko init` is instant. |
| **Portable** | Works on every OS. Easy to copy, diff, email, and version-control. |
| **Trivially parseable** | Each line is a complete JSON object. Parse one line = parse one record. No schema migrations needed for new fields (use `#[serde(default)]`). |
| **Git-friendly** | Append-only means merge conflicts are rare and mechanical. |
| **Inspectable** | `cat .roko/episodes.jsonl | jq '.model'` gives you every model used, instantly. |

For a tool that runs on developer laptops with no server dependency, these properties
outweigh the query performance of a database.

**Alternatives considered**:
- **SQLite.** Better query performance for complex queries, but adds a binary
  dependency, makes diffs opaque, complicates backup and inspection, and introduces a
  write-ahead log that can corrupt on unclean shutdown. Actually used for some
  specialized stores (roko-neuro knowledge index), but not for the primary signal log.
- **RocksDB or sled.** High-performance embedded databases. Rejected because they
  produce opaque binary files, add substantial compile-time dependencies (RocksDB adds
  5+ minutes to clean builds), and provide performance that JSONL does not need --
  Roko processes thousands of signals, not millions.
- **PostgreSQL or remote database.** Requires a running server. Unacceptable for a
  CLI tool that must work offline on a laptop. Also adds configuration, auth, and
  connection management complexity.
- **Protobuf or MessagePack.** Compact binary formats. Faster to parse but not
  human-readable. Debugging requires dedicated tools. The parse-time savings are
  negligible for Roko's data volumes.

**Trade-offs accepted**:
- Query performance is O(n) for full scans. Mitigated by bounded file sizes
  (generation rotation in roko-fs), in-memory caching for hot paths, and HDC indexes
  for semantic search. In practice, signal logs under 100MB parse in under a second.
- No ACID transactions. Mitigated by append-only semantics (no updates, no deletes)
  and the fact that individual JSON lines are atomically written (a single `write()`
  system call). Cross-file consistency is not guaranteed, but is rarely needed.
- File sizes grow for long-running projects. Mitigated by GC (garbage collection in
  `roko-fs`), cold archival (`ColdStore.archive()`), and configurable generation
  rotation.

**Where implemented**: `crates/roko-fs/src/file_substrate.rs` (FileSubstrate --
the JSONL substrate implementation), `crates/roko-fs/src/lib.rs` (GC, generation
rotation, layout).

---

## 7. Workspace-Local Data

**Decision**: All Roko state lives in the `.roko/` directory inside the project
workspace. There is no global daemon database, no shared configuration directory,
no central registry that projects depend on.

**Why**: Project isolation. Each project is self-contained:
- Copy a project directory and you bring its full history, knowledge, routing state,
  and learning data.
- Multiple projects with different configurations (different providers, different
  gate settings, different knowledge) do not interfere with each other.
- There is no "Roko corrupted my other project" failure mode.
- Offline work is natural -- there is no server to connect to.

This mirrors Git's model: `.git/` contains everything, and you can have as many
repos as you want without conflict.

**Alternatives considered**:
- **Global daemon with per-project namespaces.** More efficient for shared model
  routing data across projects, but introduces cross-project coupling and a process
  to manage. If the daemon crashes, all projects are affected.
- **XDG-style config directory** (`~/.config/roko/`). Global config does exist for
  provider API keys (so you do not repeat them per project), but all runtime state
  is per-workspace.

**Trade-offs accepted**:
- Knowledge learned in one project does not automatically transfer to another. Each
  project starts cold. Mitigated by `roko knowledge export/import` for manual transfer
  and `roko knowledge sync` for mesh sync between projects.
- Routing data does not transfer. A project that has learned optimal model routing
  over 200 tasks cannot share that learning with a new project. Acceptable because
  optimal routing depends on the codebase -- what works for a Rust project may not
  work for a TypeScript project.
- Disk usage is duplicated across projects. Each project has its own signal log,
  episode log, etc. Acceptable for developer laptops with hundreds of gigabytes of
  storage.

**Where implemented**: `crates/roko-fs/src/lib.rs` (RokoLayout, workspace structure
conventions), `crates/roko-core/src/config/loader.rs` (config discovery: workspace
`roko.toml` -> global `~/.config/roko/config.toml` -> environment variables).

---

## 8. Twelve Kernel Traits

**Decision**: The kernel defines 12 protocol traits that every capability in the
system maps to. These traits define the entire operational surface of Roko.

| # | Trait | What it defines | Key implementations |
|---|-------|----------------|---------------------|
| 1 | **Store** | Persist/retrieve Signals | FileSubstrate, MemorySubstrate, HdcSubstrate |
| 2 | **ColdStore** | Archive aged-out Signals | ArchiveColdSubstrate |
| 3 | **Score** | Rate a Signal on dimensions | SumScorer, RelevanceScorer, RecencyScorer |
| 4 | **Verify** | Check against ground truth | CompileGate, TestGate, LlmJudgeGate (19 total) |
| 5 | **Route** | Pick one candidate from many | CascadeRouter, LinUCBRouter, StaticRouter |
| 6 | **Compose** | Combine Signals into a prompt | SystemPromptBuilder, PromptComposer |
| 7 | **React** | Watch signal streams, emit interventions | Conductor watchers, circuit breaker |
| 8 | **Bus** | Publish/subscribe ephemeral Pulses | PulseBus |
| 9 | **Observe** | Import data from external sources | Chain observers, file watchers |
| 10 | **Connect** | Manage network connections | Relay connections |
| 11 | **Trigger** | Arm/disarm scheduled/event actions | Cron triggers, event triggers |
| 12 | **Substrate** | Low-level storage backend | Alias for Store |

**Why 12?** Because the self-hosting loop decomposes into exactly these operations:

```
query (Store) -> score (Score) -> route (Route) -> compose (Compose)
  -> act (agent, uses Connect) -> verify (Verify) -> write (Store)
  -> react (React, Bus) -> observe (Observe) -> trigger (Trigger)
```

Each trait captures one kind of interaction with the world. Adding a new capability
means implementing one of these traits, not inventing a new abstraction.

**Why traits, not message passing?** Rust traits provide compile-time dispatch, type
safety, and zero-cost abstractions. Message passing would require runtime dispatch,
serialization overhead, and lose the ability to express type constraints (e.g.,
"this gate requires a Signal with Kind::AgentOutput"). For a system that processes
thousands of signals per session, the performance difference matters.

**The Cell abstraction**: Every trait extends the `Cell` base trait, which provides
identity (`cell_id`), versioning (`CellVersion`), protocol metadata (`protocols()`),
capability declarations (`Capabilities`), and cost estimates. This means any
computation unit in the system is identifiable and composable into Graph DAGs.

**Alternatives considered**:
- **Fewer traits** (e.g., just Store + Transform + Verify). Too coarse. Conflating
  scoring with routing loses the separation of concerns that makes the cascade router
  and the composer work independently.
- **More traits** (e.g., separate traits for each gate type, separate traits for each
  provider type). Too fine-grained. The power of the trait system is that multiple
  implementations share a common interface. 19 gate types behind one `Verify` trait
  means the pipeline does not know or care which gate it is running.
- **No trait hierarchy** (free functions). Loses composability. You cannot build a
  Graph of Cells if cells are just functions with different signatures.
- **Actor model** (each component is an actor with a mailbox). Higher isolation but
  significantly more complex. Actors need supervisors, mailbox management, and
  serialization. The trait model is simpler and sufficient for Roko's concurrency
  needs (tokio tasks, not millions of actors).

**Where implemented**: `crates/roko-core/src/traits.rs` (all 12 trait definitions
with full doc comments, default method implementations, and datum-polymorphic entry
points), `crates/roko-core/src/cell.rs` (Cell base trait, Capabilities, CostEstimate,
PredictionRecord, TypeSchema, CellContext).

---

## 9. CoALA Cognitive Architecture

**Decision**: Roko's cognitive architecture maps to the CoALA (Cognitive
Architectures for Language Agents) framework. This is not a superficial label --
the mapping influenced real design choices about how to decompose agent capabilities.

**The mapping**:

| CoALA Component | Roko Implementation | Crate |
|----------------|---------------------|-------|
| Long-term memory | KnowledgeStore (durable entries with half-lives) | roko-neuro |
| Working memory | Signal substrate (transient/working tier signals) | roko-core/roko-fs |
| Episodic memory | EpisodeLogger (episodes.jsonl) | roko-learn |
| Procedural memory | PlaybookStore (when/then patterns) | roko-learn |
| Decision-making | CascadeRouter + DaimonState | roko-learn + roko-daimon |
| Action | AgentDispatcher (12 LLM backends) | roko-agent |
| Perception | Conductor observers (12 watchers) | roko-conductor |
| Learning | Online (feedback) + Offline (dreams) | roko-learn + roko-dreams |
| Grounding | GatePipeline (19 gates verify against reality) | roko-gate |

**Why CoALA?** Because it provides a principled decomposition of agent capabilities
that prevented ad-hoc feature accumulation. Without a cognitive architecture, agent
systems tend to be disorganized collections of features. CoALA forces you to think
about memory types, learning loops, and the relationship between perception and
action. This produced several concrete design improvements:

1. **Separate episodic and procedural memory.** Episodes record what happened (raw
   history); playbooks record what works (distilled procedures). The dream cycle
   distills episodes into playbooks. Without CoALA, these would likely be a single
   undifferentiated log.

2. **Explicit working vs. long-term memory.** The Signal tier system (Transient ->
   Working -> Consolidated -> Persistent) directly maps to CoALA's memory hierarchy.
   Transient signals are working memory; Persistent signals are long-term memory.

3. **Affect as a first-class subsystem.** CoALA identifies affect as a key driver
   of agent behavior. This justified building roko-daimon as a dedicated crate
   rather than burying temperature adjustment inside the dispatch loop.

4. **Perception as distinct from action.** The conductor's 12 watchers are passive
   observers that detect anomalies, separate from the agents that take action. This
   separation prevents the system from acting on incomplete information.

5. **Grounding through verification.** CoALA emphasizes that agent decisions must be
   grounded in reality. The gate pipeline is the grounding mechanism: it connects LLM
   output to real-world verification (does the code compile? do tests pass?).

**Where the mapping is visible**:
- `crates/roko-neuro/` (long-term memory with tier progression and distillation)
- `crates/roko-learn/` (episodic + procedural memory, model routing decisions)
- `crates/roko-daimon/` (affect engine with PAD vectors and somatic markers)
- `crates/roko-dreams/` (offline learning with 7-phase consolidation cycle)
- `crates/roko-conductor/` (perception: 12 watchers with circuit breaker)
- `crates/roko-agent/` (action: 12 LLM backends with tool loop)
- `crates/roko-gate/` (grounding: 19 gates verify against reality)

---

## 10. Nine-Layer System Prompt Builder

**Decision**: System prompts are assembled from 9 composable layers, each targeting
a different stability tier for Anthropic prompt cache alignment.

| Layer | Content | Cache Tier | Stability |
|-------|---------|------------|-----------|
| 1 | Role identity | System | Very stable (changes with role) |
| 2 | Conventions | System | Semi-stable (changes with config) |
| 3 | Domain context | Session | Semi-stable (changes with project) |
| 3c | Active signals | Session | Semi-stable (changes with active pheromones) |
| 4 | Task specification | Task | Volatile (changes every task) |
| 4b | Gate feedback | Dynamic | Volatile (changes every retry) |
| 5 | Tool instructions | System | Very stable (changes with tool set) |
| 6 | Playbooks | Task | Volatile (depends on task shape) |
| 7 | Anti-patterns | Task | Volatile (depends on error history) |
| 8 | Affect guidance | Dynamic | Volatile (changes with PAD state) |

**Why 9 layers?** Because experiments showed that system prompts produce a 3-4x
quality difference. A well-structured prompt with role context, conventions, and
anti-patterns dramatically outperforms a bare prompt. The 9-layer design ensures
that all relevant context is included while respecting token budgets (layers are
trimmed from the bottom up if the prompt exceeds the model's context window).

**Why cache alignment?** Anthropic's prompt caching charges less for the stable prefix
of a system prompt. By organizing layers so that stable content (role, conventions,
tools) comes first and volatile content (task, gate feedback, affect) comes last, the
prefix remains cacheable across multiple tasks in a plan. This reduces cost by 50-90%
on the system prompt portion.

**Why separate composition from execution?** The prompt is fully assembled before the
agent is called. This provides three benefits:
1. **Inspectable**: you can log or print exactly what prompt was sent to the model.
2. **Testable**: write tests for prompt assembly without calling an LLM.
3. **Reproducible**: replay a prompt against a different model for comparison.

**Where implemented**: `crates/roko-compose/src/system_prompt_builder.rs`
(SystemPromptBuilder with `build()`, `with_role()`, `with_task()`, etc.),
`crates/roko-compose/src/templates/` (11 role templates: implementer.rs, reviewer.rs,
researcher.rs, refactorer.rs, strategist.rs, conductor.rs, integration.rs, scribe.rs,
quick.rs, task_impl.rs, common.rs).

---

## 11. Separation of Durable Signals and Ephemeral Pulses

**Decision**: The system has two data channels: durable **Signals** (persisted to
the substrate, content-addressed, immutable) and ephemeral **Pulses** (published on
the Bus, not persisted, used for real-time coordination).

**Why**: Not every event needs persistence. A heartbeat ("agent A is still alive")
is useful for real-time monitoring but worthless 5 minutes later. A gate verdict
("tests passed") has permanent value. Forcing everything through durable storage
wastes I/O on events that will be immediately pruned.

The Bus/Pulse system provides a publish/subscribe transport for ephemeral events.
Pulses that turn out to be important can be promoted to Signals and stored. This
mirrors the human cognitive distinction between sensory buffer (fleeting) and
working memory (retained).

**Practical examples**:
- **Pulse**: "agent is writing to file X" (coordination, no persistence needed)
- **Signal**: "agent wrote 47 lines to file X, here is the diff" (permanent record)
- **Pulse**: "gate pipeline is at rung 3" (progress tracking)
- **Signal**: "gate pipeline completed with 6/7 rungs passed" (outcome record)

**Where implemented**: `crates/roko-core/src/pulse.rs` (Pulse type),
`crates/roko-core/src/traits.rs` (Bus trait with publish/subscribe),
`crates/roko-runtime/` (PulseBus implementation with EventBus<Pulse>).

---

## 12. Content-Addressed Identity (BLAKE3)

**Decision**: Signal identity is a BLAKE3 hash of the signal's content. Two signals
with identical content always have the same ID, regardless of when or where they
were created.

**Why**: Content addressing provides three guarantees:
1. **Deduplication**: the same information stored twice occupies the same logical
   slot. No duplicate signals in the substrate. The agent writing "implement login"
   twice produces one signal, not two.
2. **Integrity**: if a signal's content is corrupted, its hash will not match.
   Corruption is detectable.
3. **Lineage**: parent pointers use content hashes, forming a tamper-evident DAG.
   You can verify the entire provenance chain without trusting any intermediary.

The hash covers: kind, body, author, taint status, lineage, and tags.
The hash excludes: score, decay, timestamp, attestation, emotional tag. These mutable
metadata fields can change without changing what the signal fundamentally is.

**Why BLAKE3 over SHA-256?** BLAKE3 is faster (4-8x on modern hardware with SIMD),
produces the same 256-bit output, and has a pure Rust implementation (`blake3` crate).
Since Roko computes hashes for every signal, hash speed matters for plan execution
with hundreds of signals.

**Where implemented**: `crates/roko-core/src/hash.rs` (ContentHash type, 32-byte
wrapper), `crates/roko-core/src/signal.rs` (Signal::content_hash() method using
blake3::Hasher with structured field concatenation).

---

## 13. HDC Fingerprints for Semantic Similarity

**Decision**: Signals can carry a 10,240-bit Hyperdimensional Computing (HDC)
vector for approximate semantic similarity lookup.

**Why**: Text-based search (keyword, regex) misses semantic similarity. "Rate
limiting middleware" and "request throttling handler" describe the same concept
but share no keywords. HDC vectors capture semantic similarity in a fixed-size
binary representation.

**Why HDC over transformer embeddings?**

| Property | HDC | Embeddings |
|----------|-----|------------|
| Size | 10,240 bits (1,280 bytes) | 768-3072 floats (3-12 KB) |
| Similarity | Hamming distance (fast CPU op) | Cosine similarity (float math) |
| Composition | Bitwise XOR (combine two concepts) | Requires model call |
| Storage | Inline in Signal struct | Separate vector database |
| GPU needed | No | Usually yes for generation |
| Generation | Deterministic from text (no API call) | Requires embedding model API |

HDC vectors are:
- Fixed-size binary (no variable-length allocation)
- Composable via bitwise XOR (combine two concepts trivially)
- Searchable via Hamming distance (fast CPU operation, no GPU needed)
- Compact enough to store inline in the Signal, not in a separate vector database
- Deterministically generated from text (encoder version tracked for invalidation)

**Trade-offs accepted**:
- Lower precision than transformer embeddings. HDC similarity is approximate (recall
  ~70% vs ~95% for good embeddings). This is acceptable for knowledge retrieval and
  episode clustering, where recall matters more than precision.
- The 10,240-bit dimension was chosen empirically. Smaller dimensions lose too much
  information; larger dimensions add storage cost without proportional accuracy gains.

**Where implemented**: `crates/roko-primitives/src/hdc.rs` (HdcVector type, Hamming
distance, XOR composition), `crates/roko-index/` (fingerprint computation from source
code), `crates/roko-neuro/src/hdc.rs` (HDC-based knowledge lookup),
`crates/roko-core/src/signal.rs` (HdcFingerprint struct with encoder version tracking).

---

## 14. Self-Hosting as Architectural Driver

**Decision**: The primary design constraint for every component is "does this enable
Roko to develop itself?"

**Why**: Self-hosting is not a gimmick -- it is a forcing function. Every time Roko
develops itself and fails, it reveals a real defect in the system. A gate pipeline
that lets broken code through, a model router that picks the wrong model, a knowledge
store that forgets useful patterns -- all of these are bugs that the self-hosting loop
surfaces automatically.

**Concrete consequences of this decision**:
- The gate pipeline exists because LLM output is unreliable and the self-hosting loop
  needs to run unattended for hours.
- The cascade router exists because different self-development tasks have different
  complexity and the system needs to pick appropriate models automatically.
- The affect engine exists because the self-hosting loop encounters failure streaks
  and needs to adapt without human intervention.
- Checkpoint/resume exists because self-development plans run for hours and crashes
  must not lose progress.
- The knowledge store exists because the system encounters the same patterns across
  plans and needs to remember them.
- The dream consolidation cycle exists because patterns extracted from completed plans
  improve future plans.

**Where visible**: The self-hosting workflow (`roko plan generate -> review -> roko run`)
is documented in `CLAUDE.md` and `docs/v3/35-ARCHITECTURE.md`. The dogfood debrief
(`tmp/dogfood-2026-08-13/DOGFOOD-DEBRIEF.md`) documents the first real self-hosting
run and the bugs it exposed.

---

## 15. Separation of Composition and Execution

**Decision**: Prompt assembly (roko-compose) is a separate crate from agent dispatch
(roko-agent). The prompt is fully assembled before the agent is called.

**Why**: Coupling composition and execution makes three things hard:
1. **Inspection**: you cannot see what prompt was sent to the model without running
   the model. With separation, the assembled prompt is a string you can log, print,
   or test.
2. **Testing**: testing prompt assembly requires calling an LLM if the two are coupled.
   With separation, you can write unit tests for composition that never touch a network.
3. **Reproducibility**: replaying a prompt against a different model is trivial when
   the prompt is an independent artifact. With coupling, changing the model might
   change the prompt (different context windows, different tool formats).

**Where implemented**: roko-compose builds the prompt; roko-agent sends it.
The handoff is a `String` (the assembled system prompt) plus the dispatch parameters.

---

## 16. Four-Tier Signal Lifecycle

**Decision**: Signals have a monotonic lifecycle: Transient -> Working -> Consolidated
-> Persistent. Signals can only move forward, never backward.

**Why**: Not all information has the same retention value. A transient observation
from a single agent turn is speculative; a fact confirmed by multiple gate results
across multiple plans is durable. The tier system encodes this distinction:

- **Transient**: may be pruned aggressively (minutes). Default tier for new signals.
- **Working**: retained during active task scope. Promoted when gate-backed evidence
  confirms the signal.
- **Consolidated**: survives across sessions. Feeds learning subsystems.
- **Persistent**: permanent archive. Never auto-pruned.

Graduation requirements:
- Score above threshold (e.g., confidence > 0.7 for Working)
- Sufficient age (e.g., > 1 hour for Working, > 1 day for Consolidated)
- Sufficient accesses (e.g., > 3 for Consolidated)

**Where implemented**: `crates/roko-core/src/signal.rs` (SignalStatus enum,
GraduationError), `crates/roko-neuro/src/tier_progression.rs` (promotion logic).

---

## 17. Prospect Theory for Outcome Valuation

**Decision**: The affect engine values outcomes using prospect theory (Kahneman &
Tversky 1979): `gains = x^0.88`, `losses = -2.25 * |x|^0.88`.

**Why**: In prospect theory, losses loom larger than gains. A failure has 2.25x the
emotional impact of a success of the same magnitude. This matches the correct
behavior for a code generation system: you should be much more cautious after a
failure (it might indicate a systematic problem) than bold after a success (which
might be luck).

The curvature (0.88) provides diminishing sensitivity: the difference between 0 and
1 failures matters more than the difference between 9 and 10 failures.

**Where implemented**: `crates/roko-daimon/src/lib.rs` (the `prospect_value()` function).

---

## 18. Twelve Provider Backends (Not One)

**Decision**: Roko supports 12 distinct LLM provider backends, not just one.

| Backend | Transport | When to use |
|---------|-----------|-------------|
| AnthropicApi | HTTP API | Primary for Claude models |
| ClaudeCli | Subprocess | When CLI is installed, bare mode |
| CodexCli | Subprocess | For Codex-based models |
| OpenAiCompat | HTTP API | GPT-4, GPT-4o, and compatible APIs |
| CursorAcp | ACP protocol | When running inside Cursor editor |
| CursorCli | Subprocess | Cursor's CLI mode |
| PerplexityApi | HTTP API | Research tasks (grounded, cited) |
| GeminiApi | HTTP API | Google's Gemini models |
| GeminiCli | Subprocess | Google's CLI tools |
| CerebrasApi | HTTP API | Fast inference (Cerebras hardware) |
| Hermes | HTTP API | Self-hosted models |
| OpenClaw | HTTP API | Open-source model hosting |

**Why so many?** Because model selection is a core capability, not a configuration
detail. The cascade router learns which model works best for which task. With only
one backend, there is nothing to learn. With 12, the system can route documentation
tasks to a cheap model, complex refactoring to Claude Opus, and research to
Perplexity. The cost savings are substantial.

**Where implemented**: `crates/roko-agent/src/` -- each backend has its own module
(claude_agent.rs, cursor_agent.rs, codex_agent.rs, etc.) with a common `Agent` trait
interface.

---

## 19. Summary Table

| # | Decision | Core Rationale | Key Files |
|---|----------|---------------|-----------|
| 1 | Everything is a Signal | Uniform operations across all domains | signal.rs, kind.rs |
| 2 | Graph sole executor | One engine, one checkpoint format, one bug surface | engine.rs, topo.rs |
| 3 | 19 gates / 7 rungs | LLM output is unreliable; cheap gates first | gate_pipeline.rs |
| 4 | Cascade routing | Cold-start-safe model selection with learning | cascade_router.rs |
| 5 | Affect dispatch | Adapt to failure patterns, avoid blind retries | lib.rs (daimon) |
| 6 | JSONL state | Human-readable, no server, append-only, portable | file_substrate.rs |
| 7 | Workspace-local | Project isolation, offline-first, no cross-project coupling | lib.rs (fs) |
| 8 | 12 kernel traits | Principled decomposition of agent capabilities | traits.rs, cell.rs |
| 9 | CoALA mapping | Cognitive architecture prevents ad-hoc design | Multiple crates |
| 10 | 9-layer prompts | 3-4x quality gain, cache-aligned for cost | system_prompt_builder.rs |
| 11 | Signals + Pulses | Durable vs. ephemeral separation | pulse.rs, Bus trait |
| 12 | BLAKE3 content-addressing | Dedup, integrity, tamper-evident lineage DAG | hash.rs, content_hash() |
| 13 | HDC fingerprints | Semantic similarity without embeddings or GPU | hdc.rs (primitives) |
| 14 | Self-hosting driver | Every component justified by self-development needs | CLAUDE.md |
| 15 | Composition/execution split | Inspectable, testable, reproducible prompts | compose + agent crates |
| 16 | Four-tier lifecycle | Not all information has the same retention value | SignalStatus enum |
| 17 | Prospect theory valuation | Losses loom larger -- correct caution after failure | prospect_value() |
| 18 | 12 provider backends | Model selection is a core capability, not config | agent/ modules |
