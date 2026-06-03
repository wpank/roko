# 02. Target Architecture

The ideal generalized, abstracted, extensible architecture for the demo surface.

---

## 1. Core Principle: DataHub + Cell + Motion

Three orthogonal systems that compose:

```
DataHub (Zustand)        — WHAT data exists and how it flows
Cell system              — HOW data is rendered (visual units)
Motion system            — HOW state changes are perceived (animation)
```

Every feature is DataHub slice + Cell renderer + Motion preset. Nothing else.

---

## 2. DataHub: Centralized State (Zustand)

Replace 19 hooks + 3 context providers with a single Zustand store. Each domain is a slice.

### 2.1 Store Shape

```typescript
interface DataHub {
  // ── Connection ──
  serverStatus: 'connected' | 'checking' | 'disconnected';
  streams: {
    sse: 'connecting' | 'live' | 'error' | 'closed';
    ws: 'connecting' | 'live' | 'error' | 'closed';
    workflow: 'connecting' | 'live' | 'error' | 'closed';
  };

  // ── Domain data (all nullable, loaded on demand) ──
  config: RokoConfig | null;
  plans: Plan[];
  prds: Prd[];
  agents: Agent[];
  episodes: Episode[];
  knowledge: { entries: KnowledgeEntry[]; edges: KnowledgeEdge[] };
  benchRuns: BenchRun[];
  benchMatrix: MatrixState | null;
  signals: Signal[];
  dreams: DreamReport[];
  routing: CascadeRouterState | null;
  experiments: Experiment[];
  efficiency: EfficiencyMetric[];
  gates: GateThreshold[];
  custody: CustodyEntry[];

  // ── Workspace ──
  workspace: {
    id: string;
    path: string;
    status: 'creating' | 'ready' | 'error';
  } | null;
  workspaceCache: Map<string, WorkspaceInfo>;

  // ── Active execution state ──
  activeWorkflow: WorkflowSnapshot | null;
  activeRun: { id: string; status: string } | null;

  // ── Derived selectors (computed, not stored) ──
  // Access via: useDataHub(s => s.activePlan) etc.

  // ── Actions ──
  // REST fetchers (one per domain, idempotent)
  fetchConfig: () => Promise<void>;
  fetchPlans: () => Promise<void>;
  fetchAgents: () => Promise<void>;
  fetchEpisodes: () => Promise<void>;
  fetchKnowledge: () => Promise<void>;
  fetchBenchRuns: () => Promise<void>;
  fetchRouting: () => Promise<void>;
  fetchDreams: () => Promise<void>;

  // Workspace
  ensureWorkspace: (prefix: string) => Promise<string>;
  destroyWorkspace: (id: string) => Promise<void>;

  // Config mutations
  updateConfig: (patch: Partial<RokoConfig>) => Promise<void>;

  // ── Event handlers (called by transport layer) ──
  handleServerEvent: (event: ServerEvent) => void;
  handleWorkflowFrame: (frame: WorkflowFrame) => void;
  handleBenchEvent: (event: BenchEvent) => void;
}
```

### 2.2 Why Zustand

| Requirement | Zustand | React Context | Redux |
|-------------|---------|---------------|-------|
| Selective re-render | Yes (selector) | No (full subtree) | Yes |
| Bundle size | 1.1kB | 0 | 40kB |
| Boilerplate | Minimal | Medium | Heavy |
| React 19 compat | Yes | Yes | Partial |
| Middleware (devtools, persist) | Yes | Manual | Yes |
| Outside-React access | Yes (getState) | No | Yes |

### 2.3 Hooks as Thin Selectors

```typescript
// Convenience hooks — thin wrappers over DataHub selectors
function usePlans() {
  const plans = useDataHub(s => s.plans);
  const fetch = useDataHub(s => s.fetchPlans);
  useEffect(() => { fetch(); }, [fetch]);
  return plans;
}

function useConfig() {
  return useDataHub(s => s.config);
}

function useActiveWorkflow() {
  return useDataHub(s => s.activeWorkflow);
}
```

Each hook is 5-10 lines. No state management, no fetch logic, no error handling. Just selectors.

---

## 3. Transport Layer: Unified Adapter

One module manages all server communication. REST, SSE, and WS are implementation details hidden behind a clean interface.

### 3.1 Architecture

```
┌─────────────────────────────────────────────┐
│                roko-serve :6677              │
│                                             │
│  REST ──── typed client (api.ts)            │
│  SSE  ──── unified stream (sse.ts)          │
│  WS   ──── unified stream (ws.ts)           │
│  PTY  ──── terminal sessions (pty.ts)       │
└─────────┬───────────┬───────────┬───────────┘
          │           │           │
     ┌────▼────┐ ┌────▼────┐ ┌───▼────┐
     │ api.ts  │ │ sse.ts  │ │ ws.ts  │
     │ (fetch) │ │ (evtsrc)│ │ (sock) │
     └────┬────┘ └────┬────┘ └───┬────┘
          │           │          │
     ┌────▼───────────▼──────────▼────┐
     │         DataHub (Zustand)       │
     │    handleServerEvent(event)     │
     │    handleWorkflowFrame(frame)   │
     │    handleBenchEvent(event)      │
     └────────────────┬───────────────┘
                      │
          ┌───────────▼───────────┐
          │   React Components    │
          │   useDataHub(selector) │
          └───────────────────────┘
```

### 3.2 Transport Modules

```typescript
// transport/api.ts — typed REST client
class RokoApi {
  private base: string;

  async get<T>(path: string): Promise<T | null>;
  async post<T>(path: string, body?: unknown): Promise<T | null>;
  async delete(path: string): Promise<boolean>;

  // Health probe with caching
  async probe(): Promise<boolean>;
}

// transport/sse.ts — SSE with auto-reconnect + cursor-based replay
class SseTransport {
  constructor(path: string, onEvent: (event: ServerEvent) => void);
  connect(): void;
  disconnect(): void;
  // Auto-reconnect with exponential backoff (max 5 retries)
  // Supports Last-Event-ID for cursor replay
}

// transport/ws.ts — WebSocket with subscription filtering
class WsTransport {
  constructor(path: string, subscriptions: string[]);
  connect(): void;
  disconnect(): void;
  send(msg: unknown): void;
  onMessage(handler: (data: unknown) => void): void;
  // Auto-reconnect, ping/pong keepalive
}

// transport/pty.ts — PTY terminal WebSocket
class PtyTransport {
  constructor(sessionId: string);
  connect(): void;
  disconnect(): void;
  send(data: string): void;
  onData(handler: (data: string) => void): void;
}
```

### 3.3 Connection Lifecycle

```
App mounts
  → DataHub initializes
  → api.probe() checks server health
  → If live:
      → sse.connect('/api/events') → handleServerEvent
      → ws.connect('/api/stream')  → handleServerEvent
      → fetchConfig(), fetchPlans(), fetchAgents() (parallel)
  → If offline:
      → serverStatus = 'disconnected'
      → Re-probe every 30s
  → On SSE/WS event:
      → DataHub.handleServerEvent(event)
      → Zustand notifies subscribers
      → React re-renders affected components only
```

---

## 4. Cell System: Universal Visual Units

Every roko entity maps to a Cell — a self-contained visual unit.

### 4.1 Cell Anatomy

```
┌─────────────────────────────────────┐
│ ● STATUS    IDENTITY         ACTION │  ← header
├─────────────────────────────────────┤
│                                     │
│  CONTENT                            │  ← body (entity-specific renderer)
│                                     │
├─────────────────────────────────────┤
│  CONNECTIONS                        │  ← footer (links to related entities)
└─────────────────────────────────────┘
```

### 4.2 Cell Types

| Entity | Status states | Content renderer | Primary view |
|--------|-------------|------------------|-------------|
| PRD | idea → draft → published → planned | Title, excerpt, requirements count | CellTimeline |
| Plan | pending → active → complete → failed | Title, task count, progress bar | CellBoard |
| Task | pending → active → done → failed → blocked | Title, tier badge, model badge, gate results | CellBoard |
| Agent | idle → running → stopped → error | Name, role, domain, task count, cost | CellGrid |
| Episode | recorded → verified | Hash, agent, plan, result, cost | CellTimeline |
| Knowledge | raw → distilled → promoted | Topic, excerpt, citation count, tier | CellGrid |
| Dream | pending → running → complete | Cycle id, phase, entries consolidated | CellTimeline |
| Gate | pending → passed → failed | Name, rung, duration | Inline status |
| Bench Run | queued → running → complete → failed | Suite, model, pass rate, cost | CellGrid |
| Signal | received → scored → routed | Hash, type, substrate | CellTimeline |

### 4.3 Layout Components

```typescript
// Base containers
<Cell>              — Single entity card with status ring, identity, actions
<CellGrid>          — Responsive grid/masonry of cells
<CellTimeline>      — Chronological list with time markers
<CellBoard>         — Kanban columns (pending → active → done → failed)
<CellDetail>        — Expanded detail view (shared element from Cell)
<CellGraph>         — Force-directed graph of connected cells

// Layout primitives
<SplitView>         — Resizable two-pane layout (list + detail)
<PhaseRail>         — Horizontal step indicator with animation
<MetricStrip>       — Horizontal strip of live-updating metrics
<DataSurface>       — Loading/empty/error wrapper for any data view
<Section>           — Collapsible section with header
<CommandBar>        — ⌘K command palette

// Existing (keep)
<Mosaic>            — 1px-gap metric grid
<Pane>              — Glass panel container
<GateBar>           — Gate status strip
<Terminal>          — xterm.js embed
```

### 4.4 DataSurface: Universal State Wrapper

Every data surface uses `<DataSurface>` to handle loading/empty/error consistently:

```tsx
<DataSurface
  data={plans}
  loading={<Skeleton variant="list" count={3} />}
  empty={<EmptyState
    message="No plans yet"
    action="Run `roko plan create` to create one"
  />}
  error={<ErrorState message={error} retry={refetch} />}
>
  {(plans) => <CellGrid items={plans} render={PlanCell} />}
</DataSurface>
```

---

## 5. Motion System: 4-Layer Animation

### 5.1 Library: Motion (framer-motion successor)

The only React animation library that handles layout animations, shared element transitions, gesture interactions, and spring physics in one API.

### 5.2 Layers

| Layer | Trigger | Purpose | Budget |
|-------|---------|---------|--------|
| **Ambient** | Always running | Living system feel | ≤ 8 concurrent CSS |
| **Data-driven** | SSE/WS events | Reactive feedback | ≤ 12 Motion springs |
| **Interaction** | User actions | Tactile response | Per-interaction |
| **Transitions** | Navigation | Spatial context | Per-route |

### 5.3 Motion Tokens

```typescript
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
} as const;

export const stagger = { fast: 0.03, normal: 0.05, slow: 0.08 } as const;

export const fadeUp = {
  initial: { opacity: 0, y: 12 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -8 },
  transition: { duration: 0.35 },
};
```

### 5.4 Event-Driven Animation Map

| Server Event | Visual Response | Component |
|-------------|----------------|-----------|
| PlanStarted | Phase rail lights up | PhaseRail |
| AgentSpawned | Agent cell fades in with expanding ring | CellGrid |
| AgentOutput | Terminal streams text | Terminal |
| GateResult (pass) | Green check scales in, ripple | GateBar |
| GateResult (fail) | Red shake, failure card slides in | GateBar, TaskCell |
| PlanCompleted | Summary mosaic scales in | Mosaic |
| PhaseTransition | Phase dot slides to next position | PhaseRail |
| InferenceStarted | Model badge appears on task | TaskCell |
| InferenceCompleted | Cost/token counters spring to new values | MetricStrip |
| Episode | Episode cell slides into timeline | CellTimeline |
| BenchTaskEvent | Progress bar animates | BenchRunCell |
| ConfigReloaded | Config badge flashes | TopNav |

---

## 6. Layout Architecture

### 6.1 Page Layout Model: Scrollable Container

Pages are NOT viewport-locked. The global layout follows a strict vertical stack:

```
┌─ TopNav (position: sticky; top: 0) ──────────────────────────┐
│ ⌈ NUNCHI ⌋  │  DEMO  DASH  BENCH  ...  │  ● LIVE 2H 34M     │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  Scrollable Page Container (overflow-y: auto)                 │
│  ┌──────────────────────────────────────────────────────┐     │
│  │ Page header / metric strip                           │     │
│  ├──────────────────────────────────────────────────────┤     │
│  │ Content sections (naturally flowing, top-to-bottom)  │     │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐                │     │
│  │  │ Cell    │ │ Cell    │ │ Cell    │                │     │
│  │  └─────────┘ └─────────┘ └─────────┘                │     │
│  │                                                      │     │
│  │  ┌───────────────────────────────────────────────┐   │     │
│  │  │ Full-width section (timeline, feed, chart)    │   │     │
│  │  └───────────────────────────────────────────────┘   │     │
│  │                                                      │     │
│  │  (page continues, user scrolls)                      │     │
│  └──────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────┘
```

**Rules:**
- TopNav is the only sticky element. Content below it scrolls naturally.
- No `overflow: hidden` on `<body>`. No `height: 100vh` on page containers.
- Sections within a page use content-determined height, never fixed `min-height`.
- Internal scroll regions (event feeds, logs) use `max-height` with internal `overflow-y: auto` and fade gradients at edges.
- See `04-DESIGN-SYSTEM.md` sections 8-10 for concrete layout model, terminal aesthetic, and density token values.

### 6.2 Terminal / Demoscene Aesthetic (Architectural Principle)

The visual language is mono chrome, density-first, with a terminal/demoscene vocabulary. This is not a stylistic preference -- it is an architectural constraint that governs component design:

- **Typography register**: All chrome (labels, headings, nav, badges, dividers) uses `var(--mono)` uppercase with letter-spacing. Body text uses `var(--sans)`. Display numbers use `var(--display)` Fraunces italic.
- **ASCII vocabulary**: Frame brackets (`⌈ LABEL ⌋`), box-drawing characters (`─│┌┐└┘`), braille for density fills (`⠋⠙⠹`), block elements for inline charts (`▁▂▃▄▅▆▇█`), status glyphs (`●○◉◐◑ ✓✕`).
- **Density-first**: Tight padding (10-12px inside cards, 8px gaps). No wasted space. Content fills available width. See `04-DESIGN-SYSTEM.md` section 10 for concrete values.
- **Phosphor decay / scanline motion**: Value changes use bright-to-dim decay (~1s). Decorative surfaces use low-opacity braille noise floors. Active inputs show terminal cursor blink.

### 6.3 Component Library Structure

Components are organized by architectural role:

```
components/
├── design/        — Reusable design atoms: StatusBadge, EmptyState, ErrorState,
│                    GateBar, Skeleton, Pulse, Badge
├── layout/        — Structural primitives: DataSurface, Stack, PageShell,
│                    SplitView, Tabs, ScrollArea, PhaseRail, MetricStrip
├── cells/         — Entity renderers: Cell, CellGrid, TaskCell, AgentCell,
│                    PlanCell, EpisodeCell, BenchRunCell
├── motion/        — Animation system: tokens, AnimatedNumber, AnimatedList,
│                    Transition
├── ascii/         — Terminal/demoscene vocabulary: AsciiLabel, AsciiDivider,
│                    AsciiFrame, AsciiBraille, AsciiProgress, AsciiWaveform
├── ambient/       — (future) WebGL/canvas backgrounds: ParticleField,
│                    NoiseBackground, FluidGradient, HeartbeatLine
├── agent/         — (future) Agent-namespaced widgets: AgentContainer,
│                    AgentMetricBar, AgentFeed, AgentHeartbeat, AgentAvatar
├── feeds/         — (future) Real-time event feeds: EventStream,
│                    InferenceFeed, BlockFeed
└── overlay/       — (future) Modals, drawers, command palette, floating chat
```

Each category has a single responsibility. `design/` contains stateless visual atoms. `layout/` contains structural wrappers. `cells/` contains data-driven entity renderers. `motion/` contains animation tokens and wrappers. `ascii/` contains terminal-aesthetic primitives. Future categories (`ambient/`, `agent/`, `feeds/`, `overlay/`) are defined but not yet populated.

---

## 7. File Structure (Target)

```
demo/demo-app/src/
├── app/
│   ├── App.tsx                    # Router + DataHub provider
│   ├── DataHub.ts                 # Zustand store (all domains)
│   └── routes.tsx                 # Route config (lazy loaded)
│
├── cells/                         # Cell components
│   ├── Cell.tsx                   # Base container
│   ├── CellGrid.tsx
│   ├── CellTimeline.tsx
│   ├── CellBoard.tsx
│   ├── CellDetail.tsx
│   ├── CellGraph.tsx
│   ├── renderers/                 # Per-entity cell content
│   │   ├── PrdCell.tsx
│   │   ├── PlanCell.tsx
│   │   ├── TaskCell.tsx
│   │   ├── AgentCell.tsx
│   │   ├── EpisodeCell.tsx
│   │   ├── KnowledgeCell.tsx
│   │   ├── BenchRunCell.tsx
│   │   └── GateCell.tsx
│   └── cells.css
│
├── chrome/                        # App shell
│   ├── AppShell.tsx
│   ├── TopNav.tsx
│   ├── StreamOverlay.tsx
│   └── chrome.css
│
├── layout/                        # Layout primitives
│   ├── SplitView.tsx
│   ├── MetricStrip.tsx
│   ├── PhaseRail.tsx
│   ├── DataSurface.tsx
│   ├── Section.tsx
│   └── layout.css
│
├── motion/                        # Animation system
│   ├── tokens.ts
│   ├── AnimatedList.tsx
│   ├── AnimatedNumber.tsx
│   └── FadeIn.tsx
│
├── scenes/                        # Full-page scenes
│   ├── Orchestrate.tsx
│   ├── Observe.tsx
│   ├── Evaluate.tsx
│   ├── Build.tsx
│   └── Knowledge.tsx
│
├── terminal/                      # Terminal subsystem
│   ├── Terminal.tsx
│   ├── useTerminal.ts
│   └── terminal.css
│
├── transport/                     # Data transport
│   ├── api.ts
│   ├── sse.ts
│   ├── ws.ts
│   ├── pty.ts
│   └── types.ts
│
├── hooks/                         # Thin selector hooks
│   ├── usePlans.ts
│   ├── useAgents.ts
│   ├── useBench.ts
│   ├── useConfig.ts
│   └── useServerHealth.ts
│
├── design/                        # Reusable design components
│   ├── Pane.tsx
│   ├── Mosaic.tsx
│   ├── GateBar.tsx
│   ├── StatusBadge.tsx
│   ├── Skeleton.tsx
│   ├── EmptyState.tsx
│   ├── ErrorState.tsx
│   └── design.css
│
└── styles/
    ├── rosedust.css               # Design tokens
    ├── reset.css
    └── global.css
```

**~55 files** vs current ~100+. Each file has one job.

---

## 8. Cross-Cutting Concerns

These are handled once, shared by every page:

| Concern | Current (broken) | Target (DataHub) |
|---------|-----------------|-------------------|
| Config | 6 independent fetches | DataHub slice, fetched once |
| Workspace | 3 different creation paths | DataHub ensureWorkspace() |
| Server health | Module-level singleton | DataHub serverStatus |
| Loading states | 5+ patterns | `<DataSurface>` wrapper |
| Error states | 5+ patterns | `<ErrorState>` component |
| Status colors | Inconsistent per-component | CSS tokens + `<StatusBadge>` |
| SSE/WS events | 3 uncoordinated streams | Unified transport → DataHub |
| Page transitions | None | React Router View Transitions |
| Pipeline lifecycle | No unified state machine | Single `PipelineStage` in DataHub |
| Activity indicator | Playback bar clipped by overflow | Activity Strip in app chrome (structurally unclippable) |
| State persistence | Lost on navigation | URL-encoded + DataHub survives route changes |

---

## 9. Pipeline State Machine — Never Leave the User Guessing

The core problem in the current app: **the system does things but nothing consistently tells you what's happening, what it's waiting for, or what went wrong.** Terminals flash and go blank. "RUNNING" label blinks with no loader. Things transition between states with no in-between.

### 9.1 Root Cause: No Lifecycle State Machine

Each component manages its own state independently:
- Terminal pane: `connecting` → `connected` → `disconnected` (visually: prompt → blank → prompt → blank)
- Pipeline panel: `idle` → runs a phase → updates JSX (nothing visible between phases)
- Playback bar: sometimes disappears entirely because parent overflow clips it
- Scenario runner: imperatively calls functions, sleeps, checks refs (no structured state anyone can observe)

### 9.2 Single Observable Pipeline

Replace scattered imperative state with a **single state machine** in DataHub:

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

interface DataHub {
  // ...existing slices...
  pipeline: PipelineStage;
  pipelineHistory: PipelineStage[];  // last 50 stages for the log
}
```

Every component reads from this. Nobody guesses.

### 9.3 Activity Strip: Always Visible, Never Clipped

The Activity Strip is a first-class app chrome element in AppShell, structurally **outside** the page content's overflow container. It is impossible to clip.

```
┌──────────────────────────────────────────────────────────────┐
│ TopNav                                                       │
├──────────────────────────────────────────────────────────────┤
│  Content area (scenarios, pages, etc.)                       │
├──────────────────────────────────────────────────────────────┤
│ Activity Strip — ALWAYS HERE, NEVER CLIPPED                  │
│ ● Connecting terminal 1/2...                    [Auto] [Step]│
└──────────────────────────────────────────────────────────────┘
```

Shows: status dot (colored by stage), stage label from `PipelineStage.detail`, progress (step 2/5), agent attribution (Spectre glyph + name), controls (pause/step/reset). Always one line, long text truncates with ellipsis.

### 9.4 Loading States: Every Transition Gets One

**Every state transition that takes > 200ms MUST show a loading indicator.**

| Transition | Current behavior | Required behavior |
|---|---|---|
| Page load → terminal | Blank pane → sudden prompt | Skeleton pane with "Connecting..." + pulsing dot |
| Connected → workspace | Commands flash by → blank | "Creating workspace..." in terminal header |
| Workspace → scenario | Blank → suddenly showing output | Phase rail animates, Activity Strip shows "Preparing..." |
| Between phases | One phase ends, next starts | Transition animation, Activity Strip updates |
| Command executing | Nothing visible | Terminal shows command, Activity Strip shows "Executing: roko prd idea..." |
| Waiting for agent | "RUNNING" with no feedback | Spinner, Activity Strip shows "Waiting for PRD writer agent...", breathing Spectre |
| Gate running | Nothing → suddenly pass/fail | Gate bar segments fill left-to-right with animation |
| Error | Sometimes nothing | Red border pulse, error in Activity Strip, error entry in log |

### 9.5 Terminal Lifecycle: Visible at Every Step

The terminal pane must show its lifecycle state visually:

```
CONNECTING:   ◌ Connecting to PTY server... (Session: demo-prd-pipeline-0)
CONNECTED:    ● CONNECTED — ready for commands
EXECUTING:    ◈◈ implement-auth · EXECUTING — shows agent attribution in header
IDLE:         ○ IDLE — Waiting for next task (Previous: roko prd idea "..." ✓ done, 1.2s)
```

Rules: **Never blank** (show lifecycle state). **Never mystery characters** (header always says what's happening). **Agent attribution in header** when an agent is using the terminal. **Previous command summary** when idle.

### 9.6 Skeleton Pattern: Nothing Is Ever Blank

When a section is about to receive data but hasn't yet, show a **skeleton** with shimmer animation:

```css
.skeleton {
  background: linear-gradient(90deg, rgba(255,255,255,0.02) 25%, rgba(255,255,255,0.06) 50%, rgba(255,255,255,0.02) 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;
  border-radius: 4px;
}
```

Skeleton variants for: pipeline header, task list, log entries, metric grid, terminal pane.

### 9.7 Error Recovery: Don't Just Fail, Explain

**Every error must tell you: (1) what failed, (2) why, (3) what you can do about it.**

The error card replaces the current phase content — it doesn't flash and disappear. It stays until the user takes action. Shows possible causes and action buttons (retry with different model, view raw output).

### 9.8 State Persistence: Survive Navigation

- **URL encodes scenario + phase**: `/demo?scenario=prd-pipeline&phase=draft`
- **DataHub state survives** route changes (Zustand store is global)
- **Terminal sessions reconnect** to the same server-side PTY (session ID in URL/store)
- **Log entries persist** in DataHub, not component-local state

### 9.9 The Contract

| Principle | Rule |
|---|---|
| **Never blank** | Every container shows content, skeleton, or explicit loading/empty/error state |
| **Never guess** | The Activity Strip always says what the system is doing right now |
| **Never clip** | Activity Strip and status indicators are structurally outside overflow containers |
| **Never flash** | State transitions take ≥200ms, with animation between states |
| **Never mystery** | Errors explain what, why, and what to do next |
| **Never lose** | State survives navigation and page refresh |
| **Always attribute** | Every action shows which agent/process is responsible |
| **Always log** | Every phase start/end/error is recorded in the canonical log |
| **Always progress** | Long operations show progress (determinate if possible, indeterminate if not) |
