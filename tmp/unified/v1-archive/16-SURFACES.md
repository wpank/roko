# 16 — Surfaces

> Five named surfaces — Workbench, Agent Inbox, Generative Canvas, Stigmergy Minimap, Autonomy Slider — define the data contracts between system and user. CLI, TUI, Dashboard, and Visual Editor are implementations of these surfaces.

**Source**: wf-08 (CLI Redesign), wf-09 (TUI Redesign), wf-10 (Dashboard Redesign), wf-11 (Visual Config Wizard), combined with unified vocabulary. Elevated to spec-level data contracts via StateHub projections ([doc-09](09-TELEMETRY.md)).

---

## 1. Design Principles

A **surface** is a projection from the StateHub ([doc-09](09-TELEMETRY.md)) plus an interaction contract. The projection defines what data the surface consumes. The interaction contract defines what events the surface emits. Third parties can build entirely new surfaces by consuming the same projections and emitting the same events.

Five named surfaces. Four rendering targets (CLI, TUI, Dashboard, Visual Editor). Every surface can be rendered by any target — but each target has natural affinities.

| Surface | What it is | Primary target | Secondary targets |
|---|---|---|---|
| **Workbench** | Structured task delegation | Dashboard | TUI, CLI |
| **Agent Inbox** | Ambient notification and decision | Dashboard, TUI | CLI |
| **Generative Canvas** | Visual Graph editor | Dashboard (Visual Editor) | TUI (state-graph view) |
| **Stigmergy Minimap** | Coordination visualization | Dashboard | TUI |
| **Autonomy Slider** | Progressive trust control | Dashboard | TUI, CLI |

Every operation that one rendering target supports, every other target supports — possibly through different UX. Running a Graph, filling a Slot, promoting a Macro, inspecting a Flow, managing Triggers — all targets, same outcome.

---

## 2. Surface Contracts

Each surface is defined by three things:
1. **Projections consumed** — typed data from StateHub that the surface reads.
2. **Events emitted** — typed actions the surface sends back to the system.
3. **Invariants** — behavioral contracts that all implementations must satisfy.

### 2.1 Workbench

The primary interaction surface. Delegates structured work to agents — not a blank chat box, but a task-oriented surface modeled on Linear/Notion.

**Projections consumed:**

| Projection | Source | Shape |
|---|---|---|
| Active Flows | ExecutionEngine via StateHub | `Vec<FlowSummary>` — id, graph name, progress %, cost, duration, status |
| Agent Slots | AgentRuntime via StateHub | `Vec<SlotState>` — agent id, slot index, occupied/free, current task, vitality |
| Graph Topology | GraphRegistry via StateHub | `GraphSummary` — nodes, edges, Macros, Slots, estimated cost |
| Pending Human Input | ExecutionEngine via StateHub | `Vec<HumanInputRequest>` — run id, block id, prompt, urgency, deadline |
| Recent Completions | Store | `Vec<FlowResult>` — last N completed Flows with verdicts |

**Events emitted:**

| Event | Payload | Effect |
|---|---|---|
| `TaskAssign` | `{ graph, inputs, macros, slots, budget, deadline }` | Starts a new Flow |
| `SlotFill` | `{ agent_id, slot_index, block_ref }` | Fills an Agent's Slot with a Block |
| `MacroAdjust` | `{ run_id, macro_name, new_value }` | Adjusts a Macro on a running Flow |
| `FlowCancel` | `{ run_id }` | Cancels an active Flow |
| `FlowPause` | `{ run_id }` | Pauses an active Flow |
| `FlowResume` | `{ run_id }` | Resumes a paused Flow |
| `HumanRespond` | `{ run_id, block_id, response }` | Answers a human-input prompt |

**Invariants:**
- Active Flows are always visible. A Workbench that hides running work is broken.
- Pending human input is surfaced with urgency. The user must never miss a decision request.
- Slot filling supports type-checked drag-and-drop (or equivalent selection in CLI/TUI).
- Cost and duration are live, not polled. Updates arrive via Bus subscription.

### 2.2 Agent Inbox

Ambient notification surface. Calm technology — peripheral attention until something needs focus. Modeled on notification center, not chat. Three urgency levels at three priority bands.

**Projections consumed:**

| Projection | Source | Shape |
|---|---|---|
| Attention Pulses | Bus (filtered by `tagged_for_human: true`) | `Vec<InboxItem>` — pulse id, urgency, category, summary, timestamp |

**Urgency levels:**

| Level | Name | Behavior | Example |
|---|---|---|---|
| 1 | **Notify** | Badge count. No interruption. | "Agent completed code review." |
| 2 | **Question** | Gentle chime. Requires answer within deadline. | "Agent found 3 security issues. Deploy anyway?" |
| 3 | **Review** | Persistent banner. Blocks progress until resolved. | "Structural change proposed: new Graph. Approve?" |

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
- Items expire naturally via demurrage. Old unresolved items fade.

### 2.3 Generative Canvas

Visual Graph editor. Nodes as cards, typed cables, drag-and-drop composition. The authoring surface for Graphs, Racks, Triggers, and Profiles.

**Projections consumed:**

| Projection | Source | Shape |
|---|---|---|
| Graph TOML | GraphRegistry | `GraphDefinition` — full TOML-parsed Graph |
| Block Catalog | BlockRegistry via StateHub | `Vec<BlockManifest>` — all available Blocks with I/O schemas |
| TypeSchema Registry | TypeRegistry | `Vec<TypeSchema>` — all known types for edge compatibility |
| Live Flow State | ExecutionEngine (optional) | `FlowState` — node statuses, edge traversals, for live overlay |

**Events emitted:**

| Event | Payload | Effect |
|---|---|---|
| `NodeAdd` | `{ block_ref, position }` | Adds a Block node to the Graph |
| `NodeRemove` | `{ node_id }` | Removes a node |
| `EdgeCreate` | `{ from_node, from_port, to_node, to_port }` | Wires a typed edge |
| `EdgeRemove` | `{ edge_id }` | Removes an edge |
| `MacroPromote` | `{ node_id, param_name, macro_def }` | Promotes a Block parameter to a Rack Macro |
| `SlotDeclare` | `{ slot_name, type_constraint, default }` | Declares a new Slot on the Rack |
| `GraphSave` | `{ graph_toml }` | Persists the Graph |
| `GraphPublish` | `{ graph_name, marketplace_metadata }` | Publishes to marketplace ([doc-15](15-MARKETPLACE-AND-SHARING.md)) |

**Invariants:**
- Edge type compatibility is checked continuously. Mismatches render inline (red cable), not modal error.
- Every mouse interaction has a keyboard equivalent.
- Three views of the same data: Recipe view (linear), Graph view (DAG), Timeline view (Gantt).
- The Canvas operates on the same TOML format as the CLI. Round-trip: edit in Canvas, save, load in CLI, modify, load back in Canvas — zero data loss.

### 2.4 Stigmergy Minimap

RTS-style coordination visualization. Shows the agent population as a spatial field with fog-of-war for unknown regions and group-selection for batch operations.

**Projections consumed:**

| Projection | Source | Shape |
|---|---|---|
| Pheromone Field | Bus (pheromone Pulses) | `PheromoneGrid` — 2D field of signal intensities |
| Agent Positions | AgentRuntime via StateHub | `Vec<AgentPosition>` — id, spatial embedding, status, vitality |
| c-factor Scores | CollectiveIntelligenceLens via StateHub | `CFactorSummary` — turn-taking entropy, peer prediction accuracy, diversity |
| Cluster Membership | ExecutionEngine | `Vec<ClusterState>` — which agents form which coalitions |

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
- Agent vitality is visible (color saturation maps to vitality level).
- Fog-of-war covers regions where no agent has explored. Exploration lifts fog.
- Group selection enables batch operations (send all selected agents the same task, adjust all budgets, etc.).

### 2.5 Autonomy Slider

Progressive trust control. Five autonomy levels (see [doc-17, section 6](17-SECURITY-MODEL.md)) with per-capability granularity. The user adjusts how much trust the system has, and the system adjusts its behavior accordingly.

**Projections consumed:**

| Projection | Source | Shape |
|---|---|---|
| Agent Capabilities | AgentRuntime via StateHub | `Vec<CapabilityDeclaration>` — what each agent can do |
| Current Autonomy Levels | Space config via StateHub | `AutonomyConfig` — per-agent and per-capability levels |
| CaMeL Tags | Extension system via StateHub | `Vec<CamelTag>` — capability provenance tags on data flows |
| Safety Violations | SecurityEventStream | `Vec<SafetyViolation>` — recent violations for trust calibration |

**Five levels:**

| Level | Name | System behavior | Human involvement |
|---|---|---|---|
| 0 | **Observe** | Read-only. No mutations. | None needed |
| 1 | **Suggest** | Proposes actions as Signals. Does not execute. | Human reviews and approves each action |
| 2 | **Act-with-review** | Executes actions. Human reviews results before they persist. | Post-action review |
| 3 | **Act-with-guardrails** | Executes and persists within declared parameter ranges. | Review on bound violations |
| 4 | **Full autonomy** | Full execution within capability grant. Escalates novel situations. | Review on escalation only |

**Per-capability granularity:** the slider is not a single global knob. Each capability (FsWrite, Net, Shell, Llm, Chain, etc.) has its own level. A user might set `FsRead = 4` (full autonomy) and `Chain.write = 1` (suggest only).

**Events emitted:**

| Event | Payload | Effect |
|---|---|---|
| `AutonomyLevelChange` | `{ agent_id, capability, new_level }` | Changes autonomy level for a specific capability |
| `CapabilityGrant` | `{ agent_id, capability, constraints }` | Grants a new capability |
| `CapabilityRevoke` | `{ agent_id, capability }` | Revokes a capability |
| `BulkAutonomySet` | `{ agent_id, level }` | Sets all capabilities to the same level |

**Invariants:**
- Autonomy can only be increased by the user, never by the system.
- Reducing autonomy takes effect immediately. In-flight operations at the old level complete but new operations use the new level.
- Level 5 (structural changes to Graphs, Blocks, agent config) always requires human approval regardless of Slider position. This is enforced at the [security model](17-SECURITY-MODEL.md) layer, not the surface layer.
- Recent safety violations are visible next to the slider to inform trust calibration.

---

## 3. Rendering Targets

Four rendering targets implement the five surfaces. Each target has natural affinities but can render any surface.

### 3.1 CLI

The `roko` CLI is Graph-centric: every meaningful operation is a Graph run or a registry operation.

#### Top-level command surface

```
roko                                  # Space summary (active Space, recent Flows, pending humans)
roko help [<command>]
roko version

# Workbench surface
roko run <graph> [args]              # run a Graph (TaskAssign)
roko run cancel <run-id>              # cancel an active Flow (FlowCancel)
roko run respond <run-id> [args]      # answer a human-input prompt (HumanRespond)
roko run resume <run-id>              # resume a snapshotted Flow (FlowResume)
roko run list [--status]              # active and recent Flows
roko run show <run-id>                # detailed Flow inspection
roko run logs <run-id> [--follow]     # stream Flow logs
roko run replay <run-id>              # rerun with the same inputs

# Generative Canvas surface (CLI equivalent)
roko graph list [--installed | --catalog]
roko graph show <name>
roko graph validate <name>
roko graph new <name>
roko graph edit <name>
roko graph fork <source> <new>
roko graph remove <name>
roko graph capabilities <name>

# Blocks
roko block list
roko block show <name>
roko block install <ref>
roko block remove <name>
roko block new <name>

# Racks
roko rack list
roko rack show <name>
roko rack new <name>

# Triggers
roko trigger list
roko trigger show <name>
roko trigger create <name> --kind <kind> --graph <name> [args]
roko trigger edit <name>
roko trigger enable <name>
roko trigger disable <name>
roko trigger remove <name>
roko trigger test <name> [--payload <json>]

# Spaces
roko space list
roko space show [<name>]
roko space new <path> [--template <name>]
roko space switch <name|path>

# Agent Inbox surface (CLI equivalent)
roko inbox list [--urgency <level>]
roko inbox show <id>
roko inbox approve <id>
roko inbox reject <id> [--reason <text>]
roko inbox dismiss <id>

# Autonomy Slider surface (CLI equivalent)
roko autonomy show [--agent <name>]
roko autonomy set <agent> <capability> <level>
roko autonomy set <agent> --all <level>

# Marketplace
roko market browse [--query <q>] [--tag <tag>]
roko market show <ref>
roko market install <ref>
roko market publish <local-name>
roko market fork <ref>

# Services
roko daemon start | stop | status | logs
roko serve [--port <p>]               # HTTP control plane on :6677
roko tui                              # ratatui TUI

# Diagnostics
roko doctor                           # Space health, capabilities, daemon, providers
roko status                           # machine-wide: active Space, Flows, daemon, costs
```

#### Verb sugar

Common Graph runs get top-level verb aliases for ergonomic one-liners. Every verb expands to `roko run <graph>`:

```
roko ingest <dir> [opts]              = roko run doc-ingest --input source_dir=<dir>
roko deploy [target] [opts]           = roko run deploy --macro target=<target>
roko research <topic> [opts]          = roko run research-sweep --input topic=<topic>
roko review [pr|diff|<paths>] [opts]  = roko run code-review --input ...
roko audit [opts]                     = roko run security-audit
roko refactor <desc> [opts]           = roko run refactor-batch --input description=<desc>
roko test [scope] [opts]              = roko run test-run --input target=<scope>
roko build [opts]                     = roko run build
roko backup [opts]                    = roko run backup
roko gc [opts]                        = roko run gc
roko watch <path> --graph <name>      = roko trigger create + immediate enable
roko cron <expr> --graph <name>       = roko trigger create + immediate enable
```

`roko chat [--agent <name>]` remains as direct interactive REPL — not a Graph because it is a stateful loop, not a finite computation.

#### Argument conventions

```bash
# Inputs (per the Graph's input.schema)
roko run doc-ingest --input source_dir=tmp/ux-refresh --input incremental=true

# Macros
roko run doc-ingest --macro enable_audit=true --macro budget_usd=10.00

# Slot filling
roko run doc-ingest --slot researcher=academic-search@^1

# From file
roko run doc-ingest --from-file ingest.toml
```

#### Universal run flags

```
--workspace <name>         override active Space for this command
--from-file <path>         all inputs/macros/slots in one TOML file
--from-stdin               read TOML config from stdin
--non-interactive          never prompt; fail closed if human input needed
--detach                   start the Flow, return run-id; daemon executes
--watch                    follow the Flow output in this terminal
--dry-run                  validate + estimate, do not execute
--budget <usd>             override Graph budget
--deadline <duration>      override Graph deadline
--json                     JSON output (for scripting)
--quiet                    only emit on errors
--verbose / -v             more detail
--no-color                 disable ANSI
--profile <name>           use a saved invocation profile
```

#### Foreground vs detached

By default, `roko run` runs in the foreground: live progress to the terminal until the Flow completes. Ctrl-C cancels. `--detach` returns immediately with a run-id; the daemon executes; follow up via `roko run logs <id>` or the dashboard.

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
| 16 | Space error (registry / locked / not found) |

### 3.2 TUI

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

The bottom **Transport** strip is always visible: it shows up to 3 active Flows with one-press controls (pause, cancel, detach, respond-to-human-input). This is the Workbench surface in its most compact form.

#### Tabs

**F1 Workbench** — Task delegation and Flow overview (Workbench + Agent Inbox surfaces). Shows Space health, active Flows, pending Triggers, recent completions, cost summary, and pending Inbox items at Urgency 2-3. Auto-refreshes every 1s.

**F2 Canvas** — Graph library and state-graph viewer (Generative Canvas surface). Two-pane: Graph list (left, 30%), detail + inline edit (right, 70%). Shows Macros, Slots, capabilities, estimated cost, last 5 runs. Press `r` to launch with parameter prompt overlay. Press `->` to enter Graph View (state-graph visualization rendered with ratatui-canvas).

**F3 Flows** — Flow inspector (Workbench surface, detail view). List of active and recent Flows (left), detail (right). Right pane has tabbed sub-views:
- **Overview** — node list + status + costs
- **Graph** — state-graph rendered with live node colors (jade=complete, cyan=in-flight, amber=queued, crimson=failed, violet=human-input)
- **Artifacts** — produced artifacts with previews
- **Episodes** — per-Block episodes
- **Logs** — full event stream, searchable, filterable by level
- **Trace** — node-by-node timing waterfall

**F4 Inbox** — Agent Inbox surface. All Inbox items sorted by urgency then recency. Quick-action keys: `a` approve, `r` reject, `d` defer, `x` dismiss. Shows urgency level, source agent, summary, deadline countdown for Level 2 items.

**F5 Knowledge** — Knowledge browser. Entries list with type, confidence, age, decay state. Resonance graph (ratatui-canvas). Lineage walker. Dream cycle history. Knowledge bundle management.

**F6 System** — Space + Daemon + Providers + Costs + Autonomy Slider surface. Space details (path, schema, capabilities, models). Daemon status (pid, uptime, CPU, memory). Provider health and cost breakdown. Index statistics. Per-agent autonomy levels with inline adjustment.

#### Universal keys

```
?            help overlay
/            command palette (fuzzy across Graphs, Flows, artifacts, pages)
g <letter>   go to: gw workbench, gc canvas, gf flows, gi inbox, gk knowledge, gs system
w            Space switcher (overlay)
c            create new (overlay: graph, trigger, rack, space)
.            quick run launcher (last-used Graph, edit args, run)
:            command line (typed commands like :run doc-ingest)
q            quit
^c           cancel current focus action
^r           refresh
F1-F6        jump to tab
Tab / S-Tab  cycle between panes within the active tab
```

#### Command palette

Press `/`. Behaves like Linear/Raycast: fuzzy search across Graphs, Flows, artifacts, pages, and actions. Scoped prefixes: `>` for actions, `@` for entities, `#` for pages.

#### State-graph view

ratatui-canvas-rendered graph with live node colors:
- **Jade** — node completed
- **Cyan flashing** — node in flight
- **Amber** — node queued
- **Crimson** — node failed
- **Violet** — node awaiting human input
- **Dimmed** — node pruned by failed conditional

Arrow keys move focus between nodes; `enter` drills into the node's Block, params, output, episode.

#### Performance mode

Full-screen live view (`p` from F1 or F3) for long-running expensive Flows. Shows animated Graph, cost/time progress bars, active node detail. The "watch the band play" view.

#### Space switcher

Press `w`. Lists recent and all Spaces, templates. Switching is instant: the TUI reattaches to the new Space's daemon view.

#### File watcher integration

The existing `notify::RecommendedWatcher` (`tui/fs_watch.rs`) watches `<workspace>/.roko/runs/`, `episodes.jsonl`, and `artifacts/`. Triggers re-render on change — reactive to daemon progress without polling.

### 3.3 Dashboard

The web dashboard (`roko serve` on :6677) is the primary visual surface for operators and teams. All five surfaces are present.

#### Navigation

Six destinations mapping to surfaces:

| Destination | Surfaces | Key pages |
|---|---|---|
| **Workbench** | Workbench + Agent Inbox | Task board, Event Stream, Inbox queue |
| **Agents** | Stigmergy Minimap + Autonomy Slider | Fleet minimap, Detail, Create, Templates, Groups, Autonomy controls |
| **Work** | Generative Canvas + Workbench | Library, Editor, Flows, Triggers, Marketplace |
| **Knowledge** | -- | Store, Resonance, Lineage, Stigmergy, Dreams |
| **Arena** | -- | Browser, Leaderboard, Benchmarks, Experiments |
| **System** | Autonomy Slider | Spaces, Providers, Costs, Deployments, Settings, Autonomy config |

#### Topbar

Space switcher dropdown (left of search). Click reveals recent Spaces with run counts and cost summaries. Switching is instant — the active Space's data context drives every page. Inbox badge count in topbar (red dot for Level 3 Review items).

#### Workbench page

Active Flows strip at top of page: live cards with state-graph thumbnails, cost/time progress. Pending Human Input badge. Recent completions. Trigger health bar. Inline Inbox queue for Level 2-3 items.

#### Work / Library

Graph library organized by category. Cards show: name, version, health dot, Macro/Slot counts, recent run count, cost, estimated typical run time. "Run" CTA opens a parameter overlay (Workbench event: TaskAssign); "Edit" opens the visual editor (Generative Canvas).

#### Work / Flows

Three columns: Flow list, Flow detail, inspector. Live updates over WebSocket. Selecting a node populates the inspector with Block-level detail.

Tabs in Flow detail:
- **Graph** — state-graph with animated node colors and edge traversals
- **Artifacts** — markdown rendered, JSON pretty-printed, images displayed, diffs highlighted
- **Episodes** — per-Block episodes
- **Logs** — full event stream
- **Trace** — Gantt waterfall showing node duration, queue time, retries

#### Agents / Fleet (Stigmergy Minimap)

Full rendering of the Stigmergy Minimap surface: spatial agent layout, pheromone field visualization, fog-of-war, group selection for batch operations. c-factor scores displayed per cluster.

#### Agents / Detail (Autonomy Slider)

Per-agent detail page includes the Autonomy Slider surface: per-capability autonomy levels, recent safety violations, grant/revoke controls.

#### Work / Triggers

Two-pane: trigger list with fire frequency and health, trigger detail with binding config, health metrics, recent events. "New Trigger" wizard: kind -> source config -> Graph + binding -> policy -> review.

#### Work / Marketplace

See [doc-15 (Marketplace)](15-MARKETPLACE-AND-SHARING.md). Browse, preview, install, fork community artifacts.

#### System / Spaces

Manage all registered Spaces. Per-Space detail: capabilities, models, deploy targets, knowledge sharing config. Edit opens a form view of `workspace.toml`; "raw TOML" toggle for direct editing.

#### Real-time plumbing

- **WebSocket `/ws/events`** — every Block lifecycle event for the active Space
- **WebSocket `/ws/runs/<run-id>`** — focused stream for one Flow
- **SSE `/sse/triggers`** — Trigger fire and dispatch events
- **SSE `/sse/cost`** — live cost ticks
- **SSE `/sse/inbox`** — Inbox items for Agent Inbox surface
- **HTTP `/api/v1/graphs`, `/runs`, `/triggers`, `/spaces`, `/artifacts`, `/episodes`** — REST for queries

Every page is fully reactive: Flow list updates without refresh, Graph view animates state transitions, cost gauge ticks live.

#### Visual style

Glass morphism on panels (3 levels). Spring-physics motion (Framer Motion). Rose accent on actives. Monospace data. Animated number ticks. Breathing pulses on live indicators. ROSEDUST palette: rose accent, jade success, amber warning, crimson error, violet knowledge, sapphire active Blocks.

### 3.4 Visual Editor (Generative Canvas)

The dashboard's drag-and-drop authoring environment for Graphs, Racks, Triggers, and Profiles. The full Generative Canvas surface.

#### Layout

Three columns:
- **Palette** (left, ~240px): draggable Blocks, control-flow primitives (Branches, Loops, FanOut, FanIn, HumanInput), Macros, Slots, Triggers, Snippets, Criteria
- **Canvas** (center): the state graph — the playground
- **Inspector** (right, ~360px, collapsible): properties of the selected node / edge / Macro / Slot

#### Nodes as cards

Each node is a card with:
- Header strip with Block name + version + status badge
- Input port row (top edge, one per typed input)
- Body: collapsed compact info, expandable for full param list
- Output port row (bottom edge, one per typed output)
- Footer with cost/time estimates and capability badges
- Quick actions on hover: pin, fork-this-step, replace-block, mute (skip), solo (run only this)

Node colors:
- **Sapphire** — Block nodes
- **Violet** — Sub-Graph nodes
- **Rose** — HumanInput / Wait nodes
- **Amber** — Branch / FanOut / FanIn / Loop control nodes
- **Slate** — Slot placeholders
- **Glass** — pruned-by-condition (ghosted)

#### Edges as cables

Cable colors encode payload type: rose (doc), jade (code), sapphire (structured data), amber (evidence), violet (knowledge), white (generic). Cable thickness encodes batch size.

Cable behavior:
- Hover: last value preview in tooltip
- Click in-flight: stream viewer of flowing values
- Drag output-to-input: creates cable. Type match = instant. Mismatch = adapter prompt from registry.

#### Conditions on edges

Diamond marker on conditional edges. Click to edit with plain-language editor:

```
when:  enable_web_research is true  AND  classify.confidence > 0.8
```

Color-coded: green (valid), amber (valid but never true given current Macros), red (unparseable).

#### Slot visualization

Slots render as cards with dotted borders. Drag a compatible Block from the palette onto the Slot to fill it (Workbench event: SlotFill). Right-click to revert to default or change.

#### Loops and branches

Loop nodes wrap their body subgraph with a curved return arrow. Branch nodes have multiple output ports, each with its own condition. Click to edit predicates and `max_iterations`.

#### Sub-Graphs

Sub-Graph nodes render compact. Double-click to descend (breadcrumb shows nesting). Editing shared sub-Graphs prompts: "This sub-Graph is used in N places. Edit affects all. Continue / fork copy?"

#### Macro promotion

The signature gesture: right-click an internal Block parameter -> "Promote to Macro" (Canvas event: MacroPromote). Inline modal for name, label, kind, range, default. The parameter value is replaced with `{{ macro.name }}` and a MacroDef is added to the Rack.

A single Macro can bind to multiple internal params. The visual editor highlights all bound params when the Macro is hovered.

Macro controls render as:
- Floats / Money: rotary knob (with text input fallback)
- Integers: stepper
- Booleans: toggle switch
- Enums: segmented control
- Strings: text input
- ModelRef: searchable dropdown
- AgentRef: avatar-tile dropdown

#### Live preview

- **Last-value preview**: after any run, hovering a node shows its last output (markdown rendered, JSON pretty-printed, images thumbnailed, diffs highlighted)
- **Test-isolated**: right-click any node -> "Test in isolation". Runs only that Block in a sandbox.
- **Run-up-to-here**: right-click any node -> "Run up to here". Engine runs the partial Graph, halts, returns intermediate state.

#### Multiple views

Every Graph has three views, toggled by keyboard or button:
- **Recipe view** (linear, Apple-Shortcuts-like step list) — beginner-friendly
- **Graph view** (DAG canvas) — power authoring
- **Timeline view** (Gantt waterfall of last run) — parallelism and bottleneck visualization

#### Sound

Web Audio API generated tones, off by default:
1. Cable plug-in — perfect fifth, 60ms triangle
2. Cable mismatch — minor second, 80ms
3. Validation success — short major chord arpeggio
4. Validation failure — low resonant tone, 200ms
5. Flow start — octave, 120ms with reverb tail
6. Flow complete — high sine ping, 60ms

Principle: juice the user's actions and the agent's completions, never ambient state.

#### Motion

Spring physics throughout (Framer Motion):
- Node drag: stiffness 500, damping 30
- Node insert: stiffness 200, damping 28, scale 0.9 -> 1.0
- Cable connect: 200ms ease, brief flash
- Validation error: shake with limited amplitude
- Macro promotion: source-param card morphs into Macro card via FLIP
- Recipe <-> Graph toggle: 400ms crossfade with positional FLIP
- Flow state changes: edges flash rose during traversal, nodes pulse cyan in flight
- All animations respect `prefers-reduced-motion`

#### Keyboard

Every mouse interaction has a keyboard equivalent:

```
Cmd+K               command palette (filtered to authoring actions)
Cmd+/               search palette of Blocks to add
Cmd+E               toggle Recipe / Graph view
Cmd+R               run the Graph
Cmd+Shift+R         test-isolated the selected node
Cmd+G               group selection into sub-Graph
Cmd+Shift+G         ungroup sub-Graph
Cmd+D               duplicate selected node(s)
Cmd+Z / Shift+Cmd+Z undo / redo
Cmd+S               save
Cmd+Shift+P         publish to marketplace
Cmd+M               promote selected param to Macro
Tab                 cycle selection through outgoing edges
Arrow keys          move selected node
Esc                 deselect / collapse modal
.                   focus palette search
[ / ]               collapse / expand inspector
```

#### Inline error handling

Validation runs continuously. Errors are inline (red glow on offending node + edge), not modal. Bottom strip aggregates errors with click-to-jump.

Common errors and inline UX:
- **Type mismatch**: red cable; click offers adapter or fix
- **Required Slot empty**: red dotted border with "Fill this Slot" CTA
- **Required Macro unbound**: Macro chip in palette is red
- **Cycle detected**: cycle nodes pulse red; "Wrap in Loop?" suggestion
- **Capability not granted**: amber capability badge; "Grant in Space settings" CTA
- **Cost estimate exceeds budget**: cost gauge amber; suggestion to lower budget or trim

#### Snippets

Select a region of the canvas -> "Save as snippet". Snippets are reusable graph fragments stored at Space or user level. Insertable as sub-Graph or inline. First-class marketplace artifacts (see [doc-15](15-MARKETPLACE-AND-SHARING.md)).

#### Authoring modes

- **Compose**: drag nodes, wire cables, edit params. Default. The "playing music" mode.
- **Inspect**: read-only canvas with run-history overlay. For reviewing before forking.
- **Live**: canvas overlays the latest Flow's animation in real time.

---

## 4. Building Wizards

All rendering targets offer guided wizards for common creation tasks. The wizards follow a 4-stage flow:

### 4.1 Graph wizard

```
Stage 1: Name + template
  name: ____________________
  template: ( ) blank  ( ) doc-pipeline  ( ) deploy  ( ) custom

Stage 2: Select Blocks
  Toggle Blocks to include, organized by category.
  Auto-suggest edges based on type compatibility.

Stage 3: Configure Macros and Slots
  Promote parameters, declare Slots, set defaults.

Stage 4: Review + validate
  Live TOML preview. Validation results. [Save] [Run] [Publish]
```

Available in: TUI (`c` -> Graph), Dashboard (Work / Library / + New Graph), Visual Editor (Palette / + New).

### 4.2 Agent wizard

```
Stage 1: Name + profile
  name: ____________________
  profile: ( ) coding  ( ) research  ( ) operations  ( ) custom

Stage 2: Extensions and Connectors
  Toggle Extensions (safety, budget, episode-logger, ...).
  Configure Connectors (MCP servers, databases, APIs).

Stage 3: Models and budget
  Primary / fallback / reflexive model selection.
  Budget limits (USD, tokens, duration).

Stage 4: Review + create
  Preview roko.toml agent section. [Create] [Start]
```

Available in: CLI (`roko agent create`), TUI (F6 System), Dashboard (Agents / Create).

### 4.3 Connector wizard

```
Stage 1: Kind
  ( ) Chain RPC  ( ) MCP Server  ( ) Database  ( ) Webhook  ( ) API

Stage 2: Connection config
  Kind-specific fields (RPC URL, connection string, auth, etc.).

Stage 3: Capabilities
  What this Connector needs (Net domains, Secrets keys).

Stage 4: Test + save
  Health check. [Save] [Attach to Agent]
```

Available in: CLI (`roko config connectors add`), Dashboard (Agents / Detail / Connectors).

---

## 5. Third-Party Surfaces

Third parties build new surfaces by consuming StateHub projections and emitting surface events. The five named surfaces define the contracts; implementations are open.

Examples:
- **Mobile app**: renders Workbench + Agent Inbox surfaces for on-the-go task delegation and decision making.
- **Slack bot**: renders Agent Inbox surface into Slack messages with reaction-based approve/reject.
- **VSCode extension**: renders Generative Canvas as a panel + Workbench as a sidebar.
- **CLI dashboard (tmux)**: renders all five surfaces in tmux panes with curses widgets.

The contract is the projections and events. The rendering is unconstrained.

---

## 6. Cross-Surface Linking

Surfaces link to each other. The CLI prints clickable URLs (OSC 8 hyperlinks) and `roko://` references:

```
$ roko run doc-ingest ...
Run id: wf_01HGZK7B...
Dashboard: http://localhost:6677/runs/wf_01HGZK7B...
TUI: roko tui --run wf_01HGZK7B...
```

`roko tui --run <id>` jumps directly to the Flow inspector. Dashboard URLs deep-link to specific Flows, Graphs, and Triggers.

---

## 7. Theming

### 7.1 TUI

ROSEDUST palette adapted for terminal: rose accent on active elements, jade for success, amber for warning, crimson for error, violet for knowledge, sapphire for active Blocks. Monospace throughout. User-toggleable density (compact / comfortable / spacious) via `~/.roko/config.toml`.

### 7.2 Dashboard

Glass morphism on panels (3 levels). Spring-physics motion. Same ROSEDUST palette. Tabular nums. Stagger-children at 40ms on list mounts. Block-specific visual elements: state graph node colors match the TUI scheme, edge animations (traversed = rose flash, pruned = ghost fade), Macro sliders as DAW-style controls.

---

## 8. Acceptance Criteria

| Criterion | Verification |
|---|---|
| **Surface contracts**: Workbench, Inbox, Canvas, Minimap, Slider each have typed projections and events | Schema validation test on each surface contract |
| **Third-party surface**: an external consumer can subscribe to StateHub projections and emit surface events | Integration test: mock surface consumes projections, emits events |
| **CLI**: `roko run <graph>` runs in foreground with live progress (Workbench surface) | Manual test on doc-ingest |
| **CLI**: `roko inbox list` shows pending items (Agent Inbox surface) | Inbox population test |
| **CLI**: `roko autonomy set` changes levels (Autonomy Slider surface) | Autonomy change test |
| **CLI**: `roko --json` produces valid JSON for all commands | Schema tests |
| **CLI**: Tab completion enumerates Graphs, Blocks, Spaces, run-ids | Manual test in zsh + fish |
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
| **Visual Editor**: Drag Block from palette -> node appears with correct ports | Manual + e2e |
| **Visual Editor**: Promote-to-Macro replaces param and adds MacroDef (Canvas event: MacroPromote) | Round-trip test |
| **Visual Editor**: Recipe <-> Graph toggle preserves all data | Round-trip test |
| **Visual Editor**: Test-isolated runs selected Block within 30s | Latency test |
| **Visual Editor**: Validation errors render inline; bottom strip aggregates | Synthetic invalid Graph |
| **Visual Editor**: Sound effects play on cable connect / validate / run | Audio test |
| **Wizards**: Graph wizard emits valid TOML | Round-trip test |
| **Cross-surface**: CLI prints dashboard URL; URL deep-links to correct Flow | Link test |
