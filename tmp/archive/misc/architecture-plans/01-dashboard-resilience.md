# Plan 01: Dashboard resilience

**Layer:** 1
**Effort:** M (2-3 days)
**Depends on:** nothing

## Goal

The dashboard works fully with relay + mirage alone. When roko-serve is
unavailable, pages that depend on it show a placeholder instead of errors.
Pages that depend only on relay/mirage function without degradation.

## Current state

The dashboard has two independent API layers:

1. **mirage-api** (`/Users/will/dev/nunchi/nunchi-dashboard/src/services/mirage-api.ts`)
   -- talks to mirage-rs at `VITE_CHAIN_URL` (default `127.0.0.1:8545`).
   Provides on-chain agents, insights, tasks, pheromones, knowledge, and relay
   data. Does not depend on roko-serve.

2. **rokoApi** (`/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts`)
   -- talks to roko-serve at `VITE_ROKO_URL` (default same as CHAIN_URL, or
   overridden by `VITE_ROKO_API_URL`). Provides plans, PRDs, agents,
   experiments, learning data, jobs, and config.

The `connectivityStore` (`/Users/will/dev/nunchi/nunchi-dashboard/src/stores/connectivityStore.ts`)
already tracks whether roko-serve is online via a circuit breaker. It starts
pessimistic (`backendOnline: false`), flips to `true` on a successful probe,
and flips back to `false` after 3 consecutive failures.

`fetchRoko` in `rokoApi.ts` already short-circuits when `backendOnline` is
`false` for all paths except `/health`. It throws `"Backend offline"`.

**Problem:** React Query hooks that use `fetchRoko` still fire queries on a
15-30s interval. When roko-serve is down, every query throws, which produces
console errors and can trigger error boundaries. The error state propagates
into UI components as red banners or empty-state confusion. There is no visual
distinction between "roko-serve is down" (expected when no workspace is
connected) and "something is broken."

### Pages that need roko-serve

Read the nav sections at `/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/navSections.ts`:

| Section | Routes | Needs roko-serve? |
|---------|--------|-------------------|
| Pulse | `/app`, `/app/console`, `/app/stream`, `/app/pulse/network` | Partial -- command center mixes both |
| Fleet | `/app/fleet`, `/app/fleet/templates`, etc. | Partial -- agents come from both sources |
| Forge | `/app/forge/*` | Yes (plans, PRDs, research, execution, replay) |
| Knowledge | `/app/knowledge/*` | Partial -- mirage has its own knowledge API |
| Arena | `/app/arena/*` | Yes |
| Measurements | `/app/measurements/*` | Yes |
| Treasury | `/app/treasury/*` | Partial -- chain data is independent |
| Meta | `/app/meta/*` | Yes |
| System | `/app/system/*` | Yes |
| Settings | `/app/settings` | Yes for provider keys, no for theme/notifications |

## Tasks

### 1.1 Disable polling when roko-serve is offline — [x] DONE — implemented 2026-04-24

**What:** React Query hooks in `rokoApi.ts` should not fire at all when
`backendOnline` is `false`. The circuit breaker already blocks the fetch, but
the hooks still run, catch the thrown error, and mark the query as errored.
This causes React Query to retry, log console errors, and trigger error
boundaries.

**Source files to read:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` -- all `useQuery` hooks
- `/Users/will/dev/nunchi/nunchi-dashboard/src/stores/connectivityStore.ts` -- `backendOnline` state, `onlineInterval` helper

**Target files to modify:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts`

**Changes:**

The `connectivityStore` already exports `onlineInterval(ms)` which returns
`ms` when online and `false` when offline. But no hooks use it yet. The
`enabled` flag on each query also needs to gate on connectivity.

1. Add a helper at the top of `rokoApi.ts`:

```typescript
function useBackendOnline(): boolean {
  return useConnectivityStore((s) => s.backendOnline);
}
```

2. For every `useQuery` hook that calls `fetchRoko`, add:
   - `enabled: useBackendOnline() && <existing_enabled_condition>`
   - Replace `refetchInterval: N` with `refetchInterval: onlineInterval(N)`

Example -- `useAgents()` changes from:

```typescript
export function useAgents() {
  const q = useQuery({
    queryKey: queryKeys.agents,
    queryFn: () => fetchRoko<AgentSummary[]>("/managed-agents"),
    refetchInterval: 15_000,
    retry: 1,
  });
  ...
}
```

to:

```typescript
export function useAgents() {
  const online = useBackendOnline();
  const q = useQuery({
    queryKey: queryKeys.agents,
    queryFn: () => fetchRoko<AgentSummary[]>("/managed-agents"),
    enabled: online,
    refetchInterval: onlineInterval(15_000),
    retry: 1,
  });
  ...
}
```

3. Apply this pattern to every hook in the file. There are roughly 35 hooks.
   Hooks that already have an `enabled` condition (like `useAgent(id)`) should
   AND with online: `enabled: online && Boolean(id)`.

4. Import `onlineInterval` from `connectivityStore` at the top of the file
   (it is already exported but not imported).

**Acceptance criteria:**
- Start the dashboard WITHOUT roko-serve running.
- Open browser devtools console.
- Navigate to every page. No `fetchRoko` network requests fire. No console errors from failed fetches.
- Start roko-serve. Within 30 seconds, the health probe detects it, `backendOnline` flips to `true`, and queries begin firing.

---

### 1.2 Unified agent list with relay fallback — [x] DONE — implemented 2026-04-24

**What:** Create a `useMergedAgents()` hook that merges agents from relay and
roko-serve, with relay as the always-available source and roko-serve as an
optional enrichment layer.

**Source files to read:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/mirage-api.ts` -- `fetchRelayAgents()` function (search for `/relay/agents`), `RelayAgent` type
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` -- `useAgents()` hook, `AgentSummary` type
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/constants.ts` -- `RELAY_BASE`

**Target files to create:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/hooks/useMergedAgents.ts`

**Target files to modify:**
- Pages that import `useAgents` from rokoApi and should switch to `useMergedAgents`

**API contracts:**

Relay endpoint (always available):
```
GET {RELAY_BASE}/agents
Response: RelayAgent[]

type RelayAgent = {
  agent_id: string;
  label?: string;
  status: "connected" | "disconnected" | string;
  tier?: string;
  connected_at?: number;
  last_heartbeat?: number;
}
```

Roko-serve endpoint (optional):
```
GET /api/managed-agents
Response: AgentSummary[]

type AgentSummary = {
  id: string | number;
  label?: string;
  status?: string;
  tier?: string;
  model?: string;
  reputation?: number;
  skills?: string[];
  domain_tags?: string[];
}
```

**Implementation:**

```typescript
// /Users/will/dev/nunchi/nunchi-dashboard/src/hooks/useMergedAgents.ts

import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { RELAY_BASE } from "../services/constants";
import { useAgents } from "../services/rokoApi";
import { useConnectivityStore } from "../stores/connectivityStore";

export type MergedAgent = {
  id: string;
  label: string;
  status: string;
  source: "relay" | "roko" | "both";
  tier?: string;
  model?: string;
  reputation?: number;
  skills?: string[];
  connectedAt?: number;
  lastHeartbeat?: number;
};

function useRelayAgents() {
  return useQuery({
    queryKey: ["relay", "agents"],
    queryFn: async () => {
      const res = await fetch(`${RELAY_BASE}/agents`);
      if (!res.ok) throw new Error(`Relay agents: ${res.status}`);
      return res.json() as Promise<Array<{
        agent_id: string;
        label?: string;
        status?: string;
        tier?: string;
        connected_at?: number;
        last_heartbeat?: number;
      }>>;
    },
    refetchInterval: 10_000,
    retry: 2,
  });
}

export function useMergedAgents() {
  const relay = useRelayAgents();
  const roko = useAgents(); // already gated on backendOnline after task 1.1
  const online = useConnectivityStore((s) => s.backendOnline);

  const merged = useMemo(() => {
    const byId = new Map<string, MergedAgent>();

    // Relay agents are the base layer -- always present
    for (const agent of relay.data ?? []) {
      byId.set(agent.agent_id, {
        id: agent.agent_id,
        label: agent.label ?? agent.agent_id,
        status: agent.status ?? "unknown",
        source: "relay",
        tier: agent.tier,
        connectedAt: agent.connected_at,
        lastHeartbeat: agent.last_heartbeat,
      });
    }

    // Overlay roko-serve data when available
    if (online) {
      for (const agent of roko.data ?? []) {
        const id = String(agent.id);
        const existing = byId.get(id);
        if (existing) {
          existing.source = "both";
          existing.model = (agent as Record<string, unknown>).model as string | undefined;
          existing.reputation = agent.reputation;
          existing.skills = agent.skills;
          if (agent.tier) existing.tier = agent.tier;
        } else {
          byId.set(id, {
            id,
            label: agent.label ?? id,
            status: agent.status ?? "registered",
            source: "roko",
            tier: agent.tier,
            model: (agent as Record<string, unknown>).model as string | undefined,
            reputation: agent.reputation,
            skills: agent.skills,
          });
        }
      }
    }

    return Array.from(byId.values());
  }, [relay.data, roko.data, online]);

  return {
    data: merged,
    isLoading: relay.isLoading,
    isError: relay.isError,
    relayOnline: !relay.isError,
    rokoOnline: online && !roko.isError,
  };
}
```

**Acceptance criteria:**
- With both relay and roko-serve running: merged list shows agents from both, no duplicates by ID, relay liveness data present on all entries.
- With only relay running: list shows relay agents. No errors in console.
- With only roko-serve running: list shows roko agents (unusual but should work).
- Fleet page (`/app/fleet`) uses `useMergedAgents` and renders agents regardless of which backends are up.

---

### 1.3 Conditional UI sections (RequiresWorkspace wrapper) — [x] DONE — implemented 2026-04-24

**What:** Pages that depend entirely on roko-serve show a friendly placeholder
when no workspace is connected, instead of error states or empty tables.

**Source files to read:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/stores/connectivityStore.ts` -- `useConnectivityStore`, `backendOnline`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/navSections.ts` -- section/page inventory

**Target files to create:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/components/RequiresWorkspace.tsx`

**Target files to modify:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/forge/ForgePage.tsx`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/forge/PlansPage.tsx`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/forge/PrdsPage.tsx`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/forge/ResearchPage.tsx`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/forge/ExecutionPage.tsx`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/forge/ReplayPage.tsx`
- Any other pages that are pure roko-serve consumers (arena, measurements, meta, system)

**Implementation:**

```tsx
// /Users/will/dev/nunchi/nunchi-dashboard/src/components/RequiresWorkspace.tsx

import { useConnectivityStore } from "../stores/connectivityStore";

interface Props {
  children: React.ReactNode;
  feature?: string; // e.g. "plans, PRDs, and learning"
}

export function RequiresWorkspace({ children, feature }: Props) {
  const online = useConnectivityStore((s) => s.backendOnline);

  if (online) return <>{children}</>;

  return (
    <div className="flex items-center justify-center min-h-[60vh]">
      <div className="text-center max-w-md space-y-4">
        <div className="text-4xl opacity-30">&#x1f50c;</div>
        <h2 className="text-lg font-medium text-zinc-200">
          Connect a Roko workspace
        </h2>
        <p className="text-sm text-zinc-500">
          {feature
            ? `Access ${feature} by connecting a workspace running roko serve.`
            : "This page requires a connected workspace running roko serve."}
        </p>
        <p className="text-xs text-zinc-600 font-mono">
          roko serve --port 6677
        </p>
      </div>
    </div>
  );
}
```

NOTE: The emoji is a placeholder for an icon. Replace `&#x1f50c;` with a
lucide-react `Plug` or `Unplug` icon to match the design system. The
implementer should check what icon library is in use (lucide-react per
navSections.ts) and use that.

**Wrap each roko-dependent page:**

```tsx
// Example: ForgePage.tsx
import { RequiresWorkspace } from "../../components/RequiresWorkspace";

export default function ForgePage() {
  return (
    <RequiresWorkspace feature="plans, PRDs, and research">
      {/* existing page content */}
    </RequiresWorkspace>
  );
}
```

Pages that mix relay and roko data (Fleet, Pulse command center) should NOT
use this wrapper. They should render their relay data and show inline
indicators for missing roko sections.

**Which pages to wrap:**

| Page | Wrap? | Reason |
|------|-------|--------|
| Forge/* (PRDs, Plans, Research, Execution, Replay) | Yes | 100% roko-serve |
| Arena/* | Yes | 100% roko-serve |
| Measurements/* | Yes | 100% roko-serve |
| Meta/* | Yes | 100% roko-serve |
| System/Providers, Jobs, Extensions, Build, Gates, Delegations, Context Audit | Yes | 100% roko-serve |
| Settings (provider keys section only) | Partial | Theme/notifications work offline |
| Fleet | No | Uses merged agents from task 1.2 |
| Pulse | No | Command center mixes sources |
| Knowledge | No | mirage-api has its own knowledge endpoints |
| Treasury | No | Chain data is independent |

**Acceptance criteria:**
- Start dashboard without roko-serve.
- Navigate to `/app/forge/plans`. See the workspace placeholder, not an error.
- Navigate to `/app/fleet`. See relay agents, no errors.
- Start roko-serve. Navigate back to `/app/forge/plans`. See plans list.
- No console errors on any page.

---

### 1.4 Connection status indicator in sidebar — [x] DONE — implemented 2026-04-24

**What:** Add a visual indicator to the sidebar or header showing connectivity
status for both relay and roko-serve.

**Source files to read:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/AppLayout.tsx` -- main layout with sidebar
- `/Users/will/dev/nunchi/nunchi-dashboard/src/stores/connectivityStore.ts` -- `backendOnline` for roko-serve
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/mirage-api.ts` -- search for any existing relay health check or use the `/relay/health` endpoint

**Target files to create:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/components/ConnectionStatus.tsx`

**Target files to modify:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/AppLayout.tsx` -- render `ConnectionStatus` in sidebar footer or header

**Implementation:**

```tsx
// /Users/will/dev/nunchi/nunchi-dashboard/src/components/ConnectionStatus.tsx

import { useQuery } from "@tanstack/react-query";
import { RELAY_BASE } from "../services/constants";
import { useConnectivityStore } from "../stores/connectivityStore";

function useRelayHealth() {
  return useQuery({
    queryKey: ["relay", "health"],
    queryFn: async () => {
      const res = await fetch(`${RELAY_BASE}/health`);
      return res.ok;
    },
    refetchInterval: 15_000,
    retry: 1,
  });
}

function Dot({ on }: { on: boolean }) {
  return (
    <span
      className={`inline-block w-2 h-2 rounded-full ${
        on ? "bg-emerald-500" : "bg-zinc-600"
      }`}
    />
  );
}

export function ConnectionStatus() {
  const relayHealth = useRelayHealth();
  const rokoOnline = useConnectivityStore((s) => s.backendOnline);

  const relayOk = relayHealth.data === true;

  return (
    <div className="flex items-center gap-3 text-xs text-zinc-500 px-3 py-2">
      <span className="flex items-center gap-1.5">
        <Dot on={relayOk} />
        Relay
      </span>
      <span className="flex items-center gap-1.5">
        <Dot on={rokoOnline} />
        Workspace
      </span>
    </div>
  );
}
```

**Placement:** Add `<ConnectionStatus />` to the bottom of the sidebar in
`AppLayout.tsx`, below the nav sections and above any footer content. Read
the layout file to find the right insertion point -- look for the sidebar
`<nav>` or `<aside>` element's closing tag.

**Acceptance criteria:**
- With both relay and roko-serve running: two green dots labeled "Relay" and "Workspace."
- With only relay running: Relay green, Workspace gray.
- With nothing running: both gray.
- Kill roko-serve while dashboard is open: Workspace dot turns gray within 30 seconds (the probe interval).
- Start roko-serve while dashboard is open: Workspace dot turns green within 30 seconds.

### 1.5 Workspace auto-discovery via relay — [x] DONE — implemented 2026-04-24

**What:** Roko instances register with the relay on startup. Dashboard discovers
available workspaces automatically — no manual URL entry, no env var config.

**Architecture reference:** `04-connectivity.md` — "Workspace discovery" section.

**Source files to read (roko backend):**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs` — server startup, find where to add relay registration
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` — search for `[relay]` config section, add if missing
- `/Users/will/dev/nunchi/roko/roko/apps/agent-relay/src/main.rs` — relay server, add workspace directory alongside agent directory

**Source files to read (dashboard):**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/mirage-api.ts` — relay base URL, add workspace query
- `/Users/will/dev/nunchi/nunchi-dashboard/src/stores/connectivityStore.ts` — add workspace URL state

**Changes needed — roko-serve (backend):** DONE

1. [x] On startup, if `[relay].url` is configured in roko.toml, register with relay
   via `POST /relay/workspaces/register` with workspace_id, name, url, version,
   agents_count. Auto-detects public URL from RAILWAY_PUBLIC_DOMAIN / FLY_APP_NAME.
   **Implemented in**: `crates/roko-serve/src/relay.rs` (`start_workspace_registration`)
   **Wired in**: `crates/roko-serve/src/lib.rs` (line ~225)
2. [x] Send periodic workspace heartbeat (every 30s) with updated agent_count.
   **Implemented in**: `crates/roko-serve/src/relay.rs` (heartbeat loop)
3. [ ] On shutdown, send `workspace_goodbye`. (Not yet — graceful shutdown not wired, stale expiry handles this)

**Changes needed — agent-relay (backend):** DONE

1. [x] Add workspace directory (HashMap<String, ConnectedWorkspace>) alongside agent directory.
   **Implemented in**: `apps/agent-relay/src/state.rs` (`RelayStateInner.workspaces`)
2. [x] Handle `workspace_hello` — upsert entry.
   **Implemented in**: `apps/agent-relay/src/state.rs` (`register_workspace`)
3. [x] Handle `workspace_goodbye` — remove entry.
   **Implemented in**: `apps/agent-relay/src/state.rs` (`unregister_workspace`)
4. [x] Remove stale entries after 60s without heartbeat.
   **Implemented in**: `apps/agent-relay/src/main.rs` (background expiry task)
5. [x] Add `GET /relay/workspaces` REST endpoint — list online workspaces.
   **Implemented in**: `apps/agent-relay/src/lib.rs` (`list_workspaces`)
6. [x] Broadcast `workspace_connected` / `workspace_disconnected` events on
   `/relay/events/ws`.
   **Implemented in**: `apps/agent-relay/src/protocol.rs` (`RelayEvent::WorkspaceConnected/Disconnected`)

**Config types added:** DONE

- [x] `RelayConfig` struct in `crates/roko-core/src/config/schema.rs`
  Fields: `url`, `workspace_name`, `public_url`, `heartbeat_interval_secs`
- [x] `WorkspaceHello`, `ConnectedWorkspace` types in `apps/agent-relay/src/protocol.rs`
- [x] `agent_count: Arc<AtomicU32>` added to `AppState` in `crates/roko-serve/src/state.rs`

**Changes needed — dashboard (frontend):**

1. On load, fetch `GET /relay/workspaces`.
2. If user has Privy wallet, auto-match by `owner_wallet`.
3. If exactly one match: auto-set as roko-serve URL in connectivityStore.
4. If multiple: show a minimal picker modal.
5. If none: proceed with relay-only mode (no errors).
6. Subscribe to workspace events — auto-connect when a new workspace comes online.
7. User can still manually set URL in Settings as fallback.

```tsx
// /Users/will/dev/nunchi/nunchi-dashboard/src/hooks/useWorkspaceDiscovery.ts

export function useWorkspaceDiscovery() {
  const { data: workspaces } = useQuery({
    queryKey: ["relay", "workspaces"],
    queryFn: () => fetch(`${RELAY_BASE}/workspaces`).then(r => r.json()),
    refetchInterval: 30_000,
  });

  const wallet = usePrivyWallet();
  const setRokoUrl = useConnectivityStore((s) => s.setRokoUrl);

  useEffect(() => {
    if (!workspaces?.length) return;

    // Auto-match by wallet
    const mine = workspaces.filter(
      (w: any) => wallet && w.owner_wallet === wallet.address
    );

    if (mine.length === 1) {
      setRokoUrl(mine[0].url);
    }
    // If multiple, show picker (handled elsewhere)
  }, [workspaces, wallet]);

  return { workspaces, myWorkspaces: workspaces?.filter(/* ... */) };
}
```

**Config (roko.toml):**
```toml
[relay]
url = "wss://relay.nunchi.dev"
workspace_name = "will-dev"
# public_url is auto-detected from Railway/Fly env vars,
# or set manually for custom deployments:
# public_url = "https://my-roko.up.railway.app"
```

For Railway deployments, `public_url` is auto-detected from the `RAILWAY_PUBLIC_DOMAIN`
env var. For Fly, from `FLY_APP_NAME`. For local dev, it defaults to
`http://localhost:{port}`.

**Acceptance criteria:**
- Deploy roko on Railway with `[relay].url` set → workspace appears in relay directory.
- Open dashboard → workspace auto-connected (green dot, workspace features available).
- Stop roko → workspace disappears from relay within 60s → dashboard degrades gracefully.
- Start roko again → workspace reappears → dashboard auto-reconnects.
- Two roko instances with same wallet → dashboard shows picker.
- No roko instances → dashboard shows relay-only view, no errors.

---

## Dependencies

None. This is the foundation layer. All other plans depend on this plan
shipping first.

## Files touched (summary)

| Action | Path |
|--------|------|
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` |
| Create | `/Users/will/dev/nunchi/nunchi-dashboard/src/hooks/useMergedAgents.ts` |
| Create | `/Users/will/dev/nunchi/nunchi-dashboard/src/components/RequiresWorkspace.tsx` |
| Create | `/Users/will/dev/nunchi/nunchi-dashboard/src/components/ConnectionStatus.tsx` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/AppLayout.tsx` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/forge/ForgePage.tsx` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/forge/PlansPage.tsx` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/forge/PrdsPage.tsx` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/forge/ResearchPage.tsx` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/forge/ExecutionPage.tsx` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/forge/ReplayPage.tsx` |
| Create | `/Users/will/dev/nunchi/nunchi-dashboard/src/hooks/useWorkspaceDiscovery.ts` |
| Modify | `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs` |
| Modify | `/Users/will/dev/nunchi/roko/roko/apps/agent-relay/src/main.rs` |
| Modify | `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` |
| Modify | Additional pages under arena, measurements, meta, system |
