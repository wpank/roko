# 13 — Builtin Cell Catalog

> Every Cell that ships with Roko, organized by protocol conformance. All Cells participate in predict-publish-correct ([doc-02](02-CELL.md)): each publishes predictions as Pulses, subscribes to calibration error topics, and updates.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse duality, demurrage, Kind system), [02-CELL](02-CELL.md) (9 protocols, predict-publish-correct, Verify redesign, EFE routing), [09-TELEMETRY](09-TELEMETRY.md) (Lens system, StateHub, c-factor), [12-CONNECTIVITY](12-CONNECTIVITY.md) (Connect protocol, exoskeleton)

---

## 1. Overview

Roko ships with **45+ built-in Cells** covering all nine protocols. Each Cell declares typed I/O, capabilities, cost estimates, and protocol conformance. Cells compose into Graphs ([doc-03](03-GRAPH.md)) -- the catalog is deliberately large because more composable pieces yield more emergent value (ERC-20 precedent: combinatorial explosion of compositions).

Naming convention: kebab-case noun-or-verb-phrase. Cells describe operations; Graphs describe outcomes.

**Every Cell is a learner.** Through predict-publish-correct ([doc-02](02-CELL.md)), each Cell publishes its prediction as a Pulse on `prediction.{name}`, receives calibration updates on `calibration.{name}.updated`, and adjusts. This is structural -- not a separate subsystem bolted on. The calibration pattern for each protocol is documented in [doc-10](10-LEARNING-LOOPS.md).

### Catalog summary

| Protocol | Built-in Cells | Count | Primary domain |
|---|---|---|---|
| Store | file-store, memory-store, chain-store | 3 | Signal persistence |
| Score | llm-scorer, rule-scorer, hdc-scorer | 3 | Signal quality rating |
| Verify | compile-gate, test-gate, clippy-gate, diff-gate, llm-judge-gate, consensus-gate | 6 | Truth checking, gates |
| Route | cascade-router, rule-router, cost-router | 3 | Candidate selection |
| Compose | prompt-composer, vcg-composer, greedy-composer | 3 | Signal combination |
| React | safety-reactor, budget-reactor, escalation-reactor, calibration-policy | 4 | Policy enforcement |
| Observe | 11 Lenses (see section 8) | 11 | Telemetry |
| Connect | chain-rpc, mcp, database, webhook, api | 5 | External I/O |
| Trigger | cron, webhook, file-watch, bus, chain-event, manual, signal-pattern | 7 | Event-driven Graph firing |
| Domain | 28 domain-specific Cells (see section 11) | 28+ | Authoring, research, execution, deploy, ops, comms, code-intel |

Total: **45+ built-in Cells** at launch. Domain-specific Cells bring the shipped catalog above 70.

---

## 2. Store Cells

Cells implementing the Store protocol: `put / get / query / query_similar / prune` Signals ([doc-01](01-SIGNAL.md)).

### `file-store`

Persists Signals as JSONL on the local filesystem.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Store |
| Input | `Signal` |
| Output | `SignalRef` |
| Capabilities | `FsRead`, `FsWrite` |
| Description | Default Store for local development. Append-only JSONL with content-addressed IDs. Supports query by Kind, time range, and HDC similarity (`query_similar` via brute-force SIMD, <1ms for 800K entries). Prune removes entries below demurrage balance threshold. |

```toml
[[nodes]]
id = "persist"
block = "file-store@^1"
[nodes.params]
path = ".roko/signals.jsonl"
```

### `memory-store`

In-memory Store for ephemeral Flows and testing.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Store |
| Input | `Signal` |
| Output | `SignalRef` |
| Capabilities | (none) |
| Description | Fast, volatile Store for unit tests and ephemeral Flows. All data lost on process exit. Same query interface as file-store including `query_similar`. |

```toml
[[nodes]]
id = "test-store"
block = "memory-store@^1"
```

### `chain-store`

Persists Signal commitments on-chain for tamper-evident audit.

| Field | Value |
|---|---|
| Version | 0.1.0 |
| Protocols | Store |
| Input | `Signal` |
| Output | `SignalRef` (with tx hash) |
| Capabilities | `Chain { read: true, write: true }` |
| Description | Writes content hashes (not full content) to an on-chain registry ([doc-18](18-ON-CHAIN-REGISTRIES.md)). Used for custody proofs and cross-agent attestation. Phase 2+. |

```toml
[[nodes]]
id = "anchor"
block = "chain-store@^0.1"
[nodes.params]
network = "base-sepolia"
```

---

## 3. Score Cells

Cells implementing the Score protocol: rate Signals along 5 dimensions (relevance, quality, confidence, novelty, utility) ([doc-01, section 5](01-SIGNAL.md)).

Score Cells predict 5-axis quality, publish the prediction as a Pulse, and receive corrections from gate verdicts and episode rewards via the calibration loop. Per-axis weights update via online least-squares.

### `llm-scorer`

Model-based Signal scoring.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Score |
| Input | `Signal` |
| Output | `ScoreResult { relevance, quality, confidence, novelty, utility }` |
| Capabilities | `Llm` |
| Description | Sends the Signal to an LLM with a scoring rubric. Returns five-dimensional score. Model selected via CascadeRouter (EFE-based, [doc-02](02-CELL.md)) unless overridden. Predicts score before scoring, publishes prediction, corrects from gate verdicts. |

```toml
[[nodes]]
id = "score"
block = "llm-scorer@^1"
[nodes.params]
rubric = "code-quality"
model = "claude-haiku-4-5"
```

### `rule-scorer`

Rule-based Signal scoring. Zero LLM cost.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Score |
| Input | `Signal` |
| Output | `ScoreResult` |
| Capabilities | (none) |
| Description | Evaluates Signals against declarative rules (regex matches, field presence, length thresholds, keyword density). Pure Rust pattern matching. T0-eligible (~80% of scoring ticks cost $0). |

```toml
[[nodes]]
id = "filter-score"
block = "rule-scorer@^1"
[nodes.params]
rules = [
  { field = "content.length", op = "gte", value = 100, dimension = "quality", weight = 0.3 },
  { field = "kind", op = "eq", value = "code", dimension = "relevance", weight = 0.5 },
]
```

### `hdc-scorer`

HDC vector similarity scoring.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Score |
| Input | `Signal` (with HDC fingerprint, [doc-01, section 9](01-SIGNAL.md)) |
| Output | `ScoreResult` (similarity-weighted) |
| Capabilities | (none) |
| Description | Computes HDC Hamming similarity between the input Signal and a reference set. Returns similarity as the relevance dimension. Sub-microsecond per comparison via POPCNT. Used for knowledge retrieval ranking. Cross-domain resonance gives 15% additive bonus when domains differ. |

```toml
[[nodes]]
id = "similarity"
block = "hdc-scorer@^1"
[nodes.params]
reference_set = "knowledge"
top_k = 20
```

---

## 4. Verify Cells

Cells implementing the Verify protocol: check Signals against truth criteria, produce Verdicts.

The Verify protocol is load-bearing ([doc-02](02-CELL.md)): it is the reward function (continuous `Verdict.reward`), the relabeling oracle (hindsight on failed trajectories, [doc-10](10-LEARNING-LOOPS.md)), the safety boundary (pre-action `verify_pre`), and the economic attestation (reputation flows from verified work via ERC-8004). All four learning loops depend on it.

**Key design decisions** carried by every Verify Cell:

1. **Pre-action and post-action.** Every Verify Cell implements `verify_pre()` (can veto execution) and `verify_post()` (evaluates results). Pre-action provides safety boundary; post-action provides reward signal.
2. **Continuous reward.** `Verdict.reward: f64` is a domain-specific learning signal alongside binary pass/fail. Feeds L1 parameter tuning and L2 strategy routing.
3. **Evidence typing.** `EvidenceCollector` is separate from `Criterion`. Evidence is collected by typed collectors (19 evidence kinds) and evaluated by criteria independently.
4. **Conjunctive hard + Pareto soft.** Hard criteria are AND -- all must pass. Soft criteria are multi-objective Pareto -- no weighted sum (Goodhart-resistant).

### `compile-gate`

Checks that code compiles.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Verify |
| Input | `Signal { kind: Code }` |
| Output | `Verdict { passed, reward, confidence, findings, evidence, hard_criteria, soft_criteria }` |
| Capabilities | `Shell { commands: ["cargo", "rustc", "tsc", "go"] }`, `FsRead` |
| Description | Runs the language-appropriate compiler. `verify_pre` checks source files exist and are well-formed. `verify_post` captures stderr as Finding Signals and compiler output as Evidence (kind: ProcessOutput). Reward: binary (1.0 on pass, 0.0 on fail). Hard criterion: zero compiler errors. Gate rung: 1. |

```toml
[[nodes]]
id = "compile"
block = "compile-gate@^1"
[nodes.params]
command = "cargo check --workspace"
```

### `test-gate`

Runs test suites and checks for regressions.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Verify |
| Input | `Signal { kind: Code }` |
| Output | `Verdict` |
| Capabilities | `Shell { commands: ["cargo", "npm", "pytest", "jest"] }`, `FsRead` |
| Description | `verify_pre` checks test runner available and workspace compilable. `verify_post` runs the test runner, parses results, emits per-test Findings. Evidence kind: TestResult. Reward = pass_rate (continuous 0.0..=1.0). Hard criterion: no regressions vs baseline. Soft criterion: coverage delta. Gate rung: 2. |

```toml
[[nodes]]
id = "test"
block = "test-gate@^1"
[nodes.params]
command = "cargo test --workspace"
target = "all"
```

### `clippy-gate`

Static analysis and lint checks.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Verify |
| Input | `Signal { kind: Code }` |
| Output | `Verdict` |
| Capabilities | `Shell { commands: ["cargo", "clippy", "eslint", "ruff"] }`, `FsRead` |
| Description | `verify_pre` checks linter installed and workspace compiles. `verify_post` runs linter with `-D warnings`. Findings are lint violations with location and severity. Evidence kind: LintReport. Reward = 1.0 - (violation_count / max_acceptable). Hard criterion: zero errors. Soft criterion: zero warnings. Gate rung: 3. |

```toml
[[nodes]]
id = "lint"
block = "clippy-gate@^1"
[nodes.params]
command = "cargo clippy --workspace --no-deps -- -D warnings"
```

### `diff-gate`

Validates that a diff matches expected patterns or constraints.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Verify |
| Input | `Signal { kind: Diff }` |
| Output | `Verdict` |
| Capabilities | `FsRead`, `Shell { commands: ["git"] }` |
| Description | `verify_pre` checks diff is well-formed and within size bounds. `verify_post` checks diff against constraints: max lines changed, no secret patterns, no binary files, restricted paths respected. Evidence kind: DiffAnalysis. Hard criteria: no secrets, paths allowed. Soft criteria: diff size, file count. Gate rung: 4. |

```toml
[[nodes]]
id = "diff-check"
block = "diff-gate@^1"
[nodes.params]
max_lines = 500
forbidden_patterns = ["API_KEY", "SECRET", "password"]
```

### `llm-judge-gate`

LLM-based quality evaluation producing a Verdict.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Score, Verify |
| Input | `Signal` |
| Output | `Verdict` (with ScoreResult embedded) |
| Capabilities | `Llm` |
| Description | Multi-protocol: Score + Verify. `verify_pre` checks input is scorable (non-empty, within token limit). `verify_post` sends Signal to LLM with evaluation criteria. Evidence kind: LlmJudgement. Reward = mean criterion score (continuous). Hard criterion: all criteria above threshold. Pairwise Bradley-Terry judges for inter-model comparison. Respects the Variance Inequality: verifier model must be spectrally cleaner than generator model. Gate rung: 5. |

```toml
[[nodes]]
id = "judge"
block = "llm-judge-gate@^1"
[nodes.params]
criteria = ["correctness", "completeness", "clarity"]
threshold = 0.7
model = "claude-sonnet-4-6"
```

### `consensus-gate`

Multi-evaluator consensus verification.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Verify |
| Input | `Vec<Verdict>` (from multiple upstream Verify Cells) |
| Output | `Verdict` (aggregate) |
| Capabilities | (none) |
| Description | Takes N Verdicts, applies consensus strategy (majority, unanimous, weighted, quorum). `verify_pre` checks minimum voter count met. `verify_post` aggregates: reward = mean of upstream rewards. Evidence = union of upstream evidence sets. Hard criteria: consensus threshold met. Soft criteria: voter agreement spread. Gate rung: 6. |

```toml
[[nodes]]
id = "consensus"
block = "consensus-gate@^1"
[nodes.params]
strategy = "majority"
min_voters = 3
```

---

## 5. Route Cells

Cells implementing the Route protocol: select among candidates, learn from outcomes.

Route Cells use **Expected Free Energy (EFE)** for selection (Friston 2006; [doc-02](02-CELL.md)). EFE naturally balances exploration (epistemic value) and exploitation (pragmatic value) while being cost-aware. Each cognitive timescale (T0/T1/T2 gating, L2 routing) uses a different free-energy lower bound. Route receives `regime: Signal` for context-aware selection -- Calm regime favors exploration, Crisis regime favors cheapest reliable option.

### `cascade-router`

EFE-based model routing with learning. **Uses EFE, not LinUCB.**

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Route, Observe |
| Input | `Vec<Signal>` (candidates) |
| Output | `RouteResult { selected, confidence, reason, efe_score }` |
| Capabilities | (none) |
| Description | Selects best candidate (typically a model) using Expected Free Energy. Balances exploration (epistemic value from uncertain arms) against exploitation (pragmatic value from known-good arms), conditioned on current regime. Calm -> explore more. Crisis -> exploit cheapest reliable. Observes outcomes via predict-publish-correct: predicts selection will succeed, receives gate verdict as outcome, updates EFE posteriors. Persists state to `.roko/learn/cascade-router.json`. Multi-protocol: Route + Observe. |

```toml
[[nodes]]
id = "model-select"
block = "cascade-router@^1"
[nodes.params]
candidates = ["claude-opus-4-6", "claude-sonnet-4-6", "claude-haiku-4-5"]
context_features = ["task_complexity", "domain", "budget_remaining"]
```

### `rule-router`

Deterministic rule-based routing.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Route |
| Input | `Vec<Signal>` (candidates) |
| Output | `RouteResult` |
| Capabilities | (none) |
| Description | Evaluates candidates against declarative rules. No learning -- deterministic selection. Used for fixed routing policies (e.g., "always use Opus for security reviews"). Still publishes predictions for calibration tracking. |

```toml
[[nodes]]
id = "fixed-route"
block = "rule-router@^1"
[nodes.params]
rules = [
  { condition = "task.domain == 'security'", select = "claude-opus-4-6" },
  { condition = "task.budget_usd < 0.10", select = "claude-haiku-4-5" },
  { condition = "true", select = "claude-sonnet-4-6" },
]
```

### `cost-router`

Cheapest-viable candidate selection.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Route |
| Input | `Vec<Signal>` (candidates with cost estimates) |
| Output | `RouteResult` |
| Capabilities | (none) |
| Description | Selects cheapest candidate meeting minimum quality threshold. Useful for cost-sensitive Graphs and Conservation/Declining vitality phases ([doc-07](07-AGENT-RUNTIME.md)) where budget pressure favors cheaper options. |

```toml
[[nodes]]
id = "cheap-route"
block = "cost-router@^1"
[nodes.params]
min_quality = 0.6
prefer = "cheapest"
```

---

## 6. Compose Cells

Cells implementing the Compose protocol: combine Signals under budget into one Signal.

Compose Cells predict prompt-fits-budget-and-wins-gate, publish the prediction as a Pulse, and receive corrections from token count and gate verdict. Section effect beta-distributions track which context sections correlate with gate success, making context assembly learnable over time ([doc-10](10-LEARNING-LOOPS.md)).

### `prompt-composer`

9-layer system prompt assembly.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Compose |
| Input | `Vec<Signal>` (context Signals: role, task, knowledge, history, constraints) |
| Output | `Signal { kind: Text }` (assembled prompt) |
| Capabilities | (none) |
| Description | Assembles system prompt from up to 9 layers (role, domain, task, context, knowledge, history, constraints, tools, format). Budget-aware: truncates lower-priority layers to fit token limit. Maps to existing `RoleSystemPromptSpec` in orchestrate.rs. |

```toml
[[nodes]]
id = "build-prompt"
block = "prompt-composer@^1"
[nodes.params]
role = "strategist"
max_tokens = 8000
priority_order = ["role", "task", "context", "knowledge", "constraints"]
```

### `vcg-composer`

VCG auction-based Signal combination (Vickrey-Clarke-Groves).

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Compose |
| Input | `Vec<Signal>` (with bids from context bidders) |
| Output | `Signal` (combined) |
| Capabilities | (none) |
| Description | Runs a VCG auction among context bidders (Neuro, Task, Research, Heuristic, Episode, Pheromone, Affect, System). Each bidder declares value for token budget. VCG allocates efficiently -- pay your externality. Section effect tracking via beta-distribution posteriors adjusts bidder valuations over time. Built and exported but greedy path currently dominates at runtime. Novelty attenuation: `1/(1+ln(freq))` ([doc-01](01-SIGNAL.md)). |

```toml
[[nodes]]
id = "auction-compose"
block = "vcg-composer@^1"
[nodes.params]
max_tokens = 16000
bidders = ["neuro", "task", "research"]
```

### `greedy-composer`

Top-K by score Signal combination.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Compose |
| Input | `Vec<Signal>` (scored) |
| Output | `Signal` (combined) |
| Capabilities | (none) |
| Description | Sorts input Signals by score, takes top K that fit within budget. Simple, fast, predictable. The default composition strategy. |

```toml
[[nodes]]
id = "top-k"
block = "greedy-composer@^1"
[nodes.params]
max_signals = 10
max_tokens = 4000
sort_by = "relevance"
```

---

## 7. React Cells

Cells implementing the React protocol: watch Pulse streams, emit new Signals as interventions.

React Cells operate on **Pulses** (ephemeral), not Signals ([doc-01](01-SIGNAL.md)). This is a breaking change from v1 where Policy took `&[Engram]`. The rationale: policies react to live events (heartbeats, gate verdicts, budget warnings, calibration updates), not stored artifacts. React output can include both Pulses (ephemeral reactions) and Signals (durable reactions that graduate).

```rust
pub struct ReactOutput {
    pub pulses: Vec<Pulse>,    // ephemeral reactions
    pub signals: Vec<Signal>,  // durable reactions (graduated)
}
```

### `safety-reactor`

Halt on danger signals.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | React |
| Input | `&[Pulse]` (subscribed topics) |
| Output | `ReactOutput` |
| Capabilities | (none) |
| Description | Monitors Pulse stream for safety violations: cost anomalies, permission escalation attempts, infinite loops, prompt injection indicators. Emits halt Pulses that the execution engine respects. Graduates critical safety events to durable Signals for audit. |

```toml
[[nodes]]
id = "safety"
block = "safety-reactor@^1"
[nodes.params]
sensitivity = "high"
actions = ["halt", "alert"]
```

### `budget-reactor`

Alert and throttle on budget thresholds.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | React |
| Input | `&[Pulse]` (CostReport and BudgetAlert Pulses) |
| Output | `ReactOutput` |
| Capabilities | (none) |
| Description | Watches cost Pulses from BudgetLens. At 75% budget: emits warning Pulse. At 90%: emits throttle Pulse (engine switches to cheaper models via CascadeRouter). At 100%: emits halt Pulse. Thresholds configurable. Graduates budget breach events to durable Signals. |

```toml
[[nodes]]
id = "budget-watch"
block = "budget-reactor@^1"
[nodes.params]
warn_pct = 75
throttle_pct = 90
halt_pct = 100
```

### `escalation-reactor`

Notify humans on escalation conditions.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | React |
| Input | `&[Pulse]` (gate results, error events, confidence updates) |
| Output | `ReactOutput` |
| Capabilities | `Net` (for notification delivery) |
| Description | When conditions are met (repeated failures, confidence below threshold, structural changes proposed), emits notification Pulses routed to configured channels (Slack, email, dashboard). Graduates escalation decisions to durable Signals for audit trail. Feeds the Agent Inbox surface ([doc-16](16-SURFACES.md)). |

```toml
[[nodes]]
id = "escalate"
block = "escalation-reactor@^1"
[nodes.params]
conditions = ["gate_fail_count > 3", "confidence < 0.3"]
channels = ["slack", "dashboard"]
```

### `calibration-policy`

Per-operator calibration from prediction/outcome streams. The structural implementation of predict-publish-correct ([doc-02](02-CELL.md), [doc-10](10-LEARNING-LOOPS.md)).

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | React |
| Input | `&[Pulse]` (subscribed to `prediction.**` and `outcome.**` topics) |
| Output | `ReactOutput` |
| Capabilities | (none) |
| Description | Subscribes to all `prediction.{operator}` and `outcome.{operator}` Pulse topics. Joins predictions with outcomes by `lineage_hint` (content hash). Computes calibration error and publishes updates on `calibration.{operator}.updated`. Maintains per-operator calibration state: Scorer (online least-squares per axis), Router (EFE posterior update), Composer (section effect beta update), Gate (threshold EMA), Policy (per-policy online calibration). Persists to `.roko/learn/calibration.json`. |

```toml
[[nodes]]
id = "calibration"
block = "calibration-policy@^1"
[nodes.params]
topics = ["prediction.**", "outcome.**"]
persist_interval = "60s"
```

---

## 8. Observe Cells (Lenses)

Cells implementing the Observe protocol. Lenses are read-only observers that emit observation Signals onto the Bus. They never modify what they observe. See [doc-09](09-TELEMETRY.md) for the full Lens system, StateHub projections, and composition rules.

### 8.1 `cost-lens`

| Protocols | Observe | Scope | Cell, Graph, Agent |
|---|---|---|---|
| **Emits** | `CostReport` Signals per interval | | |
| **Description** | Aggregates USD cost across observed scope. Emits periodic CostReport Signals with total, rate, breakdown by model/provider. |

### 8.2 `latency-lens`

| Protocols | Observe | Scope | Cell, Graph |
|---|---|---|---|
| **Emits** | Latency distribution Signals (p50, p95, p99) | | |
| **Description** | Tracks execution duration across observed scope. Emits percentile distributions at configurable intervals. |

### 8.3 `quality-lens`

| Protocols | Observe | Scope | Graph |
|---|---|---|---|
| **Emits** | Pass-rate Signals from Verify Cells | | |
| **Description** | Observes Verify Cell Verdicts. Tracks `verify_pre` vetoes and `verify_post` outcomes. Emits rolling pass rate, mean continuous reward, evidence type breakdown, hard-criteria failure rate, per-gate rung breakdown. |

### 8.4 `efficiency-lens`

| Protocols | Observe | Scope | Agent |
|---|---|---|---|
| **Emits** | Tokens-per-task ratio Signals | | |
| **Description** | Tracks token usage relative to task completion. Lower ratio = more efficient agent. Feeds CascadeRouter's EFE learning loop. |

### 8.5 `error-lens`

| Protocols | Observe | Scope | Cell, Graph, Agent |
|---|---|---|---|
| **Emits** | Classified error report Signals | | |
| **Description** | Categorizes errors by type (timeout, capability, external, logic, cancelled). Emits error frequency and trend data. |

### 8.6 `drift-lens`

| Protocols | Observe | Scope | Memory |
|---|---|---|---|
| **Emits** | Knowledge quality degradation Signals | | |
| **Description** | Monitors a Memory Cell for staleness: entries losing balance via demurrage ([doc-01, section 6](01-SIGNAL.md)), citations gone dead, scores declining. Emits drift alerts with balance distribution, cold-entry count, heuristic calibration averages. |

### 8.7 `budget-lens`

| Protocols | Observe | Scope | Agent, Space |
|---|---|---|---|
| **Emits** | Threshold alert Signals | | |
| **Description** | Watches budget consumption rate. Emits alerts at configurable thresholds (warn, throttle, halt). Consumed by budget-reactor. |

### 8.8 `trend-lens`

| Protocols | Observe | Scope | Any other Lens |
|---|---|---|---|
| **Emits** | Slope, EMA, derivative Signals | | |
| **Description** | Meta-Lens: observes another Lens's output stream and computes statistical trends. Chains with any other Lens (e.g., trend-lens watching cost-lens computes cost trajectory). |

### 8.9 `anomaly-lens`

| Protocols | Observe | Scope | Any other Lens |
|---|---|---|---|
| **Emits** | Statistical outlier alert Signals | | |
| **Description** | Meta-Lens: detects anomalies in another Lens's output using Z-score and IQR methods. Configurable sensitivity. Feeds safety-reactor. |

### 8.10 `usage-lens`

| Protocols | Observe | Scope | Space, Marketplace |
|---|---|---|---|
| **Emits** | Install/run/fork count Signals | | |
| **Description** | Tracks usage metrics for published artifacts: installs, active runs, forks, error rates, cost averages. Powers marketplace creator analytics and trending algorithms ([doc-15](15-MARKETPLACE-AND-SHARING.md)). |

### 8.11 `collective-intelligence-lens`

| Protocols | Observe | Scope | Space (across all agents) |
|---|---|---|---|
| **Emits** | `Signal { kind: CFactorReport }` | | |
| **Description** | Computes the **c-factor** -- collective intelligence as a runtime observable (Woolley et al. 2010, *Science*). Derived from four components: turn-taking entropy, peer prediction accuracy, citation reciprocity, HDC diversity. The c-factor gates L4 structural adaptation ([doc-10](10-LEARNING-LOOPS.md)) -- only evolve configurations that increase genuine collective intelligence. It is a covariate, not an objective. |

```toml
[[lenses]]
name = "collective-intelligence"
block = "roko:collective-intelligence-lens@^1.0"
scope = "space"
[lenses.params]
interval = "300s"
min_agents = 2
```

---

## 9. Connect Cells (Connectors)

Cells implementing the Connect protocol: `connect / query / execute / health / disconnect`. See [doc-12](12-CONNECTIVITY.md) for the full Connector model and exoskeleton protocols.

### `chain-rpc-connector`

| Version | 1.0.0 | Capabilities | `Chain { read: true, write: configurable }`, `Net` |
|---|---|---|---|
| **Description** | Connects to blockchain RPC endpoints. Supports read queries (balances, events, contract state) and write operations (transactions). Health check via `eth_blockNumber`. Publishes Feeds on Bus topics per chain. |

### `mcp-connector`

| Version | 1.0.0 | Capabilities | (depends on MCP server) |
|---|---|---|---|
| **Description** | Wraps an MCP server as a Connector. Auto-discovered from `agent.mcp_config`. Exposes MCP tools as queryable operations. Every invocation carries CellInput/CellOutput ([doc-12, section 2.1](12-CONNECTIVITY.md)). |

### `database-connector`

| Version | 1.0.0 | Capabilities | `Net` (remote), `FsRead` (SQLite) |
|---|---|---|---|
| **Description** | Connection-pooled database access. Read queries via `query()`, mutations via `execute()`. Health check via `SELECT 1`. |

### `webhook-connector`

| Version | 1.0.0 | Capabilities | `Net { domains: configurable }` |
|---|---|---|---|
| **Description** | Delivers Signals to external HTTP endpoints. Supports retry with exponential backoff, HMAC signature, configurable headers. |

### `api-connector`

| Version | 1.0.0 | Capabilities | `Net { domains: configurable }` |
|---|---|---|---|
| **Description** | General-purpose API client. Supports authentication strategies (Bearer, API key, OAuth2). Rate limiting and circuit breaker built in. |

---

## 10. Trigger Cells

Cells implementing the Trigger protocol: `arm / disarm / poll` for events that fire Graphs. See [doc-06](06-TRIGGER-SYSTEM.md).

### `cron-trigger`

Schedule-based Graph firing. Standard 5-field cron syntax plus `@hourly`, `@daily`, `@weekly` shortcuts.

### `webhook-trigger`

Inbound HTTP webhook listener. Supports payload filtering, HMAC verification, path-based routing. Used for GitHub webhooks, Slack events.

### `file-watch-trigger`

Filesystem change detection. Debounced with configurable delay. Uses `notify::RecommendedWatcher`. Supports glob patterns.

### `bus-trigger`

Pulse Bus topic listener. Fires when a Pulse of a specific Kind appears on a Bus topic. The primary internal event mechanism for chaining Graphs.

### `chain-event-trigger`

Smart contract event listener (EVM log topics). Phase 2+.

### `manual-trigger`

User-initiated Graph firing. Every `roko run <graph>` creates an implicit manual-trigger.

### `signal-pattern-trigger`

HDC-similarity pattern matching on Pulse stream. Fires when a Pulse with HDC fingerprint similar to a reference pattern appears above a configurable threshold. Enables content-addressable event matching.

---

## 11. Domain-Specific Cells

Beyond protocol Cells, Roko ships domain Cells that implement the base `Cell` trait and compose into common Graphs.

### 11.1 Authoring

| Cell | Description | Capabilities |
|---|---|---|
| `fs-walk` | Walk directory, emit file list | `FsRead` |
| `markdown-segment` | Split markdown by headings | (none) |
| `markdown-classify` | Classify segments by intent (context, task, spec, reference) | `Llm` |
| `doc-cluster` | Cluster classified segments into logical groups | `Llm` |
| `prd-synthesize` | Generate a PRD from clustered segments | `Llm` |
| `prd-audit` | Audit a PRD for contradictions, vague language, missing criteria | `Llm` |
| `prd-plan` | Generate tasks.toml plan from published PRD | `Llm` |
| `plan-validate` | Static analysis on tasks.toml: cycles, missing deps, orphans | (none) |
| `artifact-persist` | Persist produced artifacts to Store | `FsWrite` |

### 11.2 Research

| Cell | Description | Capabilities |
|---|---|---|
| `web-search` | Web search via Perplexity or configured provider | `Net`, `Llm` |
| `academic-search` | arXiv + Semantic Scholar paper search | `Net` |
| `citation-check` | Verify cited claims against sources | `Net`, `Llm` |
| `fact-check` | Check factual claims against a corpus | `Llm` |
| `knowledge-ingest` | Import Signals into a Memory Cell | `FsWrite` |
| `knowledge-link` | Discover HDC-similar cross-domain bridges | (none) |

### 11.3 Execution

| Cell | Description | Capabilities |
|---|---|---|
| `agent-dispatch` | Dispatch task to an Agent (Claude CLI, API, Codex, etc.) | `Llm`, `Shell` |
| `test-run` | Run a test suite (`cargo test`, `npm test`, `pytest`) | `Shell`, `FsRead` |
| `build` | Compile/bundle (`cargo build`, `vite build`) | `Shell`, `FsRead` |
| `script-run` | Execute a script with capability gating | `Shell` |
| `refactor-apply` | Apply a refactor pattern across files | `FsRead`, `FsWrite`, `Llm` |

### 11.4 Deploy

| Cell | Description | Capabilities |
|---|---|---|
| `deploy-railway` | Deploy to Railway | `Net`, `Shell`, `Secrets` |
| `deploy-fly` | Deploy to Fly.io | `Net`, `Shell`, `Secrets` |
| `deploy-vercel` | Deploy to Vercel | `Net`, `Shell`, `Secrets` |
| `deploy-shell` | Custom shell-script deploy | `Shell`, `Secrets` |
| `smoke-test` | Post-deploy endpoint + page verification | `Net` |
| `rollback` | Revert a failed deployment | `Net`, `Shell`, `Secrets` |

### 11.5 Operations

| Cell | Description | Capabilities |
|---|---|---|
| `backup` | Snapshot `.roko/` to configured remote | `FsRead`, `Net`, `Secrets` |
| `restore` | Restore from backup snapshot | `FsWrite`, `Net`, `Secrets` |
| `gc` | Garbage collect old runs/artifacts/episodes | `FsWrite` |
| `cost-report` | Generate per-Graph/Agent/model cost summary | (none) |
| `dependency-update` | Bump deps and run gates | `Shell`, `FsWrite` |
| `dependency-audit` | Check deps for CVEs and abandonment | `Shell` |

### 11.6 Communication

| Cell | Description | Capabilities |
|---|---|---|
| `slack-notify` | Post message to Slack channel | `Net`, `Secrets` |
| `github-comment` | Post comment on PR or issue | `Net`, `Secrets` |
| `email-send` | Send email via configured provider | `Net`, `Secrets` |
| `discord-notify` | Post message to Discord channel | `Net`, `Secrets` |

### 11.7 Code Intelligence

| Cell | Description | Capabilities |
|---|---|---|
| `index-build` | Build the code-intel index | `FsRead`, `Shell` |
| `code-search` | Semantic + symbolic code search | `FsRead` |
| `type-check` | Language-specific type checker | `Shell`, `FsRead` |
| `symbol-graph` | Build symbol-relationship graph | `FsRead` |
| `impact-analysis` | Given a diff, report downstream impacts | `FsRead` |

---

## 12. Synergy Patterns

The catalog is designed for emergent composition. Cells combine via Graphs to create synergistic pipelines that no single Cell could provide:

| # | Pattern | Cells | Trigger | Effect |
|---|---|---|---|---|
| 1 | **Doc-to-Plan** | `fs-walk` + `markdown-classify` + `doc-cluster` + `prd-synthesize` + `prd-plan` | `file-watch-trigger` on docs/ | New docs auto-produce plans |
| 2 | **PR Review** | `webhook-trigger` (GitHub) + `agent-dispatch` (code-review) + `github-comment` | `webhook-trigger` | Every PR gets automated review |
| 3 | **Code-to-Docs** | `file-watch-trigger` on src/ + `impact-analysis` + `agent-dispatch` (doc-writer) | `file-watch-trigger` | Docs stay in sync with code |
| 4 | **Local CI** | `file-watch-trigger` on src/ + `compile-gate` + `test-gate` + `clippy-gate` | `file-watch-trigger` | Continuous local verification |
| 5 | **Ship Pipeline** | `build` + `deploy-railway` + `smoke-test` + `slack-notify` | `manual-trigger` | One-command ship |
| 6 | **Idea Pipeline** | `web-search` + `prd-synthesize` + `prd-plan` + `agent-dispatch` | `manual-trigger` | Idea to shipped code |
| 7 | **Knowledge GC** | `gc` + `knowledge-link` | `cron-trigger` weekly | Pruning + new connections |
| 8 | **Cost Alert** | `cost-lens` + `trend-lens` + `budget-reactor` + `escalation-reactor` | `bus-trigger` on CostReport | Auto-triage on cost spikes |
| 9 | **Learning Loop** | `cascade-router` + `efficiency-lens` + `trend-lens` + `calibration-policy` | Implicit (every run) | System improves model selection per Cell |
| 10 | **Collective Intelligence** | `collective-intelligence-lens` + `trend-lens` + `anomaly-lens` | `cron-trigger` or continuous | c-factor tracked and trends monitored |

These patterns are not hardcoded pipelines. They emerge from composing individual Cells via Graphs and Triggers. Users discover useful patterns and publish them as Graphs in the marketplace ([doc-15](15-MARKETPLACE-AND-SHARING.md)).

---

## 13. Implementation Tiers

| Tier | When | Cells |
|---|---|---|
| **Tier 0** (kernel) | First | All Verify Cells (gates), `prompt-composer`, `cascade-router`, `agent-dispatch`, `file-store`, `prd-synthesize`, `prd-plan`, `calibration-policy` |
| **Tier 1** (authoring) | First | `fs-walk`, `markdown-classify`, `doc-cluster`, `prd-audit`, `citation-check`, `artifact-persist`, `knowledge-ingest` |
| **Tier 2** (deploy + verify) | Soon | All Deploy Cells, `smoke-test`, `llm-judge-gate`, `consensus-gate`, `webhook-trigger`, `file-watch-trigger` |
| **Tier 3** (operations) | Soon | `backup`, `gc`, `cost-report`, `dependency-update`, all Communication Cells, `cron-trigger` |
| **Tier 4** (knowledge + observe) | Mid | All Observe Cells (Lenses) including `collective-intelligence-lens`, all React Cells, `knowledge-link`, `hdc-scorer`, `signal-pattern-trigger` |
| **Tier 5** (chain + advanced) | Late | `chain-store`, `chain-rpc-connector`, `chain-event-trigger`, `vcg-composer` |

---

## 14. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Every Tier 0 Cell ships with typed I/O, capabilities, and a TOML usage example | `roko cell list` returns the full Tier 0 set |
| Each Cell validates clean when composed in a Graph | `roko graph validate` passes for all builtin Graphs |
| Capability intersection enforced at runtime per Cell | Cell requesting ungranteed capability fails closed |
| Synergy patterns from section 12 work via Graph + Trigger composition | Multi-step integration test |
| Every Cell declares cost estimation | `roko cell show <name>` displays estimates |
| Every catalog entry's TOML lives in `<roko-install>/builtin/cells/` | Filesystem invariant check |
| All Verify Cells implement both `verify_pre` and `verify_post` | Compile check + unit test per gate |
| All Verify Cells emit typed Evidence alongside Findings | Unit test: Verdict contains Evidence with correct EvidenceKind |
| Verify Cells produce continuous `Verdict.reward` (not just binary) | Unit test: reward is f64 in (0.0..=1.0) |
| Verify Cells carry both `hard_criteria` and `soft_criteria` in Verdict | Compile check |
| CascadeRouter uses EFE (not LinUCB) for selection | Unit test: route output includes `efe_score` field |
| CascadeRouter responds to regime conditioning | Test: Calm -> higher exploration, Crisis -> exploit |
| All React Cells accept `&[Pulse]` input (not `&[Signal]`) | Compile check |
| CalibrationPolicy joins predictions with outcomes by lineage_hint | Integration test: publish prediction + outcome, verify calibration update |
| CalibrationPolicy publishes on `calibration.{operator}.updated` | Integration test: verify Pulse topic and payload |
| CollectiveIntelligenceLens computes c-factor from 4 components | Integration test with multi-agent Space |
| All Cells publish predictions for calibration tracking | Integration test: run Cell, verify prediction Pulse emitted |
