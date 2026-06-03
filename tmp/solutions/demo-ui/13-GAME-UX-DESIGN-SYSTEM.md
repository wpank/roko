# 13. Game-Like UX Design System

A design document for making the Roko demo app feel like a **living, breathing control surface** — fluid motion, cohesive patterns, extensible component architecture, and deep integration with every roko endpoint.

**Goal:** Every demo scenario reuses the same primitives. State transitions animate. Data flows are visible. The experience feels like piloting a system, not reading a dashboard.

---

## Part 1: Roko Endpoint Inventory

Every UI surface must be backed by real data. Here's the full roko-serve surface area available at `:6677`.

### 1.1 Core Orchestration (the main loop)

| Endpoint | Method | What it gives the UI |
|---|---|---|
| `/api/plans` | GET | All plans with status, tasks, progress |
| `/api/plans/:id` | GET | Single plan detail |
| `/api/plans` | POST | Create plan (returns id) |
| `/api/plans/:id/run` | POST | Execute plan → SSE stream of events |
| `/api/plans/:id/tasks` | GET | Tasks for a plan |
| `/api/plans/:id/tasks/:tid` | GET | Single task detail |
| `/api/plans/generate` | POST | Generate plan from prompt/PRD |
| `/api/plans/validate` | POST | Lint tasks without executing |
| `/api/prd` | GET | All PRDs |
| `/api/prd/:slug` | GET | Single PRD |
| `/api/prd/idea` | POST | Create idea |
| `/api/prd/draft` | POST | Create/edit draft |
| `/api/prd/:slug/promote` | POST | Promote draft → published |
| `/api/prd/:slug/plan` | POST | Generate plan from PRD |
| `/api/prd/status` | GET | PRD coverage report |
| `/api/prd/consolidate` | POST | Scan for gaps/duplicates |

### 1.2 Agents

| Endpoint | Method | What it gives the UI |
|---|---|---|
| `/api/agents` | GET | All agents with status |
| `/api/agents/:name` | GET | Agent detail + health |
| `/api/agents` | POST | Create agent |
| `/api/agents/:name/start` | POST | Start agent |
| `/api/agents/:name/stop` | POST | Stop agent |
| `/api/agents/:name/chat` | POST | Send message → response |
| `/api/agents/:name/episodes` | GET | Agent's episode history |

### 1.3 Bench (evaluation)

| Endpoint | Method | What it gives the UI |
|---|---|---|
| `/api/bench/runs` | GET | All bench runs |
| `/api/bench/runs/:id` | GET | Single run detail |
| `/api/bench/start` | POST | Start bench → SSE stream |
| `/api/bench/compare` | GET | Compare two runs |
| `/api/bench/matrix/start` | POST | Start matrix evaluation |
| `/api/bench/matrix/:id` | GET | Matrix status + lanes |
| `/api/bench/pareto` | GET | Pareto frontier data |
| `/api/bench/cost-summary` | GET | Cost/token aggregates by model |
| `/api/bench/events` | SSE | Live bench event stream |

### 1.4 Knowledge & Learning

| Endpoint | Method | What it gives the UI |
|---|---|---|
| `/api/knowledge/query` | POST | Search knowledge store |
| `/api/knowledge/entries` | GET | All knowledge entries |
| `/api/knowledge/stats` | GET | Store statistics |
| `/api/knowledge/graph` | GET | Knowledge graph (nodes + edges) |
| `/api/dreams/run` | POST | Trigger dream cycle |
| `/api/dreams/report` | GET | Dream cycle results |
| `/api/dreams/journal` | GET | Dream journal entries |
| `/api/episodes` | GET | Episode log |
| `/api/episodes/:hash` | GET | Single episode |
| `/api/learn/router` | GET | Cascade router state |
| `/api/learn/experiments` | GET | A/B experiment results |
| `/api/learn/efficiency` | GET | Efficiency metrics |
| `/api/learn/gates` | GET | Adaptive gate thresholds |
| `/api/custody/list` | GET | Custody chain entries |
| `/api/custody/:id/verify` | POST | Verify custody chain |

### 1.5 Config & System

| Endpoint | Method | What it gives the UI |
|---|---|---|
| `/api/config` | GET | Full roko.toml config (models, providers, settings) |
| `/api/config/models` | GET | Available models + routing |
| `/api/config/providers` | GET | Provider health |
| `/api/status` | GET | System status |
| `/api/signals` | GET | Signal log |
| `/api/workspace` | POST | Create workspace |
| `/api/workspace/:id` | GET | Workspace info |

### 1.6 Streaming

| Endpoint | Transport | What it gives the UI |
|---|---|---|
| `/api/plans/:id/run` | SSE | Plan execution events (phase changes, task starts/completions, gate results, errors) |
| `/api/bench/events` | SSE | Bench execution events |
| `/api/stream` | WebSocket | General event stream |
| `/api/agents/:name/stream` | WebSocket | Per-agent event stream |
| `/api/terminal` | WebSocket | PTY terminal I/O |

### 1.7 Deployment & Jobs

| Endpoint | Method | What it gives the UI |
|---|---|---|
| `/api/deploy/:target` | POST | Deploy to railway/fly/docker |
| `/api/deploy/status` | GET | Deployment status |
| `/api/jobs` | GET | Marketplace jobs |
| `/api/jobs/:id` | GET | Job detail |
| `/api/jobs` | POST | Create job |
| `/api/jobs/:id/execute` | POST | Execute job |

---

## Part 2: Current Demo App — What Exists

### 2.1 Scenarios (13 current)

| Scenario | What it shows | Components used | API dependency |
|---|---|---|---|
| prd-pipeline | PRD → plan → tasks → execute | PrdPipelinePanel, terminals | `/api/prd/*`, `/api/plans/*` |
| knowledge | Knowledge store browsing | KnowledgeEntries, KnowledgeGraph | `/api/knowledge/*` |
| dreams | Dream consolidation | DreamsView | `/api/dreams/*` |
| fleet | Agent fleet topology | AgentFleet | `/api/agents` |
| cascade | Model routing visualization | CascadeRouter | `/api/learn/router` |
| cost | Cost dashboard | CostDashboard, CostRace | `/api/bench/cost-summary` |
| integrity | Episode integrity | IntegrityView | `/api/episodes` |
| bench | Benchmark runs | Bench, BenchRunDetail | `/api/bench/*` |
| compare | Bench comparison | BenchCompare | `/api/bench/compare` |
| explore | Activity explorer | Explorer | `/api/signals`, `/api/episodes` |
| build | Prompt builder | Builder | `/api/config`, `/api/agents/*/chat` |
| settings | Configuration | Settings | `/api/config` |
| share | Share receipt | Share | hash-based |

### 2.2 Component Inventory (56 files)

**Pages (18):** Demo, Bench, BenchCompare, BenchRunDetail, Builder, Explorer, Settings, Share + dashboard sub-pages (AgentFleet, CascadeRouter, CostDashboard, DreamsView, IntegrityView, KnowledgeEntries, KnowledgeGraph)

**Components (14):** AppShell, TopNav, PrdPipelinePanel, WorkflowConstellation, DreamPhaseViz, GateBar, GateWaterfall, CostRace, TerminalPaneWithHandle, ParticleField, StatusPill, ScenarioSelector

**Hooks (19):** useAgents, useBench, useDashboard, useKnowledge, useLiveApi, useRokoConfig, useServerHealth, useSSE, useTerminal, useTerminalSession, + others

### 2.3 Problems with Current Architecture

1. **No shared data layer.** Each hook fetches independently. No coordination, no caching, no optimistic updates.
2. **No shared animation system.** Each component rolls its own transitions (CSS keyframes, inline styles, requestAnimationFrame loops).
3. **No shared state machine.** Pipeline has its own phase model. Bench has its own. Nothing shares the concept of "running → complete → failed."
4. **No shared layout primitives.** Every page builds its own grid/flex layout from scratch.
5. **Scenarios are hardcoded in Demo.tsx.** Adding a new scenario means editing a 800-line file.
6. **Terminal is tightly coupled.** useTerminal manages xterm + PTY + addons + resize + cleanup in one hook. Can't reuse terminal without the whole stack.
7. **No component composition.** PrdPipelinePanel is 427 lines. It should be 6+ composable pieces.

---

## Part 3: Extensible Component Architecture

### 3.1 Design Principle: Everything is a Cell

Roko's architecture says "everything is a Graph of Cells." The UI should mirror this. Every piece of data that flows through roko can be represented as a **Cell** — a small, self-contained visual unit with:

- **Identity** (what is this?)
- **Status** (what state is it in?)
- **Content** (what does it contain?)
- **Actions** (what can I do to it?)
- **Connections** (what does it relate to?)

### 3.2 Universal Cell Components

```
<Cell>                  — base container with status ring, identity badge
<CellGrid>             — responsive grid/masonry of cells
<CellDetail>           — expanded detail view (shared element transition from cell)
<CellTimeline>         — chronological list of cells (episodes, events, tasks)
<CellGraph>            — force-directed graph of connected cells
<CellBoard>            — kanban-style board (tasks by status column)
```

Every roko entity maps to a Cell:

| Entity | Identity | Status states | Primary view |
|---|---|---|---|
| PRD | slug + title | idea → draft → published → planned | CellTimeline |
| Plan | id + title | pending → active → complete → failed | CellBoard |
| Task | plan:task id | pending → active → done → failed → blocked | CellBoard |
| Agent | name + domain | idle → running → stopped → error | CellGrid |
| Episode | hash | recorded → verified | CellTimeline |
| Knowledge | topic + hash | raw → distilled → promoted | CellGrid |
| Dream | cycle id | pending → running → complete | CellTimeline |
| Gate | name + rung | pending → passed → failed | inline status |
| Bench Run | id | queued → running → complete → failed | CellGrid |
| Signal | hash | received → scored → routed | CellTimeline |

### 3.3 Layout Primitives

```
<SplitView>            — resizable two-pane layout (list + detail)
<PhaseRail>            — horizontal step indicator with animation
<Sidebar>              — collapsible navigation/context panel
<CommandBar>           — top command palette (⌘K)
<StreamOverlay>        — floating SSE/WS connection status
<MetricStrip>          — horizontal strip of live-updating metrics
<TerminalEmbed>        — self-contained terminal with controls
```

### 3.4 Scenario as Configuration

Instead of hardcoding scenarios in Demo.tsx, each scenario is a config object:

```typescript
interface ScenarioConfig {
  id: string;
  label: string;
  icon: string;
  description: string;

  // What data sources does this scenario need?
  dataSources: DataSource[];

  // What layout does this scenario use?
  layout: 'split' | 'full' | 'grid' | 'timeline';

  // What cell types does this scenario display?
  cellTypes: CellType[];

  // What actions are available?
  actions: ScenarioAction[];

  // What terminals does this scenario need?
  terminals: TerminalConfig[];

  // Streaming connections
  streams: StreamConfig[];
}
```

Adding a new scenario = adding a config object + optional custom cell renderer. No more editing Demo.tsx.

---

## Part 4: Motion & Animation System

### 4.1 Library: Motion (framer-motion successor)

**Why Motion:** It's the only React animation library that handles layout animations, shared element transitions, gesture interactions, and spring physics in a single coherent API. It's also the successor to framer-motion (same author, Brandon Chen), with better tree-shaking and React 19 support.

```bash
npm install motion
```

### 4.2 Animation Layers

The app has 4 animation layers, from ambient to interactive:

#### Layer 1: Ambient (always running, low-energy)

These create the feeling of a living system. They run continuously but are computationally cheap.

| Effect | Implementation | Where |
|---|---|---|
| Pulse dot on live connections | CSS `@keyframes pulse` | StreamOverlay, StatusPill |
| Breathing glow on active tasks | CSS `animation: breathe 3s ease-in-out infinite` | CellGrid active cells |
| Floating particles (existing) | Canvas/Three.js (already exists) | Background |
| Gradient shift on phase changes | CSS `transition: background 1.5s var(--ease)` | PhaseRail segments |
| Counter tick-up | Motion `useSpring` | MetricStrip numbers |

#### Layer 2: Data-Driven (triggered by SSE/WS events)

These fire when new data arrives. They make the system feel reactive.

| Trigger | Animation | Implementation |
|---|---|---|
| New task starts | Task cell slides in from right, pulses once | `motion.div` with `initial={{ x: 40, opacity: 0 }}` |
| Task completes | Green check scales up, progress bar animates | `animate={{ scale: [0, 1.2, 1] }}` + `useSpring` for bar |
| Gate passes | Green flash ripple on gate bar | CSS `@keyframes gate-pass-ripple` |
| Gate fails | Red shake on task cell, failure card slides in | `animate={{ x: [-4, 4, -4, 0] }}` |
| Phase advances | Phase rail dot slides to next position | `layoutId="phase-indicator"` (shared element) |
| New agent spawned | Agent node fades in with expanding ring | `animate={{ scale: [0, 1], opacity: [0, 1] }}` |
| Plan complete | Confetti-lite (small burst of dots) | Canvas overlay, 30 particles, 800ms |
| Metrics update | Number springs to new value | `useSpring({ value: newVal })` |
| Error | Red border flash + shake | `animate={{ borderColor: ['#ff4444', 'transparent'] }}` |

#### Layer 3: Interaction (user-initiated)

These respond to user actions. They make the UI feel responsive and tactile.

| Action | Animation | Implementation |
|---|---|---|
| Click cell → detail | Cell expands to detail view (shared element) | `layoutId={cellId}` on both views |
| Switch scenario | Cross-fade with slide direction | `AnimatePresence` + `motion.div` with exit/enter |
| Hover cell | Subtle lift + glow | `whileHover={{ y: -2, boxShadow: '0 4px 20px rgba(...)' }}` |
| Press button | Scale down + release | `whileTap={{ scale: 0.97 }}` |
| Drag to reorder | Smooth reorder with layout animation | `layout` prop on `motion.div` + `Reorder.Group` |
| Expand section | Height animation with spring | `animate={{ height: 'auto' }}` with `layout` |
| Tab switch | Content slides in direction of tab | `custom={direction}` on `AnimatePresence` variants |
| Scroll into view | Staggered fade-up | `whileInView` + stagger via `transition.delay` |

#### Layer 4: Transitions (navigation between views)

These handle page/view transitions. They maintain spatial context.

| Transition | Implementation |
|---|---|
| Page → Page | React Router `viewTransition` prop + `<ViewTransition>` (React 19 experimental) |
| List → Detail | Shared element via `layoutId` (cell morphs into detail panel) |
| Scenario → Scenario | Cross-fade with directional slide (left/right based on scenario index) |
| Modal open/close | `AnimatePresence` with backdrop fade + panel slide-up |

### 4.3 Motion Constants

```typescript
// motion-tokens.ts
export const spring = {
  gentle: { type: 'spring', stiffness: 120, damping: 20 },
  snappy: { type: 'spring', stiffness: 300, damping: 30 },
  bouncy: { type: 'spring', stiffness: 400, damping: 15 },
} as const;

export const duration = {
  instant: 0.1,
  fast: 0.2,
  normal: 0.35,
  slow: 0.6,
  ambient: 3.0,
} as const;

export const stagger = {
  fast: 0.03,
  normal: 0.05,
  slow: 0.08,
} as const;

export const fadeUp = {
  initial: { opacity: 0, y: 12 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -8 },
  transition: { duration: 0.35 },
};

export const scaleIn = {
  initial: { opacity: 0, scale: 0.9 },
  animate: { opacity: 1, scale: 1 },
  exit: { opacity: 0, scale: 0.95 },
  transition: { type: 'spring', stiffness: 300, damping: 25 },
};
```

### 4.4 Performance Budget

| Constraint | Limit |
|---|---|
| Concurrent CSS animations | ≤ 8 |
| Concurrent Motion springs | ≤ 12 |
| Canvas particles | ≤ 60 |
| Animation frame budget | ≤ 4ms per frame (leaves 12ms for React) |
| Three.js: vertices | ≤ 2000 (topology views only) |
| IntersectionObserver targets | ≤ 30 |

Use `will-change` sparingly (only on actively animating elements). Prefer `transform` and `opacity` (compositor-only properties). Never animate `width`, `height`, `top`, `left` directly — use `transform: scale()` and `transform: translate()`.

---

## Part 4B: Cross-Cutting Consistency — What Stays the Same Everywhere

This is the most important architectural constraint. Right now every page is a one-off — it fetches its own config, creates its own workspace, manages its own loading/error states, and renders status differently. The fix is making these **cross-cutting concerns** live in exactly one place, shared by every page.

### 4B.1 Configuration: One Source, Every Page

**Problem today:** `useRokoConfig()` is called independently in 6 places (Demo, Builder, Bench, Settings, ConfigWidget, AppShell). Each one fetches `/api/config` on mount. Change a model in Settings → Builder doesn't know. Switch providers → Bench still uses the old one.

**Fix:** Config lives in DataHub. Fetched once on app boot. Updated via a single `updateConfig()` action that writes to the server AND updates the local store. Every page reads from the same slice:

```typescript
// Any page, any component — same config, always in sync
const models = useDataHub(s => s.config?.models);
const defaultModel = useDataHub(s => s.config?.defaultModel);
const providers = useDataHub(s => s.config?.providers);
```

When Settings changes a model, DataHub updates → every page that reads `config` re-renders with the new value. No prop drilling, no re-fetching, no stale copies.

**Visual consistency:** Config-derived UI (model selectors, provider badges, tier labels) uses the same component everywhere:

```
<ModelChip model="claude-sonnet-4" />     — same look in Builder, Bench, Orchestrate, CascadeRouter
<ProviderBadge provider="anthropic" />    — same look everywhere providers appear
<TierBadge tier="T2" />                   — same look in task boards, routing views, bench results
```

### 4B.2 Workspace: One Context, Every Terminal

**Problem today:** Workspaces are created in 3 different ways:
1. Demo.tsx calls `POST /api/workspaces` via `useWorkspace` hook
2. Builder.tsx calls `POST /api/workspaces` with its own inline logic
3. Terminal.tsx creates workspaces via PTY shell commands (`mktemp`, `roko init`)

Each page manages its own workspace path. Navigate away → workspace reference is lost. Two pages can't share a workspace.

**Fix:** Active workspace is a DataHub concept:

```typescript
interface DataHub {
  // ...
  workspace: {
    id: string;
    path: string;
    status: 'creating' | 'ready' | 'error';
  } | null;

  ensureWorkspace: (prefix: string) => Promise<string>;  // idempotent
  activeWorkspacePath: string | null;  // derived
}
```

Every scene that needs a workspace calls `ensureWorkspace()` — if one exists with that prefix, reuse it. Terminals auto-`cd` into `activeWorkspacePath`. Navigate from Orchestrate → Build → back to Orchestrate → same workspace, same state.

**Visual consistency:** Workspace status is always visible in the app chrome:

```
┌──────────────────────────────────────────────────────────────┐
│ TopNav: [Constellation] [Orchestrate] [Observe] ...   ws: ● │
│                                          /tmp/roko-abc ready │
└──────────────────────────────────────────────────────────────┘
```

### 4B.3 Server Health: One Connection, Consistent Indicators

**Problem today:** `useServerHealth()` is called in multiple components. Connection status shows differently in different places (sometimes a dot, sometimes text, sometimes nothing).

**Fix:** Server health is in DataHub. The StreamOverlay component (always visible in AppShell chrome) shows connection status. Every page inherits this — no page needs its own health check.

```typescript
// Always available, always consistent
const status = useDataHub(s => s.serverStatus);  // 'connected' | 'checking' | 'disconnected'
const streams = useDataHub(s => s.streams);       // { sse: StreamState, ws: StreamState }
```

### 4B.4 Loading, Empty, and Error States: Same Pattern Everywhere

**Problem today:** Each page handles loading/empty/error differently. Some show a spinner, some show nothing, some crash on null data, some show stale synthetic data.

**Fix:** Three universal state components used by every data surface:

```
<DataSurface
  data={plans}
  loading={<Skeleton variant="list" count={3} />}
  empty={<EmptyState icon="plans" message="No plans yet" action={...} />}
  error={<ErrorState message={error} retry={refetch} />}
>
  {(plans) => <CellGrid items={plans} render={PlanCell} />}
</DataSurface>
```

Every data surface in the app uses `<DataSurface>`. Loading always looks the same (shimmer skeleton). Empty always looks the same (icon + message + optional action). Error always looks the same (message + retry button). No page invents its own pattern.

### 4B.5 Status Colors & Badges: One Visual Language

**Problem today:** "active" is teal in one place, green in another, blue in a third. "failed" is red here, rose there, pink elsewhere. Badge styles differ per component.

**Fix:** Status is a design token, not a per-component choice:

| Status | Color token | Dot | Badge | Ring |
|---|---|---|---|---|
| idle | `--status-idle` (gray) | ○ | `idle` | gray border |
| pending | `--status-idle` (gray) | ○ | `pending` | gray border |
| active | `--status-active` (teal) | ● pulsing | `working` | teal border + glow |
| done/success | `--status-success` (green) | ● | `done` | green border |
| failed/error | `--status-error` (rose) | ● | `failed` | rose border + glow |
| blocked | `--status-blocked` (purple) | ● | `blocked` | purple border |

One `<StatusBadge status="active" />` component used in task rows, agent cards, plan cards, bench runs, gate results — everywhere. Same colors, same animation, same semantics.

### 4B.6 Navigation & Spatial Model

**Problem today:** Scenarios are a flat list in Demo.tsx. Switching between them resets everything. There's no sense of where you are in the system.

**Fix:** Navigation has two axes:
- **Horizontal:** Scenes (Constellation → Orchestrate → Observe → Evaluate → Build → Knowledge)
- **Vertical:** Depth (list → detail → sub-detail)

Horizontal navigation uses **slide transitions** (left/right based on direction). Vertical navigation uses **shared element transitions** (cell morphs into detail view, back button reverses the morph).

Breadcrumbs always show where you are: `Orchestrate > Plan: auth-system > Task: implement-jwt`

The current scenario/scene persists in the URL. Refresh → same place. Share URL → same view.

### 4B.7 Summary: What Every Page Gets for Free

By using DataHub + Cell + Layout primitives, every new page/scene automatically gets:

| Concern | How it's handled | Pages that need to care |
|---|---|---|
| Config (models, providers) | DataHub slice, auto-fetched | None — it's just there |
| Workspace | DataHub slice, idempotent creation | None — ensureWorkspace() |
| Server health | DataHub slice, StreamOverlay in chrome | None — always visible |
| Loading states | `<DataSurface>` wrapper | None — use the wrapper |
| Empty states | `<EmptyState>` component | None — use the component |
| Error states | `<ErrorState>` component | None — use the component |
| Status colors | CSS tokens + `<StatusBadge>` | None — use the badge |
| Page transitions | React Router + View Transitions | None — router handles it |
| Cell rendering | `<CellGrid>` + entity renderers | Just pick the right renderer |
| Metrics display | `<MetricStrip>` + `<AnimatedNumber>` | Just pass the numbers |
| SSE/WS events | DataHub transport layer | None — subscribe to slices |

Adding a new scene should be ~100 lines of layout + DataHub selectors. Not 400+ lines of fetching, state management, error handling, and custom CSS.

---

## Part 5: Data Flow Architecture

### 5.1 Transport Layer

```
┌─────────────────────────────────────────────┐
│                  roko-serve :6677            │
│                                             │
│  REST ──── polling (config, status, lists)   │
│  SSE  ──── push (plan execution, bench)      │
│  WS   ──── bidirectional (terminal, stream)  │
└─────────┬───────────┬───────────┬───────────┘
          │           │           │
     ┌────▼────┐ ┌────▼────┐ ┌───▼────┐
     │ useLive │ │ useSSE  │ │ useWS  │
     │ Api     │ │         │ │        │
     └────┬────┘ └────┬────┘ └───┬────┘
          │           │          │
     ┌────▼───────────▼──────────▼────┐
     │         DataHub (Zustand)       │
     │  ─ plans[]    ─ agents[]       │
     │  ─ prds[]     ─ bench{}        │
     │  ─ episodes[] ─ knowledge{}    │
     │  ─ config{}   ─ streams{}      │
     │  ─ signals[]  ─ dreams{}       │
     └────────────────┬───────────────┘
                      │
          ┌───────────▼───────────┐
          │   React Components    │
          │   (subscribe to       │
          │    slices via hooks)   │
          └───────────────────────┘
```

### 5.2 DataHub (centralized state)

Replace scattered hooks with a single Zustand store. Each domain is a slice.

```typescript
interface DataHub {
  // Connection
  serverStatus: ServerStatus;
  streams: { sse: StreamState; ws: StreamState };

  // Domains
  plans: Plan[];
  prds: Prd[];
  agents: Agent[];
  episodes: Episode[];
  knowledge: KnowledgeEntry[];
  benchRuns: BenchRun[];
  config: RokoConfig | null;
  signals: Signal[];
  dreams: DreamReport[];
  routing: CascadeRouterState | null;
  experiments: Experiment[];
  efficiency: EfficiencyMetric[];
  gates: GateThreshold[];

  // Derived
  activePlan: Plan | null;
  activeAgents: Agent[];
  latestEpisode: Episode | null;

  // Actions
  fetchPlans: () => Promise<void>;
  fetchAgents: () => Promise<void>;
  // ... one action per domain

  // SSE handlers (called by transport layer)
  onPlanEvent: (event: PlanEvent) => void;
  onBenchEvent: (event: BenchEvent) => void;
  onAgentEvent: (event: AgentEvent) => void;
}
```

**Why Zustand:** Tiny (1.1kB), no boilerplate, supports subscriptions with selectors (only re-render when your slice changes), works with React 19 concurrent features, and has excellent devtools.

### 5.3 Hooks as Thin Selectors

```typescript
// Instead of useDashboard() that fetches + manages state:
const plans = useDataHub(s => s.plans);
const activePlan = useDataHub(s => s.activePlan);
const fetchPlans = useDataHub(s => s.fetchPlans);

// Or a convenience hook:
function usePlans() {
  const plans = useDataHub(s => s.plans);
  const fetch = useDataHub(s => s.fetchPlans);
  useEffect(() => { fetch(); }, [fetch]);
  return plans;
}
```

---

## Part 6: The Experience — Scene by Scene

### 6.1 Landing: The Constellation

When you open the app, you see a **constellation view** — a dark field with floating nodes representing roko's capabilities. Each node is a Cell. Nodes drift gently (ambient animation). Lines connect related nodes (plans → tasks → agents → gates).

- Click a node cluster → zooms into that scenario
- Hover a node → tooltip with name + status
- Active processes have pulsing nodes
- The constellation is a real-time view of roko's state (not decorative)

**Data source:** `/api/status` + `/api/plans` + `/api/agents`

### 6.2 Orchestrate: Watch Roko Build

The core demo. Shows a PRD being turned into verified code.

**Layout:** Three-column

```
┌──────────────────────────────────────────────────────┐
│ PhaseRail (horizontal, full-width)                   │
├────────────┬───────────────────────┬─────────────────┤
│            │                       │                 │
│  Context   │     Main Stage       │   Live Feed     │
│  sidebar   │                       │                 │
│            │  (adapts to phase)    │  (event log +   │
│  - PRD     │                       │   terminal)     │
│  - Plan    │  idea → PRD card      │                 │
│  - Tasks   │  draft → editor       │                 │
│  - Gates   │  plan → task board    │                 │
│  - Agent   │  run → agent view     │                 │
│    status  │  done → summary       │                 │
│            │                       │                 │
└────────────┴───────────────────────┴─────────────────┘
```

**Phase transitions:**
- Each phase advances with a **slide + fade** — the main stage content slides out left, new content slides in from right
- The PhaseRail dot animates to the next position using `layoutId`
- Context sidebar updates with a subtle highlight on the changed section
- Active tasks pulse. Completed tasks get a green check with scale-in animation.

**Key animations:**
1. PRD card appears → `fadeUp` with `spring.gentle`
2. Plan tasks populate → staggered `fadeUp` (each task 50ms apart)
3. Agent starts → agent avatar pulses, model badge appears
4. Gate runs → gate bar segments fill left-to-right
5. Gate passes → green ripple
6. Gate fails → red shake, failure detail slides in below
7. Phase completes → celebratory pulse on PhaseRail dot
8. All done → summary card scales in with metrics

### 6.3 Observe: The Control Plane

Real-time operational view of everything roko is doing.

**Layout:** Dashboard grid (responsive, 2-4 columns)

```
┌──────────────────────────────────────────────────────┐
│ MetricStrip: agents active | tasks running | gates   │
│              passed | episodes today | knowledge     │
├──────────────────────┬───────────────────────────────┤
│                      │                               │
│  Agent Fleet         │  Active Plan Board            │
│  (topology graph     │  (kanban: pending → active    │
│   with live status)  │   → done → failed)            │
│                      │                               │
├──────────────────────┼───────────────────────────────┤
│                      │                               │
│  Cascade Router      │  Gate Health                  │
│  (model routing      │  (rung pipeline with          │
│   decision tree)     │   adaptive thresholds)        │
│                      │                               │
├──────────────────────┴───────────────────────────────┤
│  Episode Timeline (horizontal scroll, last 50)       │
└──────────────────────────────────────────────────────┘
```

**Key animations:**
- MetricStrip numbers use `useSpring` — they spring to new values instead of jumping
- Agent nodes pulse when active, gray when idle, red-ring when errored
- New episodes appear at the right end of the timeline with slide-in
- Clicking any cell in the grid → shared element transition to detail view

### 6.4 Evaluate: The Evidence

Benchmark results, cost analysis, model comparisons.

**Layout:** Split view (list left, detail right)

```
┌──────────────────────────────────────────────────────┐
│ Quick Stats: total runs | best model | cost/task     │
├────────────────────┬─────────────────────────────────┤
│                    │                                 │
│  Run List          │  Run Detail                     │
│  (sortable table   │  (charts, task breakdown,       │
│   with status      │   model performance,            │
│   badges)          │   gate pass rates)              │
│                    │                                 │
│  ── or ──          │  ── or ──                       │
│                    │                                 │
│  Pareto Chart      │  Comparison View                │
│  (cost vs quality  │  (side-by-side two runs)        │
│   scatter)         │                                 │
│                    │                                 │
└────────────────────┴─────────────────────────────────┘
```

**Key interactions:**
- Click a run in the list → it morphs (shared element) into the detail panel
- Pareto chart points are interactive — hover shows model name + metrics, click opens detail
- Running benchmarks show live progress with SSE-driven updates
- Numbers animate when switching between runs

### 6.5 Build: Try It Yourself

Interactive prompt builder where you can talk to roko.

**Layout:** Chat-style with expandable panels

```
┌──────────────────────────────────────────────────────┐
│ Model selector (chips) + Config panel (collapsible)  │
├──────────────────────────────────────────────────────┤
│                                                      │
│  Chat thread                                         │
│  (messages with role badges, tool calls expandable,  │
│   streaming text with cursor animation)              │
│                                                      │
├──────────────────────────────────────────────────────┤
│ Input bar (auto-resize) + presets (quick prompts)    │
└──────────────────────────────────────────────────────┘
```

**Key animations:**
- Messages appear with `fadeUp` + stagger
- Streaming text uses a blinking cursor (CSS)
- Tool call blocks expand/collapse with `layout` animation
- Model chip selection has a sliding indicator (like iOS segmented control)

### 6.6 Knowledge: The Memory

Browse and search roko's durable knowledge store.

**Layout:** Two modes — Graph and List (toggle)

- **Graph mode:** Force-directed knowledge graph with clickable nodes. Nodes sized by citation count. Clusters colored by topic. Hover shows excerpt.
- **List mode:** Searchable, filterable timeline of knowledge entries with excerpts.

Shared element transition between graph node and list entry detail.

---

## Part 7: Design System Extension

### 7.1 Token Additions

Add these to `rosedust.css` alongside existing tokens:

```css
:root {
  /* Motion tokens */
  --motion-duration-instant: 100ms;
  --motion-duration-fast: 200ms;
  --motion-duration-normal: 350ms;
  --motion-duration-slow: 600ms;
  --motion-ease-default: cubic-bezier(0.4, 0, 0.2, 1);
  --motion-ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);
  --motion-ease-decel: cubic-bezier(0, 0, 0.2, 1);
  --motion-ease-accel: cubic-bezier(0.4, 0, 1, 1);

  /* Status colors (semantic) */
  --status-idle: var(--dust-400);
  --status-active: var(--teal-400);
  --status-success: var(--green-400);
  --status-warning: var(--amber-400);
  --status-error: var(--rose-400);
  --status-blocked: var(--purple-400);

  /* Glow effects */
  --glow-active: 0 0 12px rgba(45, 212, 191, 0.3);
  --glow-success: 0 0 12px rgba(74, 222, 128, 0.3);
  --glow-error: 0 0 12px rgba(251, 113, 133, 0.3);
  --glow-ambient: 0 0 20px rgba(255, 255, 255, 0.05);

  /* Elevation (for depth) */
  --elevation-1: 0 1px 3px rgba(0, 0, 0, 0.3);
  --elevation-2: 0 4px 12px rgba(0, 0, 0, 0.4);
  --elevation-3: 0 8px 24px rgba(0, 0, 0, 0.5);
  --elevation-hover: 0 4px 20px rgba(0, 0, 0, 0.4);

  /* Cell sizing */
  --cell-radius: 8px;
  --cell-padding: 12px 16px;
  --cell-gap: 8px;
  --cell-border: 1px solid var(--glass-border);
}
```

### 7.2 Status as a First-Class Concept

Every component that shows status should use the same visual language:

```css
/* Status dot */
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--status-idle);
}
.status-dot[data-status="active"] {
  background: var(--status-active);
  animation: pulse 2s ease-in-out infinite;
}
.status-dot[data-status="success"] { background: var(--status-success); }
.status-dot[data-status="error"] { background: var(--status-error); }
.status-dot[data-status="blocked"] { background: var(--status-blocked); }

/* Status ring (for cells) */
.status-ring {
  border: 2px solid var(--status-idle);
  border-radius: var(--cell-radius);
  transition: border-color var(--motion-duration-fast) var(--motion-ease-default);
}
.status-ring[data-status="active"] {
  border-color: var(--status-active);
  box-shadow: var(--glow-active);
}
```

### 7.3 Micro-Interactions Catalog

| Interaction | Visual | CSS/Motion |
|---|---|---|
| Button hover | Subtle lift + glow | `transform: translateY(-1px); box-shadow: var(--elevation-hover)` |
| Button press | Scale down | `transform: scale(0.97)` |
| Card hover | Border glow | `border-color: var(--accent); box-shadow: var(--glow-ambient)` |
| Card click → detail | Shared element morph | `layoutId={id}` |
| Toggle on | Track slides + color | `background: var(--status-active); transform: translateX(20px)` |
| Number change | Spring to new value | `useSpring({ to: newVal })` + display `Math.round(spring.value)` |
| List item enter | Fade up + stagger | `initial={{ opacity: 0, y: 12 }}` + `delay: i * 0.05` |
| List item exit | Fade out + collapse | `exit={{ opacity: 0, height: 0 }}` |
| Error flash | Red border pulse | `@keyframes error-flash { 0% { border-color: var(--status-error) } 100% { border-color: transparent } }` |
| Success flash | Green glow pulse | Same pattern, `--status-success` |
| Loading skeleton | Shimmer gradient | `background: linear-gradient(90deg, transparent, rgba(255,255,255,0.04), transparent); animation: shimmer 1.5s infinite` |

---

## Part 8: File Structure

```
demo/demo-app/src/
├── app/
│   ├── App.tsx                    # Router + providers
│   ├── DataHub.ts                 # Zustand store (all domains)
│   └── routes.tsx                 # Route config (lazy loaded)
│
├── cells/                         # Cell components (the building blocks)
│   ├── Cell.tsx                   # Base cell container
│   ├── CellGrid.tsx              # Responsive grid of cells
│   ├── CellTimeline.tsx          # Chronological cell list
│   ├── CellBoard.tsx             # Kanban board
│   ├── CellDetail.tsx            # Expanded detail view
│   ├── CellGraph.tsx             # Force-directed graph
│   │
│   ├── renderers/                 # Cell content renderers per entity type
│   │   ├── PrdCell.tsx
│   │   ├── PlanCell.tsx
│   │   ├── TaskCell.tsx
│   │   ├── AgentCell.tsx
│   │   ├── EpisodeCell.tsx
│   │   ├── KnowledgeCell.tsx
│   │   ├── BenchRunCell.tsx
│   │   ├── GateCell.tsx
│   │   └── SignalCell.tsx
│   │
│   └── cells.css                  # Cell styles (uses design tokens)
│
├── chrome/                        # App shell components
│   ├── AppShell.tsx
│   ├── TopNav.tsx
│   ├── Sidebar.tsx
│   ├── CommandBar.tsx             # ⌘K command palette
│   ├── StreamOverlay.tsx          # SSE/WS status
│   └── chrome.css
│
├── layout/                        # Layout primitives
│   ├── SplitView.tsx
│   ├── MetricStrip.tsx
│   ├── PhaseRail.tsx
│   ├── Section.tsx                # Collapsible section with header
│   └── layout.css
│
├── motion/                        # Animation system
│   ├── tokens.ts                  # Spring configs, durations, variants
│   ├── transitions.ts             # Shared transition presets
│   ├── AnimatedList.tsx           # List with enter/exit animations
│   ├── AnimatedNumber.tsx         # Spring-animated number display
│   ├── SharedElement.tsx          # Shared element transition wrapper
│   └── FadeIn.tsx                 # Simple fade-in-view wrapper
│
├── scenes/                        # Full-page scenes (replacing "pages")
│   ├── Constellation.tsx          # Landing / overview
│   ├── Orchestrate.tsx            # PRD → plan → run → verify
│   ├── Observe.tsx                # Control plane dashboard
│   ├── Evaluate.tsx               # Bench + cost + comparison
│   ├── Build.tsx                  # Interactive builder
│   ├── Knowledge.tsx              # Knowledge browser
│   └── Settings.tsx               # Configuration
│
├── scenarios/                     # Scenario configs (not components)
│   ├── registry.ts                # Scenario config registry
│   ├── prd-pipeline.ts
│   ├── knowledge.ts
│   ├── dreams.ts
│   ├── fleet.ts
│   ├── cascade.ts
│   ├── cost.ts
│   └── bench.ts
│
├── terminal/                      # Terminal subsystem
│   ├── Terminal.tsx               # Self-contained terminal component
│   ├── useTerminal.ts             # xterm.js lifecycle
│   ├── usePty.ts                  # PTY WebSocket
│   └── terminal.css
│
├── transport/                     # Data transport layer
│   ├── api.ts                     # REST client (typed)
│   ├── sse.ts                     # SSE client with reconnect
│   ├── ws.ts                      # WebSocket client with reconnect
│   └── types.ts                   # API response types
│
├── hooks/                         # Thin selector hooks
│   ├── usePlans.ts
│   ├── usePrds.ts
│   ├── useAgents.ts
│   ├── useBench.ts
│   ├── useKnowledge.ts
│   ├── useConfig.ts
│   └── useServerHealth.ts
│
└── styles/
    ├── rosedust.css               # Design system tokens
    ├── reset.css                  # CSS reset
    └── global.css                 # Global styles
```

---

## Part 9: Implementation Phases

### Phase 1: Foundation (motion + data + cells)

**Goal:** Working DataHub, motion system, and Cell primitives with one scene (Orchestrate).

| Task | What | Depends on |
|---|---|---|
| 1.1 | Install Motion, create `motion/tokens.ts` + presets | — |
| 1.2 | Create `DataHub.ts` (Zustand store with plans, agents, config slices) | — |
| 1.3 | Create `transport/api.ts` (typed REST client) | — |
| 1.4 | Create `transport/sse.ts` (SSE with reconnect + DataHub integration) | 1.2 |
| 1.5 | Create Cell + CellGrid + CellTimeline components | 1.1 |
| 1.6 | Create PhaseRail + MetricStrip + Section layout primitives | 1.1 |
| 1.7 | Build Orchestrate scene using cells + layout | 1.2, 1.5, 1.6 |
| 1.8 | Wire SSE to DataHub → live plan execution in Orchestrate | 1.4, 1.7 |

### Phase 2: Scenes + Navigation

**Goal:** All scenes working with transitions between them.

| Task | What | Depends on |
|---|---|---|
| 2.1 | Build Constellation (landing) with animated node graph | 1.2, 1.5 |
| 2.2 | Build Observe (control plane dashboard) | 1.2, 1.5, 1.6 |
| 2.3 | Build Evaluate (bench + cost) | 1.2, 1.5 |
| 2.4 | Build Knowledge (graph + list with toggle) | 1.2, 1.5 |
| 2.5 | Build Build (chat interface) | 1.2, 1.3 |
| 2.6 | Wire React Router with View Transitions between scenes | 2.1-2.5 |
| 2.7 | Add CommandBar (⌘K palette) for quick navigation | 2.6 |

### Phase 3: Polish + Performance

**Goal:** Every animation is buttery, every transition is seamless, every empty state is informative.

| Task | What | Depends on |
|---|---|---|
| 3.1 | Shared element transitions (cell → detail) across all scenes | 2.1-2.5 |
| 3.2 | Empty states for every data surface | 2.1-2.5 |
| 3.3 | Loading skeletons for every data surface | 2.1-2.5 |
| 3.4 | Performance audit — measure animation frame budget | 3.1 |
| 3.5 | Vendor chunk splitting (motion, xterm, three.js) | 3.4 |
| 3.6 | Accessibility pass (reduced-motion, aria, keyboard nav) | 3.1-3.3 |

### Phase 4: Terminal + Advanced

**Goal:** Terminal subsystem modernized, advanced scenarios working.

| Task | What | Depends on |
|---|---|---|
| 4.1 | Extract Terminal subsystem (Terminal.tsx, useTerminal.ts, usePty.ts) | — |
| 4.2 | Add scenario registry + config-driven scenario loading | 2.6 |
| 4.3 | Wire dream visualization into Knowledge scene | 2.4 |
| 4.4 | Wire custody chain into Observe scene | 2.2 |
| 4.5 | Add agent chat in Build scene | 2.5 |

---

## Part 10: Technical Recommendations

### 10.1 Dependencies

| Package | Version | Why |
|---|---|---|
| `motion` | ^12.x | Animation (framer-motion successor) |
| `zustand` | ^5.x | State management (tiny, fast, React 19 ready) |
| `@xterm/xterm` | ^5.5.0 | Terminal emulation (keep current) |
| `three` | ^0.170 | 3D for constellation/graph (keep current, lazy-load) |
| `react-router` | ^7.x | Routing with View Transitions |

### 10.2 Bundle Strategy

- Lazy-load scenes via `React.lazy` + `Suspense`
- Vendor chunks: `motion` (~15kB), `xterm` (~200kB), `three` (~500kB) — only load when needed
- Motion tree-shakes well — only import what you use
- SSE/WS connections are singleton (managed by DataHub, not per-component)

### 10.3 Reduced Motion

Always respect `prefers-reduced-motion`:

```typescript
import { useReducedMotion } from 'motion';

function AnimatedCell({ children }) {
  const shouldReduce = useReducedMotion();
  return (
    <motion.div
      animate={shouldReduce ? {} : { y: 0, opacity: 1 }}
      transition={shouldReduce ? { duration: 0 } : spring.gentle}
    >
      {children}
    </motion.div>
  );
}
```

### 10.4 What NOT to Use

| Technology | Why not |
|---|---|
| CSS Houdini | Browser support too limited |
| GSAP | Overkill for this use case, Motion covers everything needed |
| React Spring | Abandoned in favor of Motion |
| Rive | Wrong tool (for embedded animations, not UI transitions) |
| Lottie | Wrong tool (for After Effects exports, not live data) |
| Redux | Too much boilerplate for this app size |
| React Query | DataHub + SSE makes this unnecessary |

---

## Part 11: How This Connects to Roko's Loop

The UI should mirror roko's universal loop: **query → score → route → compose → act → verify → write → react.**

| Loop step | What the user sees |
|---|---|
| **query** | PRD appears, system shows what it's analyzing |
| **score** | Complexity badge, estimated time, model recommendations appear |
| **route** | Cascade router visualization shows which model was chosen and why |
| **compose** | System prompt being assembled (expandable, shows 9 layers) |
| **act** | Agent running, terminal output streaming, tool calls showing |
| **verify** | Gate pipeline running (compile → test → clippy → diff), results streaming |
| **write** | Files modified list updates, signal hash appears |
| **react** | If gate fails → replan animation, new tasks appear; if passes → next task starts |

This makes the demo not just a dashboard but a **window into roko's cognition**. Every step is visible, every decision is explained, every outcome is animated.

---

## Part 12: Multi-Agent Identity & Output Architecture

### 12.1 The Problem Today

When roko runs a plan, multiple agents execute tasks simultaneously. The current demo has no way to show:
- **Which agent** is doing what
- **What kind** of agent it is (research, implementation, verification, security)
- **What domain** it operates in (Rust, TypeScript, infra, DeFi)
- **Which outputs** belong to which agent
- **How agents relate** to each other (dependencies, knowledge sharing, handoffs)

Terminals just show shell output. Logs just show text. There's no identity, no attribution, no visual distinction. When 3 agents are running in parallel across 2 terminals, it's impossible to tell what's happening.

### 12.2 Agent Identity Card

Every agent in roko has structured identity from `roko.toml` config and the agent manifest:

```typescript
interface AgentIdentity {
  name: string;              // e.g., "implement-auth"
  role: string;              // e.g., "implementer", "verifier", "researcher", "security"
  domain: string;            // e.g., "rust", "typescript", "infra", "defi"
  model: string;             // e.g., "claude-sonnet-4", "claude-haiku-4"
  tier: 'T1' | 'T2' | 'T3'; // routing tier
  planId?: string;           // which plan this agent serves
  taskId?: string;           // which task this agent is executing

  // Derived visual identity (deterministic from name + role)
  spectre: SpectreIdentity;
}
```

The identity card is always visible when an agent is referenced — in task rows, terminal headers, log entries, topology graphs, everywhere.

### 12.3 The Spectre: Algorithmic Agent Avatars

Inspired by the bardo creature system, every agent gets a **Spectre** — a procedurally generated visual identity derived deterministically from the agent's name and role. Same agent always produces the same Spectre. Operators learn to recognize agents by their Spectre at a glance, like recognizing video game characters.

#### Generation Algorithm

```typescript
interface SpectreIdentity {
  seed: Uint8Array;          // BLAKE3(name + ":" + role) → 32 bytes
  archetype: SpectreArchetype;
  palette: SpectrePalette;
  eyeStyle: SpectreEyeStyle;
  glyph: string;             // 2-char Unicode glyph pair (the "eyes")
  shape: SpectreShape;
}

// 8 body archetypes mapped from seed bytes [0..4]
type SpectreArchetype =
  | 'orb'       // compact, focused — knowledge agents
  | 'column'    // tall, structured — implementation agents
  | 'sprawl'    // wide, exploratory — research agents
  | 'cluster'   // multi-node — parallel/coordinator agents
  | 'teardrop'  // directional — goal-oriented agents
  | 'ring'      // hollow center — monitoring/verification agents
  | 'fractal'   // branching — analysis agents
  | 'amorphous' // shifting — creative/generative agents
  ;

// Eye styles from seed bytes [14..16]
type SpectreEyeStyle = 'round' | 'slit' | 'compound' | 'star';
```

#### Canvas Rendering (Web)

The Spectre renders as a **64x64 pixel canvas** — a dot-cloud creature with spring physics:

```typescript
function renderSpectre(
  ctx: CanvasRenderingContext2D,
  identity: SpectreIdentity,
  state: AgentState,        // idle | active | error | done
  size: number,             // 32 | 48 | 64 px
) {
  // 40-80 particles in a shaped cloud
  // Archetype determines cloud shape
  // Palette from role + seed-based hue offset within ROSEDUST
  // Eyes: 2 bright glyph points at center
  // State drives animation:
  //   idle: slow breathing (expand/contract, 0.3Hz)
  //   active: faster breathing (0.7Hz), eye glow, slight shimmer
  //   error: constricted, jittery, crimson tint
  //   done: expanded, fading glow, settled
}
```

#### Role → Visual Mapping

| Role | Archetype | Palette | Eye Glyph | Character |
|---|---|---|---|---|
| implementer | column | rose | `◈◈` | Structured, builds things |
| researcher | sprawl | violet | `◉◉` | Wide-scanning, exploratory |
| verifier | ring | jade | `◎◎` | Watchful, monitoring |
| security | teardrop | crimson | `◆◆` | Focused, directional |
| coordinator | cluster | sapphire | `✦✦` | Multi-node, orchestrating |
| planner | fractal | amber | `◇◇` | Branching analysis |
| reviewer | orb | bone | `●●` | Dense knowledge, focused |

The seed-based hue offset (±15° within the role's palette range) ensures that two "implementer" agents have recognizably similar but distinguishable Spectres.

#### Size Variants

| Context | Size | Detail level |
|---|---|---|
| Inline (log entry, task row) | 16px | Solid dot with role color |
| Badge (terminal header, card) | 32px | Simplified dot cloud, no animation |
| Card (agent detail, topology node) | 48px | Full dot cloud, breathing animation |
| Hero (agent detail expanded) | 64px | Full detail, spring physics, eyes visible |

### 12.4 Multi-Agent Terminal Architecture

#### The Problem

Current: 2 terminal panes show raw shell output. No way to know which agent produced which output. When agents run sequentially in the same terminal, outputs blend together.

#### Solution: Agent-Attributed Terminal Streams

Each terminal pane gets an **agent attribution header** that updates when a different agent takes control:

```
┌──────────────────────────────────────────────────┐
│ ◈◈ implement-auth · T2 · claude-sonnet-4 · ACTIVE │
│ implementer · rust                                │
├──────────────────────────────────────────────────┤
│                                                  │
│  $ cargo test --lib auth                         │
│  running 4 tests                                 │
│  test auth::test_jwt_creation ... ok             │
│  test auth::test_token_validation ... ok         │
│                                                  │
└──────────────────────────────────────────────────┘
```

The header shows:
- **Spectre glyph** (the eye pair, colored by role)
- **Agent name** (bold)
- **Tier badge** (T1/T2/T3)
- **Model** (which LLM)
- **Status** (active/idle/done/error — with matching color)
- **Role + domain** (second line, dimmer)

#### Multi-Agent Split Views

When multiple agents run in parallel, the terminal layout adapts:

```
┌─ 2 agents ──────────────────────────────────────────────────────┐
│                           │                                     │
│ ◈◈ implement-auth · T2   │ ◎◎ verify-auth · T1                │
│ implementer · rust        │ verifier · rust                     │
│ ─────────────────────── │ ──────────────────────────────────  │
│                           │                                     │
│ $ roko run "implement..." │ (waiting for implement-auth)        │
│ Creating src/auth/mod.rs  │                                     │
│ Adding JWT dependency...  │                                     │
│                           │                                     │
└─────────────────────────┴─────────────────────────────────────┘
```

```
┌─ 3+ agents ─────────────────────────────────────────────────────┐
│                                                                 │
│ ◈◈ implement-auth · ACTIVE │ ◈◈ implement-api · ACTIVE         │
│ ──────────────────────────│────────────────────────────────────│
│ $ cargo build             │ $ roko run "add REST endpoints"    │
│ Compiling auth v0.1.0     │ Creating routes/user.rs            │
│                           │ Adding handler functions...         │
│                                                                 │
│ ◎◎ verify-all · PENDING   │ ◇◇ plan-review · DONE             │
│ ──────────────────────────│────────────────────────────────────│
│ (blocked by auth, api)    │ ✓ Plan approved, 4 tasks created   │
│                           │                                     │
└─────────────────────────────────────────────────────────────────┘
```

The grid auto-adapts:
- 1 agent: single pane (full width)
- 2 agents: side-by-side
- 3-4 agents: 2x2 grid
- 5+ agents: scrollable list with expandable panes (only active agents expanded)

#### Agent Output Stream Component

```typescript
interface AgentOutputStream {
  agentId: string;
  identity: AgentIdentity;
  terminal?: TerminalHandle;    // PTY-backed terminal (for shell commands)
  logStream?: LogEntry[];       // Structured log (for non-terminal output)
  mode: 'terminal' | 'log' | 'split';  // How to display
}
```

Not every agent needs a full terminal. Some agents (research, planning) produce structured output that's better shown as a log stream:

```
┌─ ◉◉ research-auth · researcher · ACTIVE ───────────────────┐
│                                                              │
│  🔍 Searching: "JWT best practices Rust 2026"                │
│  📄 Found 3 relevant sources                                 │
│  📝 Synthesizing findings...                                 │
│                                                              │
│  Key findings:                                               │
│  • Use `jsonwebtoken` crate v9+ (Ed25519 support)           │
│  • Rotate keys via env variable, not hardcoded              │
│  • Include `exp`, `iat`, `sub` claims minimum               │
│                                                              │
│  ✓ Research complete → enhancing PRD                         │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### 12.5 Agent-Attributed Logs

The unified event log attributes every entry to an agent:

```
┌─ LOG ───────────────────────────────────────────────────────┐
│                                                              │
│ 17:38:49  ◇◇ planner       PLAN    Generated 4 tasks        │
│ 17:38:51  ◈◈ implement-auth ACTIVE  Starting task auth-001   │
│ 17:38:51  ◈◈ implement-api  ACTIVE  Starting task api-001    │
│ 17:39:02  ◈◈ implement-auth GATE    compile ✓ test ✓         │
│ 17:39:05  ◈◈ implement-api  GATE    compile ✓ test ✗         │
│ 17:39:05  ◈◈ implement-api  REPLAN  Gate failure → retry     │
│ 17:39:12  ◎◎ verify-all     ACTIVE  Starting verification    │
│ 17:39:15  ◎◎ verify-all     GATE    all gates ✓              │
│ 17:39:15  ✦✦ coordinator    DONE    Plan complete            │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

Each log line has:
- **Timestamp** (mono, dim)
- **Spectre glyph** (colored by role)
- **Agent name** (bold, truncated to 16 chars)
- **Event type** (uppercase badge)
- **Message** (the actual content)

Filter controls let you show/hide by agent, by role, or by event type.

### 12.6 Agent Topology Graph

The topology view shows agents as **Spectre nodes** connected by dependency and knowledge-sharing edges:

```
         ◇◇ planner
        ╱         ╲
   ◈◈ auth    ◈◈ api     (implementation agents)
       │          │
       └────┬─────┘
            │
       ◎◎ verify          (verification agent)
            │
       ✦✦ coordinator     (reports to plan runner)
```

Each node renders the agent's Spectre at 48px with:
- **Breathing animation** when active
- **Color ring** matching status (teal=active, green=done, rose=error, gray=idle, purple=blocked)
- **Edge animations**: data flowing along edges when agents communicate (small dots moving along the line)
- **Hover**: shows agent card with full identity, current task, metrics
- **Click**: expands to agent detail view (shared element transition from node)

### 12.7 Agent Card Component

The reusable agent card used everywhere (task rows, topology, fleet, sidebar):

```typescript
function AgentCard({
  identity,
  state,
  variant,    // 'inline' | 'badge' | 'card' | 'hero'
  showTask,   // show current task info
  showMetrics, // show cost/tokens/time
}: AgentCardProps) {
  return (
    <div className={`agent-card agent-card-${variant}`}>
      <SpectreAvatar identity={identity} state={state} size={sizeForVariant(variant)} />
      <div className="agent-card-identity">
        <span className="agent-name">{identity.name}</span>
        <span className="agent-role">{identity.role} · {identity.domain}</span>
      </div>
      <StatusBadge status={state} />
      {showTask && <span className="agent-task">{identity.taskId}</span>}
      {showMetrics && <AgentMetrics agent={identity} />}
    </div>
  );
}
```

**Inline variant** (16px Spectre, used in log entries and task rows):
```
◈◈ implement-auth · rust · T2
```

**Badge variant** (32px Spectre, used in terminal headers):
```
[◈◈ dot-cloud] implement-auth
               implementer · rust · claude-sonnet-4
```

**Card variant** (48px Spectre, used in fleet grid and topology):
```
┌───────────────────────────────────┐
│ [48px animated    ] implement-auth│
│ [dot-cloud spectre] ────────────  │
│ [with breathing   ] implementer   │
│                     rust · T2     │
│                     claude-sonnet │
│               ● ACTIVE  $0.12    │
└───────────────────────────────────┘
```

**Hero variant** (64px Spectre, used in expanded detail view):
Full-size animated Spectre with spring physics, eye tracking, state-driven color shifts. Shows complete agent metrics, episode history, current task detail, tool calls, and output stream.

### 12.8 Knowledge Transfer Visualization

When agents share knowledge (via roko's neuro store), the transfer is visible:

1. **Source agent's Spectre** pulses with a brief glow
2. **A particle** (small colored dot) detaches and animates along the edge to the target agent
3. **Target agent's Spectre** absorbs the particle (brief flash)
4. **Log entry** appears: `◈◈ auth → ◎◎ verify: shared "JWT signing approach"`

This makes the invisible (knowledge store writes/reads) visible and tangible.

### 12.9 Multi-Agent Playback Controls

When multiple agents run in parallel, the playback bar shows aggregate progress:

```
┌────────────────────────────────────────────────────────────────┐
│ ▶ ⏭ ↺ │ Auto Step │ ◈◈ 2 active  ◎◎ 1 waiting  ✓ 1 done │
│         │           │ Task 3/7 · Step 2/5 · preparing auth    │
└────────────────────────────────────────────────────────────────┘
```

The agent status strip shows Spectre glyphs with counts, giving an instant read on fleet activity.

### 12.10 Implementation: SpectreAvatar Component

```typescript
// spectre/SpectreAvatar.tsx
import { useRef, useEffect, useMemo } from 'react';

interface SpectreAvatarProps {
  name: string;
  role: string;
  state: 'idle' | 'active' | 'error' | 'done';
  size: 16 | 32 | 48 | 64;
  animate?: boolean;  // default true for 48+
}

function generateSeed(name: string, role: string): Uint8Array {
  // Deterministic hash from name:role
  // Use a simple hash for web (no BLAKE3 needed — consistency, not security)
  const str = `${name}:${role}`;
  const seed = new Uint8Array(32);
  for (let i = 0; i < str.length; i++) {
    seed[i % 32] ^= str.charCodeAt(i);
    seed[(i + 7) % 32] ^= (str.charCodeAt(i) * 31) & 0xFF;
    seed[(i + 13) % 32] ^= (str.charCodeAt(i) * 127) & 0xFF;
  }
  return seed;
}

function archetypeFromSeed(seed: Uint8Array): SpectreArchetype {
  return ARCHETYPES[seed[0] % 8];
}

function paletteFromRole(role: string, seed: Uint8Array): string {
  const baseHue = ROLE_HUES[role] ?? 12;  // rose default
  const offset = ((seed[20] / 255) * 30) - 15;  // ±15° variation
  return `oklch(0.65 0.10 ${baseHue + offset})`;
}

const ROLE_HUES: Record<string, number> = {
  implementer: 12,   // rose
  researcher: 290,   // violet
  verifier: 170,     // jade
  security: 25,      // crimson
  coordinator: 250,  // sapphire
  planner: 85,       // amber
  reviewer: 55,      // bone
};

const EYE_GLYPHS: Record<string, string> = {
  implementer: '◈',
  researcher: '◉',
  verifier: '◎',
  security: '◆',
  coordinator: '✦',
  planner: '◇',
  reviewer: '●',
};

export function spectreGlyph(role: string): string {
  return EYE_GLYPHS[role] ?? '○';
}

// For 16px inline: just render the glyph pair with role color
// For 32px badge: simplified static dot cloud
// For 48/64px: full canvas with animation
export default function SpectreAvatar({ name, role, state, size, animate }: SpectreAvatarProps) {
  if (size <= 16) {
    const glyph = spectreGlyph(role);
    return <span className={`spectre-inline spectre-${state}`}>{glyph}{glyph}</span>;
  }

  if (size <= 32) {
    // Static SVG dot cloud
    return <SpectreStaticSVG name={name} role={role} state={state} size={size} />;
  }

  // Full animated canvas
  return <SpectreCanvas name={name} role={role} state={state} size={size} animate={animate ?? true} />;
}
```

### 12.11 Reference Material

The Spectre system is documented in detail in the bardo interfaces specification:

| Document | Path | What it covers |
|---|---|---|
| Creature System | `bardo-backup/prd/18-interfaces/28-creature-system.md` | Dot-cloud geometry, spring physics, eye rendering, lifecycle degradation |
| ROSEDUST + Spectre | `docs/v2-depth/16-surfaces/04-rosedust-and-spectre.md` | Deterministic identity, 8 archetypes, PAD animation, 4 renderers |
| Embodied Consciousness | `bardo-backup/prd/18-interfaces/perspective/03-embodied-consciousness.md` | Terminal as body metaphor, somatic zones, PAD-driven transformation |
| Visualization Primitives | `bardo-backup/prd/18-interfaces/rendering/02-visualization-primitives.md` | Braille rendering, plasma effects, force graphs |
| Design System | `bardo-backup/prd/18-interfaces/rendering/00-design-system.md` | ROSEDUST palette, contrast, degradation by lifecycle |

---

## Part 13: State Machine — Never Leave the User Guessing

This is the section that prevents every UX bug visible in the current app. The core problem: **the system does things but nothing consistently tells you what's happening, what it's waiting for, or what went wrong.** Terminals flash and go blank. "RUNNING" label blinks with no loader. The playback bar appears and disappears. Things transition between states with no in-between — no skeleton, no spinner, no explanation.

### 13.1 The Root Cause: No Lifecycle State Machine

The current app has no unified lifecycle model. Each component manages its own state independently:

- **Terminal pane**: `connecting` → `connected` → `disconnected` (but visually: prompt → blank → prompt → blank)
- **Pipeline panel**: `idle` → runs a phase → updates JSX (but: what happens between phases? nothing visible)
- **Playback bar**: shows "Preparing" or "Step N/M" (but: sometimes disappears entirely because the parent container's overflow clips it)
- **Scenario runner**: imperatively calls functions, sleeps, checks refs (but: no structured state anyone can observe)

Nobody owns the answer to "what is the system doing right now?"

### 13.2 The Fix: A Single Observable Pipeline

Replace scattered imperative state with a **single state machine** that every component subscribes to:

```typescript
type PipelineStage =
  | { type: 'idle' }
  | { type: 'connecting'; detail: string }         // "Connecting terminal 1/2..."
  | { type: 'workspace'; detail: string }           // "Creating workspace roko-prd-pipeline..."
  | { type: 'resolving'; detail: string }            // "Locating roko binary..."
  | { type: 'preparing'; step: string; detail: string } // "Seeding Rust CLI skeleton..."
  | { type: 'executing'; phase: PipelinePhase; agent?: AgentIdentity; command?: string }
  | { type: 'waiting'; reason: string }              // "Waiting for PRD writer agent..."
  | { type: 'gate'; name: string; status: 'running' | 'passed' | 'failed' }
  | { type: 'transitioning'; from: PipelinePhase; to: PipelinePhase }
  | { type: 'error'; message: string; recoverable: boolean }
  | { type: 'complete'; summary: CompletionSummary }
  ;

// Lives in DataHub — one source of truth
interface DataHub {
  // ...
  pipeline: PipelineStage;
  pipelineHistory: PipelineStage[];  // last 50 stages for the log
}
```

Every component reads from this. Nobody guesses.

### 13.3 The Activity Strip: Always Visible, Never Clipped

The current playback bar disappears because its parent has `overflow: hidden`. The fix is architectural: the **Activity Strip** is a first-class app chrome element, not a child of the scenario content.

```
┌──────────────────────────────────────────────────────────────┐
│ TopNav                                                       │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Content area (scenarios, pages, etc.)                       │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ Activity Strip — ALWAYS HERE, NEVER CLIPPED                  │
│ ● Connecting terminal 1/2...                    [Auto] [Step]│
└──────────────────────────────────────────────────────────────┘
```

The Activity Strip lives in AppShell, outside the page content's overflow container. It is structurally impossible to clip. It shows:

1. **Status dot** — colored by current stage (teal=working, amber=waiting, green=done, rose=error)
2. **Stage label** — what the system is doing RIGHT NOW, from `PipelineStage.detail`
3. **Progress** — if applicable (step 2/5, 45% complete)
4. **Agent attribution** — if an agent is running, show its Spectre glyph + name
5. **Controls** — pause/step/reset (always accessible)

The strip is always one line. It never wraps. Long text truncates with ellipsis. But it is ALWAYS visible.

### 13.4 Loading States: Every Transition Gets One

The current app jumps between states with no in-between. The rule:

**Every state transition that takes > 200ms MUST show a loading indicator.**

| Transition | Current behavior | Required behavior |
|---|---|---|
| Page load → terminal connected | Blank pane → sudden prompt | Skeleton pane with "Connecting..." + pulsing dot |
| Connected → workspace created | Prompt → commands flash by → blank | "Creating workspace..." label in terminal header, commands visible |
| Workspace → scenario running | Blank → suddenly showing output | Phase rail animates, Activity Strip shows "Preparing..." |
| Between pipeline phases | One phase just ends, next starts | Transition animation (slide-out/slide-in), Activity Strip updates |
| Command executing | Nothing visible | Terminal shows command being typed, Activity Strip shows "Executing: roko prd idea..." |
| Waiting for agent response | "RUNNING" label with no feedback | Spinner on RUNNING button, Activity Strip shows "Waiting for PRD writer agent...", breathing animation on agent's Spectre |
| Gate running | Nothing → suddenly pass/fail | Gate bar segments fill left-to-right with animation |
| Error | Sometimes nothing, sometimes a flash | Red border pulse on relevant container, error message in Activity Strip, error entry in log |

### 13.5 Terminal Lifecycle: Visible at Every Step

The terminal pane must show its lifecycle state visually:

```
┌─ CONNECTING ─────────────────────────────────────┐
│                                                   │
│         ◌  Connecting to PTY server...            │
│            Session: demo-prd-pipeline-0           │
│                                                   │
└───────────────────────────────────────────────────┘
```

```
┌─ ● CONNECTED ── ROKO COMMANDS ──────────────────┐
│ will …/roko/.roko ᐅ main ⊙ 17:55                │
│ $                                                 │
│                                                   │
└───────────────────────────────────────────────────┘
```

```
┌─ ◈◈ implement-auth · EXECUTING ────────────────┐
│ $ roko prd draft new "BTC Funding Alert CLI"     │
│ 📄 Creating PRD: BTC Funding Alert CLI           │
│ model: claude-sonnet-4 via anthropic             │
│ ▋                                                │
└───────────────────────────────────────────────────┘
```

```
┌─ ○ IDLE ── ROKO COMMANDS ──────────────────────┐
│ Waiting for next task assignment                  │
│                                                   │
│ Previous: roko prd idea "..." (✓ done, 1.2s)    │
│                                                   │
└───────────────────────────────────────────────────┘
```

Key rules:
- **Never blank.** If the terminal has no output, show the lifecycle state (connecting, idle, waiting).
- **Never mystery characters.** If the prompt has garbled glyphs, the terminal header should still clearly say what's happening.
- **Agent attribution in header** when an agent is using this terminal.
- **Previous command summary** when idle — what ran last, how long it took, pass/fail.

### 13.6 The Pipeline Status Log: Canonical Truth

The LOG section in the sidebar must be the **canonical record** of everything that happened. It's not just "some events that got appended." It's structured, complete, and timestamped:

```
┌─ LOG ────────────────────────────────────── 17:58:40 ─┐
│                                                        │
│ 17:55:37  SETUP    ● Starting live PRD pipeline        │
│                      Creating workspace...             │
│                      Locating roko binary...           │
│                      Seeding Rust CLI skeleton...      │
│                                                        │
│ 17:55:38  IDEA     ● Capturing idea                    │
│                      roko prd idea "Build a CLI..."    │
│                      → Captured into .roko/prd/ideas   │
│                                                        │
│ 17:55:43  DRAFT    ◌ Dispatching PRD writer agent      │
│                      model: glm-5-1 via zai            │
│                      Waiting for response...           │
│                                                        │
│ 17:58:35  DRAFT    ● PRD generated                     │
│                      "BTC Funding Alert CLI"           │
│                      0 requirements · 0 acceptance     │
│                                                        │
│ 17:58:36  PLAN     ◌ Generating implementation plan... │
│                                                        │
└────────────────────────────────────────────────────────┘
```

**Rules for the log:**
1. **Every phase gets an entry** — not just when things complete, but when they START
2. **Sub-steps are indented** — shows what happened within each phase
3. **Status indicators**: `●` = done, `◌` = in progress (pulsing), `✗` = failed
4. **Agent attribution** — when a phase involves an agent, show which one and what model
5. **Elapsed time** — show how long each phase took (after completion)
6. **Errors are prominent** — red text, full error message, not just "failed"
7. **Auto-scroll to bottom** — always shows the latest, with scroll-back available

### 13.7 The Skeleton Pattern: Nothing Is Ever Blank

When a section is about to receive data but hasn't yet, show a **skeleton** — not blank space, not a spinner in empty void.

```css
.skeleton {
  background: linear-gradient(
    90deg,
    rgba(255,255,255,0.02) 25%,
    rgba(255,255,255,0.06) 50%,
    rgba(255,255,255,0.02) 75%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;
  border-radius: 4px;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
```

Skeleton variants for each content type:

```
Pipeline header skeleton:
┌──────────────────────────────────────┐
│ ████████ · ████████                  │
│ ████████████████████████████         │
│                                      │
│ ██████████████████████████████████   │
│ ██████████████████████████           │
│                                      │
│ ████   ████   ████                   │
└──────────────────────────────────────┘

Task list skeleton:
┌──────────────────────────────────────┐
│ ● ████████████████   ██ ████████    │
│ ● ██████████████████ ██ ████████    │
│ ● ████████████       ██ ████████    │
└──────────────────────────────────────┘

Log skeleton:
┌──────────────────────────────────────┐
│ ██:██:██  ██████  ██████████████    │
│ ██:██:██  ██████  ████████████      │
└──────────────────────────────────────┘
```

### 13.8 Error Recovery: Don't Just Fail, Explain

When something goes wrong, the current app either shows nothing or a brief flash. The rule:

**Every error must tell you: (1) what failed, (2) why, (3) what you can do about it.**

```
┌─ ERROR ──────────────────────────────────────────┐
│                                                   │
│  ✗ PRD generation did not produce a draft         │
│                                                   │
│  The PRD writer agent was dispatched to           │
│  glm-5-1 via zai but did not return a             │
│  structured PRD document.                         │
│                                                   │
│  Possible causes:                                 │
│  • Model provider (zai) may be slow or offline    │
│  • The idea prompt may be too vague               │
│  • Config model may not support structured output │
│                                                   │
│  [Retry with different model]  [View raw output]  │
│                                                   │
└───────────────────────────────────────────────────┘
```

The error card replaces the current phase content — it doesn't just flash and disappear. It stays until the user takes action.

### 13.9 State Persistence: Survive Navigation

If you switch tabs (Demo → Dashboard → Demo) or refresh the page, the current state should persist:

- **URL encodes scenario + phase**: `/demo?scenario=prd-pipeline&phase=draft`
- **DataHub state survives** route changes (Zustand store is global)
- **Terminal sessions reconnect** to the same server-side PTY (session ID in URL/store)
- **Log entries persist** in DataHub, not component-local state

The user should never lose context by navigating. The system should feel like it's always running in the background, and you're just switching your view of it.

### 13.10 Summary: The Contract

| Principle | Rule |
|---|---|
| **Never blank** | Every container shows either content, a skeleton, or an explicit empty/loading/error state |
| **Never guess** | The Activity Strip always says what the system is doing right now |
| **Never clip** | The Activity Strip and status indicators are structurally outside overflow containers |
| **Never flash** | State transitions take ≥200ms, with animation between states |
| **Never mystery** | Errors explain what, why, and what to do next |
| **Never lose** | State survives navigation and page refresh |
| **Always attribute** | Every action shows which agent/process is responsible |
| **Always log** | Every phase start/end/error is recorded in the canonical log |
| **Always progress** | Long operations show progress (determinate if possible, indeterminate if not) |

---

## Appendix A: Inspiration References

- **Linear** — transitions between list and detail views, keyboard-first navigation
- **Vercel Dashboard** — real-time deployment status, streaming build logs, status indicators
- **GitHub Actions** — workflow visualization, step-by-step progress, log streaming
- **Raycast** — command palette UX, fluid animations, keyboard shortcuts
- **Stripe Dashboard** — metric animations, chart transitions, responsive grid
- **Eve Online** — ambient particle fields, glow effects, spatial navigation, dark theme with accent colors
- **Factorio** — production chain visualization, real-time flow indicators, system monitoring
- **Notion** — block-based composition, smooth drag-and-drop, contextual menus

---

## Appendix B: Key Metrics for Success

| Metric | Target |
|---|---|
| Time to understand what you're looking at | < 3 seconds |
| Time from page load to first interaction | < 1 second |
| Frame rate during animations | 60fps consistent |
| Bundle size (initial) | < 200kB gzipped |
| Largest Contentful Paint | < 1.5s |
| Number of components reused across 2+ scenes | > 15 |
| Number of raw CSS animation hacks | 0 (all through motion tokens) |
| Empty/loading states per data surface | 100% coverage |
