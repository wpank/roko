# A2: API layer -- types, query hooks, WebSocket client

## Context

**Repo:** `/Users/will/dev/nunchi/nunchi-dashboard`
**Branch:** `demo-rewrite`
**Tech stack:** React 19 + Vite 8 + TypeScript + Tailwind CSS v4
**Backend:** `roko-serve` runs at `http://localhost:6677` with ~85 REST routes + WebSocket at `ws://localhost:6677/ws`
**Auth:** Privy (env var `VITE_PRIVY_APP_ID`) with password fallback
**Design:** ROSEDUST dark palette -- bg_void `#060608`, rose `#AA7088`, bone `#C8B890`, rose_bright `#CC90A8`

### Before starting
1. `cd /Users/will/dev/nunchi/nunchi-dashboard`
2. `git checkout -b demo-rewrite 2>/dev/null || git checkout demo-rewrite`
3. `npm install`
4. Verify: `npm run dev` starts without errors

### After every task
1. `npm run typecheck` passes
2. `npm run dev` -- page renders without console errors
3. All existing tests pass: `npm test` (if test runner is configured)

---

## What this task produces

Three files that every page in the dashboard imports:
1. `types/api.ts` -- TypeScript types matching every `roko-serve` JSON response
2. `services/api.ts` -- `fetchApi` helper + TanStack Query hooks + mutations
3. `services/ws.ts` -- WebSocket client with reconnect logic, pushing events into the Zustand `wsStore`

A fourth file, `services/queryKeys.ts`, provides a typed key factory so queries can be invalidated from the WS event handler without circular imports.

**Depends on:** Task A1 (stores must exist). The Zustand stores from A1 (`wsStore`, `authStore`) are imported here.

---

## Checklist

### 1. Create API types

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/types/api.ts`:

```ts
// ─── Health & Status ─────────────────────────────────────────────────────────
//
// Source: GET /api/health in roko-serve/src/routes/status.rs
// The handler always returns status: "ok". The "degraded" / "down" variants
// are reserved for future use (circuit-breaker state).

export type HealthResponse = {
  status: "ok" | "degraded" | "down";
  version: string;
  uptime_secs: number;
  active_plans: number;
  active_agents: number;
};

// Source: GET /api/status in roko-serve/src/routes/status.rs
export type SessionStatus = {
  session_id: string | null;
  workdir: string;
  daemon_running: boolean;
  signal_count: number | null;
  episode_count: number | null;
  last_episode_passed: boolean | null;
};

// ─── Agents ──────────────────────────────────────────────────────────────────

export type AgentEndpoints = {
  rest: string | null;
  websocket: string | null;
  a2a: string | null;
  mcp: string | null;
};

export type Agent = {
  agent_id: string;
  label: string | null;
  process_id: number | null;
  owner: string;
  endpoints: AgentEndpoints;
  card_uri: string | null;
  capabilities: string[];
  domain_tags: string[];
};

export type AgentSummary = {
  id: number;
  label: string;
};

// ─── Plans ───────────────────────────────────────────────────────────────────

export type PlanTask = {
  id: string;
  description: string;
  depends_on: string[];
  files: string[];
  completed: boolean;
};

export type Plan = {
  id: string;
  title: string;
  task_count: number;
  completed: boolean;
};

export type PlanDetail = {
  id: string;
  title: string;
  description: string;
  tasks: PlanTask[];
};

// ─── Jobs (marketplace) ──────────────────────────────────────────────────────
//
// NOTE: These types MUST match the Rust `Job` struct in
// roko-core/src/jobs.rs (see task B1). The API serializes with
// #[serde(rename_all = "snake_case")] so field names are 1:1.

export type JobType =
  | "research"
  | "coding_task"
  | "review"
  | "documentation"
  | "testing"
  | (string & {});  // open-ended for future job types without losing autocomplete

export type JobState =
  | "open"
  | "assigned"
  | "in_progress"
  | "submitted"
  | "evaluated"
  | "cancelled";

export type Job = {
  id: string;
  title: string;
  description: string;
  job_type: JobType;
  state: JobState;
  posted_by: string;
  assigned_to: string | null;
  created_at: string;
  updated_at: string;
  submission: JobSubmission | null;
  evaluation: JobEvaluation | null;
  metadata: Record<string, string>;
};

export type JobSubmission = {
  agent_id: string;
  result_summary: string;
  artifacts: string[];
  gate_results: JobGateResult[];
  submitted_at: string;
};

export type JobGateResult = {
  gate: string;
  passed: boolean;
  detail: string;
};

export type JobEvaluation = {
  evaluator: string;
  accepted: boolean;
  score: number | null;
  feedback: string;
  evaluated_at: string;
};

export type CreateJobRequest = {
  title: string;
  description: string;
  job_type: JobType;
  metadata?: Record<string, string>;
};

// ─── Gates ───────────────────────────────────────────────────────────────────

export type GateResult = {
  rung: number;
  name: string;
  passed: boolean;
  message: string;
  duration_ms: number;
};

// ─── Learning ────────────────────────────────────────────────────────────────

export type CFactorSummary = {
  fleet_cfactor: number;
  solo_avg: number;
  cfactor_pct: number;
  agent_count: number;
};

export type EfficiencyBucket = {
  agent_id: string;
  model: string;
  tokens_in: number;
  tokens_out: number;
  latency_ms: number;
  cost_usd: number;
  success: boolean;
  timestamp: string;
};

export type Experiment = {
  id: string;
  name: string;
  status: "running" | "concluded" | "cancelled";
  variants: ExperimentVariant[];
  winner: string | null;
  created_at: string;
};

export type ExperimentVariant = {
  id: string;
  label: string;
  weight: number;
  successes: number;
  trials: number;
};

export type ExperimentWinner = {
  experiment_id: string;
  winner_variant: string;
  improvement_pct: number;
};

// ─── Providers ───────────────────────────────────────────────────────────────

export type Provider = {
  id: string;
  name: string;
  model_count: number;
  health: "healthy" | "degraded" | "down" | "unknown";
  default_model: string | null;
};

// ─── Network ─────────────────────────────────────────────────────────────────

export type Heartbeat = {
  agent_id: string;
  timestamp: string;
  block_number: number | null;
  latency_ms: number;
  status: "ok" | "late" | "missed";
};

export type NetworkStats = {
  total_agents: number;
  online_agents: number;
  total_heartbeats_24h: number;
  avg_latency_ms: number;
};

// ─── PRDs ─────────────────────────────────────────────────────────────────────

export type Prd = {
  slug: string;
  title: string;
  status: "idea" | "draft" | "published";
  section: string;
  has_plan: boolean;
};

// ─── Config ──────────────────────────────────────────────────────────────────

export type RokoConfig = Record<string, unknown>;

// ─── Diagnosis ───────────────────────────────────────────────────────────────

export type DiagnosisSummary = {
  agent_id: string;
  verdict: string;
  severity: "low" | "medium" | "high" | "critical";
  timestamp: string;
  details: string;
};

// ─── WebSocket events ─────────────────────────────────────────────────────────

export type WsEventPayload =
  | { type: "run_started";          run_id: string; prompt_preview: string }
  | { type: "run_completed";        run_id: string; success: boolean }
  | { type: "agent_output";         agent_id: string; content: string; done: boolean }
  | { type: "plan_started";         plan_id: string }
  | { type: "plan_completed";       plan_id: string; success: boolean }
  | { type: "gate_result";          task_id: string; rung: number; passed: boolean }
  | { type: "heartbeat";            agent_id: string; block_number: number | null }
  | { type: "error";                message: string }
  | { type: "operation_completed";  op_id: string; kind: string; success: boolean }
  | { type: string; [key: string]: unknown };  // catch-all for future event types
```

### 2. Create query key factory

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/services/queryKeys.ts`:

```ts
/**
 * Typed query-key factory.
 *
 * Every TanStack Query cache entry uses keys from this factory so that WS
 * event handlers can invalidate specific queries without circular imports.
 *
 * Keys are `as const` tuples for precise type inference — avoids the
 * `QueryKey` footgun where a plain string key type-checks but fails to
 * match at invalidation time.
 */
export const queryKeys = {
  health:             ["health"]             as const,
  status:             ["status"]             as const,
  agents:             ["agents"]             as const,
  agent:              (id: string)           => ["agents", id]              as const,
  plans:              ["plans"]              as const,
  plan:               (id: string)           => ["plans", id]               as const,
  jobs:               (filters?: Record<string, string>) =>
    filters ? (["jobs", filters] as const) : (["jobs"] as const),
  job:                (id: string)           => ["jobs", id]                as const,
  heartbeats:         ["heartbeats"]         as const,
  networkStats:       ["networkStats"]       as const,
  cfactor:            ["cfactor"]            as const,
  experiments:        ["experiments"]        as const,
  providers:          ["providers"]          as const,
  diagnosis:          ["diagnosis"]          as const,
  prds:               ["prds"]               as const,
  config:             ["config"]             as const,
  efficiency:         ["efficiency"]         as const,
  cascade:            ["cascade"]            as const,
  costTiers:          ["costTiers"]          as const,
  adaptiveThresholds: ["adaptiveThresholds"] as const,
  metricsSummary:     ["metrics", "summary"] as const,
  metricsVelocity:    ["metrics", "velocity"] as const,
} as const;
```

### 3. Create API service with query hooks

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/services/api.ts`:

```ts
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "./queryKeys";
import type {
  HealthResponse,
  Agent,
  AgentSummary,
  Plan,
  PlanDetail,
  Job,
  CreateJobRequest,
  CFactorSummary,
  Experiment,
  Provider,
  DiagnosisSummary,
  Prd,
  RokoConfig,
  EfficiencyBucket,
  SessionStatus,
} from "../types/api";

// ─── Base fetcher ─────────────────────────────────────────────────────────────

const BASE_URL = import.meta.env.VITE_ROKO_API_URL ?? "";

/**
 * Fetch wrapper that prepends the API base URL, injects the auth token
 * from localStorage, and throws a user-readable error on non-2xx responses.
 */
export async function fetchApi<T>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const token = localStorage.getItem("nunchi_auth_token");
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(options.headers as Record<string, string>),
  };
  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  const url = `${BASE_URL}/api${path}`;
  const response = await fetch(url, { ...options, headers });

  if (!response.ok) {
    // Produce a user-friendly message; avoid leaking raw server error bodies.
    const status = response.status;
    if (status === 401 || status === 403) {
      throw new Error("Not authorized. Please sign in again.");
    }
    if (status === 404) {
      throw new Error("The requested resource was not found.");
    }
    if (status >= 500) {
      throw new Error("The server encountered an error. Please try again.");
    }
    const body = await response.text().catch(() => "");
    throw new Error(body || `Request failed (${status})`);
  }

  // 204 No Content -- return undefined cast to T
  if (response.status === 204) return undefined as T;

  return response.json() as Promise<T>;
}

// ─── Query hooks ──────────────────────────────────────────────────────────────

/**
 * GET /api/health
 *
 * Response shape (from roko-serve/src/routes/status.rs):
 *   { status: "ok", version: string, uptime_secs: u64,
 *     active_plans: usize, active_agents: usize }
 *
 * Polls every 10s. Use `data.active_agents` and `data.uptime_secs`
 * for the dashboard status strip.
 */
export function useHealth() {
  return useQuery({
    queryKey: queryKeys.health,
    queryFn: () => fetchApi<HealthResponse>("/health"),
    refetchInterval: 10_000,
  });
}

/** GET /api/status -- polls every 10s */
export function useSessionStatus() {
  return useQuery({
    queryKey: queryKeys.status,
    queryFn: () => fetchApi<SessionStatus>("/status"),
    refetchInterval: 10_000,
  });
}

/** GET /api/managed-agents -- polls every 15s */
export function useAgents() {
  return useQuery({
    queryKey: queryKeys.agents,
    queryFn: () => fetchApi<AgentSummary[]>("/managed-agents"),
    refetchInterval: 15_000,
  });
}

/** GET /api/agents/:id */
export function useAgent(id: string) {
  return useQuery({
    queryKey: queryKeys.agent(id),
    queryFn: () => fetchApi<Agent>(`/agents/${id}`),
    enabled: Boolean(id),
  });
}

/** GET /api/plans -- polls every 15s */
export function usePlans() {
  return useQuery({
    queryKey: queryKeys.plans,
    queryFn: () => fetchApi<Plan[]>("/plans"),
    refetchInterval: 15_000,
  });
}

/** GET /api/plans/:id -- polls every 5s */
export function usePlan(id: string) {
  return useQuery({
    queryKey: queryKeys.plan(id),
    queryFn: () => fetchApi<PlanDetail>(`/plans/${id}`),
    refetchInterval: 5_000,
    enabled: Boolean(id),
  });
}

/** GET /api/prds -- polls every 30s */
export function usePrds() {
  return useQuery({
    queryKey: queryKeys.prds,
    queryFn: () => fetchApi<Prd[]>("/prds"),
    refetchInterval: 30_000,
  });
}

/** GET /api/metrics/c_factor -- polls every 30s */
export function useCFactor() {
  return useQuery({
    queryKey: queryKeys.cfactor,
    queryFn: () => fetchApi<CFactorSummary>("/metrics/c_factor"),
    refetchInterval: 30_000,
  });
}

/** GET /api/learn/experiments -- polls every 30s */
export function useExperiments() {
  return useQuery({
    queryKey: queryKeys.experiments,
    queryFn: () => fetchApi<Experiment[]>("/learn/experiments"),
    refetchInterval: 30_000,
  });
}

/** GET /api/providers -- polls every 30s */
export function useProviders() {
  return useQuery({
    queryKey: queryKeys.providers,
    queryFn: () =>
      fetchApi<{ providers: Provider[] }>("/providers").then((r) => r.providers),
    refetchInterval: 30_000,
  });
}

/** GET /api/diagnosis/recent -- polls every 15s */
export function useDiagnosis() {
  return useQuery({
    queryKey: queryKeys.diagnosis,
    queryFn: () => fetchApi<DiagnosisSummary[]>("/diagnosis/recent"),
    refetchInterval: 15_000,
  });
}

/** GET /api/config -- polls every 60s */
export function useConfig() {
  return useQuery({
    queryKey: queryKeys.config,
    queryFn: () => fetchApi<RokoConfig>("/config"),
    refetchInterval: 60_000,
  });
}

/** GET /api/learn/efficiency -- polls every 30s */
export function useEfficiency() {
  return useQuery({
    queryKey: queryKeys.efficiency,
    queryFn: () => fetchApi<EfficiencyBucket[]>("/learn/efficiency"),
    refetchInterval: 30_000,
  });
}

/** GET /api/learn/cascade-router -- polls every 30s */
export function useCascade() {
  return useQuery({
    queryKey: queryKeys.cascade,
    queryFn: () => fetchApi<Record<string, unknown>>("/learn/cascade-router"),
    refetchInterval: 30_000,
  });
}

/** GET /api/learn/cost-tiers -- polls every 30s */
export function useCostTiers() {
  return useQuery({
    queryKey: queryKeys.costTiers,
    queryFn: () => fetchApi<Record<string, unknown>>("/learn/cost-tiers"),
    refetchInterval: 30_000,
  });
}

/** GET /api/learn/adaptive-thresholds -- polls every 30s */
export function useAdaptiveThresholds() {
  return useQuery({
    queryKey: queryKeys.adaptiveThresholds,
    queryFn: () => fetchApi<Record<string, unknown>>("/learn/adaptive-thresholds"),
    refetchInterval: 30_000,
  });
}

/** GET /api/metrics/summary -- polls every 15s */
export function useMetricsSummary() {
  return useQuery({
    queryKey: queryKeys.metricsSummary,
    queryFn: () => fetchApi<Record<string, unknown>>("/metrics/summary"),
    refetchInterval: 15_000,
  });
}

/** GET /api/metrics/velocity -- polls every 30s */
export function useVelocity() {
  return useQuery({
    queryKey: queryKeys.metricsVelocity,
    queryFn: () => fetchApi<Record<string, unknown>>("/metrics/velocity"),
    refetchInterval: 30_000,
  });
}

// ─── Mutations ────────────────────────────────────────────────────────────────

/** POST /api/plans/:id/execute */
export function useExecutePlan() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (planId: string) =>
      fetchApi<{ id: string }>(`/plans/${planId}/execute`, { method: "POST" }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.plans });
    },
  });
}

/** POST /api/agents/:id/message */
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
      fetchApi<{ run_id: string; status: string }>(`/agents/${agentId}/message`, {
        method: "POST",
        body: JSON.stringify({ message, context }),
      }),
  });
}

/** PUT /api/config */
export function useUpdateConfig() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (partial: Record<string, unknown>) =>
      fetchApi<Record<string, unknown>>("/config", {
        method: "PUT",
        body: JSON.stringify(partial),
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.config });
    },
  });
}

/** POST /api/run */
export function useStartRun() {
  return useMutation({
    mutationFn: (prompt: string) =>
      fetchApi<{ id: string }>("/run", {
        method: "POST",
        body: JSON.stringify({ prompt }),
      }),
  });
}

/** POST /api/plans */
export function useCreatePlan() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (plan: {
      title: string;
      description: string;
      tasks?: {
        id: string;
        description: string;
        depends_on?: string[];
        files?: string[];
      }[];
    }) =>
      fetchApi<{ id: string }>("/plans", {
        method: "POST",
        body: JSON.stringify(plan),
      }),
    onSuccess: (data) => {
      // Optimistic: invalidate the list; the new plan will appear on next fetch.
      void qc.invalidateQueries({ queryKey: queryKeys.plans });
      // Pre-warm the individual plan cache so detail pages load instantly.
      void qc.prefetchQuery({
        queryKey: queryKeys.plan(data.id),
        queryFn: () => fetchApi<PlanDetail>(`/plans/${data.id}`),
      });
    },
  });
}

/** POST /api/plans/generate */
export function useGeneratePlan() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (slug: string) =>
      fetchApi<{ id: string }>("/plans/generate", {
        method: "POST",
        body: JSON.stringify({ slug }),
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.plans });
    },
  });
}

/** POST /api/prds/ideas */
export function usePostIdea() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (text: string) =>
      fetchApi<{ slug: string }>("/prds/ideas", {
        method: "POST",
        body: JSON.stringify({ text }),
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.prds });
    },
  });
}

/** POST /api/prds/:slug/draft */
export function useDraftPrd() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (slug: string) =>
      fetchApi<{ id: string }>(`/prds/${slug}/draft`, { method: "POST" }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.prds });
    },
  });
}

/** POST /api/prds/:slug/promote */
export function usePromotePrd() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (slug: string) =>
      fetchApi<{ success: boolean }>(`/prds/${slug}/promote`, { method: "POST" }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.prds });
    },
  });
}

/** POST /api/research/topic */
export function useResearchTopic() {
  return useMutation({
    mutationFn: ({ topic, intent }: { topic: string; intent?: string }) =>
      fetchApi<{ id: string }>("/research/topic", {
        method: "POST",
        body: JSON.stringify({ topic, intent }),
      }),
  });
}

// ─── Jobs ─────────────────────────────────────────────────────────────────────

/** GET /api/jobs -- polls every 15s */
export function useJobs(filters?: Record<string, string>) {
  return useQuery({
    queryKey: queryKeys.jobs(filters),
    queryFn: () => {
      const params = filters
        ? "?" + new URLSearchParams(filters).toString()
        : "";
      return fetchApi<Job[]>(`/jobs${params}`);
    },
    refetchInterval: 15_000,
  });
}

/** GET /api/jobs/:id */
export function useJob(id: string) {
  return useQuery({
    queryKey: queryKeys.job(id),
    queryFn: () => fetchApi<Job>(`/jobs/${id}`),
    enabled: Boolean(id),
  });
}

/** POST /api/jobs -- optimistically adds the new job to the list cache */
export function useCreateJob() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (req: CreateJobRequest) =>
      fetchApi<Job>("/jobs", {
        method: "POST",
        body: JSON.stringify(req),
      }),
    onSuccess: (newJob) => {
      // Optimistic update: prepend to the unfiltered job list so the UI
      // reflects the creation immediately without waiting for a refetch.
      qc.setQueryData<Job[]>(queryKeys.jobs(), (prev) =>
        prev ? [newJob, ...prev] : [newJob]
      );
      // Seed the detail cache so navigating to the job page is instant.
      qc.setQueryData(queryKeys.job(newJob.id), newJob);
    },
  });
}
```

### 4. Create WebSocket client

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/services/ws.ts`:

```ts
import { useEffect } from "react";
import { useWsStore, type WsEvent } from "../stores/wsStore";
import type { WsEventPayload } from "../types/api";

const WS_URL = import.meta.env.VITE_ROKO_WS_URL ?? "ws://localhost:6677/ws";

// ─── Module-level singleton state ─────────────────────────────────────────────

let socket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectDelay = 1_000;
const MAX_RECONNECT_DELAY = 30_000;

// ─── Handlers ────────────────────────────────────────────────────────────────

function handleOpen(): void {
  reconnectDelay = 1_000;
  useWsStore.getState().setConnected(true);
}

function handleMessage(event: MessageEvent): void {
  // Ignore binary frames (e.g. pings from proxies)
  if (typeof event.data !== "string") return;

  let payload: WsEventPayload;
  try {
    payload = JSON.parse(event.data) as WsEventPayload;
  } catch {
    // Ignore malformed JSON frames
    return;
  }

  const wsEvent: WsEvent = {
    type: payload.type,
    payload,
    receivedAt: Date.now(),
  };
  useWsStore.getState().pushEvent(wsEvent);
}

function handleClose(): void {
  useWsStore.getState().setConnected(false);
  scheduleReconnect();
}

// handleError intentionally empty — the close event fires immediately after
// and triggers the reconnect, so there is nothing to do here.
function handleError(): void {}

function scheduleReconnect(): void {
  if (reconnectTimer !== null) return;  // already scheduled
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    reconnectDelay = Math.min(reconnectDelay * 2, MAX_RECONNECT_DELAY);
    connectWs();
  }, reconnectDelay);
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Open a WebSocket connection to roko-serve.
 * Safe to call multiple times — silently no-ops if already open or connecting.
 */
export function connectWs(): void {
  if (
    socket !== null &&
    (socket.readyState === WebSocket.OPEN ||
      socket.readyState === WebSocket.CONNECTING)
  ) {
    return;
  }

  try {
    socket = new WebSocket(WS_URL);
    socket.addEventListener("open", handleOpen);
    socket.addEventListener("message", handleMessage);
    socket.addEventListener("close", handleClose);
    socket.addEventListener("error", handleError);
  } catch {
    // WebSocket constructor can throw in environments that block WS connections
    scheduleReconnect();
  }
}

/**
 * Close the WebSocket connection and cancel any pending reconnect.
 * Call this on app unmount.
 */
export function disconnectWs(): void {
  if (reconnectTimer !== null) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  if (socket !== null) {
    // Remove listeners before closing to prevent handleClose from scheduling
    // a new reconnect after an intentional disconnect.
    socket.removeEventListener("open", handleOpen);
    socket.removeEventListener("message", handleMessage);
    socket.removeEventListener("close", handleClose);
    socket.removeEventListener("error", handleError);
    socket.close();
    socket = null;
  }
  useWsStore.getState().setConnected(false);
}

/** Returns true if the WebSocket is currently open. */
export function isWsConnected(): boolean {
  return socket?.readyState === WebSocket.OPEN;
}

// ─── React hook ───────────────────────────────────────────────────────────────

/**
 * Subscribe to a specific WS event type.
 * The callback receives the full typed payload and is called on every matching event.
 * The listener is removed automatically on unmount or when `type` changes.
 *
 * @example
 *   useWsEvent("agent_output", (payload) => {
 *     if (payload.agent_id === activeAgentId) appendLine(payload.content);
 *   });
 */
export function useWsEvent<T extends WsEventPayload["type"]>(
  type: T,
  handler: (payload: Extract<WsEventPayload, { type: T }>) => void
): void {
  useEffect(() => {
    const unsubscribe = useWsStore.subscribe((state, prevState) => {
      const latest = state.events.at(-1);
      const prevLatest = prevState.events.at(-1);
      if (latest && latest !== prevLatest && latest.type === type) {
        handler(latest.payload as Extract<WsEventPayload, { type: T }>);
      }
    });
    return unsubscribe;
  }, [type, handler]);
}
```

### 5. Wire WS into App startup

- [ ] In `/Users/will/dev/nunchi/nunchi-dashboard/src/App.tsx`, add the WS connection on mount. Update the file to:

```tsx
import { useEffect } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "react-router-dom";
import { ToastProvider } from "./design-system/components";
import { router } from "./router";
import { connectWs, disconnectWs } from "./services/ws";

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
  useEffect(() => {
    connectWs();
    return () => disconnectWs();
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

---

## Verification

Run from `/Users/will/dev/nunchi/nunchi-dashboard`:

- [ ] `npm run typecheck` -- exits 0, no errors in `types/api.ts`, `services/api.ts`, `services/ws.ts`, or `services/queryKeys.ts`
- [ ] `npm run dev` -- app starts, no console errors
- [ ] Open browser devtools Network tab:
  - Verify a WebSocket connection attempt to `ws://localhost:6677/ws` (it will fail if roko-serve is not running -- that is expected; confirm the reconnect backoff fires in the console)
- [ ] Verify imports work: temporarily add `import { useHealth } from '../services/api'` in any page component and call `const { data } = useHealth()` -- `npm run typecheck` should still pass
- [ ] Verify `HealthResponse` shape: the real `/api/health` returns `{ status, version, uptime_secs, active_plans, active_agents }`. Confirm these fields exist in the type.
- [ ] Start `roko-serve` (if available): `cd /Users/will/dev/nunchi/roko/roko && cargo run -p roko-cli -- serve`
  - Reload the dashboard -- WS status in devtools should show "101 Switching Protocols"
  - The `useHealth` hook should return `{ status: "ok", active_plans: N, active_agents: N }`
- [ ] Verify `useWsEvent` compiles: add `useWsEvent("agent_output", (p) => console.log(p.content))` in a component -- `p.content` should be string (not `unknown`)
