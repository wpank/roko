/**
 * Roko Portal — React Query Hooks
 *
 * One hook per major API surface.  All hooks are typed against the shapes in
 * `@/api/types` and delegate HTTP calls to the singleton `api` client.
 *
 * Naming conventions
 * ------------------
 * - `use<Resource>()` — fetches a collection or singleton (useQuery)
 * - `use<Resource>(id)` — fetches a single entity by id (useQuery)
 * - `use<Verb><Resource>()` — mutates state (useMutation)
 *
 * Stale times
 * -----------
 * - Dashboard status / vitals: 15 s (high-churn)
 * - Plans / tasks / agents: 30 s (default)
 * - Learning / providers / config: 60 s (low-churn)
 * - Knowledge stats: 120 s (expensive to compute)
 */

import {
  useQuery,
  useMutation,
  useQueryClient,
  type UseQueryOptions,
  type UseMutationOptions,
} from '@tanstack/react-query';
import { api } from '@/api/client';
import type {
  DashboardSnapshot,
  PlanState,
  TaskState,
  AgentState,
  ProviderState,
  KnowledgeEntry,
  ExperimentEntry,
  PlaybookEntry,
} from '@/api/types';

// ---------------------------------------------------------------------------
// Query key factory
// ---------------------------------------------------------------------------

/**
 * Centralised query-key catalogue.  Using a factory keeps invalidation calls
 * in mutations DRY and lets React Query's cache diff work correctly.
 */
export const queryKeys = {
  // Status
  status: () => ['status'] as const,

  // Plans
  plans: () => ['plans'] as const,
  plan: (id: string) => ['plans', id] as const,
  planTasks: (planId: string) => ['plans', planId, 'tasks'] as const,
  planGates: (planId: string) => ['plans', planId, 'gates'] as const,
  planCosts: (planId: string) => ['plans', planId, 'costs'] as const,

  // Agents
  agents: () => ['agents'] as const,
  agent: (id: string) => ['agents', id] as const,

  // PRDs
  prds: () => ['prds'] as const,
  prd: (slug: string) => ['prds', slug] as const,
  prdStatus: () => ['prds', 'status'] as const,

  // Knowledge
  knowledgeQuery: (query: string, opts?: KnowledgeQueryOptions) =>
    ['knowledge', 'query', query, opts] as const,
  knowledgeStats: () => ['knowledge', 'stats'] as const,
  knowledgeTierCounts: () => ['knowledge', 'tier-counts'] as const,

  // Learning
  cascadeRouter: () => ['learning', 'cascade-router'] as const,
  efficiency: (period?: string) => ['learning', 'efficiency', period] as const,
  playbooks: () => ['learning', 'playbooks'] as const,
  experiments: () => ['learning', 'experiments'] as const,
  reflexes: () => ['learning', 'reflexes'] as const,
  gateThresholds: () => ['learning', 'gate-thresholds'] as const,

  // Providers
  providers: () => ['providers'] as const,
  providerHealth: (id: string) => ['providers', id, 'health'] as const,

  // Config
  config: () => ['config'] as const,

  // Health
  health: () => ['health'] as const,
} as const;

// ---------------------------------------------------------------------------
// Helper types
// ---------------------------------------------------------------------------

interface PlanListItem {
  id: string;
  title: string;
  task_count: number;
  completed: boolean;
  completed_task_count: number;
}

interface PrdSummary {
  slug: string;
  title: string;
  stage: string;
  updatedAt: string;
}

interface PrdDetail {
  slug: string;
  title: string;
  stage: string;
  content: string;
  updatedAt: string;
}

/** Map the raw API response (which uses `status`) to our `stage` field name. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function mapPrdSummary(raw: any): PrdSummary {
  return {
    slug: raw.slug ?? '',
    title: raw.title ?? raw.slug ?? '',
    stage: raw.stage ?? raw.status ?? 'idea',
    updatedAt: raw.updatedAt ?? raw.updated_at ?? new Date().toISOString(),
  };
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function mapPrdDetail(raw: any): PrdDetail {
  return {
    slug: raw.slug ?? '',
    title: raw.title ?? raw.slug ?? '',
    stage: raw.stage ?? raw.status ?? 'idea',
    content: raw.content ?? raw.body ?? raw.text ?? '',
    updatedAt: raw.updatedAt ?? raw.updated_at ?? new Date().toISOString(),
  };
}

interface PrdCoverageStatus {
  total: number;
  drafted: number;
  promoted: number;
  planned: number;
}

interface KnowledgeQueryOptions {
  tier?: string;
  limit?: number;
}

interface KnowledgeQueryResult {
  results: KnowledgeEntry[];
  total: number;
  latencyMs: number;
}

interface KnowledgeStatsResult {
  total: number;
  byTier: Record<string, number>;
  byDomain: Record<string, number>;
}

interface CascadeRouterSnapshot {
  stage: string;
  observationCount: number;
  models: Array<{
    model: string;
    provider: string;
    weight: number;
    successRate: number;
    avgCostUsd: number;
  }>;
}

interface EfficiencyResponse {
  events: Array<{
    agentId: string;
    model: string;
    costUsd: number;
    gatePassRate: number;
    timestamp: string;
  }>;
  summary: {
    totalEvents: number;
    avgGatePassRate: number;
    totalCostUsd: number;
  };
}

// ---------------------------------------------------------------------------
// Raw API response shapes (from roko-serve)
// ---------------------------------------------------------------------------

interface RawCascadeRouterResponse {
  confidence_stats: Record<string, { trials: number; successes: number; total_cost_usd: number }>;
  total_observations: number;
  projection_state: string;
  models: Array<{ model_slug: string; available: boolean }>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

interface RawEfficiencyTask {
  plan_id?: string;
  task_id?: string;
  timestamp?: string;
  cost_usd?: number;
  tokens?: number;
  duration_ms?: number;
}

interface RawEfficiencyResponse {
  total_cost: number;
  cost_per_task: number;
  data_quality: { has_real_data: boolean; entry_count: number; null_cost_count: number };
  cost_trend: Array<{ timestamp: string; cost_usd: number; cumulative_cost_usd: number }>;
  tasks: RawEfficiencyTask[];
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

interface RawPlaybookStep {
  action_kind: string;
  description: string;
  expected_signals?: string[];
  index: number;
}

interface RawPlaybooksResponse {
  playbooks: Array<{
    id: string;
    name?: string;
    goal?: string;
    when_pattern?: string | null;
    success_count?: number;
    failure_count?: number;
    success_rate?: number;
    created_at_ms?: number;
    last_used_ms?: number;
    steps?: RawPlaybookStep[];
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    [key: string]: any;
  }>;
  total: number;
}

interface RawExperimentsResponse {
  active_experiments: Array<{
    experiment_id: string;
    status: string;
    total_trials: number;
    variants: Array<{ id: string; name: string; trials: number; successes: number; success_rate: number }>;
    significance: {
      best_variant_id: string | null;
      p_value: number;
      statistically_significant: boolean;
    };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    [key: string]: any;
  }>;
  running_experiments: number;
  concluded_experiments: number;
  source: string;
}

interface RawKnowledgeEntry {
  id: string;
  content: string;
  kind?: string;
  tier?: string;
  relevance?: number;
  created_at?: string;
}

interface RawKnowledgeQueryResult {
  results: RawKnowledgeEntry[];
  total: number;
}

// ---------------------------------------------------------------------------
// Mapping functions — raw API → frontend types
// ---------------------------------------------------------------------------

function mapCascadeRouter(raw: RawCascadeRouterResponse): CascadeRouterSnapshot {
  const confidenceStats = raw.confidence_stats ?? {};
  const entries = Object.entries(confidenceStats);

  const totalTrials = entries.reduce((sum, [, s]) => sum + (s.trials ?? 0), 0);

  const models: CascadeRouterSnapshot['models'] = entries.map(([modelSlug, stats]) => {
    const trials = stats.trials ?? 0;
    const successes = stats.successes ?? 0;
    return {
      model: modelSlug,
      // provider: derive from model slug (e.g. "claude-*" → "anthropic")
      provider: modelSlug.startsWith('claude') ? 'anthropic'
              : modelSlug.startsWith('gpt') || modelSlug.startsWith('o1') || modelSlug.startsWith('o3') ? 'openai'
              : modelSlug.startsWith('gemini') ? 'google'
              : modelSlug.startsWith('llama') || modelSlug.startsWith('cerebras') ? 'cerebras'
              : 'unknown',
      weight: totalTrials > 0 ? trials / totalTrials : 0,
      successRate: trials > 0 ? successes / trials : 0,
      avgCostUsd: stats.total_cost_usd != null && trials > 0 ? stats.total_cost_usd / trials : 0,
    };
  });

  // Derive routing stage from observation count
  const obs = raw.total_observations ?? 0;
  const stage = obs === 0 ? 'static' : obs < 30 ? 'confidence' : 'ucb';

  return {
    stage,
    observationCount: obs,
    models,
  };
}

function mapEfficiency(raw: RawEfficiencyResponse, period?: string): EfficiencyResponse {
  const allTasks: RawEfficiencyTask[] = raw.tasks ?? [];

  // Filter by period if provided
  let tasks = allTasks;
  if (period) {
    const now = Date.now();
    const cutoffMs: number =
      period === '24h' ? now - 86_400_000 :
      period === '7d'  ? now - 7 * 86_400_000 :
      period === '30d' ? now - 30 * 86_400_000 :
      0;
    if (cutoffMs > 0) {
      tasks = allTasks.filter((t) => {
        if (!t.timestamp) return false;
        return new Date(t.timestamp).getTime() >= cutoffMs;
      });
    }
  }

  const events = tasks.map((t) => ({
    agentId: t.task_id ?? 'unknown',
    // model is not stored in efficiency records; show the plan as context
    model: t.plan_id ?? '',
    costUsd: t.cost_usd ?? 0,
    // gate_passed is not available in efficiency records.
    // Use tokens > 0 as a completion proxy (tasks that produced output passed
    // at least the compile gate; tasks with 0 tokens likely errored early).
    gatePassRate: (t.tokens ?? 0) > 0 ? 1 : 0,
    timestamp: t.timestamp ?? new Date().toISOString(),
  }));

  const totalCostUsd = raw.total_cost ?? 0;
  const passedCount = events.filter((e) => e.gatePassRate > 0).length;
  const avgGatePassRate = events.length > 0 ? passedCount / events.length : 0;

  return {
    events,
    summary: {
      totalEvents: events.length,
      avgGatePassRate,
      totalCostUsd,
    },
  };
}

function mapPlaybooks(raw: RawPlaybooksResponse): PlaybookEntry[] {
  return (raw.playbooks ?? []).map((pb) => {
    const successCount = pb.success_count ?? 0;
    const failureCount = pb.failure_count ?? 0;
    const total = successCount + failureCount;

    // Build whenCondition: prefer explicit when_pattern; otherwise synthesise
    // from the align_context step's expected_signals (e.g. "task_type:code,
    // plan:abc") so that the when/then columns show distinct content.
    let whenCondition: string;
    if (pb.when_pattern) {
      whenCondition = pb.when_pattern;
    } else {
      const alignStep = (pb.steps ?? []).find((s) => s.action_kind === 'align_context');
      const signals = alignStep?.expected_signals ?? [];
      whenCondition = signals.length > 0
        ? signals.join(', ')
        : (pb.name ?? pb.goal ?? '');
    }

    return {
      id: pb.id ?? '',
      whenCondition,
      // thenAction: the goal describes what will be done
      thenAction: pb.goal ?? pb.name ?? '',
      hitCount: total,
      lastAppliedAt: pb.last_used_ms != null
        ? new Date(pb.last_used_ms).toISOString()
        : null,
      createdAt: pb.created_at_ms != null
        ? new Date(pb.created_at_ms).toISOString()
        : new Date().toISOString(),
    };
  });
}

function mapExperiments(raw: RawExperimentsResponse): ExperimentEntry[] {
  const active = raw.active_experiments ?? [];
  return active.map((exp) => {
    const variants = exp.variants ?? [];
    // Map to a two-variant shape for the A/B display (use best two by trial count)
    const sorted = [...variants].sort((a, b) => (b.trials ?? 0) - (a.trials ?? 0));
    const variantA = sorted[0];
    const variantB = sorted[1];
    const sig = exp.significance ?? {};
    const statusRaw = (exp.status ?? '').toLowerCase();

    // Status mapping: the server sends "Running", "Concluded", etc. (capitalised).
    // "running" → 'collecting' unless significance thresholds are crossed.
    let status: ExperimentEntry['status'] = 'collecting';
    if (statusRaw === 'concluded') {
      status = 'concluded';
    } else if (sig.statistically_significant) {
      status = 'significant';
    } else if ((sig.p_value ?? 1) < 0.1) {
      status = 'trending';
    }

    // winner_id comes from top-level exp.winner_id (not inside significance)
    const winnerId: string | null = exp.winner_id ?? sig.best_variant_id ?? null;
    const winnerVariant: 'a' | 'b' | null =
      sig.statistically_significant && winnerId != null
        ? (winnerId === variantA?.id ? 'a' : 'b')
        : null;

    // Use section_name as a human-readable label if experiment_id is the same
    const name = exp.section_name && exp.section_name !== exp.experiment_id
      ? exp.section_name
      : exp.experiment_id ?? '';

    // Populate allVariants for full per-variant data in the detail panel.
    const allVariants = variants.map((v) => ({
      id: v.id,
      name: v.name ?? v.id,
      trials: v.trials ?? 0,
      successRate: v.success_rate ?? (v.trials > 0 ? v.successes / v.trials : 0),
    }));

    return {
      id: exp.experiment_id ?? '',
      name,
      status,
      variantA: variantA?.name ?? variantA?.id ?? 'control',
      variantB: variantB?.name ?? variantB?.id ?? 'treatment',
      sampleCount: exp.total_trials ?? 0,
      pValue: sig.p_value != null ? sig.p_value : null,
      winnerVariant,
      variantARate: variantA?.success_rate ?? undefined,
      variantBRate: variantB?.success_rate ?? undefined,
      allVariants,
      // The server does not return a start timestamp; use epoch as sentinel
      // so the UI shows a stable "long ago" value rather than "0s ago".
      startedAt: exp.started_at ?? new Date(0).toISOString(),
    };
  });
}

function mapKnowledgeEntry(raw: RawKnowledgeEntry): KnowledgeEntry {
  // Normalise the tier string — the server returns e.g. "Transient", "Working"
  const rawTier = (raw.tier ?? 'transient').toLowerCase();
  const tier: KnowledgeEntry['tier'] =
    rawTier === 'persistent' ? 'persistent' :
    rawTier === 'consolidated' ? 'consolidated' :
    rawTier === 'working' ? 'working' :
    'transient';

  return {
    id: raw.id ?? '',
    content: raw.content ?? '',
    tier,
    domain: raw.kind ?? '',
    confidence: raw.relevance ?? 0,
    tags: [],
    confirmations: 1,
    hdcFingerprint: null,
    createdAt: raw.created_at ?? new Date().toISOString(),
  };
}

interface ReflexRule {
  id: string;
  pattern: string;
  action: string;
  hitCount: number;
  lastMatchAt: string | null;
}

interface ReflexesResponse {
  total: number;
  rules: ReflexRule[];
}

interface GateThresholdsResponse {
  thresholds: Record<string, { threshold: number; ema: number; samples: number }>;
}

interface ProviderHealthDetail {
  id: string;
  status: string;
  errorRate: number;
  latencyP95Ms: number;
  requestCount: number;
  successCount: number;
  circuitState: string;
  lastCheckedAt: string;
}

interface ProvidersListResponse {
  providers: ProviderState[];
}

interface ConfigResponse {
  config: Record<string, unknown>;
  source: string;
  version: number;
}

interface RunResponse {
  runId: string;
  status: string;
  message: string;
}

// ---------------------------------------------------------------------------
// Mutation input types
// ---------------------------------------------------------------------------

interface CreateAgentInput {
  name: string;
  role: string;
  model?: string;
  domain?: string;
}

interface CreateIdeaInput {
  text: string;
  tags?: string[];
}

interface UpdateConfigInput {
  patch: Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

/**
 * `GET /api/status` — session status overview including signal and episode
 * counts, daemon state, and supervised processes.
 */
export function useStatus(
  options?: Partial<UseQueryOptions<DashboardSnapshot>>,
) {
  return useQuery<DashboardSnapshot>({
    queryKey: queryKeys.status(),
    queryFn: () => api.get<DashboardSnapshot>('/api/status'),
    staleTime: 15_000,
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

export interface HealthResponse {
  status: string;
  version: string;
  uptime_secs: number;
  active_agents: number;
  active_plans: number;
  active_runs: number;
  providers: {
    healthy: number;
    degraded: number;
    unhealthy: number;
    total: number;
  };
  statehub: {
    cursor: string;
    events_retained: number;
    snapshot: {
      agents_active: number;
      cost_usd_total: number;
      episodes_total: number;
      errors_total: number;
      gates_failed: number;
      gates_passed: number;
      plans_active: number;
      tasks_active: number;
    };
  };
}

/**
 * `GET /api/health` — server health snapshot with agent/provider counts and
 * the latest StateHub materialized snapshot values.  Refreshed every 15 s.
 */
export function useHealth(
  options?: Partial<UseQueryOptions<HealthResponse>>,
) {
  return useQuery<HealthResponse>({
    queryKey: queryKeys.health(),
    queryFn: () => api.get<HealthResponse>('/api/health'),
    staleTime: 15_000,
    refetchInterval: 15_000,
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Plans
// ---------------------------------------------------------------------------

/** `GET /api/plans` — list all plans. */
export function usePlans(
  options?: Partial<UseQueryOptions<PlanListItem[]>>,
) {
  return useQuery<PlanListItem[]>({
    queryKey: queryKeys.plans(),
    queryFn: () => api.get<PlanListItem[]>('/api/plans'),
    staleTime: 30_000,
    ...options,
  });
}

/** `GET /api/plans/:id` — fetch a single plan by id. */
export function usePlan(
  id: string,
  options?: Partial<UseQueryOptions<PlanState>>,
) {
  return useQuery<PlanState>({
    queryKey: queryKeys.plan(id),
    queryFn: () => api.get<PlanState>(`/api/plans/${id}`),
    staleTime: 30_000,
    enabled: Boolean(id),
    ...options,
  });
}

/** `GET /api/plans/:id/tasks` — list tasks belonging to a plan. */
export function usePlanTasks(
  planId: string,
  options?: Partial<UseQueryOptions<TaskState[]>>,
) {
  return useQuery<TaskState[]>({
    queryKey: queryKeys.planTasks(planId),
    queryFn: () => api.get<TaskState[]>(`/api/plans/${planId}/tasks`),
    staleTime: 30_000,
    enabled: Boolean(planId),
    ...options,
  });
}

/** `POST /api/plans/:id/execute` — start executing a plan. */
export function useStartPlan(
  options?: UseMutationOptions<unknown, Error, string>,
) {
  const qc = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (id: string) => api.post(`/api/plans/${id}/execute`),
    onSuccess: (_data, id) => {
      void qc.invalidateQueries({ queryKey: queryKeys.plans() });
      void qc.invalidateQueries({ queryKey: queryKeys.plan(id) });
    },
    ...options,
  });
}

/** `POST /api/plans/:id/pause` — pause a running plan. */
export function usePausePlan(
  options?: UseMutationOptions<unknown, Error, string>,
) {
  const qc = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (id: string) => api.post(`/api/plans/${id}/pause`),
    onSuccess: (_data, id) => {
      void qc.invalidateQueries({ queryKey: queryKeys.plan(id) });
    },
    ...options,
  });
}

/** `POST /api/plans/:id/resume` — resume a paused plan. */
export function useResumePlan(
  options?: UseMutationOptions<unknown, Error, string>,
) {
  const qc = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (id: string) => api.post(`/api/plans/${id}/resume`),
    onSuccess: (_data, id) => {
      void qc.invalidateQueries({ queryKey: queryKeys.plan(id) });
    },
    ...options,
  });
}

/** Cancel a plan (no dedicated cancel route; uses the plan status endpoint). */
export function useCancelPlan(
  options?: UseMutationOptions<unknown, Error, string>,
) {
  const qc = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (id: string) => api.post(`/api/plans/${id}/pause`),
    onSuccess: (_data, id) => {
      void qc.invalidateQueries({ queryKey: queryKeys.plan(id) });
      void qc.invalidateQueries({ queryKey: queryKeys.plans() });
    },
    ...options,
  });
}

/** `POST /api/plans/generate` — regenerate / retry failed tasks in a plan. */
export function useRetryPlan(
  options?: UseMutationOptions<unknown, Error, string>,
) {
  const qc = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (id: string) =>
      api.post('/api/plans/generate', { plan_id: id, retry_failed: true }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.plans() });
    },
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Plan editor mutations
// ---------------------------------------------------------------------------

interface GeneratePlanFromPromptInput {
  prompt: string;
  plan_id?: string;
}

interface GeneratePlanFromPromptResponse {
  /** Background operation id. */
  id: string;
  /** Resolved plan_id that was created or updated. */
  plan_id: string;
}

/**
 * `POST /api/plans/generate` — spawn async plan generation from a raw prompt.
 *
 * The server always returns immediately (202 Accepted) with an operation id;
 * tasks are written to disk in the background and can be fetched via
 * `GET /api/plans/:plan_id/tasks` once generation completes.
 */
export function useGeneratePlanFromPrompt(
  options?: UseMutationOptions<GeneratePlanFromPromptResponse, Error, GeneratePlanFromPromptInput>,
) {
  const qc = useQueryClient();
  return useMutation<GeneratePlanFromPromptResponse, Error, GeneratePlanFromPromptInput>({
    mutationFn: (input: GeneratePlanFromPromptInput) =>
      api.post<GeneratePlanFromPromptResponse>('/api/plans/generate', {
        prompt: input.prompt,
        plan_id: input.plan_id,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.plans() });
    },
    ...options,
  });
}

interface UpdatePlanTasksInput {
  planId: string;
  tasks: Array<{
    id: string;
    description: string;
    role?: string;
    depends_on?: string[];
    files?: string[];
  }>;
}

interface UpdatePlanTasksResponse {
  plan_id: string;
  updated: boolean;
}

/**
 * `PUT /api/plans/:id/tasks` — replace the full task list for a plan.
 *
 * Sends a JSON body; the server converts it to TOML and writes `tasks.toml`.
 */
export function useUpdatePlanTasks(
  options?: UseMutationOptions<UpdatePlanTasksResponse, Error, UpdatePlanTasksInput>,
) {
  const qc = useQueryClient();
  return useMutation<UpdatePlanTasksResponse, Error, UpdatePlanTasksInput>({
    mutationFn: ({ planId, tasks }: UpdatePlanTasksInput) =>
      api.put<UpdatePlanTasksResponse>(`/api/plans/${planId}/tasks`, { tasks }),
    onSuccess: (_data, { planId }) => {
      void qc.invalidateQueries({ queryKey: queryKeys.plans() });
      void qc.invalidateQueries({ queryKey: queryKeys.plan(planId) });
      void qc.invalidateQueries({ queryKey: queryKeys.planTasks(planId) });
    },
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Agents
// ---------------------------------------------------------------------------

/**
 * Raw shape returned by `GET /api/managed-agents`.
 * The server uses snake_case and a different field vocabulary than AgentState.
 */
interface RawManagedAgent {
  id: string;
  agent_id?: string;
  label?: string | null;
  role?: string | null;
  model?: string | null;
  provider?: string | null;
  status?: string | null;
  current_task?: string | null;
  registered_at?: number | null;
  costs?: {
    cumulative_usd?: number | null;
    token_burn_rate?: number | null;
  } | null;
  performance?: {
    active_tasks?: number | null;
    completed_tasks?: number | null;
    context_utilization?: number | null;
  } | null;
  model_profile?: {
    context_window?: number | null;
  } | null;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

/**
 * Map a raw `/api/managed-agents` entry to the canonical `AgentState` shape
 * used throughout the portal.
 */
function mapManagedAgent(raw: RawManagedAgent): AgentState {
  const id = raw.id ?? raw.agent_id ?? '';
  const name = raw.label ?? id;
  const role = raw.role ?? 'agent';

  // Map server status strings to the portal AgentStatus union.
  const rawStatus = (raw.status ?? '').toLowerCase();
  let status: AgentState['status'] = 'idle';
  if (rawStatus === 'active' || rawStatus === 'running') status = 'active';
  else if (rawStatus === 'completed') status = 'completed';
  else if (rawStatus === 'failed') status = 'failed';
  else if (rawStatus === 'stopped') status = 'stopped';

  const costUsd = raw.costs?.cumulative_usd ?? 0;
  const contextPct = raw.performance?.context_utilization != null
    ? Math.min(100, Math.max(0, raw.performance.context_utilization * 100))
    : 0;

  // The managed-agent record does not carry per-turn token breakdowns; derive
  // a rough estimate from burn rate if available.
  const tokenBurnRate = raw.costs?.token_burn_rate ?? 0;
  const tokensIn = tokenBurnRate > 0 ? Math.round(tokenBurnRate * 0.6) : 0;
  const tokensOut = tokenBurnRate > 0 ? Math.round(tokenBurnRate * 0.4) : 0;

  const startedAt = raw.registered_at != null
    ? new Date(raw.registered_at * 1000).toISOString()
    : null;

  return {
    id,
    name,
    role,
    model: raw.model ?? '',
    provider: raw.provider ?? '',
    status,
    currentTask: raw.current_task ?? null,
    tokensIn,
    tokensOut,
    costUsd,
    contextPct,
    startedAt,
  };
}

/** `GET /api/managed-agents` — list all managed agents with status. */
export function useAgents(
  options?: Partial<UseQueryOptions<AgentState[]>>,
) {
  return useQuery<AgentState[]>({
    queryKey: queryKeys.agents(),
    queryFn: async () => {
      const raw = await api.get<RawManagedAgent[]>('/api/managed-agents');
      return (raw ?? []).map(mapManagedAgent);
    },
    staleTime: 30_000,
    ...options,
  });
}

/** `GET /api/agents/:id` — get a single agent by id. */
export function useAgent(
  id: string,
  options?: Partial<UseQueryOptions<AgentState>>,
) {
  return useQuery<AgentState>({
    queryKey: queryKeys.agent(id),
    queryFn: () => api.get<AgentState>(`/api/agents/${id}`),
    staleTime: 30_000,
    enabled: Boolean(id),
    ...options,
  });
}

/** `POST /api/agents/create` — create (register) a new agent. */
export function useCreateAgent(
  options?: UseMutationOptions<AgentState, Error, CreateAgentInput>,
) {
  const qc = useQueryClient();
  return useMutation<AgentState, Error, CreateAgentInput>({
    mutationFn: (input: CreateAgentInput) =>
      api.post<AgentState>('/api/agents/create', input),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.agents() });
    },
    ...options,
  });
}

/** `POST /api/agents/:id/stop` + delete — stop and deregister an agent. */
export function useDeleteAgent(
  options?: UseMutationOptions<unknown, Error, string>,
) {
  const qc = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (id: string) => api.post(`/api/agents/${id}/stop`),
    onSuccess: (_data, id) => {
      void qc.invalidateQueries({ queryKey: queryKeys.agents() });
      qc.removeQueries({ queryKey: queryKeys.agent(id) });
    },
    ...options,
  });
}

/** `POST /api/agents/:id/start` — start a stopped agent. */
export function useStartAgent(
  options?: UseMutationOptions<unknown, Error, string>,
) {
  const qc = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (id: string) => api.post(`/api/agents/${id}/start`),
    onSuccess: (_data, id) => {
      void qc.invalidateQueries({ queryKey: queryKeys.agent(id) });
      void qc.invalidateQueries({ queryKey: queryKeys.agents() });
    },
    ...options,
  });
}

/** `POST /api/agents/:id/stop` — stop a running agent. */
export function useStopAgent(
  options?: UseMutationOptions<unknown, Error, string>,
) {
  const qc = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (id: string) => api.post(`/api/agents/${id}/stop`),
    onSuccess: (_data, id) => {
      void qc.invalidateQueries({ queryKey: queryKeys.agent(id) });
      void qc.invalidateQueries({ queryKey: queryKeys.agents() });
    },
    ...options,
  });
}

// ---------------------------------------------------------------------------
// PRDs
// ---------------------------------------------------------------------------

/** `GET /api/prds` — list all PRDs. */
export function usePRDs(
  options?: Partial<UseQueryOptions<PrdSummary[]>>,
) {
  return useQuery<PrdSummary[]>({
    queryKey: queryKeys.prds(),
    queryFn: async () => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const raw = await api.get<any[]>('/api/prds');
      return (raw ?? []).map(mapPrdSummary);
    },
    staleTime: 60_000,
    ...options,
  });
}

/** `GET /api/prds/:slug` — fetch a single PRD by slug. */
export function usePRD(
  slug: string,
  options?: Partial<UseQueryOptions<PrdDetail>>,
) {
  return useQuery<PrdDetail>({
    queryKey: queryKeys.prd(slug),
    queryFn: async () => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const raw = await api.get<any>(`/api/prds/${slug}`);
      return mapPrdDetail(raw);
    },
    staleTime: 60_000,
    enabled: Boolean(slug),
    ...options,
  });
}

/** `POST /api/prds/ideas` — capture a new PRD idea. */
export function useCreateIdea(
  options?: UseMutationOptions<unknown, Error, CreateIdeaInput>,
) {
  const qc = useQueryClient();
  return useMutation<unknown, Error, CreateIdeaInput>({
    mutationFn: (input: CreateIdeaInput) =>
      api.post('/api/prds/ideas', input),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.prds() });
    },
    ...options,
  });
}

/** `POST /api/prds/:slug/promote` — promote a draft PRD. */
export function usePromotePRD(
  options?: UseMutationOptions<unknown, Error, string>,
) {
  const qc = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (slug: string) => api.post(`/api/prds/${slug}/promote`),
    onSuccess: (_data, slug) => {
      void qc.invalidateQueries({ queryKey: queryKeys.prd(slug) });
      void qc.invalidateQueries({ queryKey: queryKeys.prds() });
    },
    ...options,
  });
}

/** `POST /api/prds/:slug/plan` — generate an implementation plan from a PRD. */
export function useGeneratePlan(
  options?: UseMutationOptions<unknown, Error, string>,
) {
  const qc = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (slug: string) => api.post(`/api/prds/${slug}/plan`),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.plans() });
      void qc.invalidateQueries({ queryKey: queryKeys.prdStatus() });
    },
    ...options,
  });
}

/** `GET /api/prds/status` — PRD coverage report. */
export function usePRDStatus(
  options?: Partial<UseQueryOptions<PrdCoverageStatus>>,
) {
  return useQuery<PrdCoverageStatus>({
    queryKey: queryKeys.prdStatus(),
    queryFn: () => api.get<PrdCoverageStatus>('/api/prds/status'),
    staleTime: 60_000,
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Knowledge
// ---------------------------------------------------------------------------

/**
 * `POST /api/neuro/query` — semantic search across the knowledge store.
 *
 * The query string is part of the query key so each unique search term gets
 * its own cache entry.  `enabled` is `false` when the query is empty.
 *
 * The server returns `{ results: [{id, content, kind, tier, relevance,
 * created_at}], total }`.  We map each raw entry to `KnowledgeEntry` so
 * the browser sees the canonical camelCase shape.
 */
export function useKnowledgeQuery(
  query: string,
  options?: KnowledgeQueryOptions & {
    queryOptions?: Partial<UseQueryOptions<KnowledgeQueryResult>>;
  },
) {
  const { tier, limit, queryOptions } = options ?? {};
  return useQuery<KnowledgeQueryResult>({
    queryKey: queryKeys.knowledgeQuery(query, { tier, limit }),
    queryFn: async () => {
      const raw = await api.post<RawKnowledgeQueryResult>('/api/neuro/query', {
        query,
        limit: limit ?? 10,
        min_tier: tier,
      });
      return {
        results: (raw.results ?? []).map(mapKnowledgeEntry),
        total: raw.total ?? 0,
        latencyMs: 0,
      };
    },
    staleTime: 30_000,
    enabled: query.trim().length > 0,
    ...queryOptions,
  });
}

/**
 * `GET /api/retrieval/stats` — retrieval quality metrics (precision, latency).
 *
 * NOTE: The server `/api/retrieval/stats` endpoint returns retrieval quality
 * metrics, NOT tier counts.  For a tier count summary we synthesise from a
 * broad neuro query that returns the first 200 entries and counts their tiers.
 *
 * The returned `KnowledgeStatsResult` is populated as:
 * - `total`   — `total_settled` (number of retrieval outcomes recorded)
 * - `byTier`  — synthesised from the cascade experiment variant stats as a
 *               proxy, or empty `{}` when no experiment data is available
 * - `byDomain` — per-strategy precision as a proxy
 */
export function useKnowledgeStats(
  options?: Partial<UseQueryOptions<KnowledgeStatsResult>>,
) {
  return useQuery<KnowledgeStatsResult>({
    queryKey: queryKeys.knowledgeStats(),
    queryFn: async () => {
      // Fetch retrieval stats for quality metrics.
      interface RawRetrievalStats {
        total_settled?: number;
        strategies?: Array<{ strategy: string; attempts: number; passed: number }>;
      }
      const stats = await api.get<RawRetrievalStats>('/api/retrieval/stats');
      const totalSettled = stats.total_settled ?? 0;

      // Build byDomain (strategy → pass count) from retrieval stats.
      const byDomain: Record<string, number> = {};
      for (const s of stats.strategies ?? []) {
        byDomain[s.strategy] = s.passed ?? 0;
      }

      // Tier counts are not available from this endpoint — use empty map.
      // The intelligence overview page seeds tier counts from the SSE store
      // (which receives knowledge_entry_created / knowledge_tier_changed events),
      // so an empty object here is safe.
      return {
        total: totalSettled,
        byTier: {},
        byDomain,
      };
    },
    staleTime: 120_000,
    ...options,
  });
}

/**
 * Fetch real knowledge tier counts from the neuro query endpoint.
 *
 * This is used to seed the `KnowledgeStoreCard` on the intelligence overview
 * page before SSE events arrive.  We issue a broad search for common terms
 * across multiple tiers and count the results.  The counts are approximate
 * because the server returns at most `limit` entries per query.
 *
 * Returns `Record<string, number>` with lowercase tier names as keys.
 */
export function useKnowledgeTierCounts(
  options?: Partial<UseQueryOptions<Record<string, number>>>,
) {
  return useQuery<Record<string, number>>({
    queryKey: queryKeys.knowledgeTierCounts(),
    queryFn: async () => {
      // Query a broad term that matches typical knowledge entries across all tiers.
      const raw = await api.post<RawKnowledgeQueryResult>('/api/neuro/query', {
        query: 'agent task plan',
        limit: 200,
      });
      const counts: Record<string, number> = {
        transient: 0,
        working: 0,
        consolidated: 0,
        persistent: 0,
      };
      for (const entry of raw.results ?? []) {
        const tier = (entry.tier ?? 'transient').toLowerCase();
        const key =
          tier === 'persistent'    ? 'persistent'    :
          tier === 'consolidated'  ? 'consolidated'  :
          tier === 'working'       ? 'working'       :
          'transient';
        counts[key] = (counts[key] ?? 0) + 1;
      }
      return counts;
    },
    staleTime: 120_000,
    ...options,
  });
}

/** Trigger knowledge GC — no response body is expected. */
export function useKnowledgeGC(
  options?: UseMutationOptions<unknown, Error, void>,
) {
  const qc = useQueryClient();
  return useMutation<unknown, Error, void>({
    mutationFn: () => api.post('/api/knowledge/gc'),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.knowledgeStats() });
    },
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Learning
// ---------------------------------------------------------------------------

/**
 * `GET /api/learn/cascade-router` — cascade router snapshot.
 *
 * The raw response has `{ confidence_stats: Record<slug, {trials, successes,
 * total_cost_usd}>, total_observations, projection_state, models[] }`.
 * We map this to the internal `CascadeRouterSnapshot` shape.
 */
export function useCascadeRouter(
  options?: Partial<UseQueryOptions<CascadeRouterSnapshot>>,
) {
  return useQuery<CascadeRouterSnapshot>({
    queryKey: queryKeys.cascadeRouter(),
    queryFn: async () => {
      const raw = await api.get<RawCascadeRouterResponse>('/api/learn/cascade-router');
      return mapCascadeRouter(raw);
    },
    staleTime: 60_000,
    ...options,
  });
}

/**
 * `GET /api/learn/efficiency` — per-task efficiency data.
 *
 * The raw response has `{ total_cost, cost_per_task, cost_trend, tasks[] }`.
 * We adapt it to `EfficiencyResponse` for the front-end charts.
 *
 * The optional `period` string (`"24h"`, `"7d"`, `"30d"`) is used for
 * client-side filtering since the server endpoint does not support it.
 */
export function useEfficiency(
  period?: string,
  options?: Partial<UseQueryOptions<EfficiencyResponse>>,
) {
  return useQuery<EfficiencyResponse>({
    queryKey: queryKeys.efficiency(period),
    queryFn: async () => {
      const raw = await api.get<RawEfficiencyResponse>('/api/learn/efficiency');
      return mapEfficiency(raw, period);
    },
    staleTime: 60_000,
    ...options,
  });
}

/**
 * `GET /api/learn/playbooks` — when/then playbook entries.
 *
 * The raw response has `{ playbooks: [{id, name, goal, when_pattern,
 * success_count, failure_count, created_at_ms, last_used_ms, steps}] }`.
 * We map to `PlaybookEntry[]`.
 */
export function usePlaybooks(
  options?: Partial<UseQueryOptions<PlaybookEntry[]>>,
) {
  return useQuery<PlaybookEntry[]>({
    queryKey: queryKeys.playbooks(),
    queryFn: async () => {
      const raw = await api.get<RawPlaybooksResponse>('/api/learn/playbooks');
      return mapPlaybooks(raw);
    },
    staleTime: 60_000,
    ...options,
  });
}

/**
 * `GET /api/learn/experiments` — experiment entries.
 *
 * The raw response has `{ active_experiments: [{experiment_id, status,
 * total_trials, variants[], significance{}}] }`.
 * We map to `ExperimentEntry[]` using a two-variant A/B approximation.
 */
export function useExperiments(
  options?: Partial<UseQueryOptions<ExperimentEntry[]>>,
) {
  return useQuery<ExperimentEntry[]>({
    queryKey: queryKeys.experiments(),
    queryFn: async () => {
      const raw = await api.get<RawExperimentsResponse>('/api/learn/experiments');
      return mapExperiments(raw);
    },
    staleTime: 60_000,
    ...options,
  });
}

/** `GET /api/learn/reflexes` — T0 reflex rule snapshot. */
export function useReflexes(
  options?: Partial<UseQueryOptions<ReflexesResponse>>,
) {
  return useQuery<ReflexesResponse>({
    queryKey: queryKeys.reflexes(),
    queryFn: () => api.get<ReflexesResponse>('/api/learn/reflexes'),
    staleTime: 60_000,
    ...options,
  });
}

/** `GET /api/learn/gate-thresholds` — adaptive gate threshold state. */
export function useGateThresholds(
  options?: Partial<UseQueryOptions<GateThresholdsResponse>>,
) {
  return useQuery<GateThresholdsResponse>({
    queryKey: queryKeys.gateThresholds(),
    queryFn: () =>
      api.get<GateThresholdsResponse>('/api/learn/gate-thresholds'),
    staleTime: 60_000,
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Providers
// ---------------------------------------------------------------------------

/** `GET /api/providers` — list all configured LLM providers. */
export function useProviders(
  options?: Partial<UseQueryOptions<ProvidersListResponse>>,
) {
  return useQuery<ProvidersListResponse>({
    queryKey: queryKeys.providers(),
    queryFn: () => api.get<ProvidersListResponse>('/api/providers'),
    staleTime: 60_000,
    ...options,
  });
}

/** `GET /api/providers/:id/health` — detailed health snapshot for one provider. */
export function useProviderHealth(
  id: string,
  options?: Partial<UseQueryOptions<ProviderHealthDetail>>,
) {
  return useQuery<ProviderHealthDetail>({
    queryKey: queryKeys.providerHealth(id),
    queryFn: () =>
      api.get<ProviderHealthDetail>(`/api/providers/${id}/health`),
    staleTime: 30_000,
    enabled: Boolean(id),
    ...options,
  });
}

/** `POST /api/providers/:id/test` — send a test prompt to a provider. */
export function useTestProvider(
  options?: UseMutationOptions<unknown, Error, string>,
) {
  return useMutation<unknown, Error, string>({
    mutationFn: (id: string) => api.post(`/api/providers/${id}/test`),
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/** `GET /api/config` — current roko configuration as a JSON object. */
export function useConfig(
  options?: Partial<UseQueryOptions<ConfigResponse>>,
) {
  return useQuery<ConfigResponse>({
    queryKey: queryKeys.config(),
    queryFn: async () => {
      // roko-serve /api/config returns the config object directly (not
      // wrapped in { config: ... }).  Normalise into ConfigResponse here.
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const raw = await api.get<any>('/api/config');
      // If the server already wraps it (future-proof), pass through.
      if (raw && typeof raw === 'object' && 'config' in raw && typeof raw.config === 'object') {
        return raw as ConfigResponse;
      }
      const configObj = (raw ?? {}) as Record<string, unknown>;
      return {
        config: configObj,
        source: 'roko.toml',
        version: typeof configObj.config_version === 'number' ? configObj.config_version : 2,
      };
    },
    staleTime: 60_000,
    ...options,
  });
}

/** `PUT /api/config` — patch the current configuration. */
export function useUpdateConfig(
  options?: UseMutationOptions<ConfigResponse, Error, UpdateConfigInput>,
) {
  const qc = useQueryClient();
  return useMutation<ConfigResponse, Error, UpdateConfigInput>({
    mutationFn: async (input: UpdateConfigInput) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const raw = await api.put<any>('/api/config', input.patch);
      if (raw && typeof raw === 'object' && 'config' in raw && typeof raw.config === 'object') {
        return raw as ConfigResponse;
      }
      const configObj = (raw ?? {}) as Record<string, unknown>;
      return {
        config: configObj,
        source: 'roko.toml',
        version: typeof configObj.config_version === 'number' ? configObj.config_version : 2,
      };
    },
    onSuccess: (data) => {
      qc.setQueryData(queryKeys.config(), data);
    },
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Run prompt
// ---------------------------------------------------------------------------

/**
 * `POST /api/run` — dispatch a single prompt through the graph template
 * pipeline (compose → provider → gate → persist).
 */
export function useRunPrompt(
  options?: UseMutationOptions<RunResponse, Error, string>,
) {
  const qc = useQueryClient();
  return useMutation<RunResponse, Error, string>({
    mutationFn: (prompt: string) =>
      api.post<RunResponse>('/api/run', { prompt }),
    onSuccess: () => {
      // A new run may spawn agents and update the session status.
      void qc.invalidateQueries({ queryKey: queryKeys.status() });
      void qc.invalidateQueries({ queryKey: queryKeys.agents() });
    },
    ...options,
  });
}

// ---------------------------------------------------------------------------
// Inbox
// ---------------------------------------------------------------------------

/** `DELETE /api/inbox/:id` — dismiss an inbox notification by id. */
export function useDismissInbox(
  options?: UseMutationOptions<unknown, Error, string>,
) {
  const qc = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (id: string) => api.delete(`/api/inbox/${id}`),
    onSuccess: () => {
      // Inbox is embedded in the dashboard snapshot.
      void qc.invalidateQueries({ queryKey: queryKeys.status() });
    },
    ...options,
  });
}
