# 01. Current State Analysis

Complete inventory of every subsystem in `demo/demo-app/src/`. What exists, how it works, what's broken, what's dead.

---

## 1. Component Inventory (55 .tsx files)

### Pages (14)
| File | LOC | What | Status |
|------|-----|------|--------|
| Demo.tsx | ~800 | Scenario orchestration: playback, terminals, workflow subscription | Working, over-complex |
| Bench.tsx | ~500 | Benchmark runner: matrix builder, run history, SSE events | Working |
| BenchRunDetail.tsx | ~300 | Single bench run: task table, gate waterfall, cost breakdown | Working |
| BenchCompare.tsx | ~200 | Side-by-side bench comparison | Working, no validation |
| Builder.tsx | ~400 | Prompt builder: model picker, terminal, workspace | Working |
| Explorer.tsx | ~300 | Activity stream: signals + episodes merged timeline | Working |
| Terminal.tsx | ~120 | Multi-terminal grid with workspace init | Working |
| Landing.tsx | ~50 | Redirect to demo | Stub |
| Settings.tsx | ~200 | Config display | Working |
| Share.tsx | ~100 | Hash-based receipt display | Working |

### Dashboard Sub-Views (7 + Layout)
| File | What | Status |
|------|------|--------|
| Layout.tsx | Dashboard wrapper with inner nav | Working |
| AgentFleet.tsx | Agent cards + force-directed topology | Working |
| CascadeRouter.tsx | Model routing stats + decision table | Working |
| CostDashboard.tsx | Cost metrics + efficiency bar | Working |
| DreamsView.tsx | Dream cycle phases + journal | Working |
| IntegrityView.tsx | Episode integrity verification | Working |
| KnowledgeEntries.tsx | Knowledge store entries | Working |
| KnowledgeGraph.tsx | Force-directed knowledge graph | Working |

### Components (55)
| Category | Components | Notes |
|----------|-----------|-------|
| App shell | AppShell, TopNav, Grain, Curtain, ScrollTrack, ConfigWidget | Core chrome |
| Terminal | Terminal/TerminalGrid, Terminal/TerminalPane | xterm.js wrappers |
| Pipeline | PrdPipelinePanel, WorkflowConstellation, Timeline, GateBar, GateWaterfall, GateVerdictTicker | Workflow visualization |
| Charts (8) | BarChart, CFactorSparkline, CostChart, HeatmapChart, ParetoChart, RadarChart, ScatterChart, TimelineChart | Canvas-based charts |
| Bench | MatrixBuilder, MatrixDetailView, MatrixRaceTrack, SuiteSelector, TokenVelocitySparkline, BenchLearningInsights | Benchmark UI |
| Layout | Mosaic, Pane, StatCard, Skeleton, CrushedBar, EfficiencyBar, ThresholdGauge, PhosphorNumber | Visual primitives |
| Agent | AgentOutputStream | Agent output streaming |
| Chain | ChainActivityPanel, ChainIntelPanel, LivePositionsPanel | Blockchain/mirage |
| Effects | HeroParticleField, HeroScene, AmbientParticles | Three.js backgrounds |
| Misc | ErrorBoundary, FlatIcon, LiveIndicator, ModelPicker, CommandLog, ConnectScreen, CostRace, RevealWhen, ConfigDiff, TaskTable | Various |

---

## 2. Hook Inventory (19 hooks)

| Hook | What it does | Consumers | Issues |
|------|-------------|-----------|--------|
| useTerminal | xterm.js lifecycle: create, attach, WebSocket PTY, resize, cleanup | Demo, Builder, Terminal | Listener accumulation (B1) |
| useTerminalSession | Scenario orchestration: workspace mgmt, command execution, output detection | Demo scenarios | `setupWorkspace`/`joinWorkspace` now dead code |
| useWorkspace | React context: server-side workspace creation via POST /api/workspaces | Demo, Builder, Terminal | New, working |
| useRokoConfig | React context: fetch /api/config/toml, parse config | Demo, Builder, Bench, Settings, ConfigWidget, AppShell | Fetches independently in 6 places |
| useServerHealth | Probe server health, track connection state | TopNav, multiple pages | Module-level singleton |
| useSSE | Generic SSE hook with reconnect | Unused directly (wrapped by others) | Reconnect timer leak (B27 partially fixed) |
| useBenchSSE | Bench-specific SSE: /api/bench/events | Bench page | Separate from workflow SSE |
| useEventStream | Context-based event stream provider | EventStreamContext | Zero subscribers — orphaned |
| useBench | Bench data: runs, matrix, cost | Bench, BenchRunDetail, BenchCompare | Complex, handles multiple endpoints |
| useMatrixBench | Matrix-specific bench state | Bench page | Subset of useBench concerns |
| useSweBench | SWE-bench specific state | Bench page | Another bench variant |
| useDashboard | Dashboard data aggregation | Dashboard layout | Fetches all dashboard data |
| useAgents | Agent list + topology | AgentFleet | Simple fetch |
| useKnowledge | Knowledge entries + graph | KnowledgeEntries, KnowledgeGraph | Simple fetch with fallback |
| useChain | Blockchain chain data | ChainActivityPanel, ChainIntelPanel | Polling-based |
| useLiveApi | Generic live API polling | Multiple | Replaced by workspace approach in some places |
| useApi | Base API fetcher with fallback | Used by other hooks | Good pattern |
| useDemoMode | Track demo vs live mode | Demo page | Overlaps with useServerHealth |
| useDebouncedRefetch | Debounced data refetch | Bench | Utility |

---

## 3. Data Flow Architecture (Current)

### Three Independent SSE Systems

```
roko-serve :6677
├── GET /api/events (SSE)              → EventStreamContext (ORPHANED — zero subscribers)
├── GET /api/workflows/latest/stream   → workflow-api.ts openWorkflowSubscriptions()
├── GET /api/bench/events (SSE)        → useBenchSSE hook
└── WS  /api/workflow/ws               → workflow-api.ts (parallel to SSE)
```

**Problem**: Three uncoordinated SSE connections. EventStreamContext wraps a provider around the app but nothing subscribes to it. Workflow API opens both SSE and WS in parallel for redundancy. Bench has its own separate SSE.

### REST Endpoints Consumed (~30+)

| Domain | Endpoints | Hook/Consumer |
|--------|-----------|---------------|
| Config | /api/config, /api/config/toml, /api/config/models | useRokoConfig |
| Health | /api/health | useServerHealth |
| Plans | /api/plans, /api/plans/:id | useDashboard |
| Agents | /api/managed-agents, /api/agents/topology | useAgents |
| Knowledge | /api/knowledge/entries, /api/knowledge/edges, /api/knowledge/stats | useKnowledge |
| Episodes | /api/episodes | useDashboard |
| Learn | /api/learn/cascade-router, /api/learn/experiments, /api/learn/efficiency, /api/learn/gates | useDashboard, CascadeRouter |
| Bench | /api/bench/runs, /api/bench/runs/:id, /api/bench/compare, /api/bench/cost-summary, /api/bench/matrix/*, /api/bench/pareto | useBench, useMatrixBench |
| Workflow | /api/workflows/latest, /api/workflows/:id | workflow-api.ts |
| Workspace | POST /api/workspaces, GET /api/workspaces/default | useWorkspace |
| Dreams | /api/dreams/report, /api/dreams/journal | useDashboard |
| Signals | /api/signals | Explorer |
| Terminal | WS /ws/terminal/:sessionId | useTerminal |

### Context Providers (3 active)

```tsx
<RokoConfigProvider>          // Config context — fetches /api/config/toml
  <WorkspaceProvider>         // Workspace context — caches server-created workspaces
    <AppShell>
      <EventStreamProvider>   // SSE context — ORPHANED, zero subscribers
        <Outlet />
      </EventStreamProvider>
    </AppShell>
  </WorkspaceProvider>
</RokoConfigProvider>
```

---

## 4. Server Event Taxonomy (~60+ variants)

From `crates/roko-serve/src/events.rs`, the full `ServerEvent` enum:

### Plan & Execution
- `PlanStarted { plan_id }`, `PlanCompleted { plan_id, success }`
- `PhaseTransition { plan_id, from, to }`
- `Execution { plan_id, event: ExecutionEvent }` (nested: task_started, task_completed, etc.)
- `Episode { plan_id, task_id, passed }`
- `EfficiencyEvent { plan_id, task_id, metric, value }`

### Agent
- `AgentSpawned { agent_id, role, model }`
- `AgentOutput { agent_id, run_id?, content, done, metadata? }` — sanitized stream
- `AgentTrace { agent_id, run_id?, content, tool_calls?, reasoning?, usage?, done }` — raw debug

### Gates
- `GateResult { plan_id, task_id, gate, rung, passed }`

### Inference
- `InferenceStarted { request_id, model, agent_id, auto_routed }`
- `InferenceCompleted { request_id, model, agent_id, input_tokens, output_tokens, cost_usd, duration_ms }`
- `InferenceFailed { request_id, model, agent_id, error }`

### Bench
- `BenchStarted`, `BenchCompleted`, `BenchTaskEvent`, `BenchMatrixEvent`
- `BenchSweStarted`, `BenchSweCompleted`, `BenchSweTaskEvent`, `BenchSweVerifyEvent`

### Somatic / Affect
- `SomaticMarkerFired { plan_id, task_id, valence, intensity, source_episodes, strategy_param }`

### One-Shot Run
- `RunStarted { run_id, prompt }`, `RunCompleted { run_id, success }`

### Operations
- `OperationStarted { op_id, kind }`, `OperationCompleted { op_id, kind, success }`

### Deployment
- `DeploymentCreated`, `DeploymentReady`, `DeploymentFailed`, `DeploymentTornDown`

### Jobs / Workers
- `JobCreated`, `JobPostedToCandidate`, `JobUpdated`, `JobTransitioned`
- `WorkerTaskStarted`, `WorkerTaskCompleted`

### Knowledge / Dreams
- `KnowledgeUpdated`, `DreamCycleStarted`, `DreamCycleCompleted`

### Config
- `ConfigReloaded`

---

## 5. What's Broken

### Critical
1. **EventStreamContext is orphaned** — Provider wraps the app tree but zero components subscribe to it. ~150 LOC of dead infrastructure.
2. **`setupWorkspace` / `joinWorkspace` are dead code** — Still exported from useTerminalSession.ts but no longer imported anywhere after the workspace refactor. All scenarios now use `enterWorkspace`.
3. **No workspace cleanup/GC** — Server-side workspaces accumulate in /tmp with no TTL or cleanup mechanism.

### High
4. **Config fetched 6+ times independently** — useRokoConfig creates independent fetch cycles in Demo, Builder, Bench, Settings, ConfigWidget, AppShell. No shared cache, no coordination.
5. **Three uncoordinated SSE connections** — EventStream (orphaned), workflow, bench each manage their own connection lifecycle.
6. **5+ different error handling patterns** — Some hooks swallow errors, some throw, some return null, some show fallback data, some crash.
7. **Terminal listener accumulation (B1)** — useTerminal registers xterm listeners inside connectWs(), which runs on every reconnect, stacking duplicate handlers.
8. **No loading/empty state consistency** — Each page handles loading differently. Some show spinners, some show nothing, some show stale synthetic data.

### Medium
9. **Workspace path tracked in 3 places** — useWorkspace context, Demo.tsx local state, Builder.tsx local ref. Can desync.
10. **Stale cache risk in useWorkspace** — Cache is keyed by prefix but has no TTL. Server workspace could be deleted while cache thinks it exists.
11. **No shared animation system** — Components use CSS keyframes, inline styles, and requestAnimationFrame independently.
12. **Scenario hardcoded in Demo.tsx** — Adding a scenario means editing an 800-line file with 12 scenario definitions.
13. **PrdPipelinePanel is 427 lines** — Monolithic component that should be 6+ composable pieces.

### Low
14. **Dead imports/code scattered** — Multiple files import utilities that are no longer used.
15. **No keyboard navigation** — No shortcuts for play/pause/step/reset/scenario selection.
16. **No reduced-motion support** — Animations ignore prefers-reduced-motion.

---

## 6. What Works Well

- **Workspace server-side creation** — POST /api/workspaces is fast and reliable
- **Workflow SSE/WS dual subscription** — Resilient real-time updates with cursor-based replay
- **ROSEDUST design tokens** — Consistent color palette and typography (rosedust.css)
- **Terminal subsystem** — xterm.js + PTY WebSocket is solid when not accumulating listeners
- **Chart components** — 8 canvas-based charts are well-implemented
- **Mosaic/Pane/GateBar** — Reusable visual primitives that follow design system
- **PlaybackController** — Speed control, pause/step/reset work for demo orchestration
