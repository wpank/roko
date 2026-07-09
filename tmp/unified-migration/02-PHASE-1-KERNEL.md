# Phase 1 — Kernel Upgrade

> Promote Pulse and Bus to kernel-level, wire predict-publish-correct, add demurrage, introduce Heuristic kind, upgrade routing to EFE, trigger the dream cycle, formalize Observe/Trigger/Connect protocols, and rename all types to match the unified spec.

**Spec source**: `tmp/unified/21-ROADMAP.md` §2 (Phase 1)
**Dependencies**: Phase 0 complete

---

## 1.1 Core Type Renames

Rename the fundamental types across the entire codebase. Each rename is a single find-and-replace across all crates, followed by updating all imports, docs, and tests.

- [ ] **Rename `Engram` → `Signal`** — Rename the struct, all field references, all method signatures, all test fixtures, all docs. The struct lives in `crates/roko-core/src/engram.rs` → rename file to `signal.rs`. Update `mod` declarations in `lib.rs`. Update all 28 downstream crates that import `Engram`. **Verify**: `cargo test --workspace` passes, `grep -rn 'Engram' crates/ --include='*.rs' | grep -v target/` returns zero hits.
  - Spec: `tmp/unified/01-SIGNAL.md` §2 (Signal struct)
  - Code: `crates/roko-core/src/engram.rs` and all importers

- [ ] **Rename `Substrate` → `Store`** — Rename the trait and all implementations (FileSubstrate → FileStore, MemorySubstrate → MemoryStore, ColdSubstrate → ColdStore). Update trait bounds, generic params, and docs across all crates. **Verify**: zero grep hits for `Substrate` in `*.rs` files.
  - Spec: `tmp/unified/00-INDEX.md` §Vocabulary (9 Protocols table, Store row)
  - Code: `crates/roko-core/src/traits.rs` (Substrate trait), `crates/roko-fs/src/` (FileSubstrate)

- [ ] **Rename `Scorer` → `Score`** — Rename the trait and all implementations. **Verify**: zero grep hits for `Scorer` in `*.rs`.
  - Spec: `tmp/unified/00-INDEX.md` §Vocabulary (Score row)
  - Code: `crates/roko-core/src/traits.rs`

- [ ] **Rename `Gate` → `Verify`** — Rename the trait, all 11 gate implementations, the 7-rung pipeline, and all references. `CompileGate` → `CompileVerify`, `TestGate` → `TestVerify`, etc. **Verify**: zero grep hits for `Gate` as a trait/struct name (the word "gate" in comments/docs is fine).
  - Spec: `tmp/unified/00-INDEX.md` §Vocabulary (Verify row)
  - Code: `crates/roko-core/src/traits.rs`, `crates/roko-gate/src/` (all 11 implementations)

- [ ] **Rename `Router` → `Route`** — Rename the trait and CascadeRouter → CascadeRoute. **Verify**: zero grep hits.
  - Spec: `tmp/unified/00-INDEX.md` §Vocabulary (Route row)
  - Code: `crates/roko-core/src/traits.rs`, `crates/roko-learn/src/routing/`

- [ ] **Rename `Composer` → `Compose`** — Rename the trait and all implementations. **Verify**: zero grep hits.
  - Spec: `tmp/unified/00-INDEX.md` §Vocabulary (Compose row)
  - Code: `crates/roko-core/src/traits.rs`, `crates/roko-compose/src/`

- [ ] **Rename `Policy` → `React`** — Rename the trait. Note: the method signature changes in 1.3 (Pulse/Bus) to take Pulses instead of Signals. For now, just rename. **Verify**: zero grep hits for `Policy` as a trait name.
  - Spec: `tmp/unified/00-INDEX.md` §Vocabulary (React row)
  - Code: `crates/roko-core/src/traits.rs`, `crates/roko-daimon/`, `crates/roko-conductor/`

- [ ] **Rename `Verdict` fields and types for Verify protocol extensions** — Add `reward: f64` field to Verdict. Add `verify_pre()` method to Verify trait (pre-action veto). Ensure evidence is typed separately from criteria. **Verify**: existing tests updated, new tests for pre-action veto.
  - Spec: `tmp/unified/02-CELL.md` §4 (Verify protocol)
  - Code: `crates/roko-core/src/traits.rs` (Verdict struct), `crates/roko-gate/src/`

## 1.2 Pulse/Bus Kernel

Promote Pulse and Bus from runtime utility to kernel-level types in roko-core.

- [ ] **Define `Pulse` struct in roko-core** — Fields: `seq: u64`, `topic: Topic`, `kind: PulseKind`, `body: serde_json::Value`, `source: CellId`, `lineage_hint: Option<SignalRef>`, `trace_id: TraceId`, `timestamp: DateTime<Utc>`. Implement `Pulse::graduate() -> Signal` (the only path from transport into audit DAG). Implement `Signal::to_pulse() -> Pulse` (lossy projection). **Verify**: round-trip test: Signal → Pulse → graduate → Signal preserves content hash.
  - Spec: `tmp/unified/01-SIGNAL.md` §3 (Pulse struct), §5 (Graduation)
  - Code: `crates/roko-core/src/pulse.rs` (new file)

- [ ] **Define `Bus` trait in roko-core** — Methods: `publish(pulse: Pulse) -> Result<u64>`, `subscribe(filter: TopicFilter) -> Result<Receiver>`. Define `TopicFilter` enum: `Exact(Topic)`, `Prefix(String)`, `Glob(String)`, `AnyOf(Vec<TopicFilter>)`, `And(Box<TopicFilter>, Box<TopicFilter>)`, `Not(Box<TopicFilter>)`. **Verify**: trait compiles, mock implementation passes basic pub/sub test.
  - Spec: `tmp/unified/01-SIGNAL.md` §3.3 (Bus)
  - Code: `crates/roko-core/src/bus.rs` (new file)

- [ ] **Implement `BroadcastBus`** — In-process Bus implementation using `tokio::sync::broadcast`. Move/refactor from existing `roko-runtime::event_bus`. Delete old `EventBus` type after migration. **Verify**: 1000 Pulses published/received in <10ms. Subscriber filtering by TopicFilter works.
  - Spec: `tmp/unified/21-ROADMAP.md` §2.1 (BroadcastBus)
  - Code: `crates/roko-runtime/src/bus/` (refactor from `crates/roko-runtime/src/event_bus.rs`)

- [ ] **Define topic taxonomy** — Standard topics: `orchestration.*` (flow.started, flow.completed, node.started, etc.), `prediction.*`, `outcome.*`, `calibration.*`, `extension.*`, `agent.*` (lifecycle, heartbeat), `knowledge.*`, `dream.*`. Define as constants or enum in roko-core. **Verify**: all topics documented, used in at least one test.
  - Spec: `tmp/unified/01-SIGNAL.md` §3.2 (Topic Taxonomy)
  - Code: `crates/roko-core/src/topics.rs` (new file)

- [ ] **Wire Bus into execution path** — Emit Cell/node lifecycle events as Pulses on Bus during plan execution in orchestrate.rs. Events: `orchestration.node.started`, `orchestration.node.completed`, `orchestration.node.failed`, `orchestration.flow.started`, `orchestration.flow.completed`. **Verify**: subscribe to `orchestration.*`, run a plan, confirm all lifecycle Pulses received.
  - Spec: `tmp/unified/05-EXECUTION-ENGINE.md` §5 (Lifecycle Events)
  - Code: `crates/roko-cli/src/orchestrate.rs`, `crates/roko-orchestrator/`

## 1.3 React Protocol Breaking Change

- [ ] **Update React trait to take Pulses** — Change `fn decide(&self, stream: &[Signal], ctx: &Context) -> Vec<Signal>` to `fn react(&self, pulses: &[Pulse], ctx: &CellContext) -> ReactOutput` where `ReactOutput { pulses: Vec<Pulse>, signals: Vec<Signal> }`. Migrate all existing React implementations (DaimonReact, SafetyReact, BudgetReact, CalibrationReact, EscalationReact). **Verify**: all React implementations compile and pass tests with new signature.
  - Spec: `tmp/unified/02-CELL.md` §7 (React protocol)
  - Code: `crates/roko-core/src/traits.rs` (Policy → React), `crates/roko-daimon/`, `crates/roko-conductor/`, `crates/roko-learn/`

## 1.4 Cell Trait

- [ ] **Define `Cell` trait in roko-core** — The universal computation unit. Methods: `id() -> CellId`, `name() -> &str`, `version() -> Version`, `input_schema() -> Option<&TypeSchema>`, `output_schema() -> Option<&TypeSchema>`, `capabilities() -> &Capabilities`, `protocols() -> &[ProtocolId]`, `estimated_cost() -> Option<Cost>`, `estimated_duration() -> Option<Duration>`, `execute(input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>>`. Define `CellContext` with Bus, Store, budget, trace, cancel token. **Verify**: trait compiles, a trivial NoopCell implementation passes.
  - Spec: `tmp/unified/02-CELL.md` §1 (Cell trait)
  - Code: `crates/roko-core/src/cell.rs` (new file)

- [ ] **Define `CellId`** — Content-addressed from `(name, version, author)` via SHA-256. Implement `Display`, `FromStr`, `Serialize`, `Deserialize`. **Verify**: two Cells with same (name, version, author) produce identical CellId.
  - Spec: `tmp/unified/02-CELL.md` §1.1 (CellId)
  - Code: `crates/roko-core/src/cell.rs`

- [ ] **Define `TypeSchema`** — Input/output type declarations for Cells. Supports primitives, collections, named structs, unions, optional. Used for edge validation in Graphs. **Verify**: schema validation catches type mismatch at load time.
  - Spec: `tmp/unified/14-CONFIG-AND-AUTHORING.md` §6 (TypeSchema)
  - Code: `crates/roko-core/src/schema.rs` (new file)

## 1.5 Predict-Publish-Correct

- [ ] **Implement `CalibrationReact` Cell** — A React Cell that subscribes to `prediction.*` and `outcome.*` topics on Bus, joins by `lineage_hint`, computes calibration error, and publishes `calibration.{operator}.updated` Pulses. Persists per-operator calibration state (EMA of errors). **Verify**: publish a fake prediction + outcome pair, confirm calibration Pulse emitted with correct error.
  - Spec: `tmp/unified/02-CELL.md` §9 (Predict-Publish-Correct), `tmp/unified/10-LEARNING-LOOPS.md` §2
  - Code: `crates/roko-learn/src/calibration.rs` (new file)

- [ ] **Wire Score prediction Pulses** — When a Score Cell rates a Signal, publish `Pulse("prediction.score.{cell_id}", predicted_score)`. When the gate verdict comes back, publish `Pulse("outcome.score.{cell_id}", actual_quality)`. **Verify**: Score → predict → gate → outcome → calibration update cycle works end-to-end.
  - Spec: `tmp/unified/10-LEARNING-LOOPS.md` §2.1
  - Code: `crates/roko-learn/src/scorer/`, `crates/roko-gate/src/`

- [ ] **Wire Route prediction Pulses** — When CascadeRoute selects a model, publish `Pulse("prediction.route.cascade", {model, expected_quality})`. After execution, publish outcome. **Verify**: route prediction → execution → outcome → calibration update cycle works.
  - Spec: `tmp/unified/10-LEARNING-LOOPS.md` §2.2
  - Code: `crates/roko-learn/src/routing/`

- [ ] **Wire Compose prediction Pulses** — When Compose assembles context, publish prediction of which sections will contribute to gate success. After gate, publish outcome. **Verify**: compose → predict → gate → outcome → section effect update cycle works.
  - Spec: `tmp/unified/10-LEARNING-LOOPS.md` §2.3
  - Code: `crates/roko-compose/src/`

## 1.6 Demurrage

- [ ] **Add demurrage fields to Signal** — Add `balance: f64` (starts at 1.0), `demurrage_paid: f64`, `last_touched_at: DateTime<Utc>` fields to Signal struct. Implement rate law: `balance(t+dt) = balance(t) - r*dt - beta*balance(t)*dt` where `r` = base rate, `beta` = proportional rate. **Verify**: a Signal untouched for 30 days has measurably lower balance than a fresh one.
  - Spec: `tmp/unified/01-SIGNAL.md` §6 (Demurrage), `tmp/unified/11-MEMORY-AND-KNOWLEDGE.md` §4
  - Code: `crates/roko-core/src/signal.rs`, `crates/roko-neuro/src/demurrage.rs` (new)

- [ ] **Implement reinforcement kinds** — Retrieved, Cited, GatePassed, Surprised, AgentQuoted. Each restores balance by a configurable amount with novelty weighting: `bonus * 1/(1+ln(freq))`. **Verify**: retrieving a Signal increases its balance. Citing a novel Signal gives more bonus than citing a frequently-cited one.
  - Spec: `tmp/unified/11-MEMORY-AND-KNOWLEDGE.md` §4.3 (Reinforcement)
  - Code: `crates/roko-neuro/src/demurrage.rs`

- [ ] **Wire demurrage into Store operations** — On `Store::get()`, update last_touched_at and apply reinforcement. On `Store::query()`, filter out Signals below cold threshold (balance < 0.01). On `Store::prune()`, archive cold Signals to ColdStore. **Verify**: knowledge query excludes decayed Signals. Prune moves cold Signals to archive.
  - Spec: `tmp/unified/11-MEMORY-AND-KNOWLEDGE.md` §4.4
  - Code: `crates/roko-neuro/src/store.rs`, `crates/roko-fs/src/`

- [ ] **Tier multipliers for demurrage** — Transient (0.1x slower decay), Working (0.5x), Consolidated (1.0x baseline), Persistent (5.0x slower decay). Tier promotion/demotion based on balance, time-since-touched, and citation count thresholds. **Verify**: a Persistent Signal decays 5x slower than a Consolidated one.
  - Spec: `tmp/unified/11-MEMORY-AND-KNOWLEDGE.md` §5 (Tier Progression)
  - Code: `crates/roko-neuro/src/demurrage.rs`

## 1.7 Heuristic Kind

- [ ] **Define `Kind::Heuristic` with payload** — `HeuristicPayload { when: String, then: String, falsifier: String, calibration: Calibration }`. `Calibration { trials: u32, confirmations: u32, violations: u32, brier_score: f64, wilson_ci: (f64, f64) }`. The mandatory `falsifier` field specifies what observation would disprove the heuristic. **Verify**: create a Heuristic Signal, confirm all fields serialize/deserialize.
  - Spec: `tmp/unified/01-SIGNAL.md` §4 (Heuristic Kind), `tmp/unified/11-MEMORY-AND-KNOWLEDGE.md` §6
  - Code: `crates/roko-core/src/kind.rs` (or `signal.rs`), `crates/roko-primitives/`

- [ ] **Wire heuristic calibration from Verify verdicts** — Subscribe to `outcome.verify.*` on Bus. When a Verify verdict references a Heuristic Signal in its context, update that Heuristic's calibration (increment trials, confirmations/violations based on verdict). Publish `calibration.heuristic.{id}.updated`. **Verify**: heuristic used in agent context → gate pass → heuristic.confirmations incremented.
  - Spec: `tmp/unified/11-MEMORY-AND-KNOWLEDGE.md` §6.2
  - Code: `crates/roko-learn/src/heuristic_calibration.rs` (new)

- [ ] **Wire heuristics into Compose context assembly** — Heuristic Signals are eligible for VCG context bidding. Their bid weight incorporates calibration quality (Wilson CI lower bound). Poorly-calibrated heuristics bid lower. **Verify**: a well-calibrated heuristic wins context slots over a poorly-calibrated one.
  - Spec: `tmp/unified/11-MEMORY-AND-KNOWLEDGE.md` §6.3
  - Code: `crates/roko-compose/src/bidders/`

## 1.8 EFE Routing

- [ ] **Implement Expected Free Energy (EFE) computation** — `EFE(a) = -epistemic_value(a) - pragmatic_value(a) + cost(a)` where epistemic = information gain (KL divergence of posterior vs prior), pragmatic = expected reward, cost = USD-equivalent. The agent selects the action minimizing EFE. Regime conditioning: `Calm` → explore (high epistemic weight), `Crisis` → exploit (high pragmatic weight). **Verify**: in Calm regime, EFE prefers an uncertain-but-cheap model over a known-good expensive one. In Crisis regime, the reverse.
  - Spec: `tmp/unified/02-CELL.md` §5 (Route protocol), `tmp/unified/10-LEARNING-LOOPS.md` §4
  - Code: `crates/roko-learn/src/routing/efe.rs` (new)

- [ ] **Replace LinUCB with EFE in CascadeRoute** — CascadeRoute currently uses LinUCB bandits. Replace with EFE. Port regime conditioning from roko-conductor (Calm/Normal/Volatile/Crisis detection). Wire regime Signal into RouteContext. **Verify**: CascadeRoute uses EFE for selection. Model choices change based on regime.
  - Spec: `tmp/unified/21-ROADMAP.md` §2.5
  - Code: `crates/roko-learn/src/routing/cascade.rs`, `crates/roko-conductor/`

## 1.9 Dream Cycle Trigger (Loop 3)

- [ ] **Wire dream cycle to run automatically** — Add a configurable cron/interval trigger so the dream cycle (NREM compression → REM imagination → Integration) runs without manual invocation. Wire into `roko serve` startup and `roko dashboard` background tasks. Configuration in `roko.toml` under `[learning.dreams]`. **Verify**: start `roko serve`, wait for configured interval, confirm dream cycle executes and logs to `.roko/episodes.jsonl`.
  - Spec: `tmp/unified/21-ROADMAP.md` §2.6
  - Code: `crates/roko-dreams/src/`, `crates/roko-serve/src/lib.rs`, `crates/roko-cli/src/orchestrate.rs`

## 1.10 Observe Protocol + Lenses

- [ ] **Define `Observe` trait in roko-core** — Methods: `observe(&self, ctx: &CellContext) -> Vec<Signal>`, `observes(&self) -> &[ObservableEventKind]`, `scope(&self) -> LensScope`. Define `LensScope` enum: `Cell`, `Graph`, `Agent`, `Space`, `Global`. Define `ObservableEventKind` enum covering Signal, Cell, Graph, Agent, and Space lifecycle events. **Verify**: trait compiles, trivial implementation works.
  - Spec: `tmp/unified/09-TELEMETRY.md` §2 (Observe protocol)
  - Code: `crates/roko-core/src/traits.rs`

- [ ] **Implement 10 builtin Lenses** — AgentLens (turns/tokens/cost/latency), PlanLens (tasks completed/failed/pending), VerifyLens (pass rates, threshold drift), RouteLens (model distribution, cost), MemoryLens (Signal counts, tier distribution, decay), CostLens (real-time cost), HealthLens, ErrorLens, ThroughputLens, DreamLens. Each implements Observe trait. **Verify**: each Lens produces observation Signals when attached to a running system.
  - Spec: `tmp/unified/09-TELEMETRY.md` §5 (Builtin Lenses), `tmp/unified/13-BUILTIN-BLOCK-CATALOG.md` §8
  - Code: `crates/roko-conductor/src/lenses/` (new directory)

- [ ] **Wire Lenses into TUI and HTTP routes** — TUI dashboard tabs (F1-F7) consume Lens output. HTTP routes under `/api/lenses/` expose Lens data as JSON. **Verify**: `roko dashboard` shows live Lens data. `curl localhost:6677/api/lenses/agent` returns JSON.
  - Spec: `tmp/unified/09-TELEMETRY.md` §4 (StateHub, surface consumption)
  - Code: `crates/roko-cli/src/tui/`, `crates/roko-serve/src/routes/`

## 1.11 Trigger Protocol

- [ ] **Define `Trigger` trait in roko-core** — Methods: `arm(&mut self, binding: &TriggerBinding) -> Result<TriggerHandle>`, `disarm(&mut self, handle: TriggerHandle) -> Result<()>`, `poll(&self, handle: &TriggerHandle) -> TriggerState`. Define `TriggerBinding` (connects event source to Graph), `TriggerHandle`, `TriggerState` (Armed/Firing/Cooldown/Disarmed/Failed), `TriggerSource` enum (Cron/Webhook/FileWatch/Bus/ChainEvent/Manual/SignalPattern). **Verify**: trait compiles.
  - Spec: `tmp/unified/06-TRIGGER-SYSTEM.md` §1-4
  - Code: `crates/roko-core/src/traits.rs`

- [ ] **Implement CronTrigger, BusTrigger, FileWatchTrigger** — Cron uses `tokio_cron_scheduler`. BusTrigger subscribes to Bus topics and fires when matching Pulse arrives. FileWatchTrigger uses `notify::RecommendedWatcher`. Each publishes `Pulse("trigger.fired.{id}", payload)` on Bus when firing. **Verify**: CronTrigger fires at configured interval. BusTrigger fires on matching Pulse. FileWatchTrigger fires on file change.
  - Spec: `tmp/unified/06-TRIGGER-SYSTEM.md` §6 (Builtin Triggers)
  - Code: `crates/roko-runtime/src/triggers/` (new directory)

## 1.12 Connect Protocol

- [ ] **Define `Connect` trait in roko-core** — Methods: `connect(&mut self, config: &ConnectorConfig) -> Result<ConnectionHandle>`, `query(&self, handle: &ConnectionHandle, req: QueryRequest) -> Result<QueryResponse>`, `execute(&self, handle: &ConnectionHandle, req: ExecuteRequest) -> Result<ExecuteResponse>`, `health(&self, handle: &ConnectionHandle) -> HealthStatus`, `disconnect(&mut self, handle: ConnectionHandle) -> Result<()>`. Define `ConnectorKind` enum, `ConnectorManifest`, reconnection strategy. **Verify**: trait compiles.
  - Spec: `tmp/unified/12-CONNECTIVITY.md` §1-3
  - Code: `crates/roko-core/src/traits.rs`

- [ ] **Refactor existing connectors to implement Connect** — MCP client in roko-agent, chain client in roko-chain, and any HTTP/webhook connectors should implement the Connect trait. **Verify**: MCP connection lifecycle goes through Connect trait. Health checks work.
  - Spec: `tmp/unified/12-CONNECTIVITY.md` §4 (Exoskeleton bindings)
  - Code: `crates/roko-agent/src/`, `crates/roko-chain/src/`

## 1.13 TypeSchema Validation

- [ ] **Implement TypeSchema and validation** — Define `TypeSchema` enum covering primitives (`String`, `Int`, `Float`, `Bool`, `Bytes`), collections (`List<T>`, `Map<K,V>`, `Set<T>`), named structs with typed fields, unions, and `Optional<T>`. Implement validation: given a `Signal` and a `TypeSchema`, confirm the Signal's payload conforms. Implement edge validation: given two connected Cells' output/input schemas, confirm compatibility at Graph load time. **Verify**: mismatched schemas produce clear error at load time.
  - Spec: `tmp/unified/14-CONFIG-AND-AUTHORING.md` §6, `tmp/unified/21-ROADMAP.md` §2.9
  - Code: `crates/roko-core/src/schema.rs` (new), `crates/roko-orchestrator/src/`
