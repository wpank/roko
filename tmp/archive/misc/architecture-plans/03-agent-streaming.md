# Plan 03: Agent streaming and steering

**Layer:** 3
**Effort:** L (3-5 days)
**Depends on:** Plan 01 (resilience), Plan 02 (agent creation)

## Goal

Users can see live agent output in real time, send messages to running agents,
view per-agent logs and traces, monitor heartbeats, and track costs.

## Current state

### Backend transport layers

Three event transport mechanisms exist:

1. **roko-serve WebSocket** (`/ws` route)
   - Source: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/ws.rs`
   - Carries `ServerEvent` variants: `AgentOutput`, `AgentSpawned`, `AgentStopped`, `PlanStarted`, `TaskCompleted`, `GateResult`, etc.
   - The dashboard connects to this via `rokoWs.ts`.

2. **roko-serve SSE** (`/api/events` route)
   - Source: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/sse.rs`
   - Same `ServerEvent` payload, delivered as SSE stream.
   - Dashboard uses this as fallback when WS fails (see `rokoWs.ts`).

3. **Relay WebSocket** (`/relay/events/ws` on mirage-rs)
   - Source: `apps/mirage-rs/src/relay/` (or similar -- the relay is part of mirage-rs)
   - Carries relay-level events: `agent_connected`, `agent_disconnected`, `heartbeat`, `output_chunk`.
   - The dashboard does NOT currently subscribe to this.

### Frontend WebSocket client

`/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoWs.ts` connects to
roko-serve's WS endpoint. Events go into the `wsStore` (Zustand). Components
subscribe via `useWsEvent(type, handler)`.

Event types are defined in:
- `/Users/will/dev/nunchi/nunchi-dashboard/src/types/api.ts` (search for `WsEventPayload`)
- `/Users/will/dev/nunchi/nunchi-dashboard/src/stores/wsStore.ts`

The `wsStore` pushes events into a ring buffer. Components use the
`useWsEvent` hook to react to specific event types.

### Backend event shapes

Read `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/events.rs` for the
full `ServerEvent` enum. Key variants for agent streaming:

```rust
ServerEvent::AgentOutput {
    agent_id: String,
    run_id: Option<String>,
    content: String,
    done: bool,
    metadata: Option<Value>,
}

ServerEvent::AgentSpawned {
    agent_id: String,
    role: String,
}

ServerEvent::AgentStopped {
    agent_id: String,
    exit_code: Option<i32>,
}

ServerEvent::AgentStarted {
    agent_id: String,
    pid: Option<u32>,
}
```

### Per-agent sidecar endpoints

Each agent process can run its own HTTP server (`roko agent serve`). The
dashboard already has `fetchAgentPort()` in `rokoApi.ts` for direct
agent-to-dashboard communication:

```typescript
// Already exists in rokoApi.ts
fetchAgentPort<T>(baseUrl: string, path: string, options?: RequestInit): Promise<T>
```

And these hooks already exist:
- `useAgentHealth(baseUrl)` -- `GET /health`
- `useAgentStats(baseUrl)` -- `GET /stats`
- `useAgentCapabilities(baseUrl)` -- `GET /capabilities`
- `useAgentLogs(baseUrl, tail)` -- `GET /logs?tail=N`
- `useAgentTasks(baseUrl)` -- `GET /tasks`

The base URL comes from the agent's `endpoints.rest` field in the discovery
registry.

### What is missing

- No relay WebSocket subscription in the dashboard (relay events not consumed)
- No live output panel on the agent detail page
- No message input UI (the mutation exists but no form)
- No heartbeat visualization
- No cost tracking UI
- Agent logs hook exists but the detail page may not render it

## Tasks

### 3.1 Relay WebSocket subscription

**What:** Subscribe to the relay's WebSocket endpoint to receive real-time
agent lifecycle and output events, independent of roko-serve.

**Source files to read:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoWs.ts` -- existing WS client pattern (follow the same structure)
- `/Users/will/dev/nunchi/nunchi-dashboard/src/stores/wsStore.ts` -- event store
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/constants.ts` -- `RELAY_BASE`
- Relay source (if accessible): search for the WebSocket upgrade handler in the relay server to understand event shapes

**Target files to create:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/relayWs.ts`

**Target files to modify:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/stores/wsStore.ts` -- may need to extend event types or add a separate store

**Relay WebSocket URL:** `ws://127.0.0.1:8545/relay/events/ws` (derive from `RELAY_BASE` by replacing `http` with `ws` and appending `/events/ws`).

**Relay event types** (verify by reading relay source or testing with wscat):

```typescript
type RelayWsEvent =
  | { type: "agent_connected"; agent_id: string; label?: string; timestamp: number }
  | { type: "agent_disconnected"; agent_id: string; reason?: string; timestamp: number }
  | { type: "heartbeat"; agent_id: string; tick: number; timestamp: number }
  | { type: "output_chunk"; agent_id: string; content: string; done?: boolean; timestamp: number }
  | { type: "agent_registered"; agent_id: string; timestamp: number };
```

**Implementation approach:**

Create `relayWs.ts` following the same pattern as `rokoWs.ts`:

1. Resolve the relay WS URL from `RELAY_BASE`.
2. Open a WebSocket connection.
3. On message, parse JSON and push into a relay-specific section of the store (or into the same `wsStore` with a source tag).
4. Reconnect on close with exponential backoff.
5. No circuit breaker needed -- the relay is always expected to be running.

**Design decision:** Whether to use the existing `wsStore` or a separate store.

Option A (recommended): Use the same `wsStore` but tag events with `source: "relay" | "roko"`. This lets existing `useWsEvent` consumers receive events from both transports without changes.

Option B: Create a separate `relayWsStore`. Simpler isolation but requires parallel subscription hooks.

The implementer should read `wsStore.ts` and decide which approach fits better.
If using Option A, the `WsEvent` type needs a `source` field:

```typescript
type WsEvent = {
  type: string;
  payload: WsEventPayload | RelayWsEvent;
  receivedAt: number;
  source?: "relay" | "roko";
};
```

**Startup:** Call `connectRelayWs()` from the app's initialization code (same
place that calls `connectRokoWs()`). Check `/Users/will/dev/nunchi/nunchi-dashboard/src/App.tsx`
or wherever the WS connection is established.

**Acceptance criteria:**
- Start mirage-rs with the relay. Open dashboard.
- Connect an agent to the relay (e.g. `roko agent start --name test`).
- Dashboard receives `agent_connected` event within 1 second.
- Agent disconnects. Dashboard receives `agent_disconnected` event.
- No errors when mirage-rs is not running (graceful reconnect).

---

### 3.2 Live output panel on agent detail page

**What:** Show streaming agent output in real time on the agent detail page.

**Source files to read:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/AgentDetailPage.tsx` -- existing detail page
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoWs.ts` -- `useWsEvent` hook
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` -- `useAgentLogs` (already exists, polls agent sidecar)

**Target files to modify:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/AgentDetailPage.tsx`

**Target files to create (optional):**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/components/fleet/LiveOutputPanel.tsx`

**Data sources (in order of preference):**

1. **Relay WS** (`output_chunk` events) -- real-time, works without roko-serve
2. **roko-serve WS** (`AgentOutput` events) -- real-time, requires roko-serve
3. **Agent sidecar logs** (`useAgentLogs` polling) -- near-real-time fallback

The output panel should subscribe to all available sources and deduplicate.
The simplest approach: maintain a local array of output lines, append from
whichever source delivers first.

**Implementation:**

```tsx
// LiveOutputPanel.tsx (conceptual structure)

function LiveOutputPanel({ agentId }: { agentId: string }) {
  const [lines, setLines] = useState<string[]>([]);
  const containerRef = useRef<HTMLDivElement>(null);
  const autoScroll = useRef(true);

  // Subscribe to WS events (both relay and roko)
  useWsEvent("agent_output", (event) => {
    if (event.agent_id === agentId) {
      setLines((prev) => [...prev.slice(-500), event.content]);
    }
  });

  // Also subscribe to relay output_chunk if using separate relay store
  // useRelayEvent("output_chunk", (event) => { ... });

  // Auto-scroll to bottom when new lines arrive
  useEffect(() => {
    if (autoScroll.current && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [lines]);

  // Detect user scroll-up to pause auto-scroll
  const handleScroll = () => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    autoScroll.current = scrollHeight - scrollTop - clientHeight < 50;
  };

  return (
    <div
      ref={containerRef}
      onScroll={handleScroll}
      className="h-96 overflow-y-auto font-mono text-xs bg-zinc-950 p-3 rounded"
    >
      {lines.length === 0 ? (
        <p className="text-zinc-600">Waiting for output...</p>
      ) : (
        lines.map((line, i) => (
          <div key={i} className="whitespace-pre-wrap text-zinc-300">{line}</div>
        ))
      )}
    </div>
  );
}
```

**Ring buffer:** Keep at most 500 lines in state to prevent memory growth.
Drop oldest lines when the buffer exceeds the limit.

**Placement:** Add the output panel as a tab or section on the agent detail
page. If the page already has tabs, add an "Output" tab. If not, place it
below the agent info section.

**Acceptance criteria:**
- Navigate to a running agent's detail page.
- Agent produces output (e.g. from a `roko run` command).
- Output appears line by line in the panel.
- Panel auto-scrolls to show newest output.
- Scroll up manually: auto-scroll pauses. Scroll to bottom: auto-scroll resumes.
- 500+ lines: oldest lines drop off. No memory accumulation.

---

### 3.3 Agent messaging

**What:** Add a message input to the agent detail page so users can send
messages to running agents.

**Source files to read:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs` -- search for `send_message` or `message` handler. Find the request/response shapes.
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` -- `useSendMessage()` mutation (already exists)
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/AgentDetailPage.tsx`

**Target files to modify:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/AgentDetailPage.tsx`

**The mutation already exists in rokoApi.ts:**

```typescript
export function useSendMessage() {
  return useMutation({
    mutationFn: ({
      agentId,
      message,
      context,
    }: {
      agentId: string;
      message: string;
      context?: unknown;
    }) =>
      fetchRoko<{
        run_id: string;
        agent_id?: string;
        status: "running" | "completed" | "failed" | string;
        response?: string;
        conversation_id?: string | null;
        response_mode?: string | null;
      }>(`/agents/${agentId}/message`, {
        method: "POST",
        body: JSON.stringify({ message, context }),
      }),
  });
}
```

**UI implementation:**

Add a message bar at the bottom of the agent detail page (below the live
output panel from task 3.2):

```
+------------------------------------------+
|  Live output panel (from 3.2)            |
|  ...                                     |
|  ...                                     |
+------------------------------------------+
| [Type a message...]              [Send]  |
+------------------------------------------+
```

On send:
1. Call `useSendMessage()` with the agent's ID and the message text.
2. Optimistically append the sent message to the output panel as a user message (distinct styling -- e.g. right-aligned or different color).
3. Show loading state on Send button.
4. On success, the response may include `response` text. If so, append it to the output panel.
5. On error, show error inline below the input.

**Keyboard shortcut:** Enter to send, Shift+Enter for newline.

**Acceptance criteria:**
- Navigate to a running agent's detail page.
- Type a message, press Enter.
- Message appears in output panel as a user message.
- Agent's response appears below the user message.
- Send button shows loading during request.
- Error (agent not running): inline error message, not a page crash.
- Empty message: send button is disabled.

---

### 3.4 Heartbeat visualization

**What:** Show heartbeat status on fleet agent cards and the agent detail page.

**Source files to read:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/heartbeats.rs` -- heartbeat endpoint
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` -- `useAgentHealth(baseUrl)` hook (polls agent sidecar)
- Relay events: heartbeat events from the relay WS (from task 3.1)

**Target files to create:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/components/fleet/HeartbeatIndicator.tsx`

**Target files to modify:**
- Fleet page agent cards (wherever the agent list is rendered -- read the fleet page to find the component)
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/AgentDetailPage.tsx`

**Data sources:**

1. **Relay heartbeat events** (from task 3.1): `{ type: "heartbeat", agent_id, tick, timestamp }`
2. **Agent sidecar health** (`useAgentHealth`): `{ status, agent_id, uptime_s }`
3. **roko-serve heartbeats** (if available): `GET /api/heartbeats`

**HeartbeatIndicator component:**

```tsx
// Pulsing dot that represents agent liveness.
// Green + pulsing = recently heard from.
// Green + static = heard from but not in last 30s.
// Gray = no heartbeat received.

function HeartbeatIndicator({ agentId }: { agentId: string }) {
  const [lastSeen, setLastSeen] = useState<number | null>(null);

  // Subscribe to relay heartbeats
  useWsEvent("heartbeat", (event) => {
    if (event.agent_id === agentId) {
      setLastSeen(Date.now());
    }
  });

  const isRecent = lastSeen !== null && Date.now() - lastSeen < 30_000;
  const isAlive = lastSeen !== null && Date.now() - lastSeen < 120_000;

  return (
    <span
      className={`inline-block w-2.5 h-2.5 rounded-full ${
        isRecent
          ? "bg-emerald-500 animate-pulse"
          : isAlive
          ? "bg-emerald-500"
          : "bg-zinc-600"
      }`}
      title={
        lastSeen
          ? `Last heartbeat: ${Math.round((Date.now() - lastSeen) / 1000)}s ago`
          : "No heartbeat received"
      }
    />
  );
}
```

**Tier badge:**

Read the agent's tier from the merged agent data (task 1.2 from Plan 01) and
show it as a small badge: `T0`, `T1`, `T2`. Use the tier colors from
`constants.ts` (`TIER_COLORS`).

**Acceptance criteria:**
- Fleet page: each agent card shows a heartbeat dot.
- Agent sending heartbeats: dot is green and pulsing.
- Agent stops sending heartbeats: dot goes static green, then gray after 2 minutes.
- Agent detail page: heartbeat indicator in the header area.
- Tier badge visible on cards where tier data is available.

---

### 3.5 Cost tracking display

**What:** Show per-agent cost data on the agent detail page.

**Source files to read:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs` -- search for `stats`, `cost`, `metrics` in agent-related handlers
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` -- `useAgentStats(baseUrl)` hook, `useAgentEpisodes(id)`, `useAgentTrace(id)`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/events.rs` -- `EfficiencyEvent` variant

**Target files to modify:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/AgentDetailPage.tsx`

**Target files to create (optional):**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/components/fleet/AgentCostPanel.tsx`

**Data sources:**

1. **Agent sidecar stats** (`useAgentStats`): `{ request_count, message_count }` -- basic counters.
2. **Agent episodes** (`useAgentEpisodes(id)`): array of episode records with token counts and costs.
3. **Agent trace** (`useAgentTrace(id)`): detailed execution trace with per-turn data.
4. **Efficiency events** (from WS): `{ plan_id, task_id, metric, value }` -- real-time cost updates.

The implementer should read the response shapes from the backend to determine
which fields contain cost data. Episodes typically include:
- `input_tokens`, `output_tokens` -- token counts
- `cost_usd` or similar -- dollar cost
- `model` -- which model was used
- `duration_ms` -- execution time

**UI layout on agent detail page:**

Add a "Stats" tab or section showing:

```
+--------------------+--------------------+--------------------+
| Total cost         | Tokens used        | Requests           |
| $0.42              | 24,318 in / 8,102  | 17                 |
+--------------------+--------------------+--------------------+
| Model distribution                                          |
| claude-sonnet-4: 12 calls (71%)  |  claude-haiku: 5 (29%)  |
+-------------------------------------------------------------+
```

**Acceptance criteria:**
- Navigate to an agent's detail page.
- See a stats section/tab with cost data.
- If the agent has episodes: show aggregated cost, token counts, model breakdown.
- If the agent has no episodes: show zeros, not errors.
- Data refreshes on a 30-second interval.

---

### 3.6 Agent logs tab

**What:** Show agent execution logs on the agent detail page, with auto-tailing.

**Source files to read:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` -- `useAgentLogs(baseUrl, tail)` hook (already exists, polls agent sidecar)
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/AgentDetailPage.tsx`

**Target files to modify:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/AgentDetailPage.tsx`

**Target files to create (optional):**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/components/fleet/AgentLogsPanel.tsx`

**The hook already exists:**

```typescript
export function useAgentLogs(baseUrl: string | null | undefined, tail = 200) {
  // Returns { lines: string[] }
}
```

The `baseUrl` comes from the agent's `endpoints.rest` field. If the agent has
no sidecar endpoint, logs are unavailable.

Also check if roko-serve has a proxy route for logs. Search `agents.rs` for
`logs` or `proxy_agent_logs`. If a proxy exists at `/api/agents/{id}/logs`,
create a fallback hook that uses `fetchRoko` when the direct sidecar URL is
unavailable.

**UI implementation:**

Add a "Logs" tab to the agent detail page. Contents:

```
+---------------------------------------------------+
| [Auto-tail: ON]  [Clear]  [Tail: 200 v]          |
+---------------------------------------------------+
| 2024-04-24T10:32:01Z INFO agent started           |
| 2024-04-24T10:32:02Z INFO processing task T-001   |
| 2024-04-24T10:32:05Z DEBUG model call: sonnet-4   |
| ...                                               |
| 2024-04-24T10:33:12Z INFO task T-001 completed    |
+---------------------------------------------------+
```

Features:
- Monospace font, dark background
- Auto-tail: scroll to bottom on new lines (toggle button to pause)
- Tail count: dropdown to select 50/100/200/500 lines
- Clear: clear the displayed log (does not affect the server)
- Poll interval: the hook already polls every 3 seconds

**Acceptance criteria:**
- Navigate to an agent with a running sidecar.
- See "Logs" tab. Click it.
- Log lines appear, most recent at the bottom.
- New log lines appear as the agent works (3-second poll).
- Auto-tail keeps the view scrolled to the bottom.
- Click the auto-tail toggle: scrolling stops. New lines still arrive.
- Agent with no sidecar: show "Logs unavailable -- agent has no sidecar endpoint."

## Dependencies

- **Plan 01** (resilience): merged agents, connectivity gating.
- **Plan 02** (agent creation): agents must exist before they can be streamed.
  The agent detail page from Plan 02 (start/stop controls) is where the
  streaming UI attaches.

## Files touched (summary)

| Action | Path |
|--------|------|
| Create | `/Users/will/dev/nunchi/nunchi-dashboard/src/services/relayWs.ts` |
| Create | `/Users/will/dev/nunchi/nunchi-dashboard/src/components/fleet/LiveOutputPanel.tsx` |
| Create | `/Users/will/dev/nunchi/nunchi-dashboard/src/components/fleet/HeartbeatIndicator.tsx` |
| Create | `/Users/will/dev/nunchi/nunchi-dashboard/src/components/fleet/AgentCostPanel.tsx` |
| Create | `/Users/will/dev/nunchi/nunchi-dashboard/src/components/fleet/AgentLogsPanel.tsx` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/AgentDetailPage.tsx` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/stores/wsStore.ts` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/App.tsx` (or init file) |
| Modify | Fleet page agent cards |
