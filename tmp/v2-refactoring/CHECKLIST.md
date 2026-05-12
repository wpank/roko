# V2 Refactoring — Master Checklist

Every item has: a wire target, a verification command, and a "done" definition.

**Done = compiled + tested + wired + exercised via CLI.**

---

## Phase 0: Quick Wins + Cleanup (2-3 days)

### Cleanup

- [ ] **QW-4**: Verify orchestrate.rs feature gate is clean
  - Wire: N/A (cleanup)
  - Verify: `cargo build -p roko-cli` without `legacy-orchestrate` feature
  - Done: No non-gated references to orchestrate module

- [ ] **QW-5**: Delete roko-calc skeleton
  - Wire: N/A (deletion)
  - Verify: `cargo build --workspace`
  - Done: `crates/roko-calc/` removed, workspace Cargo.toml updated

- [ ] **QW-7**: Tag all floating code with STATUS comments
  - Wire: N/A (documentation)
  - Verify: `grep -rn 'STATUS: NOT WIRED' crates/ --include='*.rs'` returns all 18 items
  - Done: Every floating module has a status header

### Quick Additions

- [ ] **QW-3**: Add `balance: f64` field to Engram/Signal
  - Wire: Store::put/get — field is persisted/loaded
  - Verify: `cargo test -p roko-core` + check JSONL backwards compat
  - Done: `balance` field exists with serde default, touch() method works

- [ ] **QW-2**: Add And/Or/Not to TopicFilter
  - Wire: PulseBus subscriptions
  - Verify: `cargo test -p roko-core` — new filter combinator tests pass
  - Done: TopicFilter has And/Or/Not variants, matches() handles them

### Quick Wiring (floating code → runtime)

- [ ] **QW-8**: Wire `calibration_policy` → CascadeRouter
  - Wire: Runner v2 agent turn completion → calibration event → router update
  - Verify: Run a plan, check `.roko/learn/cascade-router.json` shows calibration updates
  - Done: CalibrationPolicy is called after every agent turn, router confidence updates

- [ ] **DCA-1**: Wire `demurrage_consumer` → periodic Store::prune()
  - Wire: `roko serve` spawns a tokio interval task calling Store::prune()
  - Verify: Start `roko serve`, wait, check store size decreases for low-balance signals
  - Done: Demurrage pruning runs on interval in serve runtime

- [ ] **DCA-2**: Wire `run_ledger` → Runner v2 per-task cost tracking
  - Wire: Runner v2 event loop calls RunLedger::record_cost() after each task
  - Verify: Run a plan, check per-task cost breakdown in run report
  - Done: `roko plan run` output includes per-task cost summary

- [ ] **DCA-3**: Wire `error_enrichment` → gate failure handler
  - Wire: Runner v2 gate failure path calls ErrorEnrichment::enrich()
  - Verify: Trigger a gate failure, check retry prompt includes enriched error context
  - Done: Gate failure retry prompts include error pattern analysis

- [ ] **DCA-4**: Wire `jsonl_rotation` → episode/efficiency logs
  - Wire: EpisodeLogger uses JsonlRotation for log file management
  - Verify: Generate enough episodes to trigger rotation, check rotated files exist
  - Done: `.roko/episodes.jsonl` rotates at configured size threshold

- [ ] **DCA-5**: Wire `post_gate_reflection` → gate failure prompt
  - Wire: Runner v2 gate failure calls PostGateReflection::reflect()
  - Verify: Gate failure, check reflection insights in retry prompt
  - Done: Reflection output appears in agent retry context

- [ ] **DCA-6**: Wire `section_outcome` → EpisodeLogger
  - Wire: After successful task, log which prompt sections were used
  - Verify: Check episodes.jsonl contains section_outcome data
  - Done: Section effectiveness tracked in episode records

---

## Phase 1: Core Abstractions (1-2 weeks)

### 1A: Cell Gets execute()

- [ ] **P1-1**: Add CellContext struct to roko-core
  - Wire: Constructed in WorkflowEngine and Runner v2 (passed to cells)
  - Verify: `cargo build --workspace`
  - Done: CellContext has bus + store + cancel, constructable from runtime

- [ ] **P1-2**: Add execute() default method to Cell trait
  - Wire: Default impl returns error — backwards compat, no callers break
  - Verify: `cargo test --workspace` — all existing Cell impls compile unchanged
  - Done: Cell trait has execute() with default error impl

- [ ] **P1-3**: Add TypeSchema enum to roko-core
  - Wire: Cell trait gains input_schema()/output_schema() with None defaults
  - Verify: `cargo test -p roko-core`
  - Done: TypeSchema::Any/OfKind/JsonSchema exists with is_compatible_with()

- [ ] **P1-4**: Implement execute() on CompileGate
  - Wire: `roko graph run` (Phase 2) will call it — for now, integration test
  - Verify: Integration test: create CompileGate, call execute(), check verdict Signal
  - Done: CompileGate::execute() wraps verify() → Signal output

- [ ] **P1-5**: Implement execute() on at least 3 gates
  - Wire: TestGate, ClippyGate, DiffGate — same pattern as CompileGate
  - Verify: Integration tests per gate
  - Done: 4 gates have execute() implementations + tests

### 1B: Signal Rename

- [ ] **P1-6**: Rename Engram → Signal in roko-core
  - Wire: Everything that uses Engram (via deprecated alias)
  - Verify: `cargo build --workspace` — alias ensures no breaks
  - Done: `pub struct Signal` is canonical, `pub type Engram = Signal` is deprecated

- [ ] **P1-7**: Update roko-core internals to use Signal
  - Wire: N/A — internal rename
  - Verify: `cargo test -p roko-core`
  - Done: No internal uses of `Engram` in roko-core (only the deprecated alias)

- [ ] **P1-8**: Update 5 most-used crates to use Signal
  - Wire: roko-agent, roko-gate, roko-learn, roko-compose, roko-orchestrator
  - Verify: `cargo build --workspace` — deprecated warnings may appear
  - Done: 5 crates import Signal directly (not through Engram alias)

### 1C: New Protocol Traits

- [ ] **P1-9**: Add Observe trait to roko-core/src/traits.rs
  - Wire: StoreObserver impl (see below)
  - Verify: `cargo build -p roko-core`
  - Done: Observe trait exists with observe() method

- [ ] **P1-10**: Implement StoreObserver + wire into `roko status`
  - Wire: `roko status` calls StoreObserver::observe() for storage stats
  - Verify: `cargo run -p roko-cli -- status` shows store statistics
  - Done: `roko status` output includes signal count, store size via Observe trait

- [ ] **P1-11**: Add Connect trait to roko-core/src/traits.rs
  - Wire: ProviderConnection impl (see below)
  - Verify: `cargo build -p roko-core`
  - Done: Connect trait exists with open/close/health/request methods

- [ ] **P1-12**: Implement ProviderConnection + wire into provider health check
  - Wire: `roko config providers health` calls Connect::health()
  - Verify: `cargo run -p roko-cli -- config providers health` shows connection status
  - Done: Provider health check uses Connect trait

- [ ] **P1-13**: Add Trigger trait to roko-core/src/traits.rs
  - Wire: BusTrigger impl (see below)
  - Verify: `cargo build -p roko-core`
  - Done: Trigger trait exists with arm/check/disarm methods

- [ ] **P1-14**: Implement BusTrigger + wire into event subscription system
  - Wire: `roko config subscriptions` uses Trigger::arm() to register
  - Verify: Integration test: arm trigger, publish matching pulse, check fires
  - Done: BusTrigger watches Bus topics, fires on match

---

## Phase 2: Graph + Engine (4-6 weeks)

### 2A: New Crate + Types

- [ ] **P2-1**: Create roko-graph crate with Graph/Node/Edge types
  - Wire: `roko graph run` command (see below)
  - Verify: `cargo build -p roko-graph`
  - Done: Graph, Node, Edge, GraphPolicy structs compile

- [ ] **P2-2**: Implement TOML → Graph loader
  - Wire: `roko graph run <file.toml>` parses graph definition
  - Verify: Unit test: parse a TOML graph definition
  - Done: load_graph() parses TOML → Graph struct

- [ ] **P2-3**: Implement CellRegistry
  - Wire: Engine uses it to resolve CellRef::Named to Cell impls
  - Verify: Unit test: register cell, retrieve by name
  - Done: CellRegistry has register() and get() methods

- [ ] **P2-4**: Implement topological sort for Graph
  - Wire: Engine uses it to determine execution order
  - Verify: Unit test: sort a 5-node DAG, verify order
  - Done: topological_sort() handles DAGs, detects cycles

### 2B: Engine (Simplest Version)

- [ ] **P2-5**: Implement Engine with sequential linear pipeline execution
  - Wire: `roko graph run <file.toml>`
  - Verify: Run a 3-node linear graph (gate→gate→gate), verify output
  - Done: Engine::start() executes linear pipelines

- [ ] **P2-6**: Add `roko graph run` CLI command
  - Wire: CLI entry point calls Engine
  - Verify: `cargo run -p roko-cli -- graph run test-graph.toml`
  - Done: Command exists, loads graph, runs engine, prints output

- [ ] **P2-7**: Register default cells (gates, scorer, composer)
  - Wire: build_default_registry() creates cells from existing implementations
  - Verify: `roko graph run` with a gate graph resolves cells by name
  - Done: At least 5 cells registered and resolvable

- [ ] **P2-8**: Write 3 example graph TOML files
  - Wire: Used by `roko graph run` for testing and documentation
  - Verify: Each graph runs successfully via CLI
  - Done: 3 working graph definitions in `examples/graphs/`

### 2C: Engine Features (Incremental)

- [ ] **P2-9**: Add fan-out/fan-in support
  - Wire: Graph TOML with parallel nodes
  - Verify: Run a graph with 3 parallel gates, verify all execute
  - Done: FanOut splits signals, FanIn joins with configurable strategy

- [ ] **P2-10**: Add conditional edges
  - Wire: Graph TOML with `condition = "verdict.hard_pass == true"`
  - Verify: Run a graph where a gate fails, verify skip branch
  - Done: Conditional edges evaluate and skip correctly

- [ ] **P2-11**: Add sub-graph support (Graph as Cell)
  - Wire: Graph TOML with inline sub-graph node
  - Verify: Run a graph containing a nested sub-graph
  - Done: CellRef::SubGraph recursively executes

- [ ] **P2-12**: Add Flow snapshots + resume
  - Wire: `roko graph run --resume <snapshot>`
  - Verify: Kill a graph mid-execution, resume from snapshot
  - Done: FlowSnapshot persists node outputs, resume skips completed nodes

- [ ] **P2-13**: Add budget enforcement
  - Wire: GraphPolicy.max_budget → Engine cancels when exceeded
  - Verify: Run a graph with low budget, verify cancellation
  - Done: Engine tracks cost per cell, cancels on budget exceeded

- [ ] **P2-14**: Add deadline enforcement
  - Wire: GraphPolicy.deadline → Engine cancels on timeout
  - Verify: Run a graph with short deadline, verify timeout
  - Done: Engine cancels on deadline

### 2D: Agent as Cell

- [ ] **P2-15**: Implement AgentCell wrapping existing agent dispatch
  - Wire: Graph node `cell = "claude-agent"` dispatches to Claude CLI
  - Verify: `roko graph run` with an agent node, verify LLM response
  - Done: AgentCell::execute() dispatches to LLM, returns response as Signal

- [ ] **P2-16**: Implement ComposeCell wrapping SystemPromptBuilder
  - Wire: Graph node `cell = "system-prompt-builder"` builds prompts
  - Verify: Run a compose→agent graph, verify prompt assembly
  - Done: ComposeCell::execute() builds 9-layer prompt, outputs as Signal

- [ ] **P2-17**: Write a complete task-execution graph
  - Wire: Graph that does: compose → agent → verify → persist
  - Verify: `roko graph run task-execution.toml` executes a coding task
  - Done: End-to-end task execution via graph matches Runner v2 output

---

## Phase 3: Feeds + Graduation (2-3 weeks)

### 3A: Feeds

- [ ] **P3-1**: Add Feed trait to roko-core
  - Wire: FileWatchFeed impl (see below)
  - Verify: `cargo build -p roko-core`
  - Done: Feed trait with topic/start/stop/poll/status methods

- [ ] **P3-2**: Implement FileWatchFeed
  - Wire: TUI file watcher → Feed-based
  - Verify: Start feed, modify file, see Pulse on Bus
  - Done: FileWatchFeed publishes file change Pulses

- [ ] **P3-3**: Implement ProviderHealthFeed
  - Wire: `roko serve` registers feed, SSE clients receive health updates
  - Verify: Start serve, check SSE for provider health pulses
  - Done: ProviderHealthFeed polls providers, publishes status Pulses

- [ ] **P3-4**: Add `roko feed list/status` CLI
  - Wire: Shows registered feeds and their status
  - Verify: `cargo run -p roko-cli -- feed list`
  - Done: Command shows feed names, topics, rates, connection status

### 3B: Graduation

- [ ] **P3-5**: Add Pulse::graduate() method
  - Wire: Called by GraduationCell
  - Verify: Unit test: pulse.graduate() returns valid Signal
  - Done: Pulse → Signal conversion preserves kind, body, lineage

- [ ] **P3-6**: Add graduation policy config to roko.toml
  - Wire: Config loader parses graduation section
  - Verify: Parse a roko.toml with graduation policies
  - Done: GraduationPolicy structs load from config

- [ ] **P3-7**: Implement GraduationCell (React Cell)
  - Wire: Engine registers as background React Cell
  - Verify: Publish a gate verdict Pulse, check Signal appears in Store
  - Done: GraduationCell watches Bus, promotes matching Pulses to Store

- [ ] **P3-8**: Wire predict-publish-correct for CascadeRouter
  - Wire: Router publishes prediction Pulse, outcome publishes after turn,
    CalibrationPolicy joins and updates router
  - Verify: Run 3 agent tasks, check router confidence evolves
  - Done: CascadeRouter's model selection improves with calibration feedback

---

## Phase 4: Migration (2-4 weeks)

### 4A: Plan Execution via Engine

- [ ] **P4-1**: Implement plan TOML → Graph converter
  - Wire: `roko plan run` can optionally use Engine path
  - Verify: Convert an existing plan to a Graph, compare structure
  - Done: Existing task.toml plans parse into valid Graphs

- [ ] **P4-2**: Add `--engine graph` flag to `roko plan run`
  - Wire: `roko plan run plans/ --engine graph` uses Engine
  - Verify: Run same plan via both engines, compare results
  - Done: Both paths produce equivalent results

- [ ] **P4-3**: Run 5 existing plans through Engine path
  - Wire: Integration tests comparing Engine vs Runner v2 output
  - Verify: All 5 plans produce equivalent results
  - Done: Engine handles real plans correctly

- [ ] **P4-4**: Make Engine the default for `roko plan run`
  - Wire: Default engine switches from Runner v2 to Graph Engine
  - Verify: `roko plan run` uses Engine by default
  - Done: Runner v2 feature-gated like orchestrate.rs was

- [ ] **P4-5**: Feature-gate Runner v2
  - Wire: `legacy-runner-v2` feature flag
  - Verify: `cargo build -p roko-cli` without feature
  - Done: Runner v2 is feature-gated, Engine is default

### 4B: Hot Graphs (Agent Cognitive Loop)

- [ ] **P4-6**: Add Hot Graph support to Engine (tick-driven, resident)
  - Wire: `roko agent start` uses Hot Graph for agent loop
  - Verify: Start agent, verify cognitive loop ticks
  - Done: Hot Graphs persist state across ticks

- [ ] **P4-7**: Define cognitive loop as a Graph
  - Wire: Sense → Assess → Compose → Act → Verify → Persist → React
  - Verify: Agent loop runs as a graph, produces same behavior
  - Done: Agent cognitive loop is declarative TOML, not procedural code

---

## Tracking

Total items: **58**

| Phase | Items | Status |
|-------|-------|--------|
| Phase 0 | 14 | Not started |
| Phase 1 | 14 | Not started |
| Phase 2 | 17 | Not started |
| Phase 3 | 8 | Not started |
| Phase 4 | 7 | Not started |

Last updated: 2026-05-05
