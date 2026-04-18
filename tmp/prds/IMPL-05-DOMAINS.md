# IMPL-05: Domains and arenas

Implements PRD-06 (Domain-specialized agents and benchmark arenas).

## Context

Roko is a Rust workspace at `/Users/will/dev/nunchi/roko/roko/` with 18 crates and ~177K LOC. It builds agents that build themselves: read PRDs, generate plans, execute tasks via Claude, validate with gates, persist results.

The domain profile system (`DomainProfile` enum with 6 variants) already exists in roko-core. It defines gate rungs, tool categories, and context fractions per domain. The chain crate has triage pipelines, ISFR oracles, marketplace escrow, and observer infrastructure. The gap is threefold:

1. **Domain profiles are not wired into agent dispatch.** The orchestrator does not select gates, tools, or context based on the task's domain.
2. **No arena framework.** There is no way to run benchmark batches (SWE-bench, self-hosting) through the orchestrator and score results.
3. **Domain-specific agent extensions are not connected.** The chain crate has triage, ISFR, and marketplace code that compiles but is never called from the agent dispatch path.

This plan addresses all three across six phases.

---

## Crate map

| Crate | Path | Role in this plan |
|---|---|---|
| roko-core | `crates/roko-core/` | `DomainProfile`, `TypedContext`, `TaskDomain`, config schema |
| roko-cli | `crates/roko-cli/` | Orchestrator, CLI entry point, task parser |
| roko-agent | `crates/roko-agent/` | Agent dispatcher, multi-agent pool, safety layer |
| roko-gate | `crates/roko-gate/` | Gate pipeline, rung dispatch, adaptive thresholds |
| roko-chain | `crates/roko-chain/` | Chain client, triage pipeline, ISFR, marketplace, observer |
| roko-compose | `crates/roko-compose/` | Prompt composition, attention bidders, enrichment |
| roko-learn | `crates/roko-learn/` | Episode logger, C-factor, playbooks, research pipeline |
| roko-runtime | `crates/roko-runtime/` | Process supervisor, heartbeat, event bus |
| roko-neuro | `crates/roko-neuro/` | Knowledge store, context assembly |

---

## Phase 1: Wire DomainProfile into orchestration

Status: `DomainProfile` exists with 6 variants (Coding, Research, Chain, DataMl, Ops, Writing) and `TypedContext` for overrides. Neither is used at dispatch time.

### Task 1.1: Parse domain from task TOML

**Read first:**
- `crates/roko-core/src/domain_profile.rs` -- `DomainProfile` enum, `from_label()`, `TypedContext`
- `crates/roko-cli/src/task_parser.rs` -- how tasks are parsed from plan TOML files
- A sample plan file in `.roko/plans/` or `tmp/` (look for `[[tasks]]` sections)

**What to do:**
1. In the task parser, look for a `domain` field on each `[[tasks]]` entry:
   ```toml
   [[tasks]]
   id = "T1"
   title = "Implement ISFR aggregation"
   domain = "chain"  # <-- parse this
   tier = "integrative"
   ```
2. If the `domain` field exists, parse it via `DomainProfile::from_label()`.
3. If the field is missing, infer the domain from task metadata:
   - If the task title or description contains "chain", "defi", "transaction", "contract" -> `Chain`
   - If it contains "research", "paper", "citation" -> `Research`
   - If it contains "deploy", "monitor", "incident" -> `Ops`
   - If it contains "doc", "readme", "tutorial" -> `Writing`
   - Default: `Coding`
4. Store the parsed `DomainProfile` on the task struct.

**Files to modify:**
- `crates/roko-cli/src/task_parser.rs`
- The task struct definition (find it via `grep -rn 'struct.*Task' crates/roko-core/src/ --include='*.rs'` or in `crates/roko-cli/src/`)

**Test:**
- Parse a task with `domain = "chain"`. Assert `DomainProfile::Chain`.
- Parse a task without `domain` field but title "Deploy monitoring stack". Assert `DomainProfile::Ops`.
- Parse a task with unknown domain string. Assert fallback to `Coding`.

**Acceptance:**
- [ ] `domain` field parsed from task TOML
- [ ] Domain inference from title/description as fallback
- [ ] Default to Coding for unrecognized domains

---

### Task 1.2: Route gates by domain profile

**Read first:**
- `crates/roko-core/src/domain_profile.rs` -- `DomainProfile::default_gate_rungs()`
- `crates/roko-cli/src/orchestrate.rs` -- search for `select_rungs` or `gate_pipeline` or `enrich_rung_config`
- `crates/roko-gate/src/rung_selector.rs` -- `select_rungs()`, `Rung`, `RungCaps`

**What to do:**
1. In orchestrate.rs, where the gate pipeline is configured for a task, read the task's domain profile.
2. Use the profile's `default_gate_rungs()` to filter or select rungs:
   ```rust
   let domain_rungs = task.domain_profile
       .map(|dp| dp.default_gate_rungs())
       .unwrap_or(&["compile", "test", "clippy", "diff_review"]);

   // Feed into rung selection
   let selected = select_rungs(plan_complexity, &rung_caps, Some(domain_rungs));
   ```
3. If `TypedContext` overrides exist (from config), prefer those over the default domain rungs.
4. Log which rungs were selected and why: `tracing::info!(domain = %domain, rungs = ?selected, "gate rungs selected for domain")`.

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-gate/src/rung_selector.rs` -- add a `domain_filter` parameter to `select_rungs()` if it does not already accept one

**Test:**
- A `Chain` domain task gets `["compile", "simulation", "audit", "diff_review"]` rungs.
- A `Research` domain task gets `["content_review", "citation_check"]` rungs.
- A task with `TypedContext` override `gate_rungs = ["custom_review"]` gets `["custom_review"]` regardless of domain.

**Acceptance:**
- [ ] Gate rungs vary by domain profile
- [ ] TypedContext overrides take precedence
- [ ] Rung selection logged with domain attribution

---

### Task 1.3: Filter tools by domain profile

**Read first:**
- `crates/roko-core/src/domain_profile.rs` -- `DomainProfile::tool_categories()`
- `crates/roko-cli/src/orchestrate.rs` -- search for `ToolRegistry` or `tool_filter` or `allowed_tools`
- `crates/roko-core/src/` -- `ToolRegistry` struct

**What to do:**
1. After loading the task's domain profile, filter the tool registry to include only tools matching the domain's categories:
   ```rust
   let categories = task.domain_profile
       .map(|dp| dp.tool_categories())
       .unwrap_or(&["read", "write", "edit", "search", "exec", "test"]);

   let filtered_tools = tool_registry.filter_by_categories(categories);
   ```
2. If `ToolRegistry::filter_by_categories()` does not exist, implement it:
   - Each tool in the registry should have a `category` field (or infer it from the tool name).
   - Return a filtered registry containing only tools whose category matches the allowlist.
3. Pass the filtered tool set to the agent dispatcher.

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-core/src/` -- wherever `ToolRegistry` is defined (search for `struct ToolRegistry`)

**Test:**
- A `Research` domain task gets tools in `["read", "search", "web"]` categories only.
- A `Coding` domain task gets tools in `["read", "write", "edit", "search", "exec", "test"]`.
- Filtering does not remove tools that have no explicit category (they default to "other" and are excluded unless "other" is in the allowlist).

**Acceptance:**
- [ ] Tool set filtered by domain profile categories
- [ ] Filtered tools passed to agent dispatcher
- [ ] TypedContext override for tool_categories works

---

### Task 1.4: Apply context fraction by domain

**Read first:**
- `crates/roko-core/src/domain_profile.rs` -- `DomainProfile::context_fraction()`
- `crates/roko-compose/src/` -- `PromptComposer`, search for `max_tokens` or `context_window` or `budget`

**What to do:**
1. When assembling the prompt for a task, use the domain's context fraction to set the prompt budget:
   ```rust
   let fraction = task.domain_profile
       .map(|dp| dp.context_fraction())
       .unwrap_or(0.6);
   let prompt_budget = (context_window_tokens as f64 * fraction) as usize;
   ```
2. Pass `prompt_budget` to the `PromptComposer` or whatever token-budgeting mechanism is used.
3. Research tasks get 80% of the context window. Ops tasks get 50%. Coding tasks get 60%.

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-compose/src/` -- if the composer does not accept a budget parameter, add one

**Test:**
- A Research task gets prompt budget = 0.8 * context_window.
- An Ops task gets prompt budget = 0.5 * context_window.
- TypedContext override `context_fraction = 0.95` applies correctly.

**Acceptance:**
- [ ] Prompt budget scales by domain profile
- [ ] TypedContext override respected
- [ ] Budget passed to the prompt composer

---

### Task 1.5: Add [domains] section to roko.toml

**Read first:**
- `crates/roko-core/src/config/schema.rs` -- `RokoConfig` struct, existing sections
- `crates/roko-core/src/domain_profile.rs` -- `TypedContext`

**What to do:**
1. Add a `[domains]` section to the config schema:
   ```rust
   #[derive(Debug, Clone, Default, Serialize, Deserialize)]
   pub struct DomainsConfig {
       /// Custom domain profiles. Key is the domain label.
       #[serde(default)]
       pub profiles: HashMap<String, DomainOverride>,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct DomainOverride {
       /// Override gate rungs for this domain.
       pub gate_rungs: Option<Vec<String>>,
       /// Override tool categories for this domain.
       pub tool_categories: Option<Vec<String>>,
       /// Override context fraction for this domain.
       pub context_fraction: Option<f64>,
       /// Custom key-value metadata.
       #[serde(default)]
       pub metadata: HashMap<String, String>,
   }
   ```
2. Add `pub domains: DomainsConfig` to `RokoConfig`.
3. In the orchestrator, when loading a domain profile, check if the config has an override:
   ```rust
   let typed_ctx = if let Some(override_cfg) = config.domains.profiles.get(domain.label()) {
       TypedContext {
           domain,
           gate_rungs: override_cfg.gate_rungs.clone(),
           tool_categories: override_cfg.tool_categories.clone(),
           context_fraction: override_cfg.context_fraction,
           metadata: override_cfg.metadata.clone(),
       }
   } else {
       TypedContext::new(domain)
   };
   ```

**Files to modify:**
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-cli/src/orchestrate.rs`

**Example roko.toml:**
```toml
[domains.profiles.chain]
gate_rungs = ["compile", "simulation", "audit", "mev_check"]
context_fraction = 0.75

[domains.profiles.security]
gate_rungs = ["compile", "test", "audit", "fuzz"]
tool_categories = ["read", "write", "edit", "search", "exec", "test", "security"]
context_fraction = 0.7
```

**Test:**
- Parse a roko.toml with custom chain domain. Assert gate rungs match config, not defaults.
- Parse a roko.toml with a new "security" domain. Assert it loads without error and produces a TypedContext.

**Acceptance:**
- [ ] `[domains]` section parsed from roko.toml
- [ ] Custom domains override built-in defaults
- [ ] New domain labels can be defined in config

---

## Phase 2: Arena framework

Status: not built. This phase creates the abstraction for running benchmark batches through the orchestrator and scoring results.

### Task 2.1: Define Arena trait

**Read first:**
- `crates/roko-core/src/` -- `Task`, `Gate`, `Verdict`, `TaskStatus`
- `crates/roko-cli/src/orchestrate.rs` -- the plan execution loop

**What to do:**
1. Create `crates/roko-core/src/arena.rs`.
2. Define the trait:
   ```rust
   use async_trait::async_trait;

   /// A benchmark arena that samples tasks, routes them through the
   /// orchestrator, and scores results.
   #[async_trait]
   pub trait Arena: Send + Sync {
       /// Human-readable arena name.
       fn name(&self) -> &str;

       /// Sample a batch of tasks from the arena's dataset.
       async fn sample(&self, batch_size: usize) -> Result<Vec<TaskEnvelope>>;

       /// Return the gate configuration appropriate for this arena's tasks.
       fn gates_for(&self, task: &TaskEnvelope) -> Vec<String>;

       /// Score a completed task against the arena's ground truth.
       async fn score(&self, task: &TaskEnvelope, result: &TaskResult) -> Result<ArenaScore>;

       /// Enrich the agent prompt with arena-specific context.
       fn enrich_prompt(&self, task: &TaskEnvelope) -> Vec<String>;
   }
   ```
3. Define supporting structs:
   ```rust
   /// A task wrapped with arena metadata.
   pub struct TaskEnvelope {
       pub arena_name: String,
       pub task_id: String,
       pub title: String,
       pub description: String,
       pub domain: DomainProfile,
       pub ground_truth: Option<String>,
       pub metadata: HashMap<String, String>,
   }

   /// Result of running a task through the orchestrator.
   pub struct TaskResult {
       pub task_id: String,
       pub status: TaskStatus,
       pub gate_verdicts: Vec<Verdict>,
       pub agent_output: String,
       pub duration_ms: u64,
       pub token_usage: u64,
   }

   /// Score for one arena task.
   pub struct ArenaScore {
       pub task_id: String,
       pub passed: bool,
       pub score: f64,        // 0.0 - 1.0
       pub cost_usd: f64,
       pub latency_ms: u64,
       pub breakdown: HashMap<String, f64>,
   }
   ```
4. Export from `crates/roko-core/src/lib.rs`.

**Files to create:**
- `crates/roko-core/src/arena.rs`
**Files to modify:**
- `crates/roko-core/src/lib.rs` (add module + re-exports)

**Test:**
- The trait compiles and can be implemented by a mock struct.
- `TaskEnvelope` serializes/deserializes via serde.

**Acceptance:**
- [ ] `Arena` trait defined with 5 methods
- [ ] `TaskEnvelope`, `TaskResult`, `ArenaScore` structs defined
- [ ] Trait is object-safe (`Box<dyn Arena>`)

---

### Task 2.2: Implement SWE-bench arena

**Read first:**
- Task 2.1
- SWE-bench dataset format: each entry has a `repo`, `instance_id`, `problem_statement`, `patch` (ground truth)

**What to do:**
1. Create `crates/roko-cli/src/arenas/swe_bench.rs`.
2. Implement `Arena` for `SweBenchArena`:
   ```rust
   pub struct SweBenchArena {
       dataset_path: PathBuf,  // Path to SWE-bench JSONL
       entries: Vec<SweBenchEntry>,
   }

   struct SweBenchEntry {
       instance_id: String,
       repo: String,
       problem_statement: String,
       patch: String,  // ground truth
       test_patch: Option<String>,
   }
   ```
3. `sample()`: randomly select `batch_size` entries from the dataset.
4. `gates_for()`: return `["compile", "test", "diff_review"]` (coding domain).
5. `score()`: compare agent output diff against ground truth patch:
   - Exact match: 1.0
   - Test suite passes (if test_patch provided): 0.8
   - Compile succeeds: 0.4
   - Else: 0.0
6. `enrich_prompt()`: include the problem statement and repo context.
7. Load dataset from local JSONL file or fetch from HuggingFace Dataset Viewer API:
   ```
   GET https://datasets-server.huggingface.co/rows?dataset=princeton-nlp/SWE-bench&config=default&split=test&offset=0&length=100
   ```

**Files to create:**
- `crates/roko-cli/src/arenas/swe_bench.rs`
- `crates/roko-cli/src/arenas/mod.rs`
**Files to modify:**
- `crates/roko-cli/src/` -- add `pub mod arenas`

**Test:**
- Load a local JSONL with 5 mock SWE-bench entries. Sample 3. Assert 3 `TaskEnvelope`s returned with correct metadata.
- Score a perfect match. Assert score == 1.0.
- Score a compile-only success. Assert score == 0.4.

**Acceptance:**
- [ ] SWE-bench entries loaded from JSONL
- [ ] Sampling produces correctly-formed task envelopes
- [ ] Scoring differentiates exact match, test pass, compile-only, and failure

---

### Task 2.3: Implement self-hosting arena

**Read first:**
- Task 2.1
- `crates/roko-cli/src/orchestrate.rs` -- the plan execution loop
- `.roko/prd/` -- existing PRD files

**What to do:**
1. Create `crates/roko-cli/src/arenas/self_hosting.rs`.
2. Implement `Arena` for `SelfHostingArena`:
   ```rust
   pub struct SelfHostingArena {
       workspace_root: PathBuf,
   }
   ```
3. `sample()`: read PRDs from `.roko/prd/`, generate tasks from them:
   - Each PRD becomes a task: "implement the changes described in PRD-XX"
   - Or read existing plan files from `.roko/plans/`
4. `gates_for()`: return `["compile", "test", "clippy", "diff_review"]` (coding domain, roko-specific).
5. `score()`:
   - `cargo test --workspace` passes: 0.5
   - `cargo clippy --workspace --no-deps -- -D warnings` passes: 0.3
   - New code has test coverage: 0.2
   - Total up to 1.0
6. `enrich_prompt()`: include roko's CLAUDE.md, relevant crate docs, existing code context.

**File to create:** `crates/roko-cli/src/arenas/self_hosting.rs`

**Test:**
- Load 3 mock PRDs. Sample 2. Assert task envelopes reference the PRDs.
- Score a result where tests pass and clippy passes. Assert score >= 0.8.

**Acceptance:**
- [ ] Self-hosting arena reads PRDs and generates tasks
- [ ] Scoring based on compile + test + clippy + coverage
- [ ] Enrichment includes workspace-specific context

---

### Task 2.4: Add `roko bench arena` CLI command

**Read first:**
- Tasks 2.1-2.3
- `crates/roko-cli/src/main.rs` -- CLI subcommand registration (search for `clap` or `Command::new`)

**What to do:**
1. Add a `bench` subcommand group with an `arena` subcommand:
   ```
   roko bench arena --name swe-bench --batch-size 10 --dataset ./swe-bench.jsonl
   roko bench arena --name self-hosting --batch-size 5
   ```
2. The command should:
   a. Load the arena by name.
   b. Call `arena.sample(batch_size)` to get tasks.
   c. Run each task through the orchestrator (same code path as `roko plan run`).
   d. Score each result via `arena.score()`.
   e. Print a summary table:
      ```
      Arena: swe-bench | Batch: 10 | Pass rate: 70% | Mean score: 0.68 | Cost: $1.42 | Duration: 45m
      ---
      ID          Score  Cost   Time
      SWE-001     1.00   $0.12  4m
      SWE-002     0.40   $0.18  6m
      ...
      ```
   f. Write results to `.roko/bench/<arena>/<timestamp>.json`.
3. Add `--compare <path>` flag to compare two benchmark runs side by side.

**Files to modify:**
- `crates/roko-cli/src/main.rs` (or wherever CLI commands are registered)
**Files to create:**
- `crates/roko-cli/src/bench.rs`

**Test:**
- `cargo run -p roko-cli -- bench arena --name self-hosting --batch-size 1` runs without error.
- Results file written to `.roko/bench/`.

**Acceptance:**
- [ ] `roko bench arena` CLI command exists and runs
- [ ] Results printed as summary table
- [ ] Results persisted to `.roko/bench/`
- [ ] `--compare` shows diff between two runs

---

## Phase 3: Blockchain agent extensions

Status: the chain crate has `TriagePipeline`, `IsfrRegistry`, `Marketplace`, `BlockObserver`, and `MevDetector` fully implemented with tests. They are not connected to the agent dispatch path.

### Task 3.1: Create ChainSubscriberExt

**Read first:**
- `crates/roko-chain/src/observer.rs` -- `BlockObserver`, `BlockObserverConfig`, `ObservedEvent`
- `crates/roko-chain/src/client.rs` -- `ChainClient` trait
- `crates/roko-agent/src/dispatcher/mod.rs` -- how agents are dispatched

**What to do:**
1. Create `crates/roko-chain/src/agent_ext/mod.rs` (or `crates/roko-chain/src/subscriber_ext.rs`).
2. Define `ChainSubscriberExt`:
   ```rust
   pub struct ChainSubscriberExt {
       observer: BlockObserver,
       triage: TriagePipeline,
       event_tx: mpsc::Sender<TriageResult>,
   }

   impl ChainSubscriberExt {
       pub async fn start(&mut self) -> Result<()> {
           // Subscribe to newHeads and pendingTx via ChainClient
           // For each event: run through triage pipeline
           // Send triaged results to event_tx
       }
   }
   ```
3. Connect to the chain via `ChainClient::subscribe_new_heads()` and `subscribe_pending_txs()`.
4. For each incoming event, run through the `TriagePipeline` (already implemented in `crates/roko-chain/src/triage.rs`).
5. Route triaged events based on `TriageAction`:
   - `IngestKnowledge` -> feed to knowledge store
   - `AlertConductor` -> send to conductor/event bus
   - `MarketplaceHandler` -> feed to marketplace
   - `Drop` -> discard

**Files to create:**
- `crates/roko-chain/src/agent_ext/mod.rs`
- `crates/roko-chain/src/agent_ext/subscriber.rs`
**Files to modify:**
- `crates/roko-chain/src/lib.rs` (add module)

**Test:**
- With mock chain client: emit 10 events. Assert triage pipeline processes all 10. Assert routing matches triage action.
- Assert events with anomaly score > threshold trigger `AlertConductor`.

**Acceptance:**
- [ ] Subscribes to chain events via `ChainClient`
- [ ] Events pass through 4-stage triage pipeline
- [ ] Routing based on `TriageAction`

---

### Task 3.2: Create TriagePipeline integration

**Read first:**
- `crates/roko-chain/src/triage.rs` -- `TriagePipeline`, `TriageConfig`, `MidasRScorer`, `TriageResult`, `TriageAction`
- Task 3.1

**What to do:**
1. The `TriagePipeline` runs 4 stages. Verify each stage works end-to-end:
   - Stage 1 (rule filter): match against `known_contracts` and `known_topics` in config.
   - Stage 2 (MIDAS-R): anomaly scoring via `MidasRScorer`.
   - Stage 3 (enrichment): attach `EventEnrichment` metadata.
   - Stage 4 (curiosity): HDC-based information gain scoring.
2. If any stage is stubbed, implement it. Check each stage's method body.
3. Wire the pipeline into `ChainSubscriberExt` from Task 3.1.
4. Add metrics: events_processed, events_dropped, events_ingested, events_alerted.

**Files to modify:**
- `crates/roko-chain/src/triage.rs` (verify and complete stage implementations)
- `crates/roko-chain/src/agent_ext/subscriber.rs`

**Test:**
- Feed 100 synthetic events. Assert:
  - > 90% routed within 10ms per event (T0 latency)
  - Known contract events get correct labels
  - Anomalous events (sudden spike) trigger `AlertConductor`
  - Below-threshold events get `Drop`

**Acceptance:**
- [ ] All 4 triage stages functional (not stubbed)
- [ ] Per-event latency < 10ms for rule + anomaly stages
- [ ] Metrics tracked

---

### Task 3.3: Create ISFROracleExt

**Read first:**
- `crates/roko-chain/src/isfr.rs` -- `IsfrRegistry`, `IsfrConfig`, `ClearingPhase`, `IsfrSubmission`, `IsfrAggregate`

**What to do:**
1. Create `crates/roko-chain/src/agent_ext/isfr_oracle.rs`.
2. Define `ISFROracleExt`:
   ```rust
   pub struct ISFROracleExt {
       registry: IsfrRegistry,
       config: IsfrConfig,
       sources: Vec<Box<dyn IsfrSource>>,
   }

   #[async_trait]
   pub trait IsfrSource: Send + Sync {
       async fn poll_rate(&self, market_id: &str) -> Result<f64>;
   }
   ```
3. Implement the ISFR cycle:
   a. `poll_sources()`: query all configured sources for rate observations.
   b. `submit_observation()`: create an `IsfrSubmission` with the observed rate.
   c. `compute_aggregate()`: if enough submissions, run the weighted median aggregation.
   d. `publish_rate()`: commit the `IsfrAggregate` to the chain.
4. Add CRPS (Continuous Ranked Probability Score) tracking for prediction quality:
   ```rust
   pub fn crps_score(forecast_cdf: &[f64], actual: f64) -> f64 {
       // Standard CRPS computation
   }
   ```

**Files to create:**
- `crates/roko-chain/src/agent_ext/isfr_oracle.rs`
**Files to modify:**
- `crates/roko-chain/src/agent_ext/mod.rs`

**Test:**
- Create 5 mock ISFR sources. Poll all. Assert 5 submissions created.
- Submit 5 rates with 1 outlier. Assert aggregation excludes the outlier (3-sigma rule).
- CRPS score of a perfect prediction is 0.0. CRPS of a bad prediction is > 0.

**Acceptance:**
- [ ] Sources polled and rates submitted
- [ ] Weighted median aggregation with outlier exclusion
- [ ] CRPS tracking for forecast quality
- [ ] Full clearing cycle (COMMIT -> REVEAL -> SOLVE -> CERTIFICATE -> VERIFY -> SETTLE)

---

### Task 3.4: Create RiskExt (5-layer assessment)

**Read first:**
- `crates/roko-chain/src/gate/mev_gate.rs` -- `MevDetector`, `MevGate`
- `crates/roko-chain/src/gate/tx_sim_gate.rs` -- `TxSimGate`, `TxSimulator`
- `crates/roko-chain/src/gate/wallet_gate.rs` -- `WalletGate`

**What to do:**
1. Create `crates/roko-chain/src/agent_ext/risk.rs`.
2. Define `RiskExt` with 5 assessment layers:
   ```rust
   pub struct RiskExt {
       wallet_gate: WalletGate,
       tx_sim_gate: TxSimGate,
       mev_detector: MevDetector,
   }

   pub struct RiskAssessment {
       pub layers: [RiskLayer; 5],
       pub aggregate_score: f64,  // 0.0 (safe) to 1.0 (dangerous)
       pub recommendation: RiskAction,
   }

   pub enum RiskLayer {
       WalletBalance { sufficient: bool, balance: f64 },
       TxSimulation { reverted: bool, gas_estimate: u64 },
       MevExposure { sandwich_risk: f64, frontrun_risk: f64 },
       ContractAudit { verified: bool, known_vulnerabilities: usize },
       PositionRisk { concentration: f64, correlation: f64 },
   }

   pub enum RiskAction {
       Proceed,
       ProceedWithCaution { warnings: Vec<String> },
       Block { reasons: Vec<String> },
   }
   ```
3. Implement `assess()` that runs all 5 layers and produces an aggregate score.
4. Aggregate: weighted sum with wallet (0.15), simulation (0.25), MEV (0.25), audit (0.2), position (0.15).

**File to create:** `crates/roko-chain/src/agent_ext/risk.rs`

**Test:**
- Transaction that reverts in simulation: aggregate score > 0.5, recommendation = Block.
- Safe transaction on verified contract: aggregate score < 0.2, recommendation = Proceed.
- High MEV exposure: recommendation = ProceedWithCaution.

**Acceptance:**
- [ ] 5 risk layers implemented
- [ ] Weighted aggregate score
- [ ] Recommendation enum with actionable output

---

### Task 3.5: Wire blockchain agent profile

**Read first:**
- Tasks 3.1-3.4
- `crates/roko-cli/src/orchestrate.rs` -- agent dispatch
- `crates/roko-core/src/domain_profile.rs` -- `DomainProfile::Chain`

**What to do:**
1. When a task has `DomainProfile::Chain`, construct and attach the chain extensions:
   ```rust
   if task.domain_profile == Some(DomainProfile::Chain) {
       let subscriber = ChainSubscriberExt::new(chain_client.clone(), triage_config);
       let isfr_oracle = ISFROracleExt::new(isfr_config, sources);
       let risk = RiskExt::new(wallet_gate, tx_sim_gate, mev_detector);
       // Attach to the agent context or pass as task metadata
   }
   ```
2. Add a CLI shorthand: `roko agent start --profile blockchain` that pre-configures a chain agent with all extensions.
3. The chain agent should:
   - Subscribe to chain events on startup (via `ChainSubscriberExt`)
   - Triage events at T0 (< 10ms per event)
   - Run risk assessment before any transaction
   - Submit ISFR observations on a schedule

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-cli/src/main.rs` (add `agent start --profile` subcommand)

**Test:**
- `roko agent start --profile blockchain` starts without error (with mock chain client).
- Chain subscriber receives mock events and routes them through triage.
- Risk assessment runs on a test transaction and returns a recommendation.

**Acceptance:**
- [ ] `roko agent start --profile blockchain` works
- [ ] Chain extensions attached for Chain domain tasks
- [ ] Triage processes events at T0 latency

---

## Phase 4: Research agent extensions

Status: research commands exist (`roko research topic`, `roko research enhance-prd`). The gap is continuous source monitoring, entity extraction, and knowledge graph construction.

### Task 4.1: Create SourceWatcherExt

**Read first:**
- `crates/roko-learn/src/research_pipeline.rs` -- existing research pipeline (if present)
- `crates/roko-cli/src/` -- search for `research` commands

**What to do:**
1. Create `crates/roko-learn/src/source_watcher.rs`.
2. Define `SourceWatcherExt`:
   ```rust
   pub struct SourceWatcherExt {
       feeds: Vec<SourceFeed>,
       poll_interval: Duration,
       last_seen: HashMap<String, DateTime<Utc>>,
   }

   pub enum SourceFeed {
       Arxiv { query: String, max_results: usize },
       GitHub { repos: Vec<String>, event_types: Vec<String> },
       Rss { url: String },
   }

   pub struct SourceEvent {
       pub feed: String,
       pub title: String,
       pub url: String,
       pub summary: String,
       pub published_at: DateTime<Utc>,
       pub tags: Vec<String>,
   }
   ```
3. Implement polling for each source type:
   - Arxiv: HTTP GET to `http://export.arxiv.org/api/query?search_query=<query>&max_results=<n>&sortBy=submittedDate`
   - GitHub: GitHub API `/repos/{owner}/{repo}/events`
   - RSS: standard RSS/Atom parser
4. Deduplicate events by URL.
5. Emit new events to the knowledge store and event bus.

**File to create:** `crates/roko-learn/src/source_watcher.rs`
**File to modify:** `crates/roko-learn/src/lib.rs`

**Test:**
- With mock HTTP responses: poll arxiv feed, assert parsed entries match expected format.
- Deduplication: poll twice with overlapping results. Assert no duplicates in output.

**Acceptance:**
- [ ] Arxiv, GitHub, and RSS feeds supported
- [ ] Polling with configurable interval
- [ ] Deduplication by URL
- [ ] New events emitted to knowledge store

---

### Task 4.2: Create KnowledgeGraphExt

**Read first:**
- `crates/roko-neuro/src/knowledge_store.rs` -- `KnowledgeStore`, `KnowledgeEntry`
- `crates/roko-index/src/` -- parser + graph primitives (if relevant)

**What to do:**
1. Create `crates/roko-neuro/src/knowledge_graph.rs`.
2. Define the graph:
   ```rust
   pub struct KnowledgeGraph {
       entities: HashMap<String, Entity>,
       edges: Vec<Edge>,
   }

   pub struct Entity {
       pub id: String,
       pub name: String,
       pub entity_type: EntityType,
       pub properties: HashMap<String, String>,
       pub source_entries: Vec<String>,  // KnowledgeEntry IDs
   }

   pub enum EntityType {
       Concept, Person, Organization, Tool, Method, Dataset, Paper,
   }

   pub struct Edge {
       pub source: String,
       pub target: String,
       pub relation: Relation,
       pub weight: f64,
   }

   pub enum Relation {
       Uses, Extends, Contradicts, Supports, AuthoredBy, PartOf, RelatedTo,
   }
   ```
3. Implement entity extraction from knowledge entries:
   ```rust
   impl KnowledgeGraph {
       pub fn extract_entities(&mut self, entry: &KnowledgeEntry) -> Vec<Entity> {
           // Simple extraction: split on nouns, match against known entity patterns
           // More sophisticated: use LLM for NER (call agent with extraction prompt)
       }

       pub fn query(&self, entity_name: &str, max_hops: usize) -> Vec<(Entity, Vec<Edge>)> {
           // BFS from the named entity up to max_hops
       }
   }
   ```
4. Persist the graph to `.roko/neuro/knowledge-graph.json`.

**File to create:** `crates/roko-neuro/src/knowledge_graph.rs`
**File to modify:** `crates/roko-neuro/src/lib.rs`

**Test:**
- Extract entities from 3 knowledge entries. Assert entities created with correct types.
- Add edges between entities. Query from one entity with max_hops=2. Assert connected entities returned.
- Persist and reload graph. Assert roundtrip fidelity.

**Acceptance:**
- [ ] Entity extraction from knowledge entries
- [ ] Graph construction with typed edges
- [ ] BFS query with hop limit
- [ ] Persistence to disk

---

### Task 4.3: Create SynthesisExt

**Read first:**
- `crates/roko-dreams/src/imagination.rs` -- `synthesize_hypotheses()`, `CausalModel`
- Task 4.2

**What to do:**
1. Create `crates/roko-learn/src/synthesis.rs`.
2. Define `SynthesisExt`:
   ```rust
   pub struct SynthesisExt {
       graph: Arc<RwLock<KnowledgeGraph>>,
   }

   impl SynthesisExt {
       /// Cross-source reasoning: find connections between entities
       /// from different sources.
       pub fn cross_source_connections(&self) -> Vec<CrossSourceLink> {
           // For each pair of entities from different sources,
           // check if they share properties or have HDC-similar fingerprints
       }

       /// Generate hypotheses from entity patterns.
       pub fn generate_hypotheses(&self) -> Vec<Hypothesis> {
           // Find entity clusters in the graph
           // For each cluster, generate a hypothesis about why they cluster
       }

       /// Test a hypothesis against the knowledge base.
       pub fn test_hypothesis(&self, hypothesis: &Hypothesis) -> HypothesisResult {
           // Find supporting and contradicting evidence in the graph
       }
   }
   ```
3. `CrossSourceLink`: two entities from different sources that share structural similarity.
4. `Hypothesis`: a testable claim derived from entity patterns.

**File to create:** `crates/roko-learn/src/synthesis.rs`

**Test:**
- Create a graph with entities from 2 sources. Assert cross-source connections found for overlapping concepts.
- Generate a hypothesis from a cluster. Assert the hypothesis has supporting evidence.

**Acceptance:**
- [ ] Cross-source connection detection
- [ ] Hypothesis generation from entity patterns
- [ ] Hypothesis testing with evidence scoring

---

### Task 4.4: Wire research agent profile

**Read first:**
- Tasks 4.1-4.3
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-core/src/domain_profile.rs` -- `DomainProfile::Research`

**What to do:**
1. When a task has `DomainProfile::Research`, construct and attach research extensions:
   ```rust
   if task.domain_profile == Some(DomainProfile::Research) {
       let watcher = SourceWatcherExt::new(feeds, poll_interval);
       let graph = KnowledgeGraphExt::load_or_create(graph_path);
       let synthesis = SynthesisExt::new(graph.clone());
   }
   ```
2. Add CLI shorthand: `roko agent start --profile research`.
3. The research agent should:
   - Start source watchers on startup
   - Extract entities from new events and add to graph
   - Run synthesis periodically (every 100 new entities)
   - Feed hypotheses back into knowledge store

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-cli/src/main.rs`

**Test:**
- `roko agent start --profile research` starts without error.
- Mock source watcher emits events. Assert entities extracted and added to graph.

**Acceptance:**
- [ ] `roko agent start --profile research` works
- [ ] Source watchers active for research domain tasks
- [ ] Entity extraction runs on new events

---

## Phase 4B: HuggingFace integration

**Goal**: Create a native HuggingFace integration that enables dataset loading without Python, model discovery for the CascadeRouter, and a fine-tuning loop that turns successful episodes into training data.

### Task 4B.1: Create `roko-hf` crate with 5 modules

**File to create:** `crates/roko-hf/` (new crate)

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/Cargo.toml` (workspace members, workspace deps)
- `crates/roko-learn/src/cascade_router.rs` -- `CascadeRouter`, model discovery

**What to do:**

1. Create directory `/Users/will/dev/nunchi/roko/roko/crates/roko-hf/`.
2. Create `Cargo.toml` with dependencies: `roko-core`, `reqwest`, `serde`, `serde_json`, `anyhow`, `tokio`, `tracing`, `chrono`, `url`.
3. Create `src/lib.rs` with module declarations: `inference`, `hub`, `datasets`, `endpoints`, `autotrain`.
4. Add `"crates/roko-hf"` to the `[workspace].members` array.

```rust
// src/lib.rs
pub mod inference;   // HF Inference API client
pub mod hub;         // Hub API for model/dataset discovery
pub mod datasets;    // Dataset Viewer API + Parquet export
pub mod endpoints;   // Inference Endpoints management
pub mod autotrain;   // AutoTrain job trigger
```

5. Run `cargo check -p roko-hf`.

**Test:** `cargo check -p roko-hf` exits 0.

- [ ] Crate created with 5 modules
- [ ] Added to workspace members
- [ ] Compiles clean

---

### Task 4B.2: Implement Dataset Viewer API client

**File to create:** `crates/roko-hf/src/datasets.rs`

**Read first:**
- HuggingFace Dataset Viewer REST API documentation
- `crates/roko-core/src/config/schema.rs` -- config patterns

**What to do:**

1. Define the client:

```rust
pub struct DatasetViewer {
    base_url: String,
    http: reqwest::Client,
    token: Option<String>,
}

pub struct DatasetRow {
    pub fields: HashMap<String, serde_json::Value>,
}

pub struct DatasetInfo {
    pub name: String,
    pub splits: Vec<String>,
    pub num_rows: HashMap<String, usize>,
    pub features: Vec<FeatureInfo>,
}
```

2. Implement `DatasetViewer::info(dataset: &str) -> Result<DatasetInfo>`: call `/info?dataset={dataset}`.
3. Implement `DatasetViewer::rows(dataset: &str, split: &str, offset: usize, length: usize) -> Result<Vec<DatasetRow>>`: call `/rows?dataset={dataset}&split={split}&offset={offset}&length={length}`.
4. Implement `DatasetViewer::parquet_urls(dataset: &str, split: &str) -> Result<Vec<String>>`: call `/parquet?dataset={dataset}&split={split}` to get direct Parquet file URLs for bulk loading.
5. Implement `DatasetViewer::search(query: &str, limit: usize) -> Result<Vec<DatasetInfo>>`: search for datasets by keyword.

**Test:**
- Mock HTTP: `info("princeton-nlp/SWE-bench")` returns dataset with "test" and "dev" splits.
- Mock HTTP: `rows("...", "test", 0, 5)` returns 5 rows.
- Mock HTTP: `parquet_urls("...", "test")` returns Parquet file URLs.

- [ ] REST API client for HuggingFace Dataset Viewer
- [ ] `info()`, `rows()`, `parquet_urls()`, `search()` endpoints
- [ ] No Python dependency for dataset loading

---

### Task 4B.3: Implement Hub API client (model discovery for CascadeRouter)

**File to create:** `crates/roko-hf/src/hub.rs`

**Read first:**
- HuggingFace Hub API documentation
- `crates/roko-learn/src/cascade_router.rs` -- how models are registered

**What to do:**

1. Define the client:

```rust
pub struct HubClient {
    base_url: String,
    http: reqwest::Client,
    token: Option<String>,
}

pub struct ModelInfo {
    pub model_id: String,
    pub pipeline_tag: Option<String>,
    pub downloads: u64,
    pub likes: u64,
    pub created_at: String,
    pub tags: Vec<String>,
}
```

2. Implement `HubClient::list_models(author: &str, search: Option<&str>) -> Result<Vec<ModelInfo>>`: list models by author, optionally filtered by search term.
3. Implement `HubClient::model_info(model_id: &str) -> Result<ModelInfo>`.
4. Implement `HubClient::discover_fine_tuned(base_model: &str) -> Result<Vec<ModelInfo>>`: find models that are fine-tunes of a given base model (by tag or lineage).
5. Implement conversion to CascadeRouter arms:

```rust
impl ModelInfo {
    pub fn to_cascade_arm(&self) -> CascadeRouterArm {
        CascadeRouterArm {
            model: self.model_id.clone(),
            stage: RoutingStage::Confidence,
            // Start with neutral priors
            successes: 1,
            failures: 1,
        }
    }
}
```

**Test:**
- Mock HTTP: `list_models("my-org")` returns 3 models.
- Mock HTTP: `discover_fine_tuned("claude-haiku-4-5")` returns fine-tunes with matching tags.
- Conversion to `CascadeRouterArm` produces valid arm with neutral priors.

- [ ] Hub API client for model and dataset discovery
- [ ] `discover_fine_tuned()` finds fine-tunes of a base model
- [ ] Conversion to CascadeRouter arms

---

### Task 4B.4: Implement AutoTrain trigger

**File to create:** `crates/roko-hf/src/autotrain.rs`

**Read first:**
- HuggingFace AutoTrain API documentation
- Task 4B.3 output

**What to do:**

1. Define the trigger:

```rust
pub struct AutoTrainClient {
    base_url: String,
    http: reqwest::Client,
    token: String,
}

pub struct TrainingJob {
    pub job_id: String,
    pub model_id: String,
    pub dataset_id: String,
    pub status: TrainingStatus,
    pub created_at: String,
}

pub enum TrainingStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
}

pub struct TrainingConfig {
    pub base_model: String,
    pub dataset_id: String,
    pub task: String,          // e.g., "text-generation"
    pub num_epochs: u32,       // default 3
    pub learning_rate: f64,    // default 2e-5
    pub hub_model_id: String,  // where to push the result
}
```

2. Implement `AutoTrainClient::create_job(config: &TrainingConfig) -> Result<TrainingJob>`.
3. Implement `AutoTrainClient::job_status(job_id: &str) -> Result<TrainingJob>`.
4. Implement `AutoTrainClient::wait_for_completion(job_id: &str, timeout: Duration) -> Result<TrainingJob>`: poll status until completed or timeout.

**Test:**
- Mock HTTP: create job returns job_id.
- Mock HTTP: status poll returns Running, then Completed after 2 polls.

- [ ] AutoTrain API client for triggering fine-tune jobs
- [ ] Job creation with configurable training parameters
- [ ] Status polling with timeout

---

### Task 4B.5: Implement fine-tuned model discovery (CascadeRouter integration)

**File to modify:** `crates/roko-learn/src/cascade_router.rs`

**Read first:**
- `crates/roko-learn/src/cascade_router.rs` -- `CascadeRouter`, `RouterArm`
- Task 4B.3 output (Hub API model discovery)
- Task 4B.4 output (AutoTrain trigger)

**What to do:**

1. Add a model discovery method to `CascadeRouter`:

```rust
impl CascadeRouter {
    /// Scan HuggingFace Hub for new fine-tuned models and add them
    /// as new routing arms.
    pub async fn discover_models(&mut self, hub: &HubClient, org: &str) -> Result<usize> {
        let models = hub.discover_fine_tuned(&self.base_model).await?;
        let mut added = 0;
        for model in models {
            if !self.has_arm(&model.model_id) {
                self.add_arm(model.to_cascade_arm());
                tracing::info!(model = %model.model_id, "discovered new fine-tuned model");
                added += 1;
            }
        }
        Ok(added)
    }
}
```

2. Wire into the orchestrator: after plan completion, call `cascade_router.discover_models()` to check for new models.
3. Persist the updated router state to `.roko/learn/cascade-router.json`.

**Files to modify:**
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-cli/src/orchestrate.rs`

**Test:**
- Mock Hub returns 2 new models. Assert CascadeRouter gains 2 new arms.
- Mock Hub returns a model already known. Assert no duplicate arm added.
- Router state persists after discovery.

- [ ] `discover_models()` scans Hub for new fine-tunes
- [ ] New models added as CascadeRouter arms with neutral priors
- [ ] Wired into orchestrator post-plan hook
- [ ] No duplicate arms

---

### Task 4B.6: Integration test -- SWE-bench via HuggingFace

**File to create:** `crates/roko-hf/tests/swe_bench_load.rs`

**Read first:**
- Tasks 4B.1 through 4B.2

**What to do:**

1. Load 5 SWE-bench instances from the HuggingFace Dataset Viewer REST API (no Python).
2. For each instance: extract repo, instance_id, problem_statement, patch, test_patch.
3. Assert: all 5 instances load with non-empty fields.
4. Assert: no Python subprocess or pip dependency used.

**Test:** `cargo test -p roko-hf --test swe_bench_load` (requires network, gate with `#[ignore]` for CI).

- [ ] SWE-bench dataset loads via HF REST API
- [ ] No Python dependency
- [ ] 5 instances load with correct fields

---

## Phase 4C: SWE-bench native bench

**Goal**: Run SWE-bench through the full orchestrator with learning, natively in Rust.

### Task 4C.1: Implement `roko bench swe` command

**File to create:** `crates/roko-cli/src/bench.rs` (new file)

**Read first:**
- `crates/roko-cli/src/main.rs` -- existing `Subcommand` enum
- `crates/roko-hf/src/datasets.rs` -- `DatasetViewer`

**What to do:**

1. Add `Bench` subcommand to `main.rs`:

```rust
/// Run benchmark evaluations
Bench {
    #[command(subcommand)]
    bench_cmd: BenchCmd,
},

pub enum BenchCmd {
    /// Run SWE-bench evaluation
    Swe {
        /// Number of instances (default: 5)
        #[arg(long, default_value = "5")]
        count: usize,
        /// Repeat indefinitely (perpetual grinder mode)
        #[arg(long)]
        repeat: Option<usize>,
        /// Shuffle instance order
        #[arg(long)]
        shuffle: bool,
        /// Dataset split (default: "test")
        #[arg(long, default_value = "test")]
        split: String,
    },
}
```

2. Implement the `swe` handler in `crates/roko-cli/src/bench.rs`.

**Test:** `cargo run -p roko-cli -- bench swe --help` prints usage.

- [ ] `roko bench swe` command added to CLI
- [ ] Accepts `--count`, `--repeat`, `--shuffle`, `--split` flags

---

### Task 4C.2: Implement instance-to-TaskDef mapping

**File to modify:** `crates/roko-cli/src/bench.rs`

**Read first:**
- `crates/roko-hf/src/datasets.rs` -- `DatasetRow`
- `crates/roko-orchestrator/src/plan.rs` -- `TaskDef` (or equivalent task struct)

**What to do:**

1. Map each SWE-bench instance to a `TaskDef`:

```rust
pub fn instance_to_task(row: &DatasetRow) -> Result<TaskDef> {
    let repo = row.get_str("repo")?;
    let instance_id = row.get_str("instance_id")?;
    let problem = row.get_str("problem_statement")?;
    let base_commit = row.get_str("base_commit")?;

    Ok(TaskDef {
        id: instance_id.to_string(),
        title: format!("[SWE-bench] {}", instance_id),
        description: problem.to_string(),
        domain: Some("coding".to_string()),
        metadata: HashMap::from([
            ("repo".to_string(), repo.to_string()),
            ("base_commit".to_string(), base_commit.to_string()),
            ("bench".to_string(), "swe-bench".to_string()),
        ]),
        ..Default::default()
    })
}
```

2. Implement `load_swe_bench_instances(count: usize, split: &str, shuffle: bool) -> Result<Vec<TaskDef>>`.

**Test:**
- Map a known SWE-bench row. Assert task has correct repo, instance_id, and problem_statement.

- [ ] SWE-bench rows mapped to `TaskDef` structs
- [ ] Metadata includes repo, base_commit, bench identifier

---

### Task 4C.3: Implement two-tier scoring

**File to modify:** `crates/roko-cli/src/bench.rs`

**Read first:**
- SWE-bench evaluation methodology
- `crates/roko-gate/src/` -- gate pipeline

**What to do:**

1. Fast scoring (no Python):

```rust
pub fn fast_score(patch: &str, test_patch: &str, generated_patch: &str) -> FastScoreResult {
    // Apply generated_patch to a temp repo copy
    // Run `git apply --check` to verify patch applies
    // Compare diff coverage: does the generated patch touch the same files as the reference?
    FastScoreResult {
        applies: git_apply_check(generated_patch),
        file_overlap: file_overlap_ratio(generated_patch, patch),
        line_coverage: line_coverage_ratio(generated_patch, patch),
    }
}
```

2. Official scoring (Python harness, optional):

```rust
pub fn official_score(instance_id: &str, generated_patch: &str) -> Option<OfficialScoreResult> {
    // If Python is available, run the SWE-bench harness
    // Returns pass/fail per test
}
```

3. Default to fast scoring. Official scoring enabled with `--official-score` flag.

**Test:**
- Fast score: generated patch applies -> `applies = true`.
- Fast score: generated patch touches same files -> `file_overlap > 0.5`.
- Fast score: non-applying patch -> `applies = false`.

- [ ] Fast scoring via `git apply --check` (no Python)
- [ ] Official scoring via Python harness (optional)
- [ ] File overlap and line coverage metrics

---

### Task 4C.4: Implement perpetual grinder mode

**File to modify:** `crates/roko-cli/src/bench.rs`

**Read first:**
- Task 4C.1 output (`--repeat` and `--shuffle` flags)

**What to do:**

1. When `--repeat 0` is passed, the bench runs indefinitely:

```rust
let repeat_count = repeat.unwrap_or(1);
let mut round = 0;
loop {
    if repeat_count > 0 && round >= repeat_count { break; }
    let mut instances = load_swe_bench_instances(count, &split, shuffle)?;
    for task in &instances {
        let result = orchestrator.run_single_task(task).await?;
        scorer.record(task, &result);
        logger.log_episode(task, &result)?;
    }
    round += 1;
    tracing::info!(round, "completed SWE-bench round");
}
```

2. The learning subsystem (CascadeRouter, adaptive thresholds, etc.) runs across rounds, so later rounds benefit from earlier learning.
3. Print running statistics after each round: pass rate, average score, best instance, worst instance.

**Test:**
- `--repeat 2 --count 3`: runs 6 total instances across 2 rounds.
- Learning state (CascadeRouter) persists between rounds.

- [ ] Perpetual grinder runs indefinitely with `--repeat 0`
- [ ] Learning accumulates across rounds
- [ ] Running statistics printed per round

---

### Task 4C.5: Integration test -- 5 SWE-bench instances through orchestrator

**File to create:** `crates/roko-cli/tests/swe_bench_integration.rs`

**Read first:**
- Tasks 4C.1 through 4C.4

**What to do:**

1. Load 5 SWE-bench instances from mock data (not from HF API, for test stability).
2. Run through the full orchestrator with mock agent dispatcher.
3. Score results with fast scoring.
4. Verify:
   - All 5 instances dispatch to the agent.
   - Gate pipeline runs on each result.
   - Episodes logged for each instance.
   - CascadeRouter state updated after run.
   - Learning data persisted.

**Test:** `cargo test -p roko-cli --test swe_bench_integration`

- [ ] 5 SWE-bench instances through full orchestrator
- [ ] Gate pipeline validates results
- [ ] Episodes and learning data persisted
- [ ] CascadeRouter updated

---

## Phase 4D: Arena catalog (initial 5)

**Goal**: Implement the first 5 benchmark arenas from the 28-arena catalog.

### Task 4D.1: Implement SWE-bench arena (Tier 1)

**File to modify:** `crates/roko-cli/src/bench.rs`

**Read first:**
- Tasks 4C.1-4C.5

**What to do:**

This arena wraps the SWE-bench implementation from Phase 4C into the arena framework from IMPL-05 Phase 2 (if it exists) or creates a standalone arena runner.

1. Register "swe-bench" as an arena in the arena catalog.
2. Arena config: count=50 (default), split="test", scoring=fast, timeout=600s per instance.
3. Leaderboard: track pass rate, average score, cost per instance, turns per instance.

- [ ] "swe-bench" arena registered in catalog
- [ ] Configurable instance count, split, scoring mode
- [ ] Leaderboard tracks pass rate, cost, turns

---

### Task 4D.2: Implement self-hosting arena (Tier 1)

**File to create:** `crates/roko-cli/src/bench_self_host.rs` (new file)

**Read first:**
- `crates/roko-cli/src/orchestrate.rs` -- self-hosting workflow
- `.roko/prd/` -- existing PRD files

**What to do:**

1. The self-hosting arena tests roko's ability to develop itself:
   - Load a small PRD (pre-authored test PRD)
   - Generate a plan from the PRD
   - Execute the plan (1-3 tasks)
   - Gate the results
   - Score: did the code compile? Did tests pass? Did the plan complete?
2. Define 10 test PRDs covering: add a CLI flag, fix a lint, add a unit test, add a doc comment, refactor a function, wire an existing module, update a config schema, add an error variant, implement a trait, write an integration test.
3. Scoring: compile (0.3), test pass (0.3), clippy clean (0.2), task completion (0.2).

- [ ] "self-hosting" arena with 10 test PRDs
- [ ] Full pipeline: PRD -> plan -> execute -> gate -> score
- [ ] Composite scoring: compile + test + clippy + completion

---

### Task 4D.3: Implement MBPP arena (Tier 2)

**File to create:** `crates/roko-cli/src/bench_mbpp.rs` (new file)

**Read first:**
- MBPP (Mostly Basic Python Programs) dataset structure
- `crates/roko-hf/src/datasets.rs` -- `DatasetViewer`

**What to do:**

1. Load MBPP instances from HuggingFace Dataset Viewer.
2. Map to TaskDefs: each instance is a function implementation task.
3. Scoring: run the test assertions from the dataset against the generated code.
4. Since MBPP is Python, the agent generates Python code. Scoring requires Python runtime (optional, skip if unavailable).

- [ ] "mbpp" arena loads from HuggingFace
- [ ] Function implementation tasks
- [ ] Test assertion scoring

---

### Task 4D.4: Implement chain-monitor arena (Tier 2)

**File to create:** `crates/roko-cli/src/bench_chain_monitor.rs` (new file)

**Read first:**
- `crates/roko-chain/src/triage.rs` -- `TriagePipeline`
- `crates/roko-chain/src/observer.rs` -- `BlockObserver`

**What to do:**

1. The chain-monitor arena tests the blockchain agent's event detection ability:
   - Replay a sequence of 1000 historical blockchain events (from mock data)
   - Inject 50 "interesting" events (large transfers, flash loans, governance votes)
   - Score: precision and recall of interesting event detection
2. The agent runs the triage pipeline on each event.
3. Scoring: precision (flagged events that were interesting / total flagged), recall (interesting events flagged / total interesting), F1 score.

- [ ] "chain-monitor" arena with 1000 events, 50 interesting
- [ ] Precision, recall, F1 scoring
- [ ] Tests triage pipeline effectiveness

---

### Task 4D.5: Implement ISFR-prediction arena (Tier 2)

**File to create:** `crates/roko-cli/src/bench_isfr.rs` (new file)

**Read first:**
- `crates/roko-chain/src/isfr.rs` -- ISFR oracle
- `crates/roko-learn/src/crps.rs` -- CRPS scoring

**What to do:**

1. The ISFR-prediction arena tests rate prediction accuracy:
   - Provide 30 days of historical ISFR data (mock).
   - At each day, the agent predicts the next-day ISFR rate.
   - Score via CRPS (Continuous Ranked Probability Score).
2. Baseline: predict yesterday's rate (random walk). The agent should beat this baseline.
3. Scoring: CRPS < baseline CRPS means the agent adds value.

- [ ] "isfr-prediction" arena with 30 days of mock data
- [ ] CRPS scoring for prediction accuracy
- [ ] Random walk baseline for comparison

---

## Phase 5: Work market integration

Status: the marketplace (`crates/roko-chain/src/marketplace.rs`) is fully implemented with escrow, 3 hiring models, and dispute resolution. It is not connected to the agent dispatch path.

### Task 5.1: Define work market job submission

**Read first:**
- `crates/roko-chain/src/marketplace.rs` -- `Marketplace`, `MarketplaceJob`, `JobState`, `EscrowEntry`
- `crates/roko-chain/src/identity_economy_markets.rs` -- `BountySpec`, `SparrowBid`

**What to do:**
1. Create `crates/roko-chain/src/agent_ext/work_market.rs`.
2. Define the agent-facing API:
   ```rust
   pub struct WorkMarketClient {
       marketplace: Marketplace,
       wallet: Box<dyn ChainWallet>,
       passport_id: [u8; 32],
   }

   impl WorkMarketClient {
       /// Browse available jobs matching the agent's capabilities.
       pub fn browse_jobs(&self, domain: &DomainProfile, min_budget: f64) -> Vec<MarketplaceJob> { ... }

       /// Submit a sealed bid for a job (BlindAuction model).
       pub fn submit_bid(&mut self, job_id: [u8; 32], bid_amount: f64, nonce: [u8; 32]) -> Result<()> { ... }

       /// Accept a direct hire (DirectHire model).
       pub fn accept_direct_hire(&mut self, job_id: [u8; 32]) -> Result<()> { ... }

       /// Submit completed work for a job.
       pub fn submit_result(&mut self, job_id: [u8; 32], result_hash: [u8; 32], quality_score: f64) -> Result<()> { ... }
   }
   ```
3. Connect to the existing `Marketplace` methods.

**File to create:** `crates/roko-chain/src/agent_ext/work_market.rs`

**Test:**
- Browse jobs with domain filter. Assert only matching jobs returned.
- Submit a bid. Assert job state transitions to Assigned after reveal.
- Submit a result. Assert job state transitions to Submitted.

**Acceptance:**
- [ ] Agent can browse, bid on, accept, and submit results for jobs
- [ ] Escrow handled correctly (deposit on post, release on settle)
- [ ] All 3 hiring models supported

---

### Task 5.2: Implement verification and settlement

**Read first:**
- `crates/roko-chain/src/marketplace.rs` -- `SettlementResult`, `DisputeResolution`
- `crates/roko-gate/src/` -- gate pipeline for quality verification

**What to do:**
1. After an agent submits work, the verifier runs the gate pipeline on the result:
   ```rust
   pub async fn verify_and_settle(&mut self, job_id: [u8; 32]) -> Result<SettlementResult> {
       let job = self.marketplace.get_job(job_id)?;
       // Run gate pipeline
       let verdict = self.gate_pipeline.run(&job.result_payload())?;
       if verdict.passed {
           self.marketplace.settle(job_id, verdict.score)?;
       } else {
           self.marketplace.dispute(job_id, DisputeLevel::Automatic)?;
       }
   }
   ```
2. Implement the 4-level dispute resolution flow:
   - Level 1: Automatic (gate re-run with different parameters)
   - Level 2: Peer review (another agent evaluates)
   - Level 3: Oracle committee (3 agents vote)
   - Level 4: Governance (manual intervention)
3. On settlement: release escrow to the agent.

**Files to modify:**
- `crates/roko-chain/src/agent_ext/work_market.rs`
- `crates/roko-chain/src/marketplace.rs` (if dispute flow is incomplete)

**Test:**
- Submit valid work. Assert settlement succeeds, escrow released.
- Submit invalid work (fails gate). Assert dispute initiated at Level 1.
- Level 1 dispute re-run passes. Assert settlement after dispute resolution.

**Acceptance:**
- [ ] Gate pipeline runs on submitted work
- [ ] Passing work settles and releases escrow
- [ ] Failing work triggers dispute flow
- [ ] 4 dispute levels implemented

---

### Task 5.3: Implement knowledge future commitment

**Read first:**
- `crates/roko-chain/src/futures_market.rs` -- `FuturesMarket`, `FuturesMarketConfig`

**What to do:**
1. Extend `WorkMarketClient` with knowledge future support:
   ```rust
   impl WorkMarketClient {
       /// Commit to deliver a knowledge artifact by a deadline.
       pub fn commit_knowledge_future(
           &mut self,
           topic_fingerprint: HdcVector,
           quality_threshold: f64,
           deadline_block: u64,
           stake: f64,
       ) -> Result<FutureCommitment> { ... }

       /// Deliver the knowledge artifact for a committed future.
       pub fn deliver_knowledge_future(
           &mut self,
           commitment_id: [u8; 32],
           entry: KnowledgeEntry,
       ) -> Result<DeliveryResult> { ... }
   }
   ```
2. On delivery, verify the entry meets the quality threshold (confidence, tier).
3. If delivered before deadline: release stake + earn reward.
4. If deadline passed without delivery: slash stake.

**Files to modify:** `crates/roko-chain/src/agent_ext/work_market.rs`

**Test:**
- Commit a knowledge future. Deliver before deadline. Assert stake returned + reward.
- Commit a knowledge future. Let deadline pass. Assert stake slashed.

**Acceptance:**
- [ ] Knowledge future commitment with stake
- [ ] Delivery verification against quality threshold
- [ ] Reward on success, slash on timeout

---

## Phase 6: Cross-arena transfer

Status: not built. This phase measures whether knowledge learned in one arena transfers to another.

### Task 6.1: Implement cross-arena knowledge sharing

**Read first:**
- `crates/roko-neuro/src/knowledge_store.rs` -- `KnowledgeStore::query()`
- `crates/roko-neuro/src/publisher.rs` (from IMPL-04 Phase 4)
- `crates/roko-learn/src/cfactor.rs` -- `CFactor`

**What to do:**
1. Create `crates/roko-learn/src/arena_transfer.rs`.
2. Define the measurement framework:
   ```rust
   pub struct ArenaTransferMeasurement {
       pub source_arena: String,
       pub target_arena: String,
       pub knowledge_entries_shared: usize,
       pub target_score_before: f64,
       pub target_score_after: f64,
       pub transfer_delta: f64,
       pub cfactor_before: CFactor,
       pub cfactor_after: CFactor,
   }
   ```
3. Implement the measurement:
   a. Run target arena batch without source knowledge (baseline).
   b. Inject source arena knowledge entries into the knowledge store.
   c. Run target arena batch again.
   d. Compare scores.
4. A positive `transfer_delta` means knowledge transferred. A negative delta means interference.

**File to create:** `crates/roko-learn/src/arena_transfer.rs`

**Test:**
- Mock scenario: SWE-bench knowledge includes "always check return values". Self-hosting arena tasks involving error handling should benefit. Assert positive transfer_delta.
- Mock interference: chain-specific knowledge applied to coding tasks. Assert transfer_delta near 0 (no help, no harm).

**Acceptance:**
- [ ] Cross-arena transfer measured with before/after comparison
- [ ] Positive transfer detectable
- [ ] Interference (negative transfer) detectable

---

### Task 6.2: Track arena learning curves

**Read first:**
- Task 6.1
- `crates/roko-learn/src/cfactor.rs` -- `CFactor`, `detect_cfactor_regression()`

**What to do:**
1. After each arena batch, compute and store:
   ```rust
   pub struct ArenaBatchRecord {
       pub arena_name: String,
       pub batch_number: u64,
       pub timestamp: DateTime<Utc>,
       pub batch_size: usize,
       pub mean_score: f64,
       pub pass_rate: f64,
       pub total_cost_usd: f64,
       pub cfactor: CFactor,
       pub knowledge_entries_used: usize,
   }
   ```
2. Persist to `.roko/bench/<arena>/learning-curve.jsonl`.
3. Detect learning: score should increase across batches. If it plateaus or regresses, emit a warning.
4. Add `roko bench learning-curve --arena swe-bench` that plots the curve (print as ASCII table or write CSV).

**Files to modify:**
- `crates/roko-learn/src/arena_transfer.rs`
- `crates/roko-cli/src/bench.rs`

**Test:**
- Generate 5 mock batch records with increasing scores. Assert learning detected.
- Generate 5 mock batch records with flat scores. Assert plateau warning emitted.

**Acceptance:**
- [ ] Batch records persisted per arena
- [ ] Learning curve visible across batches
- [ ] Plateau and regression detection with warnings

---

## Verification checklist

```bash
# Phase 1: Domain profile wiring
cargo test -p roko-core -- domain_profile
cargo test -p roko-cli -- task_parser

# Phase 2: Arena framework
cargo test -p roko-core -- arena
cargo test -p roko-cli -- arenas

# Phase 3: Blockchain extensions
cargo test -p roko-chain -- triage
cargo test -p roko-chain -- isfr
cargo test -p roko-chain -- agent_ext

# Phase 4: Research extensions
cargo test -p roko-learn -- source_watcher
cargo test -p roko-neuro -- knowledge_graph
cargo test -p roko-learn -- synthesis

# Phase 5: Work market
cargo test -p roko-chain -- work_market
cargo test -p roko-chain -- marketplace

# Phase 6: Cross-arena
cargo test -p roko-learn -- arena_transfer

# Full workspace
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

## Dependency graph

```
Phase 1 (domain wiring) ─┬─> Phase 2 (arena framework) ──> Phase 6 (cross-arena transfer)
                          │
                          ├─> Phase 3 (blockchain ext) ──> Phase 5 (work market)
                          │
                          └─> Phase 4 (research ext)
```

Phase 1 is the foundation -- everything else depends on domain profiles being wired. Phases 2, 3, and 4 can run in parallel after Phase 1. Phase 5 depends on Phase 3. Phase 6 depends on Phase 2.

## Acceptance criteria (plan-level)

- [ ] `roko agent start --profile blockchain` spawns a blockchain agent with chain subscriber, triage, ISFR oracle, and risk assessment
- [ ] `roko agent start --profile research` spawns a research agent with source watchers, knowledge graph, and synthesis
- [ ] `roko bench arena --name swe-bench` runs a benchmark batch through the orchestrator and produces scored results
- [ ] `roko bench arena --name self-hosting` runs roko-on-roko benchmarks
- [ ] Blockchain agent triages 95%+ chain events at T0 (< 10ms per event)
- [ ] Research agent detects new publications and updates the knowledge graph
- [ ] Arena scores tracked across batches with visible learning curves
- [ ] Cross-arena transfer measurable (positive delta when knowledge helps, near-zero when irrelevant)
- [ ] Domain profiles route gates, tools, and context fraction correctly
- [ ] Custom domains configurable via `[domains]` in roko.toml
- [ ] All phases pass `cargo clippy --workspace --no-deps -- -D warnings`
- [ ] All phases pass `cargo test --workspace`
