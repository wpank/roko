# 16 — Surfaces

> Four surfaces over the same system: CLI, TUI, Dashboard, and Visual Editor. Same Blocks, Graphs, Triggers, Spaces — different interaction modes.

**Source**: wf-08 (CLI Redesign), wf-09 (TUI Redesign), wf-10 (Dashboard Redesign), wf-11 (Visual Config Wizard), combined with unified vocabulary.

---

## 1. Design Principles

All four surfaces share the same backend types, the same TOML schema, and the same execution engine. They differ only in interaction mode:

| Surface | Mode | Primary user | Runs on |
|---|---|---|---|
| **CLI** | Command-line, scriptable | Power users, CI | Terminal |
| **TUI** | ratatui, keyboard-driven | Developers monitoring | Terminal |
| **Dashboard** | Web, mouse-driven | Operators, teams | Browser |
| **Visual Editor** | Drag-and-drop canvas | Graph authors | Browser (dashboard) |

Every operation that one surface supports, every other surface supports — possibly through different UX. Running a Graph, filling a Slot, promoting a Macro, inspecting a Flow, managing Triggers — all surfaces, same outcome.

---

## 2. CLI Surface

The `roko` CLI is Graph-centric: every meaningful operation is a Graph run or a registry operation.

### 2.1 Top-level command surface

```
roko                                  # Space summary (active Space, recent Flows, pending humans)
roko help [<command>]
roko version

# Graph execution — the primary surface
roko run <graph> [args]              # run a Graph
roko run cancel <run-id>              # cancel an active Flow
roko run respond <run-id> [args]      # answer a human-input prompt
roko run resume <run-id>              # resume a snapshotted Flow
roko run list [--status]              # active and recent Flows
roko run show <run-id>                # detailed Flow inspection
roko run logs <run-id> [--follow]     # stream Flow logs
roko run replay <run-id>              # rerun with the same inputs

# Graph registry
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

### 2.2 Verb sugar

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

### 2.3 Argument conventions

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

### 2.4 Universal run flags

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

### 2.5 Foreground vs detached

By default, `roko run` runs in the foreground: live progress to the terminal until the Flow completes. Ctrl-C cancels. `--detach` returns immediately with a run-id; the daemon executes; follow up via `roko run logs <id>` or the dashboard.

### 2.6 Exit codes

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

---

## 3. TUI Surface

The TUI (`roko tui`) is a ratatui-based terminal application. Keyboard-driven, no server required. Six tabs plus a persistent transport strip.

### 3.1 Layout

```
 roko - Space: nunchi-dashboard - providers: ok - daemon: ok - cost today: $4.12

  [F1 Pulse] [F2 Graphs] [F3 Flows] [F4 Triggers] [F5 Knowledge] [F6 System]

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

The bottom **Transport** strip is always visible: it shows up to 3 active Flows with one-press controls (pause, cancel, detach, respond-to-human-input).

### 3.2 Tabs

#### F1 Pulse — Space overview

The default tab. Shows Space health (daemon, providers, triggers), active Flows, pending Triggers, recent completions, cost summary. Auto-refreshes every 1s.

#### F2 Graphs — Library and editor

Two-pane: Graph list (left, 30%), detail + inline edit (right, 70%). Shows Macros, Slots, capabilities, estimated cost, last 5 runs. Press `r` to launch with parameter prompt overlay. Press `->` to enter Graph View (state-graph visualization rendered with ratatui-canvas).

#### F3 Flows — Flow inspector

List of active and recent Flows (left), detail (right). Right pane has tabbed sub-views:
- **Overview** — node list + status + costs
- **Graph** — state-graph rendered with live node colors (jade=complete, cyan=in-flight, amber=queued, crimson=failed, violet=human-input)
- **Artifacts** — produced artifacts with previews
- **Episodes** — per-Block episodes
- **Logs** — full event stream, searchable, filterable by level
- **Trace** — node-by-node timing waterfall

#### F4 Triggers — Trigger manager

List of all Triggers with kind, bound Graph, fire frequency, health. Press `enter` to inspect, `t` to test-fire, `e` to edit, `n` to create new.

#### F5 Knowledge — Knowledge browser

Entries list with type, confidence, age, decay state. Resonance graph (ratatui-canvas). Lineage walker. Dream cycle history. Knowledge bundle management.

#### F6 System — Space + Daemon + Providers + Costs

Space details (path, schema, capabilities, models). Daemon status (pid, uptime, CPU, memory). Provider health and cost breakdown. Index statistics.

### 3.3 Universal keys

```
?            help overlay
/            command palette (fuzzy across Graphs, Flows, artifacts, pages)
g <letter>   go to: gp pulse, gg graphs, gf flows, gt triggers, gk knowledge, gs system
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

### 3.4 Command palette

Press `/`. Behaves like Linear/Raycast: fuzzy search across Graphs, Flows, artifacts, pages, and actions. Scoped prefixes: `>` for actions, `@` for entities, `#` for pages.

### 3.5 State-graph view

ratatui-canvas-rendered graph with live node colors:
- **Jade** — node completed
- **Cyan flashing** — node in flight
- **Amber** — node queued
- **Crimson** — node failed
- **Violet** — node awaiting human input
- **Dimmed** — node pruned by failed conditional

Arrow keys move focus between nodes; `enter` drills into the node's Block, params, output, episode.

### 3.6 Performance mode

Full-screen live view (`p` from F1 or F3) for long-running expensive Flows. Shows animated Graph, cost/time progress bars, active node detail. The "watch the band play" view.

### 3.7 Space switcher

Press `w`. Lists recent and all Spaces, templates. Switching is instant: the TUI reattaches to the new Space's daemon view.

### 3.8 File watcher integration

The existing `notify::RecommendedWatcher` (`tui/fs_watch.rs`) watches `<workspace>/.roko/runs/`, `episodes.jsonl`, and `artifacts/`. Triggers re-render on change — reactive to daemon progress without polling.

---

## 4. Dashboard Surface

The web dashboard (`roko serve` on :6677) is the primary visual surface for operators and teams.

### 4.1 Navigation

Six destinations:

| Destination | Key pages |
|---|---|
| **Pulse** | Dashboard, Event Stream, Network Pulse |
| **Agents** | Fleet, Detail, Create, Templates, Groups |
| **Work** | Library, Editor, Flows, Triggers, Marketplace |
| **Knowledge** | Store, Resonance, Lineage, Stigmergy, Dreams |
| **Arena** | Browser, Leaderboard, Benchmarks, Experiments |
| **System** | Spaces, Providers, Costs, Deployments, Settings |

### 4.2 Topbar

Space switcher dropdown (left of search). Click reveals recent Spaces with run counts and cost summaries. Switching is instant — the active Space's data context drives every page.

### 4.3 Pulse

Active Flows strip at top of page: live cards with state-graph thumbnails, cost/time progress. Pending Human Input badge (red dot) in topbar. Recent completions. Trigger health bar.

### 4.4 Work / Library

Graph library organized by category. Cards show: name, version, health dot, Macro/Slot counts, recent run count, cost, estimated typical run time. "Run" CTA opens a parameter overlay; "Edit" opens the visual editor.

### 4.5 Work / Flows

Three columns: Flow list, Flow detail, inspector. Live updates over WebSocket. Selecting a node populates the inspector with Block-level detail.

Tabs in Flow detail:
- **Graph** — state-graph with animated node colors and edge traversals
- **Artifacts** — markdown rendered, JSON pretty-printed, images displayed, diffs highlighted
- **Episodes** — per-Block episodes
- **Logs** — full event stream
- **Trace** — Gantt waterfall showing node duration, queue time, retries

### 4.6 Work / Triggers

Two-pane: trigger list with fire frequency and health, trigger detail with binding config, health metrics, recent events. "New Trigger" wizard: kind -> source config -> Graph + binding -> policy -> review.

### 4.7 Work / Marketplace

See [doc-15 (Marketplace)](15-MARKETPLACE-AND-SHARING.md). Browse, preview, install, fork community artifacts.

### 4.8 System / Spaces

Manage all registered Spaces. Per-Space detail: capabilities, models, deploy targets, knowledge sharing config. Edit opens a form view of `workspace.toml`; "raw TOML" toggle for direct editing.

### 4.9 Real-time plumbing

- **WebSocket `/ws/events`** — every Block lifecycle event for the active Space
- **WebSocket `/ws/runs/<run-id>`** — focused stream for one Flow
- **SSE `/sse/triggers`** — Trigger fire and dispatch events
- **SSE `/sse/cost`** — live cost ticks
- **HTTP `/api/v1/graphs`, `/runs`, `/triggers`, `/spaces`, `/artifacts`, `/episodes`** — REST for queries

Every page is fully reactive: Flow list updates without refresh, Graph view animates state transitions, cost gauge ticks live.

### 4.10 Visual style

Glass morphism on panels (3 levels). Spring-physics motion (Framer Motion). Rose accent on actives. Monospace data. Animated number ticks. Breathing pulses on live indicators. ROSEDUST palette: rose accent, jade success, amber warning, crimson error, violet knowledge, sapphire active Blocks.

---

## 5. Visual Editor Surface

The dashboard's drag-and-drop authoring environment for Graphs, Racks, Triggers, and Racks. The "video-game" authoring UX.

### 5.1 Layout

Three columns:
- **Palette** (left, ~240px): draggable Blocks, control-flow primitives (Branches, Loops, FanOut, FanIn, HumanInput), Macros, Slots, Triggers, Snippets
- **Canvas** (center): the state graph — the playground
- **Inspector** (right, ~360px, collapsible): properties of the selected node / edge / Macro / Slot

### 5.2 Nodes as cards

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

### 5.3 Edges as cables

Cable colors encode payload type: rose (doc), jade (code), sapphire (structured data), amber (evidence), violet (knowledge), white (generic). Cable thickness encodes batch size.

Cable behavior:
- Hover: last value preview in tooltip
- Click in-flight: stream viewer of flowing values
- Drag output-to-input: creates cable. Type match = instant. Mismatch = adapter prompt from registry.

### 5.4 Conditions on edges

Diamond marker on conditional edges. Click to edit with plain-language editor:

```
when:  enable_web_research is true  AND  classify.confidence > 0.8
```

Color-coded: green (valid), amber (valid but never true given current Macros), red (unparseable).

### 5.5 Slot visualization

Slots render as cards with dotted borders. Drag a compatible Block from the palette onto the Slot to fill it. Right-click to revert to default or change.

### 5.6 Loops and branches

Loop nodes wrap their body subgraph with a curved return arrow. Branch nodes have multiple output ports, each with its own condition. Click to edit predicates and `max_iterations`.

### 5.7 Sub-Graphs

Sub-Graph nodes render compact. Double-click to descend (breadcrumb shows nesting). Editing shared sub-Graphs prompts: "This sub-Graph is used in N places. Edit affects all. Continue / fork copy?"

### 5.8 Macro promotion

The signature gesture: right-click an internal Block parameter -> "Promote to Macro". Inline modal for name, label, kind, range, default. The parameter value is replaced with `{{ macro.name }}` and a MacroDef is added to the Rack.

A single Macro can bind to multiple internal params. The visual editor highlights all bound params when the Macro is hovered.

Macro controls render as:
- Floats / Money: rotary knob (with text input fallback)
- Integers: stepper
- Booleans: toggle switch
- Enums: segmented control
- Strings: text input
- ModelRef: searchable dropdown
- AgentRef: avatar-tile dropdown

### 5.9 Live preview

- **Last-value preview**: after any run, hovering a node shows its last output (markdown rendered, JSON pretty-printed, images thumbnailed, diffs highlighted)
- **Test-isolated**: right-click any node -> "Test in isolation". Runs only that Block in a sandbox.
- **Run-up-to-here**: right-click any node -> "Run up to here". Engine runs the partial Graph, halts, returns intermediate state.

### 5.10 Multiple views

Every Graph has three views, toggled by keyboard or button:
- **Recipe view** (linear, Apple-Shortcuts-like step list) — beginner-friendly
- **Graph view** (DAG canvas) — power authoring
- **Timeline view** (Gantt waterfall of last run) — parallelism and bottleneck visualization

### 5.11 Sound

Web Audio API generated tones, off by default:
1. Cable plug-in — perfect fifth, 60ms triangle
2. Cable mismatch — minor second, 80ms
3. Validation success — short major chord arpeggio
4. Validation failure — low resonant tone, 200ms
5. Flow start — octave, 120ms with reverb tail
6. Flow complete — high sine ping, 60ms

Principle: juice the user's actions and the agent's completions, never ambient state.

### 5.12 Motion

Spring physics throughout (Framer Motion):
- Node drag: stiffness 500, damping 30
- Node insert: stiffness 200, damping 28, scale 0.9 -> 1.0
- Cable connect: 200ms ease, brief flash
- Validation error: shake with limited amplitude
- Macro promotion: source-param card morphs into Macro card via FLIP
- Recipe <-> Graph toggle: 400ms crossfade with positional FLIP
- Flow state changes: edges flash rose during traversal, nodes pulse cyan in flight
- All animations respect `prefers-reduced-motion`

### 5.13 Keyboard

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

### 5.14 Inline error handling

Validation runs continuously. Errors are inline (red glow on offending node + edge), not modal. Bottom strip aggregates errors with click-to-jump.

Common errors and inline UX:
- **Type mismatch**: red cable; click offers adapter or fix
- **Required Slot empty**: red dotted border with "Fill this Slot" CTA
- **Required Macro unbound**: Macro chip in palette is red
- **Cycle detected**: cycle nodes pulse red; "Wrap in Loop?" suggestion
- **Capability not granted**: amber capability badge; "Grant in Space settings" CTA
- **Cost estimate exceeds budget**: cost gauge amber; suggestion to lower budget or trim

### 5.15 Snippets

Select a region of the canvas -> "Save as snippet". Snippets are reusable graph fragments stored at Space or user level. Insertable as sub-Graph or inline. First-class marketplace artifacts.

### 5.16 Authoring modes

- **Compose**: drag nodes, wire cables, edit params. Default. The "playing music" mode.
- **Inspect**: read-only canvas with run-history overlay. For reviewing before forking.
- **Live**: canvas overlays the latest Flow's animation in real time.

---

## 6. Building Wizards

All surfaces offer guided wizards for common creation tasks. The wizards follow a 4-stage flow:

### 6.1 Graph wizard

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

### 6.2 Agent wizard

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

### 6.3 Connector wizard

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

## 7. Cross-Surface Linking

Surfaces link to each other. The CLI prints clickable URLs (OSC 8 hyperlinks) and `roko://` references:

```
$ roko run doc-ingest ...
Run id: wf_01HGZK7B...
Dashboard: http://localhost:6677/runs/wf_01HGZK7B...
TUI: roko tui --run wf_01HGZK7B...
```

`roko tui --run <id>` jumps directly to the Flow inspector. Dashboard URLs deep-link to specific Flows, Graphs, and Triggers.

---

## 8. Theming

### 8.1 TUI

ROSEDUST palette adapted for terminal: rose accent on active elements, jade for success, amber for warning, crimson for error, violet for knowledge, sapphire for active Blocks. Monospace throughout. User-toggleable density (compact / comfortable / spacious) via `~/.roko/config.toml`.

### 8.2 Dashboard

Glass morphism on panels (3 levels). Spring-physics motion. Same ROSEDUST palette. Tabular nums. Stagger-children at 40ms on list mounts. Block-specific visual elements: state graph node colors match the TUI scheme, edge animations (traversed = rose flash, pruned = ghost fade), Macro sliders as DAW-style controls.

---

## 9. Acceptance Criteria

| Criterion | Verification |
|---|---|
| **CLI**: `roko run <graph>` runs in foreground with live progress | Manual test on doc-ingest |
| **CLI**: `roko --json` produces valid JSON for all commands | Schema tests |
| **CLI**: Tab completion enumerates Graphs, Blocks, Spaces, run-ids | Manual test in zsh + fish |
| **CLI**: Verb sugar maps correctly to Graph runs | Equivalence test |
| **TUI**: Launches via `roko tui`, renders F1 Pulse | Smoke test |
| **TUI**: F1 auto-refreshes every 1s without flicker | Visual / load test |
| **TUI**: F3 streams live Flow output within 100ms | Latency test |
| **TUI**: State-graph view renders with live node colors | Visual snapshot |
| **TUI**: Space switcher changes context; tab content updates | Multi-Space test |
| **Dashboard**: Space switcher lists all Spaces, switches data context | Multi-Space test |
| **Dashboard**: Flow inspector receives WebSocket updates within 200ms | Latency test |
| **Dashboard**: State-graph node colors update live during Flow | Visual regression |
| **Visual Editor**: Drag Block from palette -> node appears with correct ports | Manual + e2e |
| **Visual Editor**: Promote-to-Macro replaces param and adds MacroDef | Round-trip test |
| **Visual Editor**: Recipe <-> Graph toggle preserves all data | Round-trip test |
| **Visual Editor**: Test-isolated runs selected Block within 30s | Latency test |
| **Visual Editor**: Validation errors render inline; bottom strip aggregates | Synthetic invalid Graph |
| **Visual Editor**: Sound effects play on cable connect / validate / run | Audio test |
| **Wizards**: Graph wizard emits valid TOML | Round-trip test |
| **Cross-surface**: CLI prints dashboard URL; URL deep-links to correct Flow | Link test |
