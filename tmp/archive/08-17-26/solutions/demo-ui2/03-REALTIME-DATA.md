# 03. Real-Time Data Architecture

The generalized, extensible adapter layer for real-time data flow between roko-serve and the UI.

---

## 1. The Problem

Current state has **three independent streaming systems** with no coordination:

1. **EventStreamContext** (`/api/events` SSE) — wraps the app but has zero subscribers. Dead.
2. **workflow-api.ts** (`/api/workflows/latest/stream` SSE + `/api/workflow/ws` WS) — dual redundant streams for workflow projection
3. **useBenchSSE** (`/api/bench/events` SSE) — bench-specific stream

Plus **REST polling** in 10+ hooks, each with their own fetch cycle, error handling, and caching.

Result: duplicate connections, wasted bandwidth, inconsistent state, race conditions, and no shared cache.

---

## 2. Target: Unified Event Bus

One event bus receives ALL server events. Components subscribe to slices.

```
┌─────────────────────────────────────────────┐
│              roko-serve :6677                │
├───────────┬────────────┬────────────────────┤
│ SSE       │ WebSocket  │ REST               │
│ /api/events│ /api/stream│ /api/*             │
│ (push all │ (push +    │ (request/response) │
│  events)  │  subscribe)│                    │
└─────┬─────┴─────┬──────┴──────┬─────────────┘
      │           │             │
      ▼           ▼             ▼
┌─────────────────────────────────────────────┐
│           EventBus (transport/)              │
│                                             │
│  SSE adapter → normalize → dispatch         │
│  WS adapter  → normalize → dispatch         │
│  REST client → DataHub actions              │
│                                             │
│  ┌─────────────────────────────────────────┐│
│  │ Unified ServerEvent type                ││
│  │ (all 60+ variants from events.rs)      ││
│  └─────────────────────────────────────────┘│
│                                             │
│  Reconnect: exponential backoff (max 15s)   │
│  Replay: Last-Event-ID cursor               │
│  Dedup: event ID tracking                   │
│  Health: connection status per transport     │
└───────────────────┬─────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│           DataHub (Zustand)                  │
│                                             │
│  handleServerEvent(event) {                 │
│    switch (event.type) {                    │
│      case 'PlanStarted':                    │
│        set(s => activePlan = ...)           │
│      case 'AgentSpawned':                   │
│        set(s => agents = [...s.agents, a])  │
│      case 'GateResult':                     │
│        set(s => update task gate status)    │
│      case 'InferenceCompleted':             │
│        set(s => update cost/token totals)   │
│      ...                                    │
│    }                                        │
│  }                                          │
│                                             │
│  Selectors auto-notify React subscribers    │
└───────────────────┬─────────────────────────┘
                    │
                    ▼
            React Components
          (subscribe to slices)
```

### 2.1 SSE Adapter

```typescript
// transport/sse.ts

interface SseConfig {
  url: string;
  onEvent: (event: ServerEvent) => void;
  onStatus: (status: StreamStatus) => void;
  maxRetries?: number;        // default: 5
  maxBackoff?: number;        // default: 15000ms
  replayFromCursor?: number;  // Last-Event-ID for resume
}

class SseAdapter {
  private source: EventSource | null = null;
  private retryCount = 0;
  private retryTimer: ReturnType<typeof setTimeout> | null = null;
  private lastEventId: string | null = null;

  constructor(private config: SseConfig) {}

  connect(): void {
    const url = new URL(this.config.url);
    // Attach Last-Event-ID as query param if resuming
    if (this.lastEventId) {
      url.searchParams.set('cursor', this.lastEventId);
    }

    this.source = new EventSource(url.toString());
    this.source.onopen = () => {
      this.retryCount = 0;
      this.config.onStatus('live');
    };
    this.source.onerror = () => this.handleError();

    // Listen for typed events
    for (const eventType of ['state', 'delta', 'event']) {
      this.source.addEventListener(eventType, (e: Event) => {
        const me = e as MessageEvent;
        this.lastEventId = me.lastEventId || this.lastEventId;
        try {
          const parsed = JSON.parse(me.data);
          this.config.onEvent(parsed);
        } catch { /* ignore malformed */ }
      });
    }
  }

  private handleError(): void {
    this.source?.close();
    this.source = null;
    if (this.retryCount >= (this.config.maxRetries ?? 5)) {
      this.config.onStatus('error');
      return;
    }
    this.retryCount++;
    this.config.onStatus('reconnecting');
    const backoff = Math.min(1000 * 2 ** this.retryCount, this.config.maxBackoff ?? 15000);
    this.retryTimer = setTimeout(() => this.connect(), backoff);
  }

  disconnect(): void {
    if (this.retryTimer) clearTimeout(this.retryTimer);
    this.source?.close();
    this.source = null;
    this.config.onStatus('closed');
  }
}
```

### 2.2 WebSocket Adapter

```typescript
// transport/ws.ts

interface WsConfig {
  url: string;
  subscriptions?: string[];    // projection channels to subscribe
  onMessage: (data: unknown) => void;
  onStatus: (status: StreamStatus) => void;
  pingInterval?: number;       // default: 30000ms
}

class WsAdapter {
  private ws: WebSocket | null = null;
  private retryCount = 0;
  private pingTimer: ReturnType<typeof setInterval> | null = null;

  connect(): void {
    this.ws = new WebSocket(this.config.url);
    this.ws.onopen = () => {
      this.retryCount = 0;
      this.config.onStatus('live');
      // Subscribe to projections
      if (this.config.subscriptions?.length) {
        this.ws!.send(JSON.stringify({
          type: 'subscribe',
          projections: this.config.subscriptions,
        }));
      }
      // Start ping/pong
      this.pingTimer = setInterval(() => {
        this.ws?.send(JSON.stringify({ type: 'ping' }));
      }, this.config.pingInterval ?? 30000);
    };
    this.ws.onmessage = (e) => {
      try {
        const data = JSON.parse(e.data);
        if (data.type === 'pong') return;
        this.config.onMessage(data);
      } catch { /* ignore */ }
    };
    this.ws.onerror = () => this.handleError();
    this.ws.onclose = () => {
      if (this.pingTimer) clearInterval(this.pingTimer);
      if (!this.intentionalClose) this.handleError();
    };
  }
  // ... reconnect logic similar to SSE
}
```

### 2.3 REST Client

```typescript
// transport/api.ts

class RokoApi {
  private baseUrl: string;
  private probeCache: { live: boolean; at: number } | null = null;
  private probeTTL = 30_000;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  async probe(): Promise<boolean> {
    if (this.probeCache && Date.now() - this.probeCache.at < this.probeTTL) {
      return this.probeCache.live;
    }
    try {
      const res = await fetch(`${this.baseUrl}/api/health`, { signal: AbortSignal.timeout(3000) });
      const live = res.ok;
      this.probeCache = { live, at: Date.now() };
      return live;
    } catch {
      this.probeCache = { live: false, at: Date.now() };
      return false;
    }
  }

  async get<T>(path: string): Promise<T | null> {
    try {
      const res = await fetch(`${this.baseUrl}${path}`);
      if (!res.ok) return null;
      return await res.json() as T;
    } catch {
      return null;
    }
  }

  async post<T>(path: string, body?: unknown): Promise<T | null> {
    try {
      const res = await fetch(`${this.baseUrl}${path}`, {
        method: 'POST',
        headers: body ? { 'Content-Type': 'application/json' } : {},
        body: body ? JSON.stringify(body) : undefined,
      });
      if (!res.ok) return null;
      return await res.json() as T;
    } catch {
      return null;
    }
  }
}
```

---

## 3. Event Routing in DataHub

The `handleServerEvent` method routes each event type to the correct store slice:

```typescript
// DataHub.ts (inside create())

handleServerEvent: (event: ServerEvent) => {
  switch (event.type) {
    // ── Plans ──
    case 'PlanStarted':
      set(s => ({ activePlan: { id: event.plan_id, status: 'running' } }));
      break;
    case 'PlanCompleted':
      set(s => ({ activePlan: { ...s.activePlan!, status: event.success ? 'complete' : 'failed' } }));
      break;
    case 'PhaseTransition':
      set(s => ({ activePhase: event.to }));
      break;

    // ── Agents ──
    case 'AgentSpawned':
      set(s => ({
        agents: [...s.agents, { id: event.agent_id, role: event.role, model: event.model, status: 'running' }],
      }));
      break;
    case 'AgentOutput':
      // Route to terminal/output stream for the agent
      get().agentOutputHandlers.get(event.agent_id)?.(event.content, event.done);
      break;

    // ── Gates ──
    case 'GateResult':
      set(s => {
        const plans = [...s.plans];
        // Find and update the task's gate status
        // ... (immutable update)
        return { plans };
      });
      break;

    // ── Inference (cost tracking) ──
    case 'InferenceCompleted':
      set(s => ({
        totalCost: s.totalCost + event.cost_usd,
        totalTokens: s.totalTokens + event.input_tokens + event.output_tokens,
      }));
      break;

    // ── Episodes ──
    case 'Episode':
      set(s => ({
        episodes: [{ plan_id: event.plan_id, task_id: event.task_id, passed: event.passed }, ...s.episodes],
      }));
      break;

    // ── Bench ──
    case 'BenchTaskEvent':
      // Update bench run progress
      break;

    // ── Config ──
    case 'ConfigReloaded':
      get().fetchConfig(); // Re-fetch full config
      break;
  }
}
```

---

## 4. Subscription Patterns

### 4.1 Layout Context for Event Rendering

Event streams render into compact feed components within the scrollable page -- not dedicated fixed panels. The `<EventStream>`, `<AgentFeed>`, and `<InferenceFeed>` components (specified in `10-EXPRESSIVE-PRIMITIVES.md` sections 6-7) use `max-height` with internal `overflow-y: auto` and gradient fade at edges. They sit as one section among many in the page flow. This means:

- Event feeds have content-determined height up to `max-height` (300-400px), then scroll internally.
- The page itself scrolls normally; event feeds do not consume the full viewport.
- Auto-scroll to bottom is active when the user is at or near the bottom of a feed; scrolling up pauses auto-scroll and shows a "N new events" badge.
- High-frequency events (AgentOutput) bypass React and write directly to xterm.js/DOM refs (see 4.2 below).

### 4.2 Page-Level Subscription

Each scene subscribes to the DataHub slices it needs:

```typescript
// scenes/Orchestrate.tsx
function Orchestrate() {
  const activeWorkflow = useDataHub(s => s.activeWorkflow);
  const plans = useDataHub(s => s.plans);
  const agents = useDataHub(s => s.agents);

  // Only re-renders when these specific slices change
  // Other DataHub updates (bench, knowledge, etc.) don't trigger re-render
}
```

### 4.3 Agent Output Streaming

Agent output is special — it's high-frequency character-by-character data that shouldn't go through Zustand (would cause 100+ re-renders/second):

```typescript
// Use a ref-based approach for streaming output
const outputRef = useRef<string>('');

useEffect(() => {
  const unsubscribe = useDataHub.getState().subscribeAgentOutput(agentId, (content, done) => {
    outputRef.current += content;
    // Update terminal/display directly via ref, not state
    terminalRef.current?.write(content);
  });
  return unsubscribe;
}, [agentId]);
```

### 4.4 Workflow Projection

The existing workflow SSE/WS dual stream is good architecture — keep it but route through DataHub:

```typescript
// In transport layer initialization:
const workflowSse = new SseAdapter({
  url: `${SERVE_URL}/api/workflows/latest/stream?root=${root}`,
  onEvent: (frame) => useDataHub.getState().handleWorkflowFrame(frame),
  onStatus: (status) => useDataHub.setState(s => ({ streams: { ...s.streams, workflow: status } })),
});
```

---

## 5. Event Type Taxonomy

Complete mapping from server events to UI effects:

### 5.1 High-Priority (immediate visual feedback)

| Event | Frequency | UI Effect | Latency Budget |
|-------|-----------|-----------|----------------|
| AgentOutput | 10-100/s | Terminal character stream | < 16ms |
| GateResult | 1-5/run | Gate bar animation | < 200ms |
| PhaseTransition | 1-5/run | Phase rail animation | < 300ms |
| InferenceCompleted | 1-10/run | Cost/token counter update | < 100ms |

### 5.2 Medium-Priority (state updates)

| Event | Frequency | UI Effect |
|-------|-----------|-----------|
| PlanStarted/Completed | 1/run | Active plan indicator |
| AgentSpawned | 1-5/run | Agent list update |
| Episode | 1-5/run | Episode timeline entry |
| Execution | 5-20/run | Task board status changes |
| BenchTaskEvent | 1-100/bench | Bench progress update |

### 5.3 Low-Priority (background updates)

| Event | Frequency | UI Effect |
|-------|-----------|-----------|
| EfficiencyEvent | 1-5/run | Efficiency metrics (batch) |
| SomaticMarkerFired | 0-2/run | Somatic indicator (subtle) |
| ConfigReloaded | rare | Config badge flash |
| KnowledgeUpdated | rare | Knowledge store refresh |
| DreamCycleStarted/Completed | rare | Dream status update |

---

## 6. Extensibility: Adding New Event Types

To add a new event type:

1. **Server**: Add variant to `ServerEvent` enum in `events.rs`
2. **Transport**: No change needed (unified adapter handles all events)
3. **DataHub**: Add case to `handleServerEvent` switch
4. **UI**: Create/update Cell renderer or animation trigger

The transport layer is event-type-agnostic. New event types flow through automatically. Only the DataHub handler and UI components need updating.

---

## 7. Offline / Seed Data Mode

When the server is unreachable:

```typescript
// DataHub handles offline gracefully
fetchPlans: async () => {
  const api = get().api;
  const live = await api.probe();
  if (live) {
    const plans = await api.get<Plan[]>('/api/plans');
    if (plans) set({ plans, dataMode: 'live' });
  } else {
    // Show empty state or seed data
    set({ dataMode: 'offline' });
  }
}
```

The `dataMode` is visible in TopNav via `<StatusPill>`:
- `live` → green LED + "LIVE"
- `offline` → dim LED + "OFFLINE" + tooltip with instructions
- `reconnecting` → amber pulse + "RECONNECTING..."

---

## 8. Back-Pressure and Performance

### 8.1 Event Batching

High-frequency events (AgentOutput) are batched at the transport layer:

```typescript
// Batch AgentOutput events into 16ms frames
class EventBatcher {
  private queue: ServerEvent[] = [];
  private scheduled = false;

  push(event: ServerEvent) {
    this.queue.push(event);
    if (!this.scheduled) {
      this.scheduled = true;
      requestAnimationFrame(() => {
        const batch = this.queue.splice(0);
        this.scheduled = false;
        for (const event of batch) {
          this.handler(event);
        }
      });
    }
  }
}
```

### 8.2 Selector Memoization

Zustand selectors with `shallow` equality prevent unnecessary re-renders:

```typescript
import { shallow } from 'zustand/shallow';

const { plans, agents } = useDataHub(
  s => ({ plans: s.plans, agents: s.agents }),
  shallow
);
```

### 8.3 Terminal Output Bypass

Terminal character streams bypass React entirely — write directly to xterm.js via ref:

```typescript
ws.onmessage = (e) => {
  termRef.current?.write(e.data);  // Direct xterm write, no React
};
```
