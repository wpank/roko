# A8: Integration and polish — WS invalidation, right panel, responsive, audit

## Context

**Repo:** `/Users/will/dev/nunchi/nunchi-dashboard`
**Branch:** `demo-rewrite`
**Tech stack:** React 19 + Vite 8 + TypeScript + Tailwind CSS v4
**Backend:** `roko-serve` runs at `http://localhost:6677` with ~85 REST routes + WebSocket at `ws://localhost:6677/ws`
**Auth:** Privy (env var `VITE_PRIVY_APP_ID`) with password fallback
**Design:** ROSEDUST dark palette — bg_void `#060608`, rose `#AA7088`, bone `#C8B890`, rose_bright `#CC90A8`

### Before starting

1. `cd /Users/will/dev/nunchi/nunchi-dashboard`
2. `git checkout -b demo-rewrite 2>/dev/null || git checkout demo-rewrite`
3. `npm install`
4. Verify: `npm run dev` starts without errors

### After every task

1. `npm run typecheck` passes
2. `npm run dev` — page renders without console errors
3. All existing tests pass: `npm test` (if test runner is configured)

---

## What this task produces

The glue layer that makes everything feel live. Five deliverables:

1. A WS event handler that invalidates TanStack Query caches when relevant events arrive.
2. A real, route-aware right panel with live data replacing the mock content from A1.
3. Responsive breakpoint handling so the layout works on smaller screens.
4. A systematic audit of loading, error, and empty states across every page.
5. A reconnect toast and keyboard shortcut registration.

**Depends on:** Tasks A1–A7 (all pages must exist).

---

## Checklist

### 1. WS event → query invalidation

When a WebSocket event arrives, invalidate the relevant query cache so pages refresh without polling delay. This is the single most important integration point in the dashboard.

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/services/wsInvalidation.ts`:

```ts
import type { QueryClient } from "@tanstack/react-query";
import { queryKeys } from "./queryKeys";
import type { WsEvent } from "../stores/wsStore";
import { useWsStore } from "../stores/wsStore";

/**
 * Maps WS event types to the query keys that should be invalidated.
 *
 * When roko-serve emits a WebSocket event, every matching query is marked
 * stale so TanStack Query refetches it on the next render cycle.
 *
 * Keep this map in sync with:
 * - B2 job events: job_created, job_state_changed, job_submitted, job_evaluated
 * - Server heartbeat/status events
 * - Orchestrator events: run_*, plan_*, gate_result, agent_output
 */
const EVENT_INVALIDATION_MAP: Readonly<Record<string, ReadonlyArray<readonly string[]>>> = {
  // Orchestrator events
  run_started:         [queryKeys.status],
  run_completed:       [queryKeys.status, queryKeys.plans, queryKeys.agents],
  plan_started:        [queryKeys.plans],
  plan_completed:      [queryKeys.plans, queryKeys.status],
  gate_result:         [queryKeys.plans, queryKeys.adaptiveThresholds],
  agent_output:        [queryKeys.agents],
  operation_completed: [queryKeys.plans, queryKeys.prds, queryKeys.status],

  // Job marketplace events (B2)
  job_created:        [queryKeys.jobs],
  job_state_changed:  [queryKeys.jobs],
  job_submitted:      [queryKeys.jobs],
  job_evaluated:      [queryKeys.jobs],

  // Heartbeat: refresh agent list and health
  heartbeat: [queryKeys.agents, queryKeys.health],

  // Error events do not invalidate queries — just log them.
  error: [],
};

/** Process a single WS event and invalidate any matching caches. */
function handleEvent(queryClient: QueryClient, event: WsEvent): void {
  const keys = EVENT_INVALIDATION_MAP[event.type];
  // Unknown event types are silently ignored — they may be added by future
  // server versions.
  if (!keys) return;

  for (const key of keys) {
    queryClient.invalidateQueries({ queryKey: [...key] });
  }
}

/**
 * Subscribe to the WS store and invalidate TanStack Query caches when
 * relevant events arrive.
 *
 * Call once at app startup (e.g., in a `useEffect` in App.tsx).
 * Returns an unsubscribe function — call it on unmount.
 *
 * @example
 * ```ts
 * useEffect(() => {
 *   const unsub = subscribeWsInvalidation(queryClient);
 *   return unsub;
 * }, []);
 * ```
 */
export function subscribeWsInvalidation(queryClient: QueryClient): () => void {
  let processedCount = 0;

  return useWsStore.subscribe((state) => {
    const currentCount = state.events.length;
    if (currentCount <= processedCount) return;

    const newEvents = state.events.slice(processedCount);
    processedCount = currentCount;

    for (const event of newEvents) {
      handleEvent(queryClient, event);
    }
  });
}
```

- [ ] Wire into `App.tsx`. Replace the full file at `/Users/will/dev/nunchi/nunchi-dashboard/src/App.tsx`:

```tsx
import { useEffect, useRef } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "react-router-dom";
import { ToastProvider } from "./design-system/components";
import { router } from "./router";
import { connectWs, disconnectWs } from "./services/ws";
import { subscribeWsInvalidation } from "./services/wsInvalidation";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      retry: 2,
      refetchOnWindowFocus: false,
    },
  },
});

export default function App() {
  const unsubRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    connectWs();
    unsubRef.current = subscribeWsInvalidation(queryClient);
    return () => {
      disconnectWs();
      unsubRef.current?.();
    };
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <ToastProvider>
        <RouterProvider router={router} />
      </ToastProvider>
    </QueryClientProvider>
  );
}
```

### 2. Right panel — route-aware live data

The right panel shows different content depending on the current route. On most routes it shows WS activity, C-Factor, health, and active agents. On the chat route it collapses to a minimal status strip.

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/RightPanel.tsx`:

```tsx
import { useLocation } from "react-router-dom";
import { useCFactor, useAgents, useHealth } from "../services/api";
import { useWsStore } from "../stores/wsStore";
import { Card, Gauge, Skeleton, Sparkline, StatusDot } from "../design-system/components";

/** Build event-rate buckets for the sparkline: N events per 10-second window. */
function buildEventBuckets(events: { receivedAt: number }[], buckets = 12): number[] {
  const now = Date.now();
  return Array.from({ length: buckets }, (_, i) => {
    const start = now - (buckets - i) * 10_000;
    const end = start + 10_000;
    return events.filter((e) => e.receivedAt >= start && e.receivedAt < end).length;
  });
}

function formatUptime(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

export function RightPanel() {
  const location = useLocation();
  const { data: cfactor, isLoading: cfLoading } = useCFactor();
  const { data: agents, isLoading: agLoading } = useAgents();
  const { data: health } = useHealth();
  const { connected, events, lastEventAt } = useWsStore();

  const eventBuckets = buildEventBuckets(events);
  const isChatRoute = location.pathname === "/app/chat";

  return (
    <div className="p-4 space-y-4">
      {/* WebSocket connection status */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <StatusDot status={connected ? "online" : "offline"} />
          <span className="text-[10px] text-[var(--rd-fg-muted)]">
            {connected ? "WS connected" : "WS disconnected"}
          </span>
        </div>
        {lastEventAt != null && (
          <span className="text-[9px] font-mono text-[var(--rd-fg-muted)]">
            {new Date(lastEventAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })}
          </span>
        )}
      </div>

      {/* Show only minimal status on the chat route to avoid visual crowding. */}
      {!isChatRoute && (
        <>
          {/* Event rate sparkline (2-minute window) */}
          <div>
            <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-1">
              Event rate (2 min)
            </div>
            <Sparkline data={eventBuckets} width={240} height={24} color="var(--rd-rose)" />
          </div>

          {/* C-Factor */}
          <Card padding="sm">
            <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-1">
              C-Factor
            </div>
            {cfLoading ? (
              <Skeleton height="2rem" />
            ) : cfactor ? (
              <>
                <div className="text-2xl font-mono text-[var(--rd-rose-bright)]">
                  {(cfactor.cfactor_pct ?? 0) >= 0 ? "+" : ""}
                  {typeof cfactor.cfactor_pct === "number"
                    ? cfactor.cfactor_pct.toFixed(1)
                    : "0"}%
                </div>
                <div className="text-[10px] text-[var(--rd-fg-muted)] mt-0.5">
                  Fleet: {cfactor.fleet_cfactor?.toFixed(3) ?? "—"} ·{" "}
                  Solo: {cfactor.solo_avg?.toFixed(3) ?? "—"}
                </div>
              </>
            ) : (
              <div className="text-sm text-[var(--rd-fg-muted)]">—</div>
            )}
          </Card>

          {/* Backend health */}
          <Card padding="sm">
            <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-1">
              Backend
            </div>
            <div className="flex items-center gap-2">
              <StatusDot status={health ? "online" : "offline"} />
              <span className="text-xs text-[var(--rd-fg-secondary)]">
                {health ? `roko-serve ${health.status}` : "Offline"}
              </span>
            </div>
            {health?.uptime_secs != null && (
              <div className="text-[10px] text-[var(--rd-fg-muted)] mt-0.5">
                Uptime: {formatUptime(health.uptime_secs)}
              </div>
            )}
          </Card>

          {/* Active agents */}
          <div>
            <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-2">
              Active agents
            </div>
            {agLoading ? (
              <div className="space-y-1.5">
                <Skeleton height="28px" />
                <Skeleton height="28px" />
              </div>
            ) : agents && agents.length > 0 ? (
              <div className="space-y-1">
                {agents.map((agent) => (
                  <div
                    key={agent.id}
                    className="flex items-center gap-2 px-2 py-1.5 rounded-md bg-[var(--rd-bg-surface-1)]"
                  >
                    <StatusDot status="online" />
                    <span className="text-xs text-[var(--rd-fg-secondary)] truncate flex-1">
                      {agent.label || `Agent ${agent.id}`}
                    </span>
                    <span className="text-[9px] font-mono text-[var(--rd-fg-muted)]">
                      PID {agent.id}
                    </span>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-xs text-[var(--rd-fg-muted)] px-2">
                No agents running
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
```

- [ ] Update `/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/AppLayout.tsx`:

  1. Add import: `import { RightPanel } from "./RightPanel";`
  2. Find the `{/* MOCK: C-Factor, ISFR, agent cards — wire in A8 */}` block inside the right-panel aside and replace the entire aside inner content with `<RightPanel />`.

  The full right-panel aside should become:
  ```tsx
  {rightPanelVisible && (
    <aside className="fixed right-0 top-14 bottom-0 w-[280px] bg-[var(--rd-bg-surface-0)] border-l border-[var(--rd-bg-surface-2)] z-30 overflow-y-auto">
      <RightPanel />
    </aside>
  )}
  ```

### 3. Responsive breakpoints

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/hooks/useMediaQuery.ts`:

```ts
import { useEffect, useState } from "react";

/**
 * Returns whether a CSS media query currently matches.
 * Updates reactively on viewport resize.
 *
 * @example
 * ```ts
 * const isSmall = useMediaQuery("(max-width: 1024px)");
 * ```
 */
export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState<boolean>(() => {
    if (typeof window === "undefined") return false;
    return window.matchMedia(query).matches;
  });

  useEffect(() => {
    const mql = window.matchMedia(query);
    const handler = (e: MediaQueryListEvent) => setMatches(e.matches);
    mql.addEventListener("change", handler);
    // Sync immediately in case the viewport changed between render and effect.
    setMatches(mql.matches);
    return () => mql.removeEventListener("change", handler);
  }, [query]);

  return matches;
}
```

- [ ] Update `/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/AppLayout.tsx`:

  Add after imports:
  ```tsx
  import { useMediaQuery } from "../hooks/useMediaQuery";
  ```

  Inside the component, add breakpoint detectors:
  ```tsx
  const isSmallScreen  = useMediaQuery("(max-width: 1024px)");
  const isMediumScreen = useMediaQuery("(max-width: 1280px)");
  ```

  Add responsive effects — run after the hook declarations:
  ```tsx
  // Auto-collapse the sidebar on small screens.
  useEffect(() => {
    if (isSmallScreen && !sidebarCollapsed) {
      toggleSidebar();
    }
    // toggleSidebar is stable; omitting from deps is intentional.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isSmallScreen]);

  // Hide the right panel on medium screens.
  useEffect(() => {
    if (isMediumScreen && rightPanelVisible) {
      toggleRightPanel();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isMediumScreen]);
  ```

### 4. Contrast verification

Verify every text/background color pair meets WCAG AA (4.5:1 for body text, 3:1 for large text / UI elements). Check these pairs — computed ratios are approximations based on the hex values:

| Foreground | Background | Hex pair | Approx. ratio | WCAG AA |
|---|---|---|---|---|
| `--rd-fg-primary` (#E8E4DE) | `--rd-bg-surface-1` (#141418) | #E8E4DE / #141418 | ~14:1 | Pass |
| `--rd-fg-secondary` (#9B9590) | `--rd-bg-surface-1` (#141418) | #9B9590 / #141418 | ~5.6:1 | Pass |
| `--rd-fg-muted` (#6B655F) | `--rd-bg-surface-1` (#141418) | #6B655F / #141418 | ~3.2:1 | Pass (large text / UI) |
| `--rd-fg-muted` (#6B655F) | `--rd-bg-surface-0` (#0E0E12) | #6B655F / #0E0E12 | ~2.9:1 | **Borderline — lighten muted** |
| `--rd-rose-bright` (#CC90A8) | `--rd-bg-void` (#060608) | #CC90A8 / #060608 | ~6.5:1 | Pass |
| White (#FFFFFF) | `--rd-rose` (#AA7088) | #FFFFFF / #AA7088 | ~4.7:1 | Pass |
| `--rd-bone` (#C8B890) | `--rd-bg-surface-1` (#141418) | #C8B890 / #141418 | ~8.4:1 | Pass |

**If `--rd-fg-muted` on `--rd-bg-surface-0` fails (ratio < 3:1):** lighten `--rd-fg-muted` from `#6B655F` to `#7A746E` in the CSS variables file. Recheck after the change.

Additional pairs to verify manually (open the Color Contrast Analyser tool or browser DevTools):
- Badge text on badge background for each variant (rose, success, warning, danger, info)
- Input placeholder text on input background
- Disabled button text on disabled button background

### 5. Loading / error / empty state audit

Walk every page and verify three states render correctly. Fix any page that is missing a state.

| Route | Loading | Error | Empty |
|---|---|---|---|
| `/app/observatory/agents` | Skeleton rows | Retry button | "No agents running" |
| `/app/observatory/plans` | Skeleton rows | Retry button | "No plans found" |
| `/app/observatory/learning` | Skeleton grid | — (graceful degradation, no crash) | Zero-value cards |
| `/app/observatory/conductor` | Skeletons | Retry | "No diagnoses" |
| `/app/observatory/costs` | Skeletons | — | "No cost data" |
| `/app/network/agents` | Skeleton | Retry | "No agents in the network" |
| `/app/network/pheromones` | — | — | Always renders (heatmap with type selector) |
| `/app/network/knowledge` | — | — | Search empty: "No matches" |
| `/app/marketplace` | Skeletons | — | "No jobs match your filters" |
| `/app/marketplace/:id` | Skeleton | — | Not found: "Job not found" + back button |
| `/app/atelier/prds` | Skeletons | Retry | "No PRDs found" |
| `/app/atelier/execution` | — | — | "No execution events yet" message |
| `/app/settings` | Skeleton | Retry | — |
| `/app/chat` | — | — | Centered logo + prompt |
| `/app/research` | — | — | "No research yet" in history section |

- [ ] For each page in the table, confirm the state renders correctly in the browser.
- [ ] Fix any page that crashes or shows a blank screen in any of the three states.

### 6. Toast on WS reconnect

Show a toast when the WebSocket reconnects after a disconnect. The first connection is not toasted — only reconnects.

- [ ] Add a custom event dispatch to `/Users/will/dev/nunchi/nunchi-dashboard/src/services/ws.ts`.

  In the `handleOpen` callback, after setting `connected = true`, dispatch a reconnect event:
  ```ts
  // Dispatch only on reconnects, not the initial connection.
  // The store's `connected` flag starts as false, so a transition from
  // false → true in handleOpen is always a connect or reconnect.
  // The hook below tracks first-connect state to skip the initial event.
  window.dispatchEvent(new CustomEvent("ws-reconnected"));
  ```

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/hooks/useWsReconnectToast.ts`:

```ts
import { useEffect, useRef } from "react";
import { useToast } from "../design-system/components";

/**
 * Shows a toast when the WebSocket reconnects after a disconnect.
 *
 * The very first connection is not toasted — only subsequent reconnects.
 * Mount this hook once in `AppLayout`.
 */
export function useWsReconnectToast(): void {
  const { toast } = useToast();
  const isFirstConnection = useRef(true);

  useEffect(() => {
    function handleReconnect() {
      if (isFirstConnection.current) {
        isFirstConnection.current = false;
        return;
      }
      toast("WebSocket reconnected", "success");
    }

    window.addEventListener("ws-reconnected", handleReconnect);
    return () => window.removeEventListener("ws-reconnected", handleReconnect);
  }, [toast]);
}
```

- [ ] Call `useWsReconnectToast()` in `AppLayout` (inside the component body, before the return statement).

### 7. Keyboard shortcuts

Register global keyboard shortcuts so power users can navigate without the mouse.

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/hooks/useKeyboardShortcuts.ts`:

```ts
import { useEffect } from "react";
import { useNavigate } from "react-router-dom";

type Shortcut = {
  key: string;
  /** Whether the shortcut requires no modifier keys. */
  plain?: boolean;
  action: (navigate: ReturnType<typeof useNavigate>) => void;
};

const SHORTCUTS: Shortcut[] = [
  { key: "/", plain: true, action: () => {
    // Focus the global search input if it exists.
    const el = document.querySelector<HTMLInputElement>("[data-global-search]");
    el?.focus();
  }},
];

/**
 * Registers application-level keyboard shortcuts.
 *
 * - `/` focuses the global search input.
 * - `Escape` blurs the currently focused input.
 *
 * Shortcuts are suppressed when focus is inside an input, textarea, or
 * contenteditable element to avoid interfering with typing.
 */
export function useKeyboardShortcuts(): void {
  const navigate = useNavigate();

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      const target = e.target as HTMLElement;
      const inInput =
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable;

      if (e.key === "Escape" && inInput) {
        (target as HTMLElement).blur();
        return;
      }

      if (inInput) return;

      for (const shortcut of SHORTCUTS) {
        const modifierFree = !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey;
        if (shortcut.plain && !modifierFree) continue;
        if (e.key === shortcut.key) {
          e.preventDefault();
          shortcut.action(navigate);
          return;
        }
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [navigate]);
}
```

- [ ] Call `useKeyboardShortcuts()` in `AppLayout`.

---

## Verification

Run from `/Users/will/dev/nunchi/nunchi-dashboard`:

- [ ] `npm run typecheck` — exits 0

**With roko-serve running (`cargo run -p roko-cli -- serve`):**

- [ ] Right panel shows real C-Factor value (or "—" if no data yet)
- [ ] Right panel shows real agent list or "No agents running"
- [ ] WS status indicator shows "WS connected" with green dot
- [ ] Event rate sparkline animates as events arrive
- [ ] Navigate to the Plans page — when a plan completes via WS event, the page auto-refreshes without a manual reload
- [ ] On `/app/chat`, the right panel shows only the WS status strip (no full panel content)

**Without roko-serve running:**

- [ ] Right panel shows "Offline" for backend, "WS disconnected" with grey dot
- [ ] All pages show appropriate empty/error states
- [ ] No crashes, no uncaught promise rejections in the console

**Responsive behavior:**

- [ ] Resize to 1024px wide — left nav auto-collapses to icon-only mode
- [ ] Resize to 1280px — right panel hides, left nav stays full width
- [ ] Main content fills available space at all breakpoints

**Interactions:**

- [ ] Kill roko-serve, then restart it — toast "WebSocket reconnected" appears and green dot returns
- [ ] Press `/` while not in an input — global search input (if present) receives focus
- [ ] Press `Escape` inside a text input — input loses focus
- [ ] No console errors at any point
