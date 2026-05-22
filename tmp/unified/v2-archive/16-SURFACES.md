# 16 — Surfaces

> Five named surfaces -- Workbench, Agent Inbox, Generative Canvas, Stigmergy Minimap, Autonomy Slider -- define protocol-level data contracts between system and user. Surfaces are projections from the StateHub plus interaction contracts. Third parties build new surfaces consuming the same projections.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse, Bus), [02-CELL](02-CELL.md) (Cell protocol), [03-GRAPH](03-GRAPH.md) (Graph composition), [04-SPECIALIZATIONS](04-SPECIALIZATIONS.md) (Rack, Slot, Macro, Agent), [07-AGENT-RUNTIME](07-AGENT-RUNTIME.md) (vitality, CorticalState), [09-TELEMETRY](09-TELEMETRY.md) (StateHub, Lenses, c-factor), [14-CONFIG-AND-AUTHORING](14-CONFIG-AND-AUTHORING.md) (TOML schemas, domain profiles), [17-SECURITY-MODEL](17-SECURITY-MODEL.md) (autonomy levels, CaMeL)

---

## 1. Design Principles

A **surface** is a projection from the StateHub ([doc-09](09-TELEMETRY.md)) plus an interaction contract. The projection defines what data the surface consumes. The interaction contract defines what events the surface emits. Third parties can build entirely new surfaces by consuming the same projections and emitting the same events.

Five named surfaces. Four rendering targets (CLI, TUI, Dashboard, Visual Editor). Every surface can be rendered by any target -- but each target has natural affinities.

| Surface | What it is | Primary target | Secondary targets |
|---|---|---|---|
| **Workbench** | Structured task delegation (Linear/Notion pattern, not blank chat) | Dashboard | TUI, CLI |
| **Agent Inbox** | Ambient notification (calm technology) | Dashboard, TUI | CLI |
| **Generative Canvas** | Visual Graph editor (nodes-as-cards, typed cables, drag-and-drop) | Dashboard (Visual Editor) | TUI (state-graph view) |
| **Stigmergy Minimap** | RTS-style coordination (fog-of-war, group selection) | Dashboard | TUI |
| **Autonomy Slider** | Progressive trust (5 levels, per-capability granularity) | Dashboard | TUI, CLI |

Every operation that one rendering target supports, every other target supports -- possibly through different UX. Running a Graph, filling a Slot, promoting a Macro, inspecting a Flow, managing Triggers -- all targets, same outcome. The contract is the projections and events. The rendering is unconstrained.

---

## 2. Surface Contracts

Each surface is defined by three things:
1. **Projections consumed** -- typed data from StateHub that the surface reads.
2. **Events emitted** -- typed actions the surface sends back to the system.
3. **Invariants** -- behavioral contracts that all implementations must satisfy.

### 2.1 Workbench

The primary interaction surface. Delegates structured work to agents -- not a blank chat box, but a task-oriented surface modeled on Linear/Notion.

**Projections consumed:**

| Projection | Source | Shape |
|---|---|---|
| Active Flows | ExecutionEngine via StateHub | `Vec<FlowSummary>` -- id, graph name, progress %, cost, duration, status |
| Agent Slots | AgentRuntime via StateHub | `Vec<SlotState>` -- agent id, slot index, occupied/free, current task, vitality |
| Graph Topology | GraphRegistry via StateHub | `GraphSummary` -- nodes, edges, Macros, Slots, estimated cost |
| Pending Human Input | ExecutionEngine via StateHub | `Vec<HumanInputRequest>` -- run id, block id, prompt, urgency, deadline |
| Recent Completions | Store | `Vec<FlowResult>` -- last N completed Flows with verdicts |

**Events emitted:**

| Event | Payload | Effect |
|---|---|---|
| `TaskAssign` | `{ graph, inputs, macros, slots, budget, deadline }` | Starts a new Flow |
| `SlotFill` | `{ agent_id, slot_index, block_ref }` | Fills an Agent's Slot with a Cell |
| `MacroAdjust` | `{ run_id, macro_name, new_value }` | Adjusts a Macro on a running Flow |
| `FlowCancel` | `{ run_id }` | Cancels an active Flow |
| `FlowPause` | `{ run_id }` | Pauses an active Flow |
| `FlowResume` | `{ run_id }` | Resumes a paused Flow |
| `HumanRespond` | `{ run_id, block_id, response }` | Answers a human-input prompt |

```rust
pub struct FlowSummary {
    pub run_id: RunId,
    pub graph_name: String,
    pub progress_pct: f64,
    pub cost_usd: f64,
    pub elapsed: Duration,
    pub status: FlowStatus,
    pub active_nodes: Vec<NodeId>,
    pub pending_human: Option<HumanInputRequest>,
}

pub enum FlowStatus {
    Running,
    Paused,
    WaitingHuman,
    Completed { verdict: Verdict },
    Failed { error: String },
    Cancelled,
}
```

**Invariants:**
- Active Flows are always visible. A Workbench that hides running work is broken.
- Pending human input is surfaced with urgency. The user must never miss a decision request.
- Slot filling supports type-checked drag-and-drop (or equivalent selection in CLI/TUI).
- Cost and duration are live, not polled. Updates arrive via Bus subscription.

### 2.2 Agent Inbox

Ambient notification surface. Calm technology -- peripheral attention until something needs focus. Modeled on notification center, not chat. Three urgency levels at three priority bands.

**Projections consumed:**

| Projection | Source | Shape |
|---|---|---|
| Attention Pulses | Bus (filtered by `tagged_for_human: true`) | `Vec<InboxItem>` |

**Urgency levels:**

| Level | Name | Behavior | Example |
|---|---|---|---|
| 1 | **Notify** | Badge count. No interruption. | "Agent completed code review." |
| 2 | **Question** | Gentle chime. Requires answer within deadline. | "Agent found 3 security issues. Deploy anyway?" |
| 3 | **Review** | Persistent banner. Cells progress until resolved. | "Structural change proposed: new Graph. Approve?" |

```rust
pub struct InboxItem {
    pub id: PulseId,
    pub urgency: UrgencyLevel,
    pub category: InboxCategory,
    pub summary: String,
    pub detail: Value,
    pub source_agent: Option<AgentId>,
    pub timestamp: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,   // for Level 2 Questions
    pub blocking_run: Option<RunId>,       // for Level 3 Reviews
}

pub enum UrgencyLevel {
    Notify,     // badge only
    Question,   // chime + deadline
    Review,     // persistent, blocks progress
}
```

**Events emitted:**

| Event | Payload | Effect |
|---|---|---|
| `Approve` | `{ pulse_id }` | Approves a Question or Review item |
| `Reject` | `{ pulse_id, reason }` | Rejects with reason |
| `Defer` | `{ pulse_id, defer_until }` | Defers decision to a later time |
| `Dismiss` | `{ pulse_id }` | Dismisses a Notify item |

**Invariants:**
- Level 3 (Review) items block relevant Flows until resolved. The surface must make them unmissable.
- Level 2 (Question) items have deadlines. The surface must show countdown.
- Level 1 (Notify) items do not interrupt focus. Badge count only.
- Items expire naturally via demurrage ([doc-01](01-SIGNAL.md)). Old unresolved items fade.

### 2.3 Generative Canvas

Visual Graph editor. Nodes as cards, typed cables, drag-and-drop composition. The authoring surface for Graphs, Racks, Triggers, and Profiles.

**Design lineage**: Ableton Live (session view, macro knobs), Bitwig Grid (modular patching, typed cables), n8n (node-based automation, visual data flow).

**Projections consumed:**

| Projection | Source | Shape |
|---|---|---|
| Graph TOML | GraphRegistry | `GraphDefinition` -- full TOML-parsed Graph |
| Cell Catalog | CellRegistry via StateHub | `Vec<CellManifest>` -- all available Cells with I/O schemas |
| TypeSchema Registry | TypeRegistry | `Vec<TypeSchema>` -- all known types for edge compatibility |
| Live Flow State | ExecutionEngine (optional) | `FlowState` -- node statuses, edge traversals, for live overlay |

**Events emitted:**

| Event | Payload | Effect |
|---|---|---|
| `NodeAdd` | `{ block_ref, position }` | Adds a Cell node to the Graph |
| `NodeRemove` | `{ node_id }` | Removes a node |
| `EdgeCreate` | `{ from_node, from_port, to_node, to_port }` | Wires a typed edge |
| `EdgeRemove` | `{ edge_id }` | Removes an edge |
| `MacroPromote` | `{ node_id, param_name, macro_def }` | Promotes a Cell parameter to a Rack Macro |
| `SlotDeclare` | `{ slot_name, type_constraint, default }` | Declares a new Slot |
| `GraphSave` | `{ graph_toml }` | Persists the Graph |
| `GraphPublish` | `{ graph_name, marketplace_metadata }` | Publishes to marketplace ([doc-15](15-MARKETPLACE-AND-SHARING.md)) |

**Invariants:**
- Edge type compatibility is checked continuously. Mismatches render inline (red cable), not modal error.
- Every mouse interaction has a keyboard equivalent.
- Three views of the same data: Recipe view (linear), Graph view (DAG), Timeline view (Gantt).
- The Canvas operates on the same TOML format as the CLI. Round-trip: edit in Canvas, save, load in CLI, modify, load back in Canvas -- zero data loss.

### 2.4 Stigmergy Minimap

RTS-style coordination visualization. Shows the agent population as a spatial field with fog-of-war for unknown regions and group-selection for batch operations.

**Projections consumed:**

| Projection | Source | Shape |
|---|---|---|
| Pheromone Field | Bus (pheromone Pulses) | `PheromoneGrid` -- 2D field of signal intensities |
| Agent Positions | AgentRuntime via StateHub | `Vec<AgentPosition>` -- id, spatial embedding, status, vitality |
| c-factor Scores | CollectiveIntelligenceLens via StateHub | `CFactorSummary` -- turn-taking entropy, peer prediction accuracy, diversity |
| Cluster Membership | ExecutionEngine | `Vec<ClusterState>` -- which agents form which coalitions |

```rust
pub struct AgentPosition {
    pub id: AgentId,
    pub x: f64,                        // spatial embedding (2D projection of HDC space)
    pub y: f64,
    pub status: AgentStatus,
    pub vitality: f64,                 // 0.0..=1.0 (doc-07)
    pub profile: String,
    pub current_task: Option<String>,
}

pub struct CFactorSummary {
    pub overall: f64,                  // composite c-factor
    pub turn_taking_entropy: f64,      // how evenly distributed agent turns are
    pub peer_prediction_accuracy: f64, // how well agents predict each other
    pub citation_reciprocity: f64,     // knowledge attribution balance
    pub hdc_diversity: f64,            // spread of episode fingerprints
}
```

**Events emitted:**

| Event | Payload | Effect |
|---|---|---|
| `SpawnAgent` | `{ profile, position_hint }` | Spawns a new Agent |
| `CullAgent` | `{ agent_id }` | Gracefully terminates an Agent |
| `GroupSelect` | `{ agent_ids }` | Selects a group for batch operations |
| `TopologyAdjust` | `{ cluster_id, new_topology }` | Reconfigures cluster shape |
| `PheromoneDeposit` | `{ position, kind, intensity }` | Manual pheromone signal (human-guided coordination) |

**Invariants:**
- The minimap updates in real time (Bus subscription, not polling).
- Agent vitality is visible (color saturation maps to vitality level, [doc-07](07-AGENT-RUNTIME.md)).
- Fog-of-war covers regions where no agent has explored. Exploration lifts fog.
- Group selection enables batch operations (send all selected agents the same task, adjust all budgets).

### 2.5 Autonomy Slider

Progressive trust control. Five autonomy levels ([doc-17](17-SECURITY-MODEL.md)) with per-capability granularity. The user adjusts how much trust the system has, and the system adjusts its behavior accordingly.

**Projections consumed:**

| Projection | Source | Shape |
|---|---|---|
| Agent Capabilities | AgentRuntime via StateHub | `Vec<CapabilityDeclaration>` |
| Current Autonomy Levels | Space config via StateHub | `AutonomyConfig` |
| CaMeL Tags | Extension system via StateHub | `Vec<CamelTag>` |
| Safety Violations | SecurityEventStream | `Vec<SafetyViolation>` |

**Five levels:**

| Level | Name | System behavior | Human involvement |
|---|---|---|---|
| 0 | **Observe** | Read-only. No mutations. | None needed |
| 1 | **Suggest** | Proposes actions as Signals. Does not execute. | Approves each action |
| 2 | **Act-with-review** | Executes actions. Human reviews before persist. | Post-action review |
| 3 | **Act-with-guardrails** | Executes within declared parameter ranges. | Review on bound violations |
| 4 | **Full autonomy** | Full execution within capability grant. Escalates novel situations. | Review on escalation only |

**Per-capability granularity:** the slider is not a single global knob. Each capability (FsWrite, Net, Shell, Llm, Chain) has its own level:

```rust
pub struct AutonomyConfig {
    pub agent_id: AgentId,
    pub per_capability: HashMap<Capability, AutonomyLevel>,
    pub default_level: AutonomyLevel,
}
```

A user might set `FsRead = 4` (full autonomy) and `Chain.write = 1` (suggest only).

**Events emitted:**

| Event | Payload | Effect |
|---|---|---|
| `AutonomyLevelChange` | `{ agent_id, capability, new_level }` | Changes autonomy for a capability |
| `CapabilityGrant` | `{ agent_id, capability, constraints }` | Grants a new capability |
| `CapabilityRevoke` | `{ agent_id, capability }` | Revokes a capability |
| `BulkAutonomySet` | `{ agent_id, level }` | Sets all capabilities to same level |

**Invariants:**
- Autonomy can only be increased by the user, never by the system.
- Reducing autonomy takes effect immediately. In-flight operations complete at old level; new operations use new level.
- Level 5 (structural changes to Graphs, Cells, agent config) always requires human approval regardless of Slider position. Enforced at [doc-17](17-SECURITY-MODEL.md), not the surface layer.
- Recent safety violations are visible next to the slider to inform trust calibration.

---

## 3. Rendering Targets

Four rendering targets implement the five surfaces.

### 3.1 CLI Surface

The `roko` CLI is Graph-centric: every meaningful operation is a Graph run or a registry operation.

#### Workbench commands

```
roko run <graph> [args]              # run a Graph (TaskAssign)
roko run cancel <run-id>              # cancel (FlowCancel)
roko run respond <run-id> [args]      # answer human-input (HumanRespond)
roko run resume <run-id>              # resume snapshotted Flow (FlowResume)
roko run list [--status]              # active and recent Flows
roko run show <run-id>                # detailed Flow inspection
roko run logs <run-id> [--follow]     # stream Flow logs
roko run replay <run-id>              # rerun with same inputs
```

#### Canvas commands

```
roko graph list [--installed | --catalog]
roko graph show <name>
roko graph validate <name>
roko graph new <name>
roko graph edit <name>
roko graph fork <source> <new>
```

#### Inbox commands

```
roko inbox list [--urgency <level>]
roko inbox show <id>
roko inbox approve <id>
roko inbox reject <id> [--reason <text>]
roko inbox dismiss <id>
```

#### Autonomy commands

```
roko autonomy show [--agent <name>]
roko autonomy set <agent> <capability> <level>
roko autonomy set <agent> --all <level>
```

#### Verb sugar

Common Graph runs get top-level aliases for ergonomic one-liners. Every verb expands to `roko run <graph>`:

```
roko ingest <dir>           = roko run doc-ingest --input source_dir=<dir>
roko deploy [target]        = roko run deploy --macro target=<target>
roko research <topic>       = roko run research-sweep --input topic=<topic>
roko review [pr|diff]       = roko run code-review --input ...
roko audit                  = roko run security-audit
roko test [scope]           = roko run test-run --input target=<scope>
```

#### Exit codes

| Code | Meaning |
|---|---|
| 0 | Flow succeeded |
| 1 | Generic failure |
| 2 | Bad CLI usage |
| 10 | Graph validation failed |
| 11 | Capability denied |
| 12 | Budget exceeded |
| 13 | Deadline exceeded |
| 14 | Human input timeout |
| 15 | Cancelled |
| 16 | Space error |

### 3.2 TUI Surface

The TUI (`roko tui`) is a ratatui-based terminal application. Keyboard-driven, no server required. Six tabs plus a persistent transport strip.

#### Layout

```
 roko - Space: nunchi-dashboard - providers: ok - daemon: ok - cost today: $4.12

  [F1 Workbench] [F2 Canvas] [F3 Flows] [F4 Inbox] [F5 Knowledge] [F6 System]

  +------------------------------------------------------------------------+
  |                                                                        |
  |                          (active tab content)                          |
  |                                                                        |
  +------------------------------------------------------------------------+

  Transport:
  * doc-ingest   53%  $1.84/$10  4m 12s  [pause] [cancel] [detach]
  | deploy-rc   pending human input  [respond]

  [?] help  [/] palette  [g] go to  [w] space  [c] create  [q] quit
```

The bottom **Transport** strip is always visible: shows up to 3 active Flows with one-press controls (pause, cancel, detach, respond-to-human-input). This is the Workbench surface in its most compact form.

#### Tabs

**F1 Workbench** -- Task delegation and Flow overview. Shows Space health, active Flows, pending Triggers, recent completions, cost summary, pending Inbox items at Urgency 2-3. Auto-refreshes every 1s.

**F2 Canvas** -- Graph library and state-graph viewer. Two-pane: Graph list (left, 30%), detail + inline edit (right, 70%). Shows Macros, Slots, capabilities, estimated cost, last 5 runs. Press `r` to launch. Press `->` for Graph View (state-graph via ratatui-canvas).

**F3 Flows** -- Flow inspector. List of active and recent Flows (left), detail (right). Sub-views: Overview, Graph (live node colors), Artifacts, Episodes, Logs, Trace (timing waterfall).

**F4 Inbox** -- Agent Inbox surface. Items sorted by urgency then recency. Quick-action keys: `a` approve, `r` reject, `d` defer, `x` dismiss. Shows urgency level, source agent, summary, deadline countdown.

**F5 Knowledge** -- Knowledge browser. Entries with type, confidence, age, decay state. Resonance graph. Lineage walker. Dream cycle history.

**F6 System** -- Space + Daemon + Providers + Costs + Autonomy Slider. Space details, daemon status, provider health, cost breakdown. Per-agent autonomy levels with inline adjustment.

#### Universal keys

```
?            help overlay
/            command palette (fuzzy across Graphs, Flows, artifacts, pages)
g <letter>   go to: gw workbench, gc canvas, gf flows, gi inbox, gk knowledge, gs system
w            Space switcher (overlay)
c            create new (overlay: graph, trigger, rack, space)
.            quick run launcher (last-used Graph, edit args, run)
:            command line (typed commands)
q            quit
^c           cancel current focus action
F1-F6        jump to tab
Tab / S-Tab  cycle panes within active tab
```

#### State-graph view

ratatui-canvas-rendered graph with live node colors:
- **Jade** -- node completed
- **Cyan flashing** -- node in flight
- **Amber** -- node queued
- **Crimson** -- node failed
- **Violet** -- node awaiting human input
- **Dimmed** -- node pruned by failed conditional

#### File watcher integration

The existing `notify::RecommendedWatcher` (`tui/fs_watch.rs`) watches `.roko/runs/`, `episodes.jsonl`, and `artifacts/`. Triggers re-render on change -- reactive to daemon progress without polling.

### 3.3 Dashboard

The web dashboard (`roko serve` on :6677) is the primary visual surface. All five surfaces present.

#### Navigation

| Destination | Surfaces | Key pages |
|---|---|---|
| **Workbench** | Workbench + Agent Inbox | Task board, Event Stream, Inbox queue |
| **Agents** | Stigmergy Minimap + Autonomy Slider | Fleet minimap, Detail, Create, Groups, Autonomy |
| **Work** | Generative Canvas + Workbench | Library, Editor, Flows, Triggers, Marketplace |
| **Knowledge** | -- | Store, Resonance, Lineage, Dreams |
| **Arena** | -- | Browser, Leaderboard, Experiments |
| **System** | Autonomy Slider | Spaces, Providers, Costs, Deployments, Settings |

#### Real-time plumbing

- **WebSocket `/ws/events`** -- every Cell lifecycle event for the active Space
- **WebSocket `/ws/runs/<run-id>`** -- focused stream for one Flow
- **SSE `/sse/triggers`** -- Trigger fire and dispatch events
- **SSE `/sse/cost`** -- live cost ticks
- **SSE `/sse/inbox`** -- Inbox items for Agent Inbox surface
- **HTTP REST** -- `/api/v1/graphs`, `/runs`, `/triggers`, `/spaces`, `/artifacts`, `/episodes`

Every page is fully reactive: Flow list updates without refresh, Graph view animates state transitions, cost gauge ticks live.

### 3.4 Visual Editor (Generative Canvas)

The dashboard's drag-and-drop authoring environment. The full Generative Canvas surface.

#### Three-column layout

- **Palette** (left, ~240px): draggable Cells, control-flow primitives (Branches, Loops, FanOut, FanIn, HumanInput), Macros, Slots, Triggers, Snippets, Criteria
- **Canvas** (center): the state graph -- the playground
- **Inspector** (right, ~360px, collapsible): properties of selected node / edge / Macro / Slot

#### Node cards

Each node is a card with:
- Header strip with Cell name + version + status badge
- Input port row (top edge, one per typed input)
- Body: collapsed compact info, expandable for full param list
- Output port row (bottom edge, one per typed output)
- Footer with cost/time estimates and capability badges
- Quick actions on hover: pin, fork-this-step, replace-block, mute (skip), solo (run only this)

Node colors: **Sapphire** (Cell), **Violet** (Sub-Graph), **Rose** (HumanInput/Wait), **Amber** (Branch/FanOut/FanIn/Loop), **Slate** (Slot placeholder), **Glass** (pruned-by-condition).

#### Cable colors

Cable colors encode payload type: rose (doc), jade (code), sapphire (structured data), amber (evidence), violet (knowledge), white (generic). Cable thickness encodes batch size.

Cable behavior:
- Hover: last value preview in tooltip
- Click in-flight: stream viewer of flowing values
- Drag output-to-input: creates cable. Type match = instant. Mismatch = adapter prompt from registry.

#### Macro promotion

The signature gesture: right-click an internal Cell parameter -> "Promote to Macro" (Canvas event: `MacroPromote`). Inline modal for name, label, kind, range, default. The parameter value is replaced with `{{ macro.name }}` and a MacroDef is added to the Rack.

Macro controls render as: floats/money as rotary knob, integers as stepper, booleans as toggle, enums as segmented control, strings as text input, ModelRef as searchable dropdown, AgentRef as avatar dropdown.

#### Live preview

- **Last-value preview**: after any run, hovering a node shows its last output (markdown rendered, JSON pretty-printed, images thumbnailed, diffs highlighted)
- **Test-isolated**: right-click any node -> "Test in isolation". Runs only that Cell in a sandbox.
- **Run-up-to-here**: right-click any node -> "Run up to here". Engine runs partial Graph, halts.

#### Multiple views

Every Graph has three views, toggled by keyboard or button:
- **Recipe view** (linear, Apple-Shortcuts-like step list) -- beginner-friendly
- **Graph view** (DAG canvas) -- power authoring
- **Timeline view** (Gantt waterfall of last run) -- parallelism and bottleneck visualization

#### Sound (Web Audio)

Generated tones, off by default:
1. Cable plug-in -- perfect fifth, 60ms triangle
2. Cable mismatch -- minor second, 80ms
3. Validation success -- short major chord arpeggio
4. Validation failure -- low resonant tone, 200ms
5. Flow start -- octave, 120ms with reverb tail
6. Flow complete -- high sine ping, 60ms

Principle: juice the user's actions and the agent's completions, never ambient state.

#### Motion (spring physics, Framer Motion)

- Node drag: stiffness 500, damping 30
- Node insert: stiffness 200, damping 28, scale 0.9 -> 1.0
- Cable connect: 200ms ease, brief flash
- Validation error: shake with limited amplitude
- Macro promotion: source-param card morphs into Macro card via FLIP
- Recipe <-> Graph toggle: 400ms crossfade with positional FLIP
- Flow state changes: edges flash rose during traversal, nodes pulse cyan in flight
- All animations respect `prefers-reduced-motion`

#### Keyboard shortcuts

```
Cmd+K               command palette (filtered to authoring actions)
Cmd+/               search palette of Cells to add
Cmd+E               toggle Recipe / Graph view
Cmd+R               run the Graph
Cmd+Shift+R         test-isolated the selected node
Cmd+G               group selection into sub-Graph
Cmd+Shift+G         ungroup sub-Graph
Cmd+D               duplicate selected node(s)
Cmd+Z / Shift+Cmd+Z undo / redo
Cmd+S               save
Cmd+Shift+P         publish to marketplace (doc-15)
Cmd+M               promote selected param to Macro
Tab                 cycle selection through outgoing edges
Arrow keys          move selected node
Esc                 deselect / collapse modal
.                   focus palette search
[ / ]               collapse / expand inspector
```

---

## 4. Cross-Surface Linking

Surfaces link to each other. The CLI prints clickable URLs (OSC 8 hyperlinks) and `roko://` references:

```
$ roko run doc-ingest ...
Run id: wf_01HGZK7B...
Dashboard: http://localhost:6677/runs/wf_01HGZK7B...
TUI: roko tui --run wf_01HGZK7B...
```

`roko tui --run <id>` jumps directly to the Flow inspector. Dashboard URLs deep-link to specific Flows, Graphs, and Triggers. All rendering targets share the same run-id and entity-id namespace.

---

## 5. Third-Party Surfaces

Third parties build new surfaces by consuming StateHub projections and emitting surface events. The five named surfaces define the contracts; implementations are open.

Examples:
- **Mobile app**: renders Workbench + Agent Inbox for on-the-go task delegation and decision making.
- **Slack bot**: renders Agent Inbox into Slack messages with reaction-based approve/reject.
- **VSCode extension**: renders Generative Canvas as a panel + Workbench as a sidebar.
- **CLI dashboard (tmux)**: renders all five surfaces in tmux panes with curses widgets.
- **Custom monitoring**: renders only CostLens and BudgetLens projections for finance teams.

The contract is the projections and events. The rendering is unconstrained.

---

## 6. Theming

### TUI

ROSEDUST palette adapted for terminal: rose accent on active elements, jade for success, amber for warning, crimson for error, violet for knowledge, sapphire for active Cells. Monospace throughout. User-toggleable density (compact / comfortable / spacious) via `~/.roko/config.toml`.

### Dashboard

Glass morphism on panels (3 levels). Spring-physics motion (Framer Motion). Same ROSEDUST palette. Tabular nums. Stagger-children at 40ms on list mounts. Cell-specific visual elements: state graph node colors match the TUI scheme, edge animations (traversed = rose flash, pruned = ghost fade), Macro sliders as DAW-style controls.

---

## 7. Acceptance Criteria

| Criterion | Verification |
|---|---|
| **Surface contracts**: Workbench, Inbox, Canvas, Minimap, Slider each have typed projections and events | Schema validation test on each surface contract |
| **Third-party surface**: external consumer subscribes to StateHub projections and emits surface events | Integration test: mock surface consumes projections, emits events |
| **CLI**: `roko run <graph>` runs in foreground with live progress (Workbench) | Manual test on doc-ingest |
| **CLI**: `roko inbox list` shows pending items (Agent Inbox) | Inbox population test |
| **CLI**: `roko autonomy set` changes levels (Autonomy Slider) | Autonomy change test |
| **CLI**: `roko --json` produces valid JSON for all commands | Schema tests |
| **CLI**: Tab completion enumerates Graphs, Cells, Spaces, run-ids | Manual test in zsh + fish |
| **CLI**: Verb sugar maps correctly to Graph runs | Equivalence test |
| **TUI**: Launches via `roko tui`, renders F1 Workbench | Smoke test |
| **TUI**: F1 auto-refreshes every 1s without flicker | Visual / load test |
| **TUI**: F3 streams live Flow output within 100ms | Latency test |
| **TUI**: F4 Inbox shows items with urgency levels and quick-action keys | Inbox rendering test |
| **TUI**: State-graph view renders with live node colors | Visual snapshot |
| **TUI**: Space switcher changes context; tab content updates | Multi-Space test |
| **Dashboard**: Space switcher lists all Spaces, switches data context | Multi-Space test |
| **Dashboard**: Flow inspector receives WebSocket updates within 200ms | Latency test |
| **Dashboard**: State-graph node colors update live during Flow | Visual regression |
| **Dashboard**: Stigmergy Minimap renders agent positions and pheromone field | Minimap rendering test |
| **Dashboard**: Autonomy Slider per-capability controls update agent config | Slider round-trip test |
| **Dashboard**: Inbox badge shows count with red dot for Level 3 | Badge rendering test |
| **Visual Editor**: Drag Cell from palette -> node appears with correct ports | Manual + e2e |
| **Visual Editor**: Promote-to-Macro replaces param and adds MacroDef | Round-trip test |
| **Visual Editor**: Recipe <-> Graph toggle preserves all data | Round-trip test |
| **Visual Editor**: Test-isolated runs selected Cell within 30s | Latency test |
| **Visual Editor**: Validation errors render inline; bottom strip aggregates | Synthetic invalid Graph |
| **Visual Editor**: Sound effects play on cable connect / validate / run | Audio test |
| **Visual Editor**: Three views (Recipe, Graph, Timeline) all render same data | View equivalence test |
| **Wizards**: Graph wizard emits valid TOML | Round-trip test |
| **Cross-surface**: CLI prints dashboard URL; URL deep-links to correct Flow | Link test |
| **Autonomy Slider**: User increases trust, system never autonomously increases | Invariant test |
| **Autonomy Slider**: Reducing autonomy takes effect on next operation | Mid-flow level change test |
