# 05. Page Specifications

Scene-by-scene specs for the 5 primary views. Each spec includes: purpose, layout, data sources, animations, density, and what earns its space.

---

## Layout Principles

Every page follows the same scrollable container model. These are hard rules, not suggestions.

### The Container Model

Every scene is a single `<PageShell>` that renders as an `overflow-y: auto` container filling the viewport below TopNav. Content flows top-to-bottom and scrolls naturally. There is no viewport-locking of inner panels.

```
┌── TopNav (sticky, outside page) ──────────────────────┐
├───────────────────────────────────────────────────────┤
│  <PageShell>                      overflow-y: auto    │
│  ┌─ page header ────────────────────────────────────┐ │
│  │ AsciiLabel title + hero line                     │ │
│  └──────────────────────────────────────────────────┘ │
│  ┌─ MetricGrid ────────────────────────────────────┐  │
│  │ compact metric cells, auto-height               │  │
│  └──────────────────────────────────────────────────┘ │
│  ┌─ Tabs (if applicable) ──────────────────────────┐  │
│  │ tab bar flows inline, not sticky                │  │
│  └──────────────────────────────────────────────────┘ │
│  ┌─ content sections ──────────────────────────────┐  │
│  │ panels, tables, grids — all flow in document    │  │
│  │ order, height determined by content             │  │
│  └──────────────────────────────────────────────────┘ │
│                                                       │
│  (page scrolls naturally to show all content)         │
└───────────────────────────────────────────────────────┘
```

### Hard Rules

1. **No `height: 100vh` or `min-height: 100vh`** on any page container. Pages grow with content.
2. **No viewport-locked inner panels.** The old Demo.tsx pattern (fixed terminal panes that split the viewport) is deprecated. Terminal panes live inside the scrollable container with `max-height` and internal `overflow-y: auto`.
3. **Content flows top-to-bottom:** header, metrics, tabs, content sections. The page scrolls to reveal all of them.
4. **Cards and panels fill available width.** Height is determined by content, never forced to fill viewport.
5. **`<ScrollArea>` is for bounded sub-regions** (terminal output, log feeds, long tables) -- not for the page itself. The page IS the scroll container.

### Density & Spacing

All pages follow the spacing tokens from `04-DESIGN-SYSTEM.md` and the layout primitives from `09-DESIGN-PRIMITIVES.md`:

| Context | Token | Value |
|---------|-------|-------|
| Page padding (sides) | `--gap-lg` | 24px |
| Section gaps | `--gap-lg` | 24px |
| Card/panel internal padding | `--cell-padding` | 10px 12px |
| Grid gaps (metric cells) | `--cell-gap` | 8px |
| Tight grids (mosaic style) | 1px gap | `--border` color showing through |
| Inline element spacing | `--gap-sm` | 8px |
| Label-to-value gap | `--gap-xs` | 4px |

**Density target:** Content-dense, not sparse. Metric grids should use `<Grid columns="auto" minWidth="200px">` so cells pack tightly. No large empty regions between sections. If a section has only 1-2 items, it shares a row with adjacent content rather than taking a full-width block.

### Loading, Empty, Error

Every data-dependent section is wrapped in `<DataSurface>`:

- **Loading:** `<Skeleton>` placeholder matching the shape of the expected content. MetricGrid shows shimmer cells. Tables show 3-5 shimmer rows. Terminal panes show a dim "Connecting..." label.
- **Empty:** `<EmptyState>` with: what is empty, what action to take, and a technical hint if applicable. Example: "No benchmark runs yet. Run `roko bench start` to create one."
- **Error:** `<DataSurface>` error state with message + retry button. Errors do not blank the whole page -- only the affected section shows the error.

---

## Navigation: 5 Scenes

```
TopNav:  ◆ ROKO     ORCHESTRATE  OBSERVE  EVALUATE  BUILD  KNOWLEDGE    ● LIVE
```

| Scene | URL | Purpose |
|-------|-----|---------|
| Orchestrate | /app/orchestrate | Watch roko turn a request into verified code |
| Observe | /app/observe | Operational control plane: agents, routing, knowledge |
| Evaluate | /app/evaluate | Economic evidence: benchmarks, Pareto frontier |
| Build | /app/build | Try it yourself: prompt builder + terminal |
| Knowledge | /app/knowledge | Browse the knowledge store and dream cycles |

---

## 1. Orchestrate — "Watch It Build"

**Hero line:** *"One request. Autonomous planning, routing, execution, and verification."*

### 1.1 Scroll Structure

```
<PageShell title="Orchestrate">
  PhaseRail                     ← inline, flows with content
  scenario picker / phase content
  terminal pane (if active)     ← inside scroll container, max-height bounded
  MetricGrid (if active)        ← compact, auto-height
</PageShell>
```

The entire page scrolls. The `<PhaseRail>` is NOT sticky -- it scrolls with the page header. During the run phase, the terminal pane and metrics grid are stacked vertically inside the scroll container, not viewport-locked side by side.

### 1.2 Phase-Driven Progressive Disclosure

The page layout changes based on pipeline phase. It does NOT show everything at once.

**Phase 0 -- Idle:**
```
┌─────────────────────────────────────────────────────────┐
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐    │
│  │ Simple       │ │ Medium       │ │ Complex      │    │  <- scenario cards
│  │ 3 tasks, T1  │ │ 5 tasks, T2  │ │ 6 tasks, T3  │    │     <Grid> auto-fill
│  └──────────────┘ └──────────────┘ └──────────────┘    │
│                                                         │
│        Build a BTC funding alert CLI from               │  <- scenario desc
│        Hyperliquid and email me when funding            │     Fraunces 30px
│        flips negative.                                  │
│                                                         │
│                   ▶  START LIVE RUN                      │
│                                                         │
│  ○ idea   ○ prd   ○ plan   ○ tasks   ○ run   ○ verify   │  <- PhaseRail
└─────────────────────────────────────────────────────────┘
```

Scenario cards use `<Panel>` with `<AsciiLabel>` headers. Cards arranged in a `<Grid columns="auto" minWidth="240px">` -- tight `--cell-gap` spacing.

**Phase 1-2 -- Artifact Generation:**
```
┌─────────────────────────────────────────────────────────┐
│  ● idea   ◉ prd   ○ plan   ○ tasks   ○ run   ○ verify  │  <- PhaseRail
├─────────────────────────────────────────────────────────┤
│  ┌─ GENERATED PRD ───────────────────────────────────┐  │
│  │ AsciiLabel: "PRD"    StatusBadge: generating      │  │
│  │ BTC Funding Alert CLI                             │  │
│  │ Requirements: 5 · Acceptances: 4                  │  │
│  │ Slug: btc-funding-cli                             │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌─ EVIDENCE ────────────────────────────────────────┐  │
│  │ <ScrollArea maxHeight="300px">                    │  │
│  │   $ roko prd idea "..."                           │  │
│  │   ✓ idea captured                                 │  │
│  │   $ roko prd draft new...                         │  │
│  │   → generating PRD...                             │  │
│  │ </ScrollArea>                                     │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

Artifact panel and evidence terminal are stacked vertically. Evidence terminal uses `<ScrollArea maxHeight="300px">` with internal scroll and fade edges. Both panels are full-width `<Panel>` components.

**Phase 3 -- Plan Generated:**
```
┌─────────────────────────────────────────────────────────┐
│  ● idea   ● prd   ◉ plan   ○ tasks   ○ run   ○ verify  │
├─────────────────────────────────────────────────────────┤
│  ┌─ GENERATED PLAN ─────────────────────────────────┐   │
│  │ 6 tasks · 3 tiers · Estimated: ~$0.05            │   │
│  │ Split into: DeFi data, flip detection, email,    │   │
│  │ orchestration, verify.                           │   │
│  └──────────────────────────────────────────────────┘   │
│  ┌─ ROUTING & MODELS ──────────────────────────────┐    │
│  │ MetricGrid: [ T1: 2 haiku ] [ T2: 3 sonnet ]   │    │
│  │             [ T3: 1 opus  ]                     │    │
│  └──────────────────────────────────────────────────┘   │
│  GateBar: ○ compile  ○ test  ○ clippy                   │
└─────────────────────────────────────────────────────────┘
```

Plan summary, routing grid, and `<GateBar>` all flow vertically. `<MetricGrid columns={3}>` for the tier distribution.

**Phase 4-5 -- Tasks Running:**
```
┌─────────────────────────────────────────────────────────┐
│  ● idea  ● prd  ● plan  ● tasks  ◉ run   ○ verify      │
├─────────────────────────────────────────────────────────┤
│  ┌─ TASK BOARD ──────────────────────────────── 1/6 ─┐  │
│  │ <DataTable>                                       │  │
│  │ ● DONE   Define CLI contract     T1·haiku  1.2s  │  │
│  │          GateBar: ✓ compile  ✓ test               │  │
│  │ ◉ RUN    Implement DeFi fetcher  T2·sonnet ...   │  │
│  │          GateBar: ◉ compile  ○ test               │  │
│  │ ○ PEND   Add email notification  T2·sonnet       │  │
│  │ ○ PEND   Wire configuration      T1·haiku        │  │
│  │ ○ PEND   Integration tests       T3·opus         │  │
│  │ ○ PEND   Final verification      T1·haiku        │  │
│  │ </DataTable>                                      │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌─ TERMINAL ────────────────────────────────────────┐  │
│  │ <ScrollArea maxHeight="400px">                    │  │
│  │   $ roko plan run ...                             │  │
│  │   → T2 implementing...                            │  │
│  │   [streaming output]                              │  │
│  │ </ScrollArea>                                     │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌─ MetricGrid ─────────────────────────────────────┐   │
│  │ Cost: $0.024   Time: 12.4s   Pass: 100% (6/6)   │   │
│  │ (vs $0.18)     (3.1s avg)                        │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

Task board uses `<DataTable>` with `<StatusBadge>` and inline `<GateBar>` per row. Terminal is a `<Panel>` containing `<ScrollArea maxHeight="400px">` -- it scrolls internally but the page also scrolls to show the full task board + terminal + metrics. `<MetricGrid>` at the bottom uses `<AnimatedNumber>` for live-updating values.

**Phase 6 -- Complete:**
```
┌─────────────────────────────────────────────────────────┐
│  ● idea  ● prd  ● plan  ● tasks  ● run   ● verify      │
├─────────────────────────────────────────────────────────┤
│  ✓  All 6 tasks completed                               │
│  $0.042 total · 18.3s · 6/6 gates passed                │
│                                                         │
│  MetricGrid (6 cells, compact):                         │
│  ┌──────┬──────┬──────┬──────┬──────┬──────┐           │
│  │ COST │TOKENS│ TIME │GATES │MODEL │TASKS │           │
│  │$0.042│18.2K │18.3s │ 6/6  │3 tier│ 6/6  │           │
│  └──────┴──────┴──────┴──────┴──────┴──────┘           │
│                                                         │
│  TASK SUMMARY (DataTable):                              │
│  ✓ Define CLI contract     T1 haiku   $0.003   1.2s    │
│  ✓ Implement DeFi fetcher  T2 sonnet  $0.017   3.4s    │
│  ...                                                    │
└─────────────────────────────────────────────────────────┘
```

Summary `<MetricGrid columns={6}>` with `<AnimatedNumber>` values. Task summary as `<DataTable>` with `<StatusBadge>` and inline costs.

### 1.3 Density Notes

- Scenario cards: `<Grid columns="auto" minWidth="240px" gap="sm">`. Cards show complexity, task count, tier count, estimated cost -- no empty space.
- Phase content: panels stack with `--gap-md` (16px) between them. No extra breathing room.
- Task board rows: compact `<DataTable>` with 32px row height, inline `<GateBar>` per row (no expanding needed for gate status).
- Terminal pane: `max-height: 400px`, internal `<ScrollArea>`. Auto-scrolls to bottom during streaming.
- Metrics: `<MetricGrid>` 1px-gap mosaic style. Values use `<AnimatedNumber>` with value-flash on change.
- Loading: task board shows 4 `<Skeleton>` rows. Terminal shows "Waiting for agent output..." in `--text-ghost`. Metrics show shimmer cells.
- Empty (Phase 0): scenario cards visible, `<PhaseRail>` all pending, "Select a scenario and press Start" as helper text.

### 1.4 Data Sources

- `POST /api/workspaces` -- create workspace before run
- Workflow SSE: `/api/workflows/latest/stream?root=...` -- real-time execution state
- Workflow WS: `/api/workflow/ws` -- parallel redundant stream
- PTY WS: `/ws/terminal/:sessionId` -- terminal output

### 1.5 Playback Controller

- Speed: 0.5x / 1x / 2x / 4x
- Pause/resume at step boundaries
- Keyboard: Space=play/pause, N=next, R=reset, 1-3=scenario

### 1.6 Scenario Configuration

```typescript
interface ScenarioConfig {
  id: string;
  label: string;
  description: string;  // Fraunces 30px display
  complexity: 'simple' | 'medium' | 'complex';
  tasks: number;
  tiers: number;
  estimatedCost: string;
  estimatedTime: string;
  prompt: string;       // The actual roko command
}
```

Scenarios are config objects in a registry file. Adding a new scenario = adding an object. No editing page components.

---

## 2. Observe — "The Control Plane"

**Hero line:** *"The system behind the system. Agents, routing, knowledge -- all live."*

### 2.1 Scroll Structure

```
<PageShell title="Observe">
  AsciiLabel: "OBSERVE" + hero line
  MetricGrid: health summary (compact, auto-height)
  Tabs: Status · Fleet · Knowledge · Routing · Dreams
  Tab content (flows in document order, scrolls naturally)
</PageShell>
```

The `<Tabs variant="line">` bar is NOT sticky. It flows with the page content. Tab content sections grow to fit their data.

### 2.2 Tab Layout

```
OBSERVE    Status · Fleet · Knowledge · Routing · Dreams
```

**Status tab (default):**
- Health `<MetricGrid columns={4}>`: status, uptime, version, C-factor, total cost, episode count -- 1px-gap mosaic
- C-factor breakdown: 5 bar charts in a `<Grid columns="auto" minWidth="160px">` (gate pass rate, cost efficiency, speed, reuse, learning)
- Provider health: `<StatusBadge>` dots for each configured provider, arranged in a `<Stack direction="horizontal" gap="sm">`
- Recent episodes: `<DataTable>` with expandable rows, auto-refresh 10s
- Recent events: `<ScrollArea maxHeight="300px">` live event stream from SSE

**Fleet tab:**
- Agent cards: `<Grid columns="auto" minWidth="280px">` of `<Panel>` cards -- name (`<AsciiLabel>`), role, tier, task count, cost, `<StatusBadge>` activity indicator
- Topology graph: `<AsciiFrame variant="single" title="TOPOLOGY">` containing force-directed canvas, 20px+ nodes, labeled edges, energy-based stop
- Click agent -> detail overlay with episode history in `<ScrollArea>`

**Knowledge tab:**
- Graph mode: force-directed graph within `<AsciiFrame>`, nodes sized by citations, colored by topic
- List mode: `<DataTable>` with searchable `<Input variant="search">`, filterable timeline with excerpts
- Toggle between modes via `<Tabs variant="pill">`

**Routing tab:**
- Stats `<MetricGrid columns={3}>`: total decisions, unique models, cost savings
- Distribution chart: by count vs by cost toggle
- Decision `<DataTable>`: model, pass rate, cost/task, best-for description

**Dreams tab:**
- Dream cycle `<StepProgress>`: phases (hypnagogia -> imagination -> consolidation)
- Journal entries: timestamped list in `<ScrollArea maxHeight="400px">`
- Archive: cold storage entries

### 2.3 Density Notes

- Health mosaic: `<MetricGrid columns={4}>` 1px-gap, 6 cells. Compact `--cell-padding`. Values are `<AnimatedNumber>`.
- C-factor bars: inline with health section, same visual weight. Not a separate "section" with its own header -- it continues below the mosaic.
- Provider health: one horizontal row of `<StatusBadge>` dots, no extra wrapping panel.
- Episodes table: `<DataTable>` with 32px rows, `<StatusBadge>` inline, expandable detail uses `<ScrollArea>` with `maxHeight="200px"`.
- Fleet cards: auto-fill grid, tight gaps (`--cell-gap`). Cards are compact: name, role, 2-3 inline metrics, LED dot. No tall cards.
- Loading: mosaic shows shimmer cells. Tables show 5 skeleton rows. Graph shows `<AsciiBraille pattern="noise">` placeholder.
- Empty: "No agents registered. Start one with `roko agent start --name X`."

### 2.4 Data Sources

- `GET /api/health`, `/api/metrics/c_factor`
- `GET /api/managed-agents`, `/api/agents/topology`
- `GET /api/knowledge/entries`, `/api/knowledge/edges`
- `GET /api/learn/cascade-router`
- `GET /api/episodes`
- `GET /api/providers/health`
- `GET /api/dreams/report`, `/api/dreams/journal`
- SSE `/api/events` -- live event stream

---

## 3. Evaluate — "The Evidence"

**Hero line:** *"Not just working -- economically superior."*

### 3.1 Scroll Structure

```
<PageShell title="Evaluate">
  AsciiLabel: "EVALUATE" + hero line
  MetricGrid: quick stats (compact, auto-height)
  Run list + detail (stacked, not side-by-side split)
  Matrix builder section
</PageShell>
```

The entire page scrolls. Run detail appears below (or replaces) the run list when a row is selected, using a shared-element transition -- not a viewport-locked side panel.

### 3.2 Layout

```
┌─────────────────────────────────────────────────────────┐
│  AsciiLabel: "EVALUATE"                                 │
│  MetricGrid: runs | best model | cost/task | pass rate  │
├─────────────────────────────────────────────────────────┤
│  Run List (DataTable, sortable, full-width)             │
│  Click row -> detail expands below                      │
├─────────────────────────────────────────────────────────┤
│  Run Detail / Pareto / Compare                          │
│  (context-dependent, full-width Panel)                  │
├─────────────────────────────────────────────────────────┤
│  Matrix Builder (AsciiFrame, collapsible)               │
│  Configure + live progress + race track                 │
└─────────────────────────────────────────────────────────┘
```

**Quick stats:** `<MetricGrid columns={4}>` -- runs count, best model, avg cost/task, overall pass rate. 1px-gap mosaic.

**Run list:**
- `<DataTable>` full-width: date, suite, model, pass rate, cost, duration
- `<StatusBadge>` per row: queued/running/complete/failed
- Click row -> detail section appears below with shared-element transition

**Run detail** (appears below selected row):
1. **Single run**: task breakdown, gate waterfall (`<GateBar>` per task), cost per task, model distribution in `<MetricGrid>`
2. **Pareto chart**: cost vs quality scatter within `<AsciiFrame>` (interactive points)
3. **Comparison**: side-by-side two runs with diff highlighting, using `<Grid columns={2}>`

**Matrix builder** (bottom section):
- Collapsible `<Panel>` with `<AsciiLabel>` header
- Configure multi-model evaluation
- Live progress via SSE during execution
- Race track visualization for concurrent lanes in `<ScrollArea maxHeight="300px">`

### 3.3 Density Notes

- Quick stats: same 1px-gap mosaic treatment as other pages. 4 cells, compact.
- Run list: `<DataTable>` with 36px rows, inline `<StatusBadge>`, sortable column headers. No wide padding.
- Run detail: appears in place (not in a side panel). Uses `<Panel>` with `<MetricGrid>` for task-level cost breakdown, `<GateBar>` per task row, `<DataTable>` for task list.
- Matrix builder: collapsed by default. When open, race track lanes use `<Grid columns="auto" minWidth="200px">` for model lanes, each lane a `<Panel>` with progress bar.
- Loading: run list shows 5 skeleton rows. Quick stats show shimmer. Detail area shows `<Skeleton>` matching layout.
- Empty: "No benchmark runs yet. Run `roko bench start` to create one. API: `POST /api/bench/start`."

### 3.4 Data Sources

- `GET /api/bench/runs`, `/api/bench/runs/:id`
- `GET /api/bench/compare`, `/api/bench/pareto`
- `GET /api/bench/cost-summary`
- `POST /api/bench/start`, `POST /api/bench/matrix/start`
- SSE `/api/bench/events` -- live bench stream

---

## 4. Build — "Try It Yourself"

**Hero line:** *"Type what you want built. Watch it happen."*

### 4.1 Scroll Structure

```
<PageShell title="Build">
  AsciiLabel: "BUILD" + hero line
  Model selector chips + config toggle
  Chat thread (ScrollArea, max-height bounded)
  Terminal pane (ScrollArea, max-height bounded, toggleable)
  Input bar + preset prompts
</PageShell>
```

The page scrolls to show all sections. Chat thread and terminal pane each have internal scroll via `<ScrollArea>` with bounded `max-height`. Neither is viewport-locked.

### 4.2 Layout

```
┌─────────────────────────────────────────────────────────┐
│  Model selector: [haiku] [sonnet] [opus]  ⚙ Config     │
│  AsciiLabel: roko run "prompt" --model haiku            │
├─────────────────────────────────────────────────────────┤
│  ┌─ CHAT ────────────────────────────────────────────┐  │
│  │ <ScrollArea maxHeight="500px" fadeEdges>           │  │
│  │   user: "Build a CLI calculator"                  │  │
│  │   assistant: [streaming...] █                     │  │
│  │   tool: [expandable block]                        │  │
│  │ </ScrollArea>                                     │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌─ TERMINAL (toggle: T) ────────────────────────────┐  │
│  │ <ScrollArea maxHeight="400px" fadeEdges>           │  │
│  │   $ cd /workspace/...                             │  │
│  │   [pty output]                                    │  │
│  │ </ScrollArea>                                     │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌─ INPUT ───────────────────────────────────────────┐  │
│  │ [auto-resize textarea]                [Send]      │  │
│  │ Presets: [CLI calc] [Auth] [Fix CI]               │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**Chat thread:**
- `<Panel>` containing `<ScrollArea maxHeight="500px" fadeEdges>` -- internal scroll, auto-scrolls to latest message
- Messages with role `<StatusBadge>` (user / assistant / tool)
- Streaming text with blinking cursor
- Tool call blocks: expandable/collapsible with syntax highlighting inside `<AsciiFrame>`
- Code blocks: mono font with copy button

**Terminal pane:**
- Toggle with keyboard shortcut (T)
- `<Panel>` containing `<ScrollArea maxHeight="400px" fadeEdges>`
- Auto-`cd` into workspace when opened
- Real PTY output visible alongside chat
- Hidden by default -- when shown, it flows below the chat panel in the page scroll

**Preset prompts:**
- Quick-start chips below input: "Build a CLI calculator", "Add auth to Express app", "Fix CI pipeline"
- Click -> fills input, does not auto-send

**Model selector:**
- Chips for available models, active one highlighted with `--border-active`
- Shows cost indicator per model
- CLI equivalent shown below in `<AsciiLabel>`: `roko run "prompt" --model haiku`

### 4.3 Density Notes

- Model selector: horizontal chip row, `--gap-sm` between chips. Config toggle is inline, not a separate row.
- Chat messages: compact. Role badge + timestamp on one line, content below. Tool call blocks collapsed by default (just show tool name + status).
- Terminal: hidden by default. When shown, tight `--gap-md` between chat and terminal panels.
- Input bar: fixed to bottom of the page scroll container (not viewport-fixed). Auto-resize up to 120px.
- Loading: chat area shows `<EmptyState>` "Type a prompt to begin, or select a preset below."
- Empty: preset chips visible, model selector loaded, terminal hidden. No blank space.

### 4.4 Data Sources

- `POST /api/workspace` -- create workspace
- `POST /api/run` -- execute prompt (SSE response)
- `GET /api/config/models` -- available models
- PTY WS: `/ws/terminal/:sessionId`

---

## 5. Knowledge — "The Memory"

**Hero line:** *"Everything the system has learned -- browseable, searchable, alive."*

### 5.1 Scroll Structure

```
<PageShell title="Knowledge">
  AsciiLabel: "KNOWLEDGE" + hero line
  MetricGrid: entry count, edge count, avg tier, top topic
  Tabs (pill): Graph · List
  Tab content (graph or list, flows in page)
  Dream cycles section
</PageShell>
```

Page scrolls naturally. Graph mode constrains the canvas to a `max-height` with internal pan/zoom. List mode is a `<DataTable>` that grows with entries.

### 5.2 Layout: Two Modes

**Graph mode:**
- `<AsciiFrame variant="single" title="KNOWLEDGE GRAPH">` containing force-directed canvas
- Canvas has `max-height: 600px` with internal pan/zoom -- does not expand to fill viewport
- Nodes sized by citation count (8 + citations * 1.5 radius)
- Clusters colored by topic
- Hover: tooltip with topic + excerpt
- Click: shared element transition to detail `<Panel>` below the graph

**List mode:**
- `<Input variant="search">` at top
- Filter row: `<Stack direction="horizontal">` of chips for tier, topic, date
- `<DataTable>` with entries: topic (`<AsciiLabel>`), excerpt, citation count, tier `<StatusBadge>`, date
- Expandable rows for full content in `<ScrollArea maxHeight="200px">`

**Detail panel** (appears below graph/list on click):
- `<Panel>` with: title, tier `<StatusBadge>`, citation count
- Full content
- Related entries: `<Grid columns="auto" minWidth="200px">` of linked `<Panel>` cards
- Usage history: `<DataTable>` of which plans/tasks referenced this entry

### 5.3 Dream Cycles

Integrated below the main knowledge content:

- `<AsciiDivider variant="line" label="DREAMS" />` separator
- Dream cycle `<StepProgress>`: phases (hypnagogia -> imagination -> consolidation)
- Journal entries: `<ScrollArea maxHeight="300px">` with timestamped entries
- Archive: expandable cold storage entries

### 5.4 Density Notes

- Graph: canvas fills available width, bounded height (600px max). Controls (zoom, filter) are an overlay strip at the top of the canvas, not a separate section.
- List: `<DataTable>` with 36px rows, inline badges. Search + filters share one row above the table.
- Detail panel: compact. Title + metadata on one line, content below, related entries in tight grid.
- Dream section: compact sub-section, not a separate page-height block. Step indicators + journal in a single `<Panel>`.
- Loading: graph shows `<AsciiBraille pattern="noise">` fill. List shows 5 skeleton rows. Metrics show shimmer.
- Empty: "No knowledge entries yet. Run a plan to generate knowledge, or use `roko knowledge query` to search."

### 5.5 Data Sources

- `GET /api/knowledge/entries`, `/api/knowledge/edges`, `/api/knowledge/stats`
- `GET /api/knowledge/graph`
- `POST /api/knowledge/query` -- search
- `GET /api/dreams/report`, `/api/dreams/journal`
- `POST /api/dreams/run` -- trigger cycle

---

## 6. Cross-Page Patterns

### 6.1 Every Page Gets for Free

| Concern | How |
|---------|-----|
| Page wrapper | `<PageShell>` -- entrance animation, document title, error boundary |
| Config (models, providers) | DataHub slice, auto-fetched |
| Workspace | DataHub ensureWorkspace() |
| Server health | `<StatusPill>` in TopNav |
| Loading states | `<DataSurface>` wrapper per data section |
| Empty states | `<EmptyState>` with message + action + hint |
| Error states | `<DataSurface>` error with retry button |
| Status colors | CSS tokens + `<StatusBadge>` |
| Page transitions | React Router View Transitions |
| Live metrics | `<AnimatedNumber>` + `<MetricGrid>` |
| SSE/WS events | DataHub transport layer |
| Gate status | `<GateBar>` -- consistent everywhere |
| Step progress | `<PhaseRail>` / `<StepProgress>` |
| Mono labels | `<AsciiLabel>` -- uppercase mono with frame variants |
| Scrollable regions | `<ScrollArea>` -- thin rose scrollbar, fade edges |

### 6.2 Navigation Transitions

- **Horizontal** (scene <-> scene): Slide left/right based on direction
- **Vertical** (list -> detail): Shared element transition (`<Panel>` morphs into detail `<Panel>`)
- **Tab** (within scene): Fade + content slide in tab direction

### 6.3 Keyboard Shortcuts

```
SPACE       Play / Pause (Orchestrate)
N           Next step (Orchestrate, paused)
R           Reset scenario (Orchestrate)
1/2/3       Select scenario (Orchestrate)
T           Toggle terminal (Build)
Cmd+K       Command palette (anywhere)
D           Toggle debug panel
?           Show help overlay
```
