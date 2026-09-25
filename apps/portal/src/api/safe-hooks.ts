/**
 * Roko Portal — Safe React Query Hooks
 *
 * All hooks in this file delegate to `safeApi` so every data value a component
 * receives has been validated and defaulted.  No component should import from
 * `@/api/hooks` for data fetching; import from here instead.
 *
 * Conventions
 * -----------
 * - `useSafe<Resource>()` — useQuery over a collection or singleton
 * - `useSafe<Resource>(id)` — useQuery scoped to a single entity
 * - Mutations remain in `@/api/hooks` (they only send data outward).
 *
 * Stale times match the originals in `@/api/hooks`:
 * - Dashboard / vitals: 15 s
 * - Plans / tasks / agents: 30 s
 * - Learning / providers / config: 60 s
 * - Knowledge stats: 120 s
 */

import { useQuery, type UseQueryOptions } from '@tanstack/react-query';
import { safeApi } from '@/api/safe-api';
import type {
  PlanState,
  TaskState,
  AgentState,
  GateResult,
  ProviderState,
  KnowledgeEntry,
  ExperimentEntry,
  PlaybookEntry,
  DashboardSnapshot,
} from '@/api/types';
import type {
  PlanListItem,
  CascadeRouterSnapshot,
  EfficiencyResponse,
  HealthResponse,
  PrdSummary,
  PrdDetail,
  PrdCoverageStatus,
  KnowledgeQueryResult,
  KnowledgeStatsResult,
  DoctorResponse,
  ConfigResponse,
  ReflexesResponse,
  GateThresholdsResponse,
} from '@/api/safe-api';

// ---------------------------------------------------------------------------
// Re-export query key factory so callers can invalidate from one place
// ---------------------------------------------------------------------------

export { queryKeys } from '@/api/hooks';

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

/**
 * `GET /api/health` — safe server health snapshot.
 * Returns fully defaulted `HealthResponse`; never throws.
 */
export function useSafeHealth(
  options?: Partial<UseQueryOptions<HealthResponse>>,
) {
  return useQuery<HealthResponse>({
    queryKey: ['health'],
    queryFn: () => safeApi.getHealth(),
    staleTime: 15_000,
    refetchInterval: 15_000,
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Plans
// ---------------------------------------------------------------------------

/**
 * `GET /api/plans` — list all plans.
 * Returns `PlanListItem[]`; empty array when the server is unreachable.
 */
export function useSafePlans(
  options?: Partial<UseQueryOptions<PlanListItem[]>>,
) {
  return useQuery<PlanListItem[]>({
    queryKey: ['plans'],
    queryFn: () => safeApi.getPlans(),
    staleTime: 30_000,
    ...options,
  });
}

/**
 * `GET /api/plans/:id` — fetch a single plan.
 * Returns a fully-defaulted `PlanState` when the server returns an unexpected
 * shape or when the request fails.
 */
export function useSafePlan(
  id: string,
  options?: Partial<UseQueryOptions<PlanState>>,
) {
  return useQuery<PlanState>({
    queryKey: ['plans', id],
    queryFn: () => safeApi.getPlan(id),
    staleTime: 30_000,
    enabled: Boolean(id),
    ...options,
  });
}

/**
 * `GET /api/plans/:id/tasks` — list tasks for a plan.
 * Returns `TaskState[]`; empty array on error.
 */
export function useSafePlanTasks(
  planId: string,
  options?: Partial<UseQueryOptions<TaskState[]>>,
) {
  return useQuery<TaskState[]>({
    queryKey: ['plans', planId, 'tasks'],
    queryFn: () => safeApi.getPlanTasks(planId),
    staleTime: 30_000,
    enabled: Boolean(planId),
    ...options,
  });
}

/**
 * `GET /api/plans/:id/gates` — gate results for a plan.
 * Returns `GateResult[]`; empty array on error.
 */
export function useSafePlanGateResults(
  planId: string,
  options?: Partial<UseQueryOptions<GateResult[]>>,
) {
  return useQuery<GateResult[]>({
    queryKey: ['plans', planId, 'gates'],
    queryFn: () => safeApi.getPlanGateResults(planId),
    staleTime: 30_000,
    enabled: Boolean(planId),
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Agents
// ---------------------------------------------------------------------------

/**
 * `GET /api/managed-agents` — list all agents.
 * Returns `AgentState[]`; empty array on error.
 */
export function useSafeAgents(
  options?: Partial<UseQueryOptions<AgentState[]>>,
) {
  return useQuery<AgentState[]>({
    queryKey: ['agents'],
    queryFn: () => safeApi.getAgents(),
    staleTime: 30_000,
    ...options,
  });
}

/**
 * `GET /api/agents/:id` — fetch a single agent.
 * Returns `null` when the agent is not found or the request fails.
 */
export function useSafeAgent(
  id: string,
  options?: Partial<UseQueryOptions<AgentState | null>>,
) {
  return useQuery<AgentState | null>({
    queryKey: ['agents', id],
    queryFn: () => safeApi.getAgent(id),
    staleTime: 30_000,
    enabled: Boolean(id),
    ...options,
  });
}

// ---------------------------------------------------------------------------
// PRDs
// ---------------------------------------------------------------------------

/**
 * `GET /api/prds` — list all PRDs.
 * Returns `PrdSummary[]`; empty array on error.
 */
export function useSafePRDs(
  options?: Partial<UseQueryOptions<PrdSummary[]>>,
) {
  return useQuery<PrdSummary[]>({
    queryKey: ['prds'],
    queryFn: () => safeApi.getPrds(),
    staleTime: 60_000,
    ...options,
  });
}

/**
 * `GET /api/prds/:slug` — fetch a single PRD.
 * Returns a fully-defaulted `PrdDetail` on error.
 */
export function useSafePRD(
  slug: string,
  options?: Partial<UseQueryOptions<PrdDetail>>,
) {
  return useQuery<PrdDetail>({
    queryKey: ['prds', slug],
    queryFn: () => safeApi.getPrd(slug),
    staleTime: 60_000,
    enabled: Boolean(slug),
    ...options,
  });
}

/**
 * `GET /api/prds/status` — PRD coverage report.
 * Returns zeroed `PrdCoverageStatus` on error.
 */
export function useSafePRDStatus(
  options?: Partial<UseQueryOptions<PrdCoverageStatus>>,
) {
  return useQuery<PrdCoverageStatus>({
    queryKey: ['prds', 'status'],
    queryFn: () => safeApi.getPrdStatus(),
    staleTime: 60_000,
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Knowledge
// ---------------------------------------------------------------------------

/**
 * `POST /api/neuro/query` — semantic knowledge search.
 * Returns `{ results: [], total: 0, latencyMs: 0 }` on error.
 */
export function useSafeKnowledgeQuery(
  query: string,
  opts?: { limit?: number; minTier?: string },
  options?: Partial<UseQueryOptions<KnowledgeQueryResult>>,
) {
  return useQuery<KnowledgeQueryResult>({
    queryKey: ['knowledge', 'query', query, opts],
    queryFn: () => safeApi.queryKnowledge(query, opts),
    staleTime: 30_000,
    enabled: query.trim().length > 0,
    ...options,
  });
}

/**
 * `GET /api/retrieval/stats` — knowledge store statistics.
 * Returns zeroed `KnowledgeStatsResult` on error.
 */
export function useSafeKnowledgeStats(
  options?: Partial<UseQueryOptions<KnowledgeStatsResult>>,
) {
  return useQuery<KnowledgeStatsResult>({
    queryKey: ['knowledge', 'stats'],
    queryFn: () => safeApi.getKnowledgeStats(),
    staleTime: 120_000,
    ...options,
  });
}

/**
 * Approximate knowledge tier counts via a broad neuro query.
 * Returns a zero-count record on error.
 */
export function useSafeKnowledgeTierCounts(
  options?: Partial<UseQueryOptions<Record<string, number>>>,
) {
  return useQuery<Record<string, number>>({
    queryKey: ['knowledge', 'tier-counts'],
    queryFn: async () => {
      const result = await safeApi.queryKnowledge('agent task plan', { limit: 200 });
      const counts: Record<string, number> = {
        transient: 0,
        working: 0,
        consolidated: 0,
        persistent: 0,
      };
      for (const entry of result.results) {
        const tier = entry.tier ?? 'transient';
        counts[tier] = (counts[tier] ?? 0) + 1;
      }
      return counts;
    },
    staleTime: 120_000,
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Learning
// ---------------------------------------------------------------------------

/**
 * `GET /api/learn/cascade-router` — cascade router snapshot.
 * Returns `{ stage: 'static', observationCount: 0, models: [] }` on error.
 */
export function useSafeCascadeRouter(
  options?: Partial<UseQueryOptions<CascadeRouterSnapshot>>,
) {
  return useQuery<CascadeRouterSnapshot>({
    queryKey: ['learning', 'cascade-router'],
    queryFn: () => safeApi.getCascadeRouter(),
    staleTime: 60_000,
    ...options,
  });
}

/**
 * `GET /api/learn/efficiency` — per-task efficiency data.
 * Returns an empty `EfficiencyResponse` on error.
 *
 * @param period  Optional client-side filter: `"24h"`, `"7d"`, or `"30d"`.
 */
export function useSafeEfficiency(
  period?: string,
  options?: Partial<UseQueryOptions<EfficiencyResponse>>,
) {
  return useQuery<EfficiencyResponse>({
    queryKey: ['learning', 'efficiency', period],
    queryFn: () => safeApi.getEfficiency(period),
    staleTime: 60_000,
    ...options,
  });
}

/**
 * `GET /api/learn/playbooks` — when/then playbook entries.
 * Returns `PlaybookEntry[]`; empty array on error.
 */
export function useSafePlaybooks(
  options?: Partial<UseQueryOptions<PlaybookEntry[]>>,
) {
  return useQuery<PlaybookEntry[]>({
    queryKey: ['learning', 'playbooks'],
    queryFn: () => safeApi.getPlaybooks(),
    staleTime: 60_000,
    ...options,
  });
}

/**
 * `GET /api/learn/experiments` — A/B experiment entries.
 * Returns `ExperimentEntry[]`; empty array on error.
 */
export function useSafeExperiments(
  options?: Partial<UseQueryOptions<ExperimentEntry[]>>,
) {
  return useQuery<ExperimentEntry[]>({
    queryKey: ['learning', 'experiments'],
    queryFn: () => safeApi.getExperiments(),
    staleTime: 60_000,
    ...options,
  });
}

/**
 * `GET /api/learn/reflexes` — T0 reflex rule snapshot.
 * Returns `{ total: 0, rules: [] }` on error.
 */
export function useSafeReflexes(
  options?: Partial<UseQueryOptions<ReflexesResponse>>,
) {
  return useQuery<ReflexesResponse>({
    queryKey: ['learning', 'reflexes'],
    queryFn: () => safeApi.getReflexes(),
    staleTime: 60_000,
    ...options,
  });
}

/**
 * `GET /api/learn/gate-thresholds` — adaptive gate threshold state.
 * Returns `{ thresholds: {} }` on error.
 */
export function useSafeGateThresholds(
  options?: Partial<UseQueryOptions<GateThresholdsResponse>>,
) {
  return useQuery<GateThresholdsResponse>({
    queryKey: ['learning', 'gate-thresholds'],
    queryFn: () => safeApi.getGateThresholds(),
    staleTime: 60_000,
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Providers
// ---------------------------------------------------------------------------

/**
 * `GET /api/providers` — list all configured LLM providers.
 * Returns `ProviderState[]`; empty array on error.
 */
export function useSafeProviders(
  options?: Partial<UseQueryOptions<ProviderState[]>>,
) {
  return useQuery<ProviderState[]>({
    queryKey: ['providers'],
    queryFn: () => safeApi.getProviders(),
    staleTime: 60_000,
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/**
 * `GET /api/config` — current roko configuration.
 * Returns `{ config: {}, source: 'roko.toml', version: 2 }` on error.
 */
export function useSafeConfig(
  options?: Partial<UseQueryOptions<ConfigResponse>>,
) {
  return useQuery<ConfigResponse>({
    queryKey: ['config'],
    queryFn: () => safeApi.getConfig(),
    staleTime: 60_000,
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

/**
 * `GET /api/doctor` — workspace diagnostic report.
 * Returns an empty `DoctorResponse` with `healthy: false` on error.
 */
export function useSafeDoctor(
  options?: Partial<UseQueryOptions<DoctorResponse>>,
) {
  return useQuery<DoctorResponse>({
    queryKey: ['doctor'],
    queryFn: () => safeApi.getDoctor(),
    staleTime: 30_000,
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Dashboard snapshot
// ---------------------------------------------------------------------------

/**
 * `GET /api/status` — full dashboard snapshot.
 * Returns a fully-defaulted `DashboardSnapshot` on error; component state
 * built from the SSE store already guarantees safe shapes, but this hook is
 * provided for polling-only consumers.
 */
export function useSafeStatus(
  options?: Partial<UseQueryOptions<DashboardSnapshot>>,
) {
  return useQuery<DashboardSnapshot>({
    queryKey: ['status'],
    queryFn: () => safeApi.getStatus(),
    staleTime: 15_000,
    ...options,
  });
}
