# Phase 2 — Graph Engine + Agent Runtime

> Replace the plan executor with a proper Graph engine. Introduce type-state Agent lifecycle, CognitiveWorkspace, StateHub, Surfaces, Rack, the 5-tier SPI, and Marketplace v1.

**Spec source**: `tmp/unified/21-ROADMAP.md` §3 (Phase 2)
**Dependencies**: Phase 1 complete

---

## 2.1 Graph Schema and Loader

- [ ] **Define Graph TOML schema** — `Graph { id, name, version, nodes: Vec<Node>, edges: Vec<Edge>, entry: NodeId, exits: Vec<NodeId>, input_schema, output_schema, policy: GraphPolicy }`. `Node { id, label, kind: NodeKind, failure_strategy, max_retries, timeout, execution_class }`. `NodeKind` enum: `Cell`, `SubGraph`, `Branch`, `FanOut`, `FanIn`, `Loop`, `HumanInput`, `Wait`, `Slot`, `Noop`. `Edge { from, to, mapping: TypeMapping }`. `GraphPolicy { budget, deadline, failure_strategy, parallelism, hot: bool, clock_binding }`. **Verify**: a sample Graph TOML parses and validates.
  - Spec: `tmp/unified/03-GRAPH.md` §1-4
  - Code: `crates/roko-orchestrator/src/graph/schema.rs` (new)

- [ ] **Implement Graph loader with TypeSchema validation** — Load Graph from TOML. Validate all edges: output TypeSchema of source node must be compatible with input TypeSchema of target node. Validate capabilities: intersection of Cell capabilities with Graph allow-list. Detect cycles and classify as intentional loops vs errors. **Verify**: invalid edge types produce clear error. Valid Graph loads successfully.
  - Spec: `tmp/unified/03-GRAPH.md` §5 (Validation), `tmp/unified/05-EXECUTION-ENGINE.md` §3
  - Code: `crates/roko-orchestrator/src/graph/loader.rs` (new)

## 2.2 Graph Executor (Flow)

- [ ] **Implement Graph executor with Flow lifecycle** — `Flow` = Graph at runtime with `RunId`, state machine (`Created → Running → Completed/Failed/Cancelled/Paused`), per-node state tracking, and event emission on Bus. Executor walks the state graph: ready nodes execute in parallel (respecting semaphore), completed nodes unlock dependents, failed nodes follow failure strategy. **Verify**: a 5-node DAG Graph executes correctly with parallel middle nodes. Flow transitions through lifecycle states.
  - Spec: `tmp/unified/05-EXECUTION-ENGINE.md` §1-4
  - Code: `crates/roko-orchestrator/src/graph/executor.rs` (new), `crates/roko-orchestrator/src/graph/flow.rs` (new)

- [ ] **Implement failure strategies** — Per-node: `Fail` (propagate), `Retry { max, backoff }`, `Escalate { to_model }`, `Compensate { graph_id }`, `Skip`, `Detour { graph_id }`. Graph-level default strategy overridden per-node. **Verify**: a node configured with Retry(3) retries 3 times before failing. Skip bypasses a failed non-critical node.
  - Spec: `tmp/unified/05-EXECUTION-ENGINE.md` §6 (Failure Strategies)
  - Code: `crates/roko-orchestrator/src/graph/executor.rs`

- [ ] **Implement snapshot/resume** — `FlowSnapshot` captures: Flow state, per-node completion status, intermediate Signals, Bus subscription positions. Serialize to JSON. Resume from snapshot: skip completed nodes, replay deterministic orchestration, re-execute non-deterministic activities. **Verify**: pause a running Flow, serialize snapshot, resume from snapshot, complete successfully.
  - Spec: `tmp/unified/05-EXECUTION-ENGINE.md` §7 (Resumability)
  - Code: `crates/roko-orchestrator/src/graph/snapshot.rs` (new)

## 2.3 Hot Graph (Resident Execution)

- [ ] **Implement Hot Flow variant** — A Flow that stays resident in memory and re-fires on each clock tick. Used for the Agent's 9-step pipeline. State persists between ticks. Registration: `engine.register_hot(graph, clock_binding)`. Deregistration on agent shutdown. Hot Flows emit lifecycle Pulses like regular Flows. **Verify**: register a Hot Graph, send 3 ticks, confirm it executes 3 times with state carrying over.
  - Spec: `tmp/unified/05-EXECUTION-ENGINE.md` §8 (Hot Graphs), `tmp/unified/04-SPECIALIZATIONS.md` §1.1 (Hot Flow)
  - Code: `crates/roko-orchestrator/src/graph/hot.rs` (new)

## 2.4 Migration Tool

- [ ] **Implement `roko plan migrate` CLI** — Converts existing `tasks.toml` plans to Graph TOML format. Maps `[[task]]` entries to `[[node]]` with `kind = "Cell"`. Maps `depends_on` to `[[edge]]`. Preserves all task metadata (prompt, agent config, gate config). The existing plan format continues to work during transition (executor detects format). **Verify**: migrate an existing plan, execute via `roko plan run`, same results as old format.
  - Spec: `tmp/unified/21-ROADMAP.md` §7.2 (Plan-to-Graph Migration)
  - Code: `crates/roko-cli/src/migrate.rs` (new)

- [ ] **Wire Graph executor into `roko plan run`** — Detect plan format (old tasks.toml vs new Graph TOML). Dispatch to appropriate executor. Eventually deprecate old format. **Verify**: `roko plan run` works with both formats.
  - Code: `crates/roko-cli/src/orchestrate.rs`

## 2.5 Type-State Agent Lifecycle

- [ ] **Define type-state Agent struct** — `Agent<S: AgentState>` with states: `Provisioning`, `Active`, `Dreaming`, `Terminal`. Compile-time enforced transitions: `Agent<Provisioning> → Agent<Active>` (only after config + extensions loaded), `Agent<Active> ↔ Agent<Dreaming>`, `Agent<Active> → Agent<Terminal>`, `Agent<Dreaming> → Agent<Terminal>`. Methods available only in the appropriate state (e.g., `execute()` only on `Agent<Active>`). **Verify**: calling `execute()` on `Agent<Provisioning>` is a compile error.
  - Spec: `tmp/unified/07-AGENT-RUNTIME.md` §2 (Type-State Lifecycle)
  - Code: `crates/roko-agent/src/lifecycle.rs` (new)

- [ ] **Implement vitality model** — `vitality = remaining_budget / initial_budget` (0.0..1.0). Five behavioral phases: Thriving (1.0–0.7), Stable (0.7–0.4), Conservation (0.4–0.2), Declining (0.2–0.05), Terminal (<0.05). Phase affects: model tier allowed, exploration vs exploitation, task acceptance, knowledge transfer behavior. **Verify**: an Agent with 30% budget remaining is in Conservation phase and restricted to T0/T1 models.
  - Spec: `tmp/unified/07-AGENT-RUNTIME.md` §3 (Vitality)
  - Code: `crates/roko-agent/src/vitality.rs` (new)

- [ ] **Implement multi-slot state** — Agent manages N concurrent execution contexts (slots) with shared global limits (total budget, max concurrent, model pool). Each slot has its own CellContext but shares the Agent's Store and Bus. **Verify**: Agent with 3 slots can run 3 tasks concurrently while respecting shared budget.
  - Spec: `tmp/unified/07-AGENT-RUNTIME.md` §6 (Multi-Slot)
  - Code: `crates/roko-agent/src/slots.rs` (new)

## 2.6 CognitiveWorkspace

- [ ] **Implement CognitiveWorkspace with VCG auction** — Context assembly via Vickrey-Clarke-Groves auction. 8+ bidders (Neuro, Task, Research, Heuristic, Episode, Pheromone, Affect, System) bid for context window slots. VCG ensures truthful bidding: each bidder's payment = marginal social cost of their inclusion. Budget constraint: total context fits within model's context window. **Verify**: VCG selects higher-value context sections. Removing a bidder changes others' payments correctly.
  - Spec: `tmp/unified/07-AGENT-RUNTIME.md` §7 (CognitiveWorkspace)
  - Code: `crates/roko-compose/src/workspace.rs` (new or refactor existing)

- [ ] **Implement section effect tracking** — Beta-distribution posterior per context section type. After each Verify verdict, update the posterior for each section that was in context: `Beta(alpha + successes, beta + failures)`. Use posterior mean as section's bidding weight in future VCG auctions. **Verify**: a context section consistently present during gate passes develops high posterior. One consistently present during failures develops low posterior.
  - Spec: `tmp/unified/07-AGENT-RUNTIME.md` §7.2 (Section Effects)
  - Code: `crates/roko-compose/src/section_effects.rs` (new)

## 2.7 StateHub

- [ ] **Define StateHub projection types** — `FlowSummary`, `AgentStatus`, `SpaceStatus`, `MemoryStats`, `RouteStats`, `VerifyStats`, `CostSummary`. Each is a versioned struct with TTL. StateHub computes projections from Lens outputs and caches them. Projections are the data contract between system and surfaces. **Verify**: StateHub produces all projection types. Projections update when underlying Lens data changes.
  - Spec: `tmp/unified/09-TELEMETRY.md` §3 (StateHub)
  - Code: `crates/roko-conductor/src/statehub.rs` (new)

- [ ] **Wire StateHub into TUI, HTTP, and SSE/WebSocket** — TUI tabs consume StateHub projections. HTTP routes under `/api/statehub/` expose projections as JSON. SSE/WebSocket streams push projection updates. **Verify**: all three surfaces reflect the same data.
  - Spec: `tmp/unified/09-TELEMETRY.md` §4
  - Code: `crates/roko-cli/src/tui/`, `crates/roko-serve/src/routes/`

## 2.8 Five Named Surfaces

- [ ] **Define surface protocol contracts** — Each surface declares: projections consumed (from StateHub), events emitted (user actions), invariants (e.g., "Workbench always shows active Flows"). Define for all 5: Workbench (task delegation), Agent Inbox (ambient notification), Generative Canvas (visual Graph editor), Stigmergy Minimap (coordination overview), Autonomy Slider (progressive trust). **Verify**: contracts compile as trait definitions.
  - Spec: `tmp/unified/16-SURFACES.md` §2-6
  - Code: `crates/roko-core/src/surfaces.rs` (new)

- [ ] **Implement Workbench in TUI** — Task delegation surface. Shows: active Flows, agent slots, Graph topology, pending human input, recent completions. User can: assign tasks, fill slots, adjust macros, cancel/pause Flows. **Verify**: `roko dashboard` shows Workbench tab with live Flow data.
  - Spec: `tmp/unified/16-SURFACES.md` §3 (Workbench)
  - Code: `crates/roko-cli/src/tui/`

- [ ] **Implement Agent Inbox in TUI** — Notification surface. Three urgency levels: Critical, Urgent, Notice. Aggregation: groups related notifications. Notification lifecycle: created → read → acted → archived. **Verify**: agent errors appear as Critical notifications in Inbox.
  - Spec: `tmp/unified/16-SURFACES.md` §4 (Agent Inbox)
  - Code: `crates/roko-cli/src/tui/`

- [ ] **Implement Autonomy Slider in TUI** — Progressive trust control. 5 levels (0: full human control → 4: full autonomy). Per-capability granularity (e.g., FsWrite at level 2, Shell at level 1). Adjustable at runtime. Level changes emit Pulses on Bus. **Verify**: reducing autonomy level causes pending tool calls to require confirmation.
  - Spec: `tmp/unified/16-SURFACES.md` §6 (Autonomy Slider), `tmp/unified/17-SECURITY-MODEL.md` §6
  - Code: `crates/roko-cli/src/tui/`

## 2.9 Rack (Parameterized Graphs)

- [ ] **Define Rack struct** — `Rack { graph: Graph, macros: Vec<Macro>, slots: Vec<Slot> }`. `Macro { name, description, type_schema, default_value }` = user-adjustable parameters (knobs). `Slot { name, description, cell_schema }` = late-bound Cell references (jacks). Macros are expanded at load time (variable substitution in Graph TOML). Slots are bound at runtime. **Verify**: a Rack with 2 macros and 1 slot can be instantiated with different macro values and slot bindings.
  - Spec: `tmp/unified/04-SPECIALIZATIONS.md` §3 (Rack)
  - Code: `crates/roko-orchestrator/src/graph/rack.rs` (new)

## 2.10 5-Tier SPI

- [ ] **Tier 1: Prompt loader** — Load Markdown files with TOML front-matter as prompt templates. Resolution order: workspace > user > builtin. **Verify**: a `.roko/prompts/custom.md` file is discovered and usable in Compose.
  - Spec: `tmp/unified/14-CONFIG-AND-AUTHORING.md` §3 (5-Tier SPI)
  - Code: `crates/roko-compose/src/prompts/`

- [ ] **Tier 2: Config profile deep merge** — Load profile TOML files that customize agent behavior (model selection, temperature, gate config, etc.). Profiles merge with deep override semantics. **Verify**: workspace profile overrides user profile overrides builtin.
  - Spec: `tmp/unified/14-CONFIG-AND-AUTHORING.md` §3
  - Code: `crates/roko-cli/src/config/`

- [ ] **Tier 3: Declarative tool loader** — TOML-defined tools that wrap subprocesses, HTTP endpoints, or MCP servers. No Rust code needed. **Verify**: a TOML-defined tool can be used by an Agent.
  - Spec: `tmp/unified/14-CONFIG-AND-AUTHORING.md` §3
  - Code: `crates/roko-std/src/tools/`

- [ ] **Tier 4: WASM Cell runtime** — Define wit-bindgen ABI for Cells compiled to WASM. Integrate wasmtime for WASM execution with fuel metering (sandboxed). **Verify**: a simple Cell compiled to WASM executes within the Graph engine. Fuel limit prevents infinite loops.
  - Spec: `tmp/unified/14-CONFIG-AND-AUTHORING.md` §3, `tmp/unified/20-DEPLOYMENT.md` §5
  - Code: `crates/roko-core/src/wasm_abi/` (new), `crates/roko-runtime/src/wasm/` (new)

## 2.11 Marketplace v1

- [ ] **Cell manifest and local registry** — Define Cell manifest format (TOML: name, version, author, description, protocols, capabilities, input/output schemas). Implement local Cell registry that discovers, indexes, and resolves Cells from the workspace. **Verify**: `roko marketplace list` shows all discoverable Cells with their protocols.
  - Spec: `tmp/unified/15-MARKETPLACE-AND-SHARING.md` §3-4
  - Code: `crates/roko-core/src/manifest.rs` (new), `crates/roko-orchestrator/src/registry.rs` (new)

- [ ] **Implement `roko marketplace publish/install/fork` CLI** — Publish: package Cell + manifest + tests into artifact. Install: download and register artifact. Fork: copy Cell with new author, linking provenance to original. **Verify**: publish a Cell, install it in a fresh workspace, fork it with modifications.
  - Spec: `tmp/unified/15-MARKETPLACE-AND-SHARING.md` §5-7
  - Code: `crates/roko-cli/src/marketplace.rs` (new)

- [ ] **Marketplace HTTP routes** — `/api/marketplace/search`, `/api/marketplace/publish`, `/api/marketplace/install/{id}`, `/api/marketplace/fork/{id}`. **Verify**: HTTP API mirrors CLI functionality.
  - Spec: `tmp/unified/15-MARKETPLACE-AND-SHARING.md`
  - Code: `crates/roko-serve/src/routes/marketplace.rs` (new)
