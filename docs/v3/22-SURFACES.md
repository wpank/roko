# 22 -- Named Surfaces

> Five named surfaces -- Workbench, Agent Inbox, Generative Canvas, Stigmergy Minimap,
> Autonomy Slider -- define protocol-level data contracts between system and user.
> Surfaces are typed projections from the StateHub plus interaction contracts. CLI, TUI,
> Dashboard, and Visual Editor parity is the target architecture. Third parties can build
> surfaces consuming the same projections and emitting the same events.

> **Implementation status (E37, verified 2026-09-15): PARTIAL -- contract/backend tranche
> complete (9/9); rendering partial.** Typed Workbench, Inbox, Canvas, Minimap, and Autonomy
> projections; Inbox and autonomy events; 12 object types; five dedicated StateHub-backed
> HTTP routes; OpenAPI discovery; and the legacy-tab compatibility mapping are implemented.
> Full named-surface TUI rendering remains a product residual: the eleven legacy TUI tabs do
> not yet render every named surface, SurfaceEvents are not a command-ingress API, no
> production Inbox publisher/action consumer or live AutonomyConfig store exists, Workbench
> human input has no live source, Canvas graph identity comes from dashboard plan IDs, and
> Minimap coordinates use a deterministic layout rather than HDC. OpenAPI response bodies
> remain generic JSON schemas, and unresolved Inbox receipt timestamps are recomputed during
> JSONL replay.

### Implementation sources

| Surface | Authority | Shipped boundary |
|---------|-----------|-----------------|
| Projection contract types | `crates/roko-serve/src/projection_contract.rs` | `WorkbenchProjection`, `InboxProjection`, `CanvasProjection`, `MinimapProjection`, `AutonomyProjection`, `ProjectionEnvelope<T>`, `FlowSummary`, `SlotState`, `InboxItem`, `AgentPosition`, `CFactorSummary` |
| Five dedicated surface routes | `crates/roko-serve/src/routes/projections.rs` | `GET /api/projections/{workbench,inbox,canvas,minimap,autonomy}` |
| StateHub core projections | `crates/roko-core/src/telemetry_projections.rs` | 7 portable schemas: `CohortHealthProjection`, `ActiveTasksProjection`, `GatePipelineProjection`, `CostMeterProjection`, `KnowledgeHealthProjection`, `CFactorProjection`, `AgentVitalityProjection` |
| Dashboard event model | `crates/roko-core/src/dashboard_snapshot.rs` | `DashboardSnapshot`, `DashboardEvent`, `InboxCategory` (8 variants), `UrgencyLevel` (3 variants), `InboxRouting`, `inbox_routing()` |
| SurfaceEvent commands | `crates/roko-core/src/runtime_event.rs` | 7 variants: `TaskAssign`, `SlotFill`, `MacroAdjust`, `FlowCancel`, `FlowPause`, `FlowResume`, `HumanRespond` |
| Autonomy model | `crates/roko-core/src/agent.rs` | `AutonomyLevel` (5 levels, 0-4), `AutonomyConfig` (per-capability `HashMap`) |
| TUI tab/surface mapping | `crates/roko-cli/src/tui/tabs.rs` | `V2Surface` enum (7 identities), `Tab::v2_surfaces()` compatibility mapping |
| Surface inventory | `crates/roko-cli/src/surface_inventory.rs` | `SurfaceEntry`, `SurfaceStatus`, `SurfaceKind`, `full_inventory()` |
| Projection catalog + policies | `crates/roko-serve/src/projection_contract.rs` | `projection_policies()` returns 30 `ProjectionCatalogEntry` items with invalidation triggers |
| SSE stream plumbing | `crates/roko-serve/src/routes/projections.rs` | `stream_projection()`, `stream_telemetry()` with cursor-based replay and lag recovery |
| SurfaceEvent HTTP ingress | `crates/roko-serve/src/routes/run.rs` | `POST /api/surface-events` accepting `SurfaceEvent` JSON |
| TUI SurfaceEvent emission | `crates/roko-cli/src/tui/app/mod.rs` | `emit_surface_event()`, `subscribe_surface_events()` broadcast channel |

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse, Bus),
[02-CELL](02-CELL.md) (Cell protocol), [03-GRAPH](03-GRAPH.md) (Graph composition),
[05-AGENT](05-AGENT.md) (vitality, CorticalState),
[08-LEARNING](08-LEARNING.md) (episodes, efficiency, cost tracking),
[12-SAFETY](12-SAFETY.md) (autonomy levels 0-4, capability model)

**Depth files**:
[projection-contract](depth/22-surfaces/projection-contract.md),
[surface-events](depth/22-surfaces/surface-events.md),
[legacy-tab-mapping](depth/22-surfaces/legacy-tab-mapping.md),
[openapi-surface-routes](depth/22-surfaces/openapi-surface-routes.md)

---

## 1. Design Principles

A **surface** is a projection from the StateHub plus an interaction contract. The projection
defines what data the surface consumes. The interaction contract defines what events the
surface emits. Third parties can build new surfaces by consuming the same projections and
emitting the same events.

Five named surfaces. Four rendering targets. Every surface can be rendered by any target --
but each target has natural affinities.

| Surface | What it is | Primary target | Secondary targets |
|---|---|---|---|
| **Workbench** | Structured task delegation (Linear/Notion pattern, not blank chat) | Dashboard | TUI, CLI |
| **Agent Inbox** | Ambient notification (calm technology) | Dashboard, TUI | CLI |
| **Generative Canvas** | Visual Graph editor (nodes-as-cards, typed cables, drag-and-drop) | Dashboard (Visual Editor) | TUI (state-graph view) |
| **Stigmergy Minimap** | RTS-style coordination (fog-of-war, group selection) | Dashboard | TUI |
| **Autonomy Slider** | Progressive trust (5 levels 0-4, per-capability granularity) | Dashboard | TUI, CLI |

Target invariant: every operation that one rendering target supports should be available to
the others through appropriate UX. Today the shared projection/event contracts are ahead of
renderer parity.

### 1.1 Surface Architecture

Surfaces follow a strict separation between **data contracts** and **rendering targets**.
The data contract is the stable API; rendering targets are interchangeable implementations.

```
                    +-----------+
                    |  StateHub |  (Lens projections, DashboardSnapshot)
                    +-----+-----+
                          |
                +---------+---------+
                |                   |
         Typed Projections    DashboardEvents
                |                   |
    +-----------+-----------+      SSE / Bus
    |     |     |     |     |
    WB   IB   CV   MM   AU   (5 surface projection structs)
    |     |     |     |     |
    +-----+-----+-----+-----+
          |           |
    HTTP routes    TUI bridge
          |           |
    Dashboard      ratatui
     (REST)        (F1-F11)
```

Every surface projection is wrapped in a `ProjectionEnvelope<T>` that carries schema version,
monotonic cursor, staleness information, and a recovery flag. Consumers detect stale data,
handle schema changes, and distinguish recovered (post-restart) state from live state.

```rust
pub struct ProjectionEnvelope<T> {
    pub name: String,        // e.g., "workbench"
    pub version: u32,        // bumped on breaking shape changes
    pub cursor: u64,         // monotonic StateHub sequence
    pub computed_at: String,  // RFC 3339 timestamp
    pub recovered: bool,     // true = loaded from disk, not live stream
    pub data: T,             // the typed projection body
}
```

### 1.2 Surface-to-StateHub Projection Cross-Reference

Each surface consumes a subset of the 7 core StateHub projections. This table is the
canonical mapping between surfaces and telemetry.

| StateHub Projection | Type | Surfaces That Consume It |
|---|---|---|
| `cohort_health` | `CohortHealthProjection` | Agent Inbox, Stigmergy Minimap |
| `active_tasks` | `ActiveTasksProjection` | Workbench, Generative Canvas |
| `gate_pipeline` | `GatePipelineProjection` | Workbench, Generative Canvas |
| `cost_meter` | `CostMeterProjection` | Workbench |
| `knowledge_health` | `KnowledgeHealthProjection` | Stigmergy Minimap |
| `c_factor` | `CFactorProjection` | Stigmergy Minimap, Autonomy Slider |
| `agent_vitality` | `AgentVitalityProjection` | Agent Inbox, Autonomy Slider |

Surfaces never read raw Lens output. The projection schemas (defined in
`crates/roko-core/src/telemetry_projections.rs`) are the stable API between telemetry and UX.

---

## 2. The 12 Primitive Object Types

The authoring system treats every object as a typed composition of primitives. No
special-case configuration blobs.

| # | Type | What it represents |
|---|---|---|
| 1 | **Agent** | Configured runtime: domain + tool profiles + gate pipelines + model preferences + budget |
| 2 | **Extension** | Modular behavior unit: Roko-native (Rust, 23 hooks, 8 layers), or Roko-enhanced (JS/TS + heartbeat) |
| 3 | **Connector** | External system I/O adapter: chain RPC, exchange API, MCP server, database, webhook |
| 4 | **Gate** | Verification step: shell command, Rust function, chain simulation, risk check |
| 5 | **Feed** | Continuous data stream: price feeds, block events, CI status, file changes, webhooks |
| 6 | **Recipe** | Data transform pipeline: map, filter, window, aggregate, score over feeds |
| 7 | **Plan** | Task DAG with dependencies, checkpoints, error policy, budget |
| 8 | **Scorer** | Quality evaluator: metric computation along specific dimensions |
| 9 | **Arena** | Evaluation environment: task source + scoring function + leaderboard rules |
| 10 | **Group** | Agent collective: cluster topology, coordination policy, resource sharing |
| 11 | **Knowledge** | Curated knowledge bundle: Signal collections with provenance |
| 12 | **Config** | Configuration as Signal: content-addressed, versioned, lineage-tracked |

---

## 3. Surface Contracts

Each surface is defined by three things:
1. **Projections consumed** -- typed data from StateHub that the surface reads.
2. **Events emitted** -- typed actions the surface sends back to the system.
3. **Invariants** -- behavioral contracts that all implementations must satisfy.

### 3.1 Workbench

The primary interaction surface. Delegates structured work to agents -- not a blank chat
box, but a task-oriented surface modeled on Linear/Notion.

**Projections consumed:**

| Projection | StateHub Core Projection | Shape |
|---|---|---|
| Active Flows | `active_tasks` | `Vec<FlowSummary>` -- id, graph name, progress %, cost, duration, status |
| Agent Slots | `agent_vitality` | `Vec<SlotState>` -- agent id, slot index, occupied/free, current task, vitality |
| Pending Human Input | `active_tasks` | `Vec<HumanInputRequest>` -- run id, cell id, prompt, urgency, deadline |
| Gate Status | `gate_pipeline` | `GatePipelineProjection` -- rung snapshots, pass rates, avg reward |
| Cost | `cost_meter` | `CostMeterProjection` -- total, remaining, burn rate, trend |

**Workbench projection type (shipped):**

```rust
pub struct WorkbenchProjection {
    pub flows: Vec<FlowSummary>,
    pub slots: Vec<SlotState>,
    pub gate_pipeline: GatePipelineProjection,
    pub cost_meter: CostMeterProjection,
}

pub struct FlowSummary {
    pub run_id: String,
    pub graph_name: String,
    pub progress_pct: f64,
    pub cost_usd: f64,
    pub elapsed: Duration,
    pub status: FlowStatus,
    pub active_nodes: Vec<String>,
    pub pending_human: Option<HumanInputRequest>,
}

pub enum FlowStatus {
    Running,
    Paused,
    WaitingHuman,
    Completed { verdict: String },
    Failed { error: String },
    Cancelled,
}

pub struct SlotState {
    pub agent_id: String,
    pub slot_index: usize,
    pub occupied: bool,
    pub current_task: Option<String>,
    pub vitality: f64,
}
```

**Events emitted:**

| Event | Payload | Effect |
|---|---|---|
| `TaskAssign` | `{ graph, inputs, budget, deadline }` | Contract exists; a command consumer remains open |
| `SlotFill` | `{ agent_id, slot_index, cell_ref }` | Fills an Agent's Slot with a Cell |
| `MacroAdjust` | `{ run_id, macro_name, new_value }` | Adjusts a Macro on a running Flow |
| `FlowCancel` | `{ run_id }` | Cancels an active Flow |
| `FlowPause` | `{ run_id }` | Pauses an active Flow |
| `FlowResume` | `{ run_id }` | Resumes a paused Flow |
| `HumanRespond` | `{ run_id, cell_id, response }` | Answers a human-input prompt |

**Invariants:**
- Active Flows are always visible. A Workbench that hides running work is broken.
- Pending human input is surfaced with urgency. The user must never miss a decision request.
- Cost and duration are live, not polled. Updates arrive via Bus subscription.

### 3.2 Agent Inbox

Ambient notification surface. Calm technology -- peripheral attention until something needs
focus. Modeled on notification center, not chat. Three urgency levels at three priority bands.

**Projections consumed:**

| Projection | StateHub Core Projection | Shape |
|---|---|---|
| Attention Pulses | Bus (filtered by `tagged_for_human: true`) | `Vec<InboxItem>` |
| Agent Health | `agent_vitality` | Agent vitality + phase for context |
| Cohort Overview | `cohort_health` | Error rate, regime distribution for triage |

**Inbox projection type (shipped):**

```rust
pub struct InboxProjection {
    pub items: Vec<InboxItem>,
    pub pending_count: usize,
    pub agent_vitality: AgentVitalityProjection,
    pub cohort_health: CohortHealthProjection,
}

pub struct InboxItem {
    pub id: String,
    pub urgency: UrgencyLevel,
    pub category: InboxCategory,
    pub summary: String,
    pub detail: Value,
    pub source_agent: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub blocking_run: Option<String>,
}
```

**InboxCategory enum (8 variants, shipped in `roko-core`):**

```rust
pub enum InboxCategory {
    GateVerdict,       // Gate verdict requiring human decision
    AgentQuestion,     // Agent requesting clarification
    BudgetAlert,       // Budget threshold crossed
    TaskCompletion,    // Agent completed a task or Flow
    StructuralChange,  // New Graph, config modification proposed
    SecurityEvent,     // Capability violation, quarantine, anomaly
    KnowledgeEvent,    // Dream cycle complete, heuristic falsified
    SystemEvent,       // Daemon status, provider health, deployment
}
```

**Inbox routing by urgency and category:**

| InboxCategory | Typical Urgency | Transport Strip | Dashboard Badge | Full Inbox Panel |
|---|---|---|---|---|
| `GateVerdict` | Review (L3) | Yes | Yes (red dot) | Yes |
| `AgentQuestion` | Question (L2) | Yes | Yes | Yes |
| `BudgetAlert` | Question (L2) | Yes | Yes | Yes |
| `TaskCompletion` | Notify (L1) | No | Yes (count) | Yes |
| `StructuralChange` | Review (L3) | Yes | Yes (red dot) | Yes |
| `SecurityEvent` | Review (L3) | Yes | Yes (red dot) | Yes |
| `KnowledgeEvent` | Notify (L1) | No | Yes (count) | Yes |
| `SystemEvent` | Notify (L1) | No | Yes (count) | Yes |

The routing policy is implemented as `inbox_routing()` in `roko-core/src/dashboard_snapshot.rs`
and returns `InboxRouting { urgency, transport_strip, badge, full_panel }` per category.

**Urgency levels:**

| Level | Name | Behavior | Example |
|---|---|---|---|
| 1 | **Notify** | Badge count. No interruption. | "Agent completed code review." |
| 2 | **Question** | Gentle chime. Requires answer within deadline. | "Agent found 3 security issues. Deploy anyway?" |
| 3 | **Review** | Persistent banner. Blocks progress until resolved. | "Structural change proposed: new Graph. Approve?" |

**Invariants:**
- Level 3 (Review) items block relevant Flows until resolved. The surface must make them unmissable.
- Level 2 (Question) items have deadlines. The surface must show countdown.
- Level 1 (Notify) items do not interrupt focus. Badge count only.
- Items expire naturally via demurrage. Old unresolved items fade.

### 3.3 Generative Canvas

Visual Graph editor. Nodes as cards, typed cables, drag-and-drop composition. The authoring
surface for Graphs, Racks, Triggers, and Profiles.

**Projections consumed:**

| Projection | StateHub Core Projection | Shape |
|---|---|---|
| Graph Identities | Dashboard plan IDs (current source) | `Vec<String>` -- available graph/plan names |
| Live Flow State | `active_tasks` | Task statuses for live overlay |
| Gate Overlay | `gate_pipeline` | Per-node pass/fail indicators |

**Canvas projection type (shipped):**

```rust
pub struct CanvasProjection {
    pub graph_names: Vec<String>,
    /// Source used for graph identities until GraphRegistry joins AppState.
    pub graph_source: String,  // currently "dashboard_plan_ids"
    pub active_tasks: ActiveTasksProjection,
    pub gate_pipeline: GatePipelineProjection,
}
```

**Events emitted:**

| Event | Payload | Effect |
|---|---|---|
| `NodeAdd` | `{ cell_ref, position }` | Adds a Cell node to the Graph |
| `NodeRemove` | `{ node_id }` | Removes a node |
| `EdgeCreate` | `{ from_node, from_port, to_node, to_port }` | Wires a typed edge |
| `EdgeRemove` | `{ edge_id }` | Removes an edge |
| `MacroPromote` | `{ node_id, param_name, macro_def }` | Promotes a Cell parameter to a Rack Macro |
| `GraphSave` | `{ graph_toml }` | Persists the Graph |

**Invariants:**
- Edge type compatibility is checked continuously. Mismatches render inline, not modal.
- Three views of the same data: Recipe view (linear), Graph view (DAG), Timeline view (Gantt).
- The Canvas operates on the same TOML format as the CLI. Round-trip: edit in Canvas, save,
  load in CLI, modify, load back in Canvas -- zero data loss.

### 3.4 Stigmergy Minimap

RTS-style coordination visualization. Shows the agent population as a spatial field with
fog-of-war for unknown regions and group-selection for batch operations.

**Projections consumed:**

| Projection | StateHub Core Projection | Shape |
|---|---|---|
| Agent Positions | `agent_vitality` | `Vec<AgentPosition>` -- id, spatial embedding, status, vitality |
| c-factor Scores | `c_factor` | `CFactorSummary` -- turn-taking entropy, diversity |
| Cluster Membership | `cohort_health` | Error rate, regime distribution |
| Knowledge Landscape | `knowledge_health` | Tier distribution, cold entries -- fog density correlates with knowledge gaps |

**Minimap projection type (shipped):**

```rust
pub struct MinimapProjection {
    pub agents: Vec<AgentPosition>,
    /// Coordinate source; currently a stable layout, not an HDC embedding.
    pub position_source: String,  // "deterministic_layout"
    pub c_factor: CFactorSummary,
    pub cohort_health: CohortHealthProjection,
    pub knowledge_health: KnowledgeHealthProjection,
}

pub struct AgentPosition {
    pub id: String,
    pub x: f64,  // deterministic: index % 8
    pub y: f64,  // deterministic: index / 8
    pub status: String,
    pub vitality: f64,
    pub profile: String,
    pub current_task: Option<String>,
}

pub struct CFactorSummary {
    pub overall: f64,
    pub turn_taking_entropy: f64,
    pub peer_prediction_accuracy: f64,
    pub citation_reciprocity: f64,
    pub hdc_diversity: f64,
}
```

**Invariants:**
- The minimap updates in real time (Bus subscription, not polling).
- Agent vitality is visible (color saturation maps to vitality level).
- Fog-of-war covers regions where no agent has explored. Exploration lifts fog.
- Group selection enables batch operations.

### 3.5 Autonomy Slider

Progressive trust control. The Slider exposes five autonomy levels (0-4) with per-capability
granularity. Level 5 (structural evolution) is defined in [12-SAFETY](12-SAFETY.md) and is
not part of the Slider -- structural evolution requires a separate L4 approval flow and
cannot be granted via the Slider UI.

**Projections consumed:**

| Projection | StateHub Core Projection | Shape |
|---|---|---|
| Current Autonomy Levels | No live config source yet; endpoint reports `config_source = "unavailable"` | `AutonomyConfig` when a store is added |
| Agent Vitality | `agent_vitality` | Vitality + phase for context |
| Collective Intelligence | `c_factor` | c-factor trend informs trust calibration |

**Autonomy projection type (shipped):**

```rust
pub struct AutonomyProjection {
    pub configs: Vec<AutonomyConfig>,
    /// Explicit source state so an empty list is never mistaken for defaults.
    pub config_source: String,  // "unavailable" or "disk:.roko/state/autonomy.json"
    pub agent_vitality: AgentVitalityProjection,
    pub c_factor: CFactorSummary,
}
```

**Autonomy model (shipped in `roko-core/src/agent.rs`):**

```rust
pub enum AutonomyLevel {
    Observe    = 0,  // Read-only. No mutations.
    Suggest    = 1,  // Proposes actions as Signals. Does not execute.
    ActReview  = 2,  // Executes actions. Human reviews before persist.
    Guardrails = 3,  // Executes within declared parameter ranges.
    Full       = 4,  // Full execution within capability grant.
    // Level 5 (StructuralEvolution) is NOT representable here.
}

pub struct AutonomyConfig {
    pub agent_id: String,
    pub per_capability: HashMap<String, AutonomyLevel>,
    pub default_level: AutonomyLevel,
}
```

**Slider levels (0-4):**

| Level | Name | System behavior | Human involvement |
|---|---|---|---|
| 0 | **Observe** | Read-only. No mutations. | None needed |
| 1 | **Suggest** | Proposes actions as Signals. Does not execute. | Approves each action |
| 2 | **Act-with-review** | Executes actions. Human reviews before persist. | Post-action review |
| 3 | **Act-with-guardrails** | Executes within declared parameter ranges. | Review on bound violations |
| 4 | **Full autonomy** | Full execution within capability grant. Escalates novel situations. | Review on escalation only |

Level 5 (Structural Evolution) operates outside the Slider. An agent at L4 may propose
structural changes (Graph modifications, Cell additions, config evolution), but these
proposals are routed through the L4 approval flow: the proposal appears as a Review-urgency
InboxItem (SS3.2), the human approves or rejects, and only then does the structural change
take effect. The Slider never shows or allows setting Level 5.

**Invariants:**
- Autonomy can only be increased by the user, never by the system.
- Reducing autonomy takes effect immediately. In-flight operations complete at old level;
  new operations use new level.
- The Slider range is 0-4. Level 5 requires the separate L4 approval flow.
- Recent safety violations are visible next to the slider to inform trust calibration.

---

## 4. SurfaceEvent Command Model

Surfaces emit `SurfaceEvent` commands to change system state. These are kept separate from
engine-emitted `RuntimeEvent` values so a transport can authorize commands before translating
them into effects.

```rust
// crates/roko-core/src/runtime_event.rs
pub enum SurfaceEvent {
    TaskAssign { graph: String, inputs: Value, budget: Option<f64>, deadline: Option<DateTime<Utc>> },
    SlotFill { agent_id: String, slot_index: usize, cell_ref: String },
    MacroAdjust { run_id: String, macro_name: String, new_value: Value },
    FlowCancel { run_id: String },
    FlowPause { run_id: String },
    FlowResume { run_id: String },
    HumanRespond { run_id: String, cell_id: String, response: Value },
}
```

**HTTP ingress**: `POST /api/surface-events` accepts a `SurfaceEvent` JSON body. The route
handler in `crates/roko-serve/src/routes/run.rs` pattern-matches the event kind and logs
the appropriate effect. `FlowCancel` and `FlowPause` resolve by `run_id`; `HumanRespond`
resolves by `run_id` and `cell_id`.

**TUI emission**: The ratatui application broadcasts `SurfaceEvent` values through a
`tokio::sync::broadcast` channel (`crates/roko-cli/src/tui/app/mod.rs`). TUI actions
(approve, reject, pause, cancel) call `emit_surface_event()`, and any runner or subscriber
can receive them via `subscribe_surface_events()`.

---

## 5. Legacy-Tab Mapping

The TUI (`roko dashboard`) retains its existing eleven tabs (F1-F10 plus `-` for Providers).
E37 adds a separate compatibility mapping to seven v2 navigation identities without
renaming or re-keying those tabs.

### 5.1 V2Surface Enum

```rust
// crates/roko-cli/src/tui/tabs.rs
pub enum V2Surface {
    Workbench,
    Canvas,
    Flows,
    Inbox,
    Knowledge,
    System,
    Agents,
}
```

### 5.2 Tab-to-Surface Mapping

| Existing TUI Tab | v2 Compatibility Identities | Notes |
|---|---|---|
| **F1 Dashboard** | Workbench, Inbox | Inbox is a badge/attention contribution; full named rendering remains open |
| **F2 Plans** | Canvas, Flows | Compatibility shell for graph authoring and flow state |
| **F3 Agents** | Agents | Compatibility shell for the Stigmergy Minimap |
| **F4 Git** | *(none)* | Existing rendering-target-specific view |
| **F5 Logs** | *(none)* | Existing rendering-target-specific view |
| **F6 Config** | System | Compatibility shell for system/autonomy controls |
| **F7 Inspect** | Knowledge | Existing knowledge inspector |
| **F8 Marketplace** | *(none)* | Existing rendering-target-specific view |
| **F9 Atelier** | *(none)* | Existing rendering-target-specific view |
| **F10 Learning** | *(none)* | Existing rendering-target-specific view |
| **- Providers** | *(none)* | Existing rendering-target-specific view |

The mapping is implemented in `Tab::v2_surfaces()` which returns a `Vec<V2Surface>` per tab.
Five tabs carry no v2 surface identity and remain purely rendering-target-specific views.

### 5.3 Target Surface Layout (not yet rendered)

The intended named-surface TUI replaces the current tab assignments:

| Target Tab | Target Surfaces | Current State |
|---|---|---|
| **F1 Workbench** | Workbench + Agent Inbox badges | Current F1 Dashboard carries identities but not the full renderer |
| **F2 Canvas** | Generative Canvas (graph library, state-graph viewer) | Current F2 Plans carries identities only |
| **F3 Flows** | Flow inspector | Current F3 is Agents; does not implement this layout |
| **F4 Inbox** | Full Agent Inbox surface | Current F4 is Git; no quick-action Inbox renderer wired |
| **F5 Knowledge** | Knowledge browser | Current knowledge identity is F7 Inspect |
| **F6 System** | Space + Daemon + Autonomy Slider | Current F6 Config has no live autonomy store |
| **F7 Agents** | Fleet overview + compact Stigmergy Minimap | Current Agents is F3 |

---

## 6. Projection Invalidation and Streaming

Each projection carries an `InvalidationPolicy` that declares its maximum cache age,
whether it supports incremental (delta) updates, and which event types trigger
eager invalidation.

### 6.1 Surface Projection Policies

| Projection | `max_age_secs` | `incremental` | Invalidation Triggers |
|---|---|---|---|
| `workbench` | 5 | false | `plan_started`, `task_started`, `task_completed`, `agent_spawned`, `agent_completed`, `gate_result`, `projection_updated` |
| `inbox` | 5 | false | `inbox_item_received`, `inbox_approve`, `inbox_reject`, `inbox_defer`, `inbox_dismiss`, `projection_updated` |
| `canvas` | 5 | false | `plan_started`, `plan_completed`, `task_started`, `task_completed`, `gate_result`, `projection_updated` |
| `minimap` | 5 | false | `agent_spawned`, `agent_completed`, `projection_updated` |
| `autonomy` | 5 | false | `agent_spawned`, `agent_completed`, `projection_updated` |

### 6.2 SSE Stream Protocol

The `GET /api/projections/{name}/stream` endpoint provides an SSE stream with cursor-based
replay and lag recovery:

1. **Initial `state` event**: full projection snapshot with the current cursor.
2. **`delta` events**: each matching `DashboardEvent` is emitted as a delta frame with
   `channel`, `cursor`, and `delta` fields.
3. **Lag recovery**: if the subscriber falls behind the ring buffer, the stream emits a
   replacement `state` event with a fresh snapshot and re-subscribes from the new cursor.

The `projection_accepts_event()` function filters which DashboardEvent variants reach each
surface stream. For example, the `workbench` stream accepts `PlanStarted`, `TaskStarted`,
`TaskCompleted`, `AgentSpawned`, `AgentCompleted`, `EfficiencyEvent`, and `GateResult`
events, plus any `ProjectionUpdated` event targeting `active_tasks`, `agent_vitality`,
`gate_pipeline`, or `cost_meter`.

### 6.3 StateHub History

`GET /api/statehub/{projection_id}/history` returns bounded retained versions of any
materialized Lens projection, supporting:

- **Time range**: `from` and `to` RFC 3339 timestamps
- **Version range**: `from_version` and `to_version`
- **Resolution coalescing**: `resolution` parameter (e.g., `5s`, `1m`, `1h`) buckets
  versions and keeps the newest per bucket
- **Limit**: default 250, maximum 10,000

The response includes `retained` (total versions in store), `matched` (after filters),
`coalesced` (after resolution), `capacity`, and `retention_seconds`.

---

## 7. Cross-Surface Linking

Surfaces link to each other. The CLI prints clickable URLs (OSC 8 hyperlinks) and
`roko://` references:

```
$ roko run doc-ingest ...
Run id: wf_01HGZK7B...
Dashboard: http://localhost:6677/runs/wf_01HGZK7B...
TUI: roko tui --run wf_01HGZK7B...
```

All rendering targets share the same run-id and entity-id namespace. Dashboard URLs deep-link
to specific Flows, Graphs, and Triggers.

---

## 8. Third-Party Surfaces

Third parties build new surfaces by consuming StateHub projections and emitting surface
events. The five named surfaces define the contracts; implementations are open.

Examples:
- **Mobile app**: renders Workbench + Agent Inbox for on-the-go task delegation.
- **Slack bot**: renders Agent Inbox into Slack messages with reaction-based approve/reject.
- **VSCode extension**: renders Generative Canvas as a panel + Workbench as a sidebar.
- **CLI dashboard (tmux)**: renders all five surfaces in tmux panes with curses widgets.
- **Custom monitoring**: renders only CostLens and BudgetLens projections for finance teams.

---

## 9. Rendering Targets

Four rendering targets implement the five surfaces.

### 9.1 CLI Surface

The `roko` CLI exposes surfaces through structured subcommands. Every meaningful operation
is a Graph run or a registry operation.

**Workbench commands**: `roko run <graph> [args]`, `roko run cancel <run-id>`,
`roko run respond <run-id> [args]`, `roko run resume <run-id>`, `roko run list [--status]`.

**Inbox commands** (contract defined, not yet live): `roko inbox list [--urgency <level>]`,
`roko inbox show <id>`, `roko inbox approve <id>`, `roko inbox reject <id> [--reason <text>]`.

**Autonomy commands** (contract defined, not yet live): `roko autonomy show [--agent <name>]`,
`roko autonomy set <agent> <capability> <level>`, `roko autonomy set <agent> --all <level>`.

### 9.2 TUI Surface

The ratatui-based TUI (`roko dashboard`) with eleven tabs. The TUI bridge
(`crates/roko-cli/src/runner/tui_bridge.rs`) translates `DashboardEvent` values into
view state consumed by tab renderers. File watchers (`tui/fs_watch.rs`) trigger re-render
on `.roko/` changes without polling.

### 9.3 Dashboard (Web)

The HTTP control plane (`roko serve` on :6677) hosts all five surface projections as REST
endpoints. Real-time plumbing uses SSE for projection streams, WebSocket for run-scoped
events, and REST for on-demand reads.

### 9.4 Visual Editor (Generative Canvas)

The dashboard's drag-and-drop authoring environment for Graphs. Three-column layout:
Palette (left), Canvas (center), Inspector (right). Three views: Recipe (linear),
Graph (DAG), Timeline (Gantt). Design lineage: Ableton Live, Bitwig Grid, n8n.

---

## 10. Verification

### Verification checklist

| # | Assertion | Evidence |
|---|---|---|
| V1 | Five typed projection structs exist | `WorkbenchProjection`, `InboxProjection`, `CanvasProjection`, `MinimapProjection`, `AutonomyProjection` in `crates/roko-serve/src/projection_contract.rs` |
| V2 | Five dedicated HTTP routes serve surface data | `GET /api/projections/{workbench,inbox,canvas,minimap,autonomy}` in `crates/roko-serve/src/routes/projections.rs` |
| V3 | `ProjectionEnvelope<T>` wraps every surface response | Generic envelope with name, version, cursor, computed_at, recovered flag |
| V4 | InboxCategory has 8 variants with routing policy | `InboxCategory` enum + `inbox_routing()` in `crates/roko-core/src/dashboard_snapshot.rs` |
| V5 | UrgencyLevel has 3 variants (Notify, Question, Review) | `UrgencyLevel` enum in `crates/roko-core/src/dashboard_snapshot.rs` |
| V6 | SurfaceEvent has 7 variants | `SurfaceEvent` enum in `crates/roko-core/src/runtime_event.rs` |
| V7 | SurfaceEvent HTTP ingress exists | `POST /api/surface-events` in `crates/roko-serve/src/routes/run.rs` |
| V8 | TUI broadcasts SurfaceEvent values | `emit_surface_event()` + `subscribe_surface_events()` in `crates/roko-cli/src/tui/app/mod.rs` |
| V9 | Legacy tab mapping implemented | `V2Surface` enum + `Tab::v2_surfaces()` in `crates/roko-cli/src/tui/tabs.rs` |
| V10 | AutonomyLevel has 5 levels (0-4), Level 5 not representable | `AutonomyLevel` enum with `#[repr(u8)]` in `crates/roko-core/src/agent.rs` |
| V11 | AutonomyConfig supports per-capability granularity | `per_capability: HashMap<String, AutonomyLevel>` field |
| V12 | Projection catalog with invalidation policies | `projection_policies()` returns 30 entries in `crates/roko-serve/src/projection_contract.rs` |
| V13 | SSE streaming with cursor replay and lag recovery | `stream_projection()` in `crates/roko-serve/src/routes/projections.rs` |
| V14 | StateHub history with time/version/resolution queries | `GET /api/statehub/{projection_id}/history` with RFC 3339 range, resolution coalescing |
| V15 | Surface inventory documents wiring status | `full_inventory()` in `crates/roko-cli/src/surface_inventory.rs` |

### Product residuals

| Residual | Status | Notes |
|---|---|---|
| Full named-surface TUI rendering | Open | Legacy tabs carry compatibility identities but do not render the full named-surface layout |
| SurfaceEvent as command-ingress API | Open | HTTP endpoint logs events but does not execute them as system commands |
| Production Inbox publisher | Open | No live producer populates InboxItems; the projection reads from snapshot state |
| Live AutonomyConfig store | Open | Endpoint reports `config_source = "unavailable"` unless `autonomy.json` exists on disk |
| Workbench human input source | Open | `HumanInputRequest` struct exists but no live source populates it |
| Canvas GraphRegistry source | Open | `graph_source` is `"dashboard_plan_ids"`, not a proper GraphRegistry |
| Minimap HDC coordinates | Open | `position_source` is `"deterministic_layout"` (index-based grid), not HDC embedding |
| OpenAPI response schemas | Open | Response bodies are generic JSON schemas, not typed per-surface OpenAPI specs |
| Inbox receipt timestamps | Open | Unresolved items recompute timestamps from `received_at_ms` during JSONL replay |

---

## 11. References

| What | Where |
|---|---|
| Projection contract types | `crates/roko-serve/src/projection_contract.rs` |
| Surface HTTP routes | `crates/roko-serve/src/routes/projections.rs` |
| Dashboard snapshot model | `crates/roko-core/src/dashboard_snapshot.rs` |
| StateHub telemetry projections | `crates/roko-core/src/telemetry_projections.rs` |
| SurfaceEvent enum | `crates/roko-core/src/runtime_event.rs` |
| Autonomy model | `crates/roko-core/src/agent.rs` |
| TUI tab/surface mapping | `crates/roko-cli/src/tui/tabs.rs` |
| Surface inventory | `crates/roko-cli/src/surface_inventory.rs` |
| TUI SurfaceEvent broadcast | `crates/roko-cli/src/tui/app/mod.rs` |
| SurfaceEvent HTTP handler | `crates/roko-serve/src/routes/run.rs` |
| v2 spec (predecessor) | `docs/v2/20-SURFACES.md` |
| Depth: Projection contracts | `docs/v3/depth/22-surfaces/projection-contract.md` |
| Depth: Surface events | `docs/v3/depth/22-surfaces/surface-events.md` |
| Depth: Legacy tab mapping | `docs/v3/depth/22-surfaces/legacy-tab-mapping.md` |
| Depth: OpenAPI surface routes | `docs/v3/depth/22-surfaces/openapi-surface-routes.md` |
