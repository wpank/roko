/**
 * Roko Portal — Safe API Client
 *
 * Every function in this module returns a typed value or a safe default.
 * Nothing in here throws on unexpected API shapes; shape mismatches are
 * logged as warnings in development so they are visible without breaking
 * the UI.
 *
 * Layered design
 * --------------
 * 1. Primitive safe accessors (safeString, safeNumber, …)
 * 2. Entity mappers (mapPlan, mapTask, mapAgent, …)
 * 3. SafeApi class — wraps the singleton `api` client and applies mappers
 * 4. Singleton `safeApi` export
 *
 * Adding a new entity
 * -------------------
 * a. Write a `mapFoo(raw: unknown): Foo` function using the safe accessors.
 * b. Add the corresponding method(s) to SafeApi.
 * c. Expose a hook in safe-hooks.ts.
 */

import { api } from '@/api/client';
import type {
  PlanState,
  TaskState,
  AgentState,
  GateResult,
  EpisodeEntry,
  ProviderState,
  KnowledgeEntry,
  ExperimentEntry,
  PlaybookEntry,
  DreamJournalEntry,
  DashboardSnapshot,
  VitalsSnapshot,
  AffectState,
  LearningState,
  ErrorEntry,
  InboxItem,
  DashboardEvent,
} from '@/api/types';

// ---------------------------------------------------------------------------
// Dev-mode warning helper
// ---------------------------------------------------------------------------

const IS_DEV =
  typeof process !== 'undefined'
    ? process.env.NODE_ENV !== 'production'
    : false;

function warnShape(context: string, field: string, received: unknown): void {
  if (IS_DEV) {
    // eslint-disable-next-line no-console
    console.warn(
      `[safe-api] Shape mismatch in ${context}: field "${field}" received`,
      received,
    );
  }
}

// ---------------------------------------------------------------------------
// Primitive safe accessors
// ---------------------------------------------------------------------------

/** Returns the value when it is a non-empty string, otherwise `fallback`. */
export function safeString(value: unknown, fallback = ''): string {
  if (typeof value === 'string') return value;
  if (value !== null && value !== undefined) {
    warnShape('safeString', '(value)', value);
  }
  return fallback;
}

/** Returns the value when it is a finite number, otherwise `fallback`. */
export function safeNumber(value: unknown, fallback = 0): number {
  if (typeof value === 'number' && isFinite(value)) return value;
  if (value !== null && value !== undefined) {
    warnShape('safeNumber', '(value)', value);
  }
  return fallback;
}

/** Returns the value when it is a boolean, otherwise `fallback`. */
export function safeBool(value: unknown, fallback = false): boolean {
  if (typeof value === 'boolean') return value;
  if (value !== null && value !== undefined) {
    warnShape('safeBool', '(value)', value);
  }
  return fallback;
}

/**
 * Returns a valid array.  When `mapper` is supplied each element is passed
 * through it; the mapper itself should never throw.
 */
export function safeArray<T>(value: unknown, mapper?: (item: unknown) => T): T[] {
  if (!Array.isArray(value)) {
    if (value !== null && value !== undefined) {
      warnShape('safeArray', '(value)', value);
    }
    return [];
  }
  if (mapper) {
    return value.map((item) => {
      try {
        return mapper(item);
      } catch (err) {
        if (IS_DEV) {
          // eslint-disable-next-line no-console
          console.warn('[safe-api] mapper threw on item', item, err);
        }
        return mapper({});  // call mapper with empty object to get safe defaults
      }
    });
  }
  return value as T[];
}

/**
 * Returns a valid plain object.  When `value` is not an object (or is null /
 * an array) an empty object is returned instead.
 */
export function safeObject(value: unknown): Record<string, unknown> {
  if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return {};
}

/** Returns `value` when it is null or a string, otherwise `null`. */
export function safeStringOrNull(value: unknown): string | null {
  if (value === null || value === undefined) return null;
  if (typeof value === 'string') return value;
  warnShape('safeStringOrNull', '(value)', value);
  return null;
}

/** Returns `value` when it is null or a number, otherwise `null`. */
export function safeNumberOrNull(value: unknown): number | null {
  if (value === null || value === undefined) return null;
  if (typeof value === 'number' && isFinite(value)) return value;
  warnShape('safeNumberOrNull', '(value)', value);
  return null;
}

// ---------------------------------------------------------------------------
// Type-narrowing helpers
// ---------------------------------------------------------------------------

/** Coerce an unknown value into a Record so field access is safe. */
function asRecord(raw: unknown): Record<string, unknown> {
  if (raw !== null && typeof raw === 'object' && !Array.isArray(raw)) {
    return raw as Record<string, unknown>;
  }
  return {};
}

// ---------------------------------------------------------------------------
// Entity mappers
// ---------------------------------------------------------------------------

// --- Plan -------------------------------------------------------------------

/** Minimal list-item shape returned by GET /api/plans. */
export interface PlanListItem {
  id: string;
  title: string;
  taskCount: number;
  completedTaskCount: number;
  completed: boolean;
  status: string;
}

export function mapPlanListItem(raw: unknown): PlanListItem {
  const r = asRecord(raw);
  return {
    id: safeString(r.id),
    title: safeString(r.title ?? r.name ?? r.id),
    taskCount: safeNumber(r.task_count ?? r.taskCount),
    completedTaskCount: safeNumber(r.completed_task_count ?? r.completedTaskCount),
    completed: safeBool(r.completed),
    status: safeString(r.status, 'pending'),
  };
}

export function mapPlanState(raw: unknown): PlanState {
  const r = asRecord(raw);
  const progress = asRecord(r.progress);
  return {
    id: safeString(r.id),
    name: safeString(r.name ?? r.title ?? r.id),
    status: safeString(r.status, 'pending') as PlanState['status'],
    progress: {
      total: safeNumber(progress.total ?? r.task_count),
      completed: safeNumber(progress.completed ?? r.completed_task_count),
      failed: safeNumber(progress.failed),
    },
    tasks: safeArray(r.tasks, (t) => safeString(t)),
    costUsd: safeNumber(r.costUsd ?? r.cost_usd),
    budgetUsd: safeNumberOrNull(r.budgetUsd ?? r.budget_usd),
    startedAt: safeStringOrNull(r.startedAt ?? r.started_at),
    completedAt: safeStringOrNull(r.completedAt ?? r.completed_at),
  };
}

// --- Task -------------------------------------------------------------------

export function mapGateResult(raw: unknown): GateResult {
  const r = asRecord(raw);
  const rung = safeNumber(r.rung);
  const validRung = (rung >= 0 && rung <= 6 ? rung : 0) as GateResult['rung'];
  return {
    id: safeString(r.id),
    taskId: safeString(r.taskId ?? r.task_id),
    planId: safeString(r.planId ?? r.plan_id),
    gateName: safeString(r.gateName ?? r.gate_name),
    rung: validRung,
    passed: safeBool(r.passed),
    summary: safeString(r.summary),
    output: safeString(r.output),
    durationMs: safeNumber(r.durationMs ?? r.duration_ms),
    timestamp: safeString(r.timestamp, new Date().toISOString()),
  };
}

export function mapTaskState(raw: unknown): TaskState {
  const r = asRecord(raw);
  return {
    id: safeString(r.id),
    planId: safeString(r.planId ?? r.plan_id),
    name: safeString(r.name ?? r.title ?? r.id),
    status: safeString(r.status, 'pending') as TaskState['status'],
    wave: safeNumber(r.wave),
    agentName: safeStringOrNull(r.agentName ?? r.agent_name),
    model: safeStringOrNull(r.model),
    costUsd: safeNumber(r.costUsd ?? r.cost_usd),
    startedAt: safeStringOrNull(r.startedAt ?? r.started_at),
    completedAt: safeStringOrNull(r.completedAt ?? r.completed_at),
    dependsOn: safeArray(r.dependsOn ?? r.depends_on, (t) => safeString(t)),
    gateResults: safeArray(r.gateResults ?? r.gate_results, mapGateResult),
    description: r.description !== undefined ? safeString(r.description) : undefined,
    files: r.files !== undefined ? safeArray(r.files, (f) => safeString(f)) : undefined,
    tier: r.tier !== undefined ? safeString(r.tier) : undefined,
  };
}

// --- Agent ------------------------------------------------------------------

export function mapAgentState(raw: unknown): AgentState {
  const r = asRecord(raw);
  const rawStatus = safeString(r.status).toLowerCase();

  let status: AgentState['status'] = 'idle';
  if (rawStatus === 'active' || rawStatus === 'running') status = 'active';
  else if (rawStatus === 'completed') status = 'completed';
  else if (rawStatus === 'failed') status = 'failed';
  else if (rawStatus === 'stopped') status = 'stopped';

  // Support both managed-agent (snake_case) and canonical (camelCase) shapes.
  const costs = asRecord(r.costs);
  const performance = asRecord(r.performance);
  const costUsd = safeNumber(
    r.costUsd ?? r.cost_usd ?? costs.cumulative_usd,
  );
  const contextPct = (() => {
    const raw = r.contextPct ?? r.context_pct ?? performance.context_utilization;
    const n = safeNumber(raw);
    // context_utilization may come as 0–1 fraction
    return n > 1 ? Math.min(100, n) : Math.min(100, n * 100);
  })();
  const tokenBurnRate = safeNumber(costs.token_burn_rate);
  const tokensIn = safeNumber(
    r.tokensIn ?? r.tokens_in ?? (tokenBurnRate > 0 ? Math.round(tokenBurnRate * 0.6) : 0),
  );
  const tokensOut = safeNumber(
    r.tokensOut ?? r.tokens_out ?? (tokenBurnRate > 0 ? Math.round(tokenBurnRate * 0.4) : 0),
  );
  const startedAt = (() => {
    const v = r.startedAt ?? r.started_at ?? r.registered_at;
    if (typeof v === 'number') return new Date(v * 1000).toISOString();
    return safeStringOrNull(v);
  })();

  return {
    id: safeString(r.id ?? r.agent_id),
    name: safeString(r.name ?? r.label ?? r.id),
    role: safeString(r.role, 'agent'),
    model: safeString(r.model),
    provider: safeString(r.provider),
    status,
    currentTask: safeStringOrNull(r.currentTask ?? r.current_task),
    tokensIn,
    tokensOut,
    costUsd,
    contextPct,
    startedAt,
  };
}

// --- Provider ---------------------------------------------------------------

export function mapProviderState(raw: unknown): ProviderState {
  const r = asRecord(raw);
  const health = asRecord(r.health);
  const totalAttempts = safeNumber(health.total_attempts ?? r.requestCount ?? r.request_count);
  const totalSuccesses = safeNumber(health.total_successes);
  const consecutiveFailures = safeNumber(health.consecutive_failures);
  const hasApiKey = safeBool(r.has_api_key, true);

  let status: ProviderState['status'];
  if (!r.health) {
    status = hasApiKey ? 'healthy' : 'unconfigured';
  } else {
    const healthState = safeString(health.state).toLowerCase();
    if (healthState === 'degraded') status = 'degraded';
    else if (healthState === 'unhealthy') status = 'unhealthy';
    else status = 'healthy';
  }

  let circuitState: ProviderState['circuitState'];
  const rawCircuit = safeString(
    r.circuitState ?? r.circuit_state ?? health.circuit_state,
  ).toLowerCase();
  if (rawCircuit === 'open' || consecutiveFailures >= 5) circuitState = 'open';
  else if (rawCircuit === 'half_open' || consecutiveFailures >= 2) circuitState = 'half_open';
  else circuitState = 'closed';

  const passRate =
    totalAttempts > 0
      ? totalSuccesses / totalAttempts
      : safeNumber(r.passRate ?? r.pass_rate, 1);

  return {
    id: safeString(r.id ?? r.name),
    name: safeString(r.name ?? r.id),
    status,
    modelCount: safeNumber(r.modelCount ?? r.model_count),
    requestCount: totalAttempts,
    passRate,
    avgLatencyMs: safeNumber(r.avgLatencyMs ?? r.avg_latency_ms),
    costUsd: safeNumber(r.costUsd ?? r.cost_usd),
    circuitState,
  };
}

// --- Episode ----------------------------------------------------------------

export function mapEpisodeEntry(raw: unknown): EpisodeEntry {
  const r = asRecord(raw);
  const rawGate = safeString(r.gateResult ?? r.gate_result, 'pass').toLowerCase();
  const gateResult: EpisodeEntry['gateResult'] =
    rawGate === 'fail' ? 'fail' : rawGate === 'skip' ? 'skip' : 'pass';
  return {
    id: safeString(r.id),
    agentId: safeString(r.agentId ?? r.agent_id),
    taskId: safeString(r.taskId ?? r.task_id),
    model: safeString(r.model),
    tokensIn: safeNumber(r.tokensIn ?? r.tokens_in),
    tokensOut: safeNumber(r.tokensOut ?? r.tokens_out),
    costUsd: safeNumber(r.costUsd ?? r.cost_usd),
    hdcFingerprint: safeStringOrNull(r.hdcFingerprint ?? r.hdc_fingerprint),
    gateResult,
    timestamp: safeString(r.timestamp, new Date().toISOString()),
  };
}

// --- Knowledge --------------------------------------------------------------

export function mapKnowledgeEntry(raw: unknown): KnowledgeEntry {
  const r = asRecord(raw);
  const rawTier = safeString(r.tier, 'transient').toLowerCase();
  const tier: KnowledgeEntry['tier'] =
    rawTier === 'persistent'   ? 'persistent'   :
    rawTier === 'consolidated' ? 'consolidated' :
    rawTier === 'working'      ? 'working'      :
    'transient';

  return {
    id: safeString(r.id),
    content: safeString(r.content),
    tier,
    domain: safeString(r.domain ?? r.kind),
    confidence: safeNumber(r.confidence ?? r.relevance),
    tags: safeArray(r.tags, (t) => safeString(t)),
    confirmations: safeNumber(r.confirmations, 1),
    hdcFingerprint: safeStringOrNull(r.hdcFingerprint ?? r.hdc_fingerprint),
    createdAt: safeString(r.createdAt ?? r.created_at, new Date().toISOString()),
  };
}

// --- Cascade Router ---------------------------------------------------------

export interface CascadeRouterModel {
  model: string;
  provider: string;
  weight: number;
  successRate: number;
  avgCostUsd: number;
}

export interface CascadeRouterSnapshot {
  stage: string;
  observationCount: number;
  models: CascadeRouterModel[];
}

export function mapCascadeRouter(raw: unknown): CascadeRouterSnapshot {
  const r = asRecord(raw);
  const confidenceStats = asRecord(r.confidence_stats);
  const entries = Object.entries(confidenceStats);
  const totalObs = safeNumber(r.total_observations);
  const totalTrials = entries.reduce((sum, [, s]) => sum + safeNumber(asRecord(s).trials), 0);

  const models: CascadeRouterModel[] = entries.map(([modelSlug, s]) => {
    const stats = asRecord(s);
    const trials = safeNumber(stats.trials);
    const successes = safeNumber(stats.successes);
    return {
      model: modelSlug,
      provider:
        modelSlug.startsWith('claude')  ? 'anthropic' :
        modelSlug.startsWith('gpt') || modelSlug.startsWith('o1') || modelSlug.startsWith('o3') ? 'openai' :
        modelSlug.startsWith('gemini')  ? 'google'    :
        modelSlug.startsWith('llama') || modelSlug.startsWith('cerebras') ? 'cerebras' :
        'unknown',
      weight: totalTrials > 0 ? trials / totalTrials : 0,
      successRate: trials > 0 ? successes / trials : 0,
      avgCostUsd:
        safeNumber(stats.total_cost_usd) > 0 && trials > 0
          ? safeNumber(stats.total_cost_usd) / trials
          : 0,
    };
  });

  const stage =
    totalObs === 0 ? 'static' : totalObs < 30 ? 'confidence' : 'ucb';

  return { stage, observationCount: totalObs, models };
}

// --- Efficiency -------------------------------------------------------------

export interface EfficiencyEvent {
  agentId: string;
  model: string;
  costUsd: number;
  gatePassRate: number;
  timestamp: string;
}

export interface EfficiencyResponse {
  events: EfficiencyEvent[];
  summary: {
    totalEvents: number;
    avgGatePassRate: number;
    totalCostUsd: number;
  };
}

export function mapEfficiency(raw: unknown, period?: string): EfficiencyResponse {
  const r = asRecord(raw);
  const allTasks = safeArray(r.tasks);

  let tasks = allTasks;
  if (period) {
    const now = Date.now();
    const cutoff =
      period === '24h' ? now - 86_400_000 :
      period === '7d'  ? now - 7 * 86_400_000 :
      period === '30d' ? now - 30 * 86_400_000 :
      0;
    if (cutoff > 0) {
      tasks = allTasks.filter((t) => {
        const ts = safeString(asRecord(t).timestamp);
        return ts ? new Date(ts).getTime() >= cutoff : false;
      });
    }
  }

  const events: EfficiencyEvent[] = tasks.map((t) => {
    const tr = asRecord(t);
    return {
      agentId: safeString(tr.task_id ?? tr.agentId, 'unknown'),
      model: safeString(tr.plan_id ?? tr.model),
      costUsd: safeNumber(tr.cost_usd ?? tr.costUsd),
      gatePassRate: safeNumber(tr.tokens ?? tr.tokensOut) > 0 ? 1 : 0,
      timestamp: safeString(tr.timestamp, new Date().toISOString()),
    };
  });

  const passedCount = events.filter((e) => e.gatePassRate > 0).length;
  return {
    events,
    summary: {
      totalEvents: events.length,
      avgGatePassRate: events.length > 0 ? passedCount / events.length : 0,
      totalCostUsd: safeNumber(r.total_cost ?? r.totalCost),
    },
  };
}

// --- Playbook ---------------------------------------------------------------

export function mapPlaybookEntry(raw: unknown): PlaybookEntry {
  const r = asRecord(raw);
  const successCount = safeNumber(r.success_count ?? r.successCount);
  const failureCount = safeNumber(r.failure_count ?? r.failureCount);

  let whenCondition: string;
  if (typeof r.when_pattern === 'string' && r.when_pattern) {
    whenCondition = r.when_pattern;
  } else if (typeof r.whenCondition === 'string' && r.whenCondition) {
    whenCondition = r.whenCondition;
  } else {
    const steps = safeArray(r.steps);
    const alignStep = steps.find(
      (s) => safeString(asRecord(s).action_kind) === 'align_context',
    );
    const signals = safeArray(alignStep ? asRecord(alignStep).expected_signals : undefined, (s) => safeString(s));
    whenCondition = signals.length > 0
      ? signals.join(', ')
      : safeString(r.name ?? r.goal);
  }

  const lastUsedMs = r.last_used_ms ?? r.lastUsedMs;
  const createdAtMs = r.created_at_ms ?? r.createdAtMs;

  return {
    id: safeString(r.id),
    whenCondition,
    thenAction: safeString(r.goal ?? r.thenAction ?? r.name),
    hitCount: successCount + failureCount,
    lastAppliedAt:
      lastUsedMs != null
        ? new Date(safeNumber(lastUsedMs)).toISOString()
        : safeStringOrNull(r.lastAppliedAt ?? r.last_applied_at),
    createdAt:
      createdAtMs != null
        ? new Date(safeNumber(createdAtMs)).toISOString()
        : safeString(r.createdAt ?? r.created_at, new Date().toISOString()),
  };
}

// --- Experiment -------------------------------------------------------------

export function mapExperimentEntry(raw: unknown): ExperimentEntry {
  const r = asRecord(raw);
  const variants = safeArray(r.variants);
  const sorted = [...variants]
    .map(asRecord)
    .sort((a, b) => safeNumber(b.trials) - safeNumber(a.trials));
  const va = sorted[0] ?? {};
  const vb = sorted[1] ?? {};
  const sig = asRecord(r.significance);

  const statusRaw = safeString(r.status).toLowerCase();
  let status: ExperimentEntry['status'] = 'collecting';
  if (statusRaw === 'concluded') status = 'concluded';
  else if (safeBool(sig.statistically_significant)) status = 'significant';
  else if (safeNumber(sig.p_value, 1) < 0.1) status = 'trending';

  const winnerId = safeStringOrNull(r.winner_id ?? sig.best_variant_id);
  const winnerVariant: 'a' | 'b' | null =
    safeBool(sig.statistically_significant) && winnerId != null
      ? (winnerId === safeString(va.id) ? 'a' : 'b')
      : null;

  const name =
    typeof r.section_name === 'string' && r.section_name !== r.experiment_id
      ? r.section_name
      : safeString(r.experiment_id ?? r.id ?? r.name);

  const allVariants = variants.map((v) => {
    const vr = asRecord(v);
    const trials = safeNumber(vr.trials);
    const successes = safeNumber(vr.successes);
    return {
      id: safeString(vr.id),
      name: safeString(vr.name ?? vr.id),
      trials,
      successRate: safeNumber(vr.success_rate, trials > 0 ? successes / trials : 0),
    };
  });

  return {
    id: safeString(r.experiment_id ?? r.id),
    name,
    status,
    variantA: safeString(va.name ?? va.id, 'control'),
    variantB: safeString(vb.name ?? vb.id, 'treatment'),
    sampleCount: safeNumber(r.total_trials ?? r.sampleCount),
    pValue: safeNumberOrNull(sig.p_value),
    winnerVariant,
    variantARate: va.success_rate != null ? safeNumber(va.success_rate) : undefined,
    variantBRate: vb.success_rate != null ? safeNumber(vb.success_rate) : undefined,
    allVariants,
    startedAt: safeString(r.started_at ?? r.startedAt, new Date(0).toISOString()),
  };
}

// --- Dream Journal ----------------------------------------------------------

export function mapDreamJournalEntry(raw: unknown): DreamJournalEntry {
  const r = asRecord(raw);
  const rawPhase = safeString(r.phase).toLowerCase();
  const phase: DreamJournalEntry['phase'] =
    rawPhase === 'imagination'   ? 'imagination'   :
    rawPhase === 'consolidation' ? 'consolidation' :
    'hypnagogia';
  return {
    id: safeString(r.id),
    phase,
    promotedCount: safeNumber(r.promotedCount ?? r.promoted_count),
    demotedCount: safeNumber(r.demotedCount ?? r.demoted_count),
    createdCount: safeNumber(r.createdCount ?? r.created_count),
    insightsGained: safeNumber(r.insightsGained ?? r.insights_gained),
    durationMs: safeNumber(r.durationMs ?? r.duration_ms),
    timestamp: safeString(r.timestamp, new Date().toISOString()),
  };
}

// --- Doctor -----------------------------------------------------------------

export type DoctorCheckStatus = 'ok' | 'warn' | 'fail';

export interface DoctorCheck {
  id: string;
  status: DoctorCheckStatus;
  message: string;
  detail?: string;
  path?: string;
  fix?: string;
}

export interface DoctorSummary {
  ok: number;
  warn: number;
  fail: number;
  total: number;
  skipped: number;
}

export interface DoctorResponse {
  healthy: boolean;
  workdir: string;
  checks: DoctorCheck[];
  summary: DoctorSummary;
}

export function mapDoctorCheck(raw: unknown): DoctorCheck {
  const r = asRecord(raw);
  const rawStatus = safeString(r.status).toLowerCase();
  const status: DoctorCheckStatus =
    rawStatus === 'ok'   ? 'ok'   :
    rawStatus === 'warn' ? 'warn' :
    'fail';
  return {
    id: safeString(r.id),
    status,
    message: safeString(r.message),
    detail: r.detail !== undefined ? safeString(r.detail) : undefined,
    path: r.path !== undefined ? safeString(r.path) : undefined,
    fix: r.fix !== undefined ? safeString(r.fix) : undefined,
  };
}

export function mapDoctorResponse(raw: unknown): DoctorResponse {
  const r = asRecord(raw);
  const summary = asRecord(r.summary);
  return {
    healthy: safeBool(r.healthy, true),
    workdir: safeString(r.workdir),
    checks: safeArray(r.checks, mapDoctorCheck),
    summary: {
      ok: safeNumber(summary.ok),
      warn: safeNumber(summary.warn),
      fail: safeNumber(summary.fail),
      total: safeNumber(summary.total),
      skipped: safeNumber(summary.skipped),
    },
  };
}

// --- Config -----------------------------------------------------------------

export interface ConfigResponse {
  config: Record<string, unknown>;
  source: string;
  version: number;
}

export function mapConfigResponse(raw: unknown): ConfigResponse {
  const r = asRecord(raw);
  // The server returns either { config: {...} } or the flat config object.
  if ('config' in r && r.config !== null && typeof r.config === 'object' && !Array.isArray(r.config)) {
    return {
      config: r.config as Record<string, unknown>,
      source: safeString(r.source, 'roko.toml'),
      version: safeNumber(r.version, 2),
    };
  }
  return {
    config: r,
    source: safeString(r.source, 'roko.toml'),
    version: safeNumber(r.config_version, 2),
  };
}

// --- Vitals & Affect --------------------------------------------------------

export function mapVitalsSnapshot(raw: unknown): VitalsSnapshot {
  const r = asRecord(raw);
  return {
    activeAgents: safeNumber(r.activeAgents ?? r.active_agents),
    totalAgents: safeNumber(r.totalAgents ?? r.total_agents),
    gatePassRate: safeNumber(r.gatePassRate ?? r.gate_pass_rate),
    costToday: safeNumber(r.costToday ?? r.cost_today),
    cfactor: safeNumber(r.cfactor),
    healthyProviders: safeNumber(r.healthyProviders ?? r.healthy_providers),
    totalProviders: safeNumber(r.totalProviders ?? r.total_providers),
    affectWord: safeString(r.affectWord ?? r.affect_word, 'neutral'),
  };
}

export function mapAffectState(raw: unknown): AffectState | null {
  if (raw === null || raw === undefined) return null;
  const r = asRecord(raw);
  return {
    pleasure: safeNumber(r.pleasure),
    arousal: safeNumber(r.arousal),
    dominance: safeNumber(r.dominance),
    word: safeString(r.word, 'neutral'),
    updatedAt: safeString(r.updatedAt ?? r.updated_at, new Date().toISOString()),
  };
}

export function mapLearningState(raw: unknown): LearningState {
  const r = asRecord(raw);
  const rawTrend = safeString(r.cfactorTrend ?? r.cfactor_trend, 'flat').toLowerCase();
  const cfactorTrend: LearningState['cfactorTrend'] =
    rawTrend === 'up'   ? 'up'   :
    rawTrend === 'down' ? 'down' :
    'flat';
  const rawStage = safeString(r.learningStage ?? r.learning_stage, 'static').toLowerCase();
  const learningStage: LearningState['learningStage'] =
    rawStage === 'confidence' ? 'confidence' :
    rawStage === 'ucb'        ? 'ucb'        :
    'static';
  return {
    cfactor: safeNumber(r.cfactor),
    cfactorDelta: safeNumber(r.cfactorDelta ?? r.cfactor_delta),
    cfactorTrend,
    learningStage,
    observationCount: safeNumber(r.observationCount ?? r.observation_count),
    knowledgeTierCounts: (() => {
      const tc = r.knowledgeTierCounts ?? r.knowledge_tier_counts;
      if (tc !== null && typeof tc === 'object' && !Array.isArray(tc)) {
        const out: Record<string, number> = {};
        for (const [k, v] of Object.entries(tc as Record<string, unknown>)) {
          out[k] = safeNumber(v);
        }
        return out;
      }
      return {};
    })(),
    activeExperiments: safeNumber(r.activeExperiments ?? r.active_experiments),
  };
}

// --- ErrorEntry & InboxItem -------------------------------------------------

export function mapErrorEntry(raw: unknown): ErrorEntry {
  const r = asRecord(raw);
  const rawSev = safeString(r.severity).toLowerCase();
  const severity: ErrorEntry['severity'] =
    rawSev === 'critical' ? 'critical' :
    rawSev === 'high'     ? 'high'     :
    rawSev === 'medium'   ? 'medium'   :
    'low';
  return {
    id: safeString(r.id),
    source: safeString(r.source),
    message: safeString(r.message),
    severity,
    timestamp: safeString(r.timestamp, new Date().toISOString()),
    planId: r.planId !== undefined ? safeString(r.planId ?? r.plan_id) : undefined,
    taskId: r.taskId !== undefined ? safeString(r.taskId ?? r.task_id) : undefined,
  };
}

export function mapInboxItem(raw: unknown): InboxItem {
  const r = asRecord(raw);
  const rawKind = safeString(r.kind).toLowerCase();
  const validKinds: InboxItem['kind'][] = [
    'gate_failure', 'safety_incident', 'budget_warning',
    'provider_degraded', 'approval_needed', 'plan_completed', 'cost_anomaly',
  ];
  const kind: InboxItem['kind'] = (validKinds.includes(rawKind as InboxItem['kind'])
    ? rawKind
    : 'gate_failure') as InboxItem['kind'];
  const rawSev = safeString(r.severity).toLowerCase();
  const severity: InboxItem['severity'] =
    rawSev === 'error'   ? 'error'   :
    rawSev === 'warning' ? 'warning' :
    rawSev === 'success' ? 'success' :
    'info';
  return {
    id: safeString(r.id),
    kind,
    severity,
    title: safeString(r.title),
    description: safeString(r.description),
    actionUrl: r.actionUrl !== undefined ? safeString(r.actionUrl ?? r.action_url) : undefined,
    dismissedAt: safeStringOrNull(r.dismissedAt ?? r.dismissed_at),
    timestamp: safeString(r.timestamp, new Date().toISOString()),
  };
}

// --- DashboardSnapshot ------------------------------------------------------

export function mapDashboardSnapshot(raw: unknown): DashboardSnapshot {
  const r = asRecord(raw);

  const mapRecord = <T>(
    field: unknown,
    mapper: (v: unknown) => T,
  ): Record<string, T> => {
    const obj = asRecord(field);
    const out: Record<string, T> = {};
    for (const [k, v] of Object.entries(obj)) {
      try {
        out[k] = mapper(v);
      } catch {
        // skip malformed entries
      }
    }
    return out;
  };

  return {
    plans: mapRecord(r.plans, mapPlanState),
    tasks: mapRecord(r.tasks, mapTaskState),
    agents: mapRecord(r.agents, mapAgentState),
    recentGates: safeArray(r.recentGates ?? r.recent_gates, mapGateResult),
    recentEpisodes: safeArray(r.recentEpisodes ?? r.recent_episodes, mapEpisodeEntry),
    recentErrors: safeArray(r.recentErrors ?? r.recent_errors, mapErrorEntry),
    inboxItems: safeArray(r.inboxItems ?? r.inbox_items, mapInboxItem),
    vitals: mapVitalsSnapshot(r.vitals),
    affect: r.affect != null ? mapAffectState(r.affect) : null,
    providers: mapRecord(r.providers, mapProviderState),
    learning: mapLearningState(r.learning),
  };
}

// --- DashboardEvent safe coercion -------------------------------------------
//
// The SSE client already validates that `type` is a string; this function
// adds a second defence layer that coerces any nested entities that could
// crash a component if they arrive malformed.

export function safeDashboardEvent(raw: unknown): DashboardEvent | null {
  const r = asRecord(raw);
  const type = safeString(r.type);
  if (!type) return null;

  try {
    switch (type) {
      // Snapshot contains all nested entities — pass through the full mapper.
      case 'snapshot':
        return {
          type: 'snapshot',
          snapshot: mapDashboardSnapshot(r.snapshot),
          cursor: safeString(r.cursor),
          timestamp: safeString(r.timestamp, new Date().toISOString()),
        };

      // Gate result carries a nested GateResult.
      case 'gate_result':
        return {
          type: 'gate_result',
          result: mapGateResult(r.result),
          timestamp: safeString(r.timestamp, new Date().toISOString()),
        };

      // Agent output — keep chunk as-is since it is streaming text.
      case 'agent_output':
        return {
          type: 'agent_output',
          agentId: safeString(r.agentId ?? r.agent_id),
          taskId: safeStringOrNull(r.taskId ?? r.task_id),
          chunk: safeString(r.chunk),
          done: safeBool(r.done),
          timestamp: safeString(r.timestamp, new Date().toISOString()),
        };

      // All other event types carry only primitive fields — return as-is since
      // they were already structurally validated by the SSE client.
      default:
        return raw as DashboardEvent;
    }
  } catch (err) {
    if (IS_DEV) {
      // eslint-disable-next-line no-console
      console.warn('[safe-api] safeDashboardEvent: mapper threw on event type', type, err);
    }
    return null;
  }
}

// ---------------------------------------------------------------------------
// Health response
// ---------------------------------------------------------------------------

export interface HealthResponse {
  status: string;
  version: string;
  uptimeSecs: number;
  activeAgents: number;
  activePlans: number;
  activeRuns: number;
  providers: {
    healthy: number;
    degraded: number;
    unhealthy: number;
    total: number;
  };
  statehub: {
    cursor: string;
    eventsRetained: number;
    snapshot: {
      agentsActive: number;
      costUsdTotal: number;
      episodesTotal: number;
      errorsTotal: number;
      gatesFailed: number;
      gatesPassed: number;
      plansActive: number;
      tasksActive: number;
    };
  };
}

export function mapHealthResponse(raw: unknown): HealthResponse {
  const r = asRecord(raw);
  const providers = asRecord(r.providers);
  const statehub = asRecord(r.statehub);
  const snapshot = asRecord(statehub.snapshot);
  return {
    status: safeString(r.status, 'unknown'),
    version: safeString(r.version),
    uptimeSecs: safeNumber(r.uptime_secs ?? r.uptimeSecs),
    activeAgents: safeNumber(r.active_agents ?? r.activeAgents),
    activePlans: safeNumber(r.active_plans ?? r.activePlans),
    activeRuns: safeNumber(r.active_runs ?? r.activeRuns),
    providers: {
      healthy: safeNumber(providers.healthy),
      degraded: safeNumber(providers.degraded),
      unhealthy: safeNumber(providers.unhealthy),
      total: safeNumber(providers.total),
    },
    statehub: {
      cursor: safeString(statehub.cursor),
      eventsRetained: safeNumber(statehub.events_retained ?? statehub.eventsRetained),
      snapshot: {
        agentsActive: safeNumber(snapshot.agents_active ?? snapshot.agentsActive),
        costUsdTotal: safeNumber(snapshot.cost_usd_total ?? snapshot.costUsdTotal),
        episodesTotal: safeNumber(snapshot.episodes_total ?? snapshot.episodesTotal),
        errorsTotal: safeNumber(snapshot.errors_total ?? snapshot.errorsTotal),
        gatesFailed: safeNumber(snapshot.gates_failed ?? snapshot.gatesFailed),
        gatesPassed: safeNumber(snapshot.gates_passed ?? snapshot.gatesPassed),
        plansActive: safeNumber(snapshot.plans_active ?? snapshot.plansActive),
        tasksActive: safeNumber(snapshot.tasks_active ?? snapshot.tasksActive),
      },
    },
  };
}

// ---------------------------------------------------------------------------
// PRD types
// ---------------------------------------------------------------------------

export interface PrdSummary {
  slug: string;
  title: string;
  stage: string;
  updatedAt: string;
}

export interface PrdDetail {
  slug: string;
  title: string;
  stage: string;
  content: string;
  updatedAt: string;
}

export interface PrdCoverageStatus {
  total: number;
  drafted: number;
  promoted: number;
  planned: number;
}

export function mapPrdSummary(raw: unknown): PrdSummary {
  const r = asRecord(raw);
  return {
    slug: safeString(r.slug),
    title: safeString(r.title ?? r.slug),
    stage: safeString(r.stage ?? r.status, 'idea'),
    updatedAt: safeString(r.updatedAt ?? r.updated_at, new Date().toISOString()),
  };
}

export function mapPrdDetail(raw: unknown): PrdDetail {
  const r = asRecord(raw);
  return {
    slug: safeString(r.slug),
    title: safeString(r.title ?? r.slug),
    stage: safeString(r.stage ?? r.status, 'idea'),
    content: safeString(r.content ?? r.body ?? r.text),
    updatedAt: safeString(r.updatedAt ?? r.updated_at, new Date().toISOString()),
  };
}

export function mapPrdCoverageStatus(raw: unknown): PrdCoverageStatus {
  const r = asRecord(raw);
  return {
    total: safeNumber(r.total),
    drafted: safeNumber(r.drafted),
    promoted: safeNumber(r.promoted),
    planned: safeNumber(r.planned),
  };
}

// ---------------------------------------------------------------------------
// Knowledge stats
// ---------------------------------------------------------------------------

export interface KnowledgeQueryResult {
  results: KnowledgeEntry[];
  total: number;
  latencyMs: number;
}

export interface KnowledgeStatsResult {
  total: number;
  byTier: Record<string, number>;
  byDomain: Record<string, number>;
}

export function mapKnowledgeQueryResult(raw: unknown): KnowledgeQueryResult {
  const r = asRecord(raw);
  return {
    results: safeArray(r.results, mapKnowledgeEntry),
    total: safeNumber(r.total),
    latencyMs: safeNumber(r.latency_ms ?? r.latencyMs),
  };
}

// ---------------------------------------------------------------------------
// Reflexes & Gate thresholds
// ---------------------------------------------------------------------------

export interface ReflexRule {
  id: string;
  pattern: string;
  action: string;
  hitCount: number;
  lastMatchAt: string | null;
}

export interface ReflexesResponse {
  total: number;
  rules: ReflexRule[];
}

export function mapReflexRule(raw: unknown): ReflexRule {
  const r = asRecord(raw);
  return {
    id: safeString(r.id),
    pattern: safeString(r.pattern),
    action: safeString(r.action),
    hitCount: safeNumber(r.hitCount ?? r.hit_count),
    lastMatchAt: safeStringOrNull(r.lastMatchAt ?? r.last_match_at),
  };
}

export function mapReflexesResponse(raw: unknown): ReflexesResponse {
  const r = asRecord(raw);
  return {
    total: safeNumber(r.total),
    rules: safeArray(r.rules, mapReflexRule),
  };
}

export interface GateThresholdEntry {
  threshold: number;
  ema: number;
  samples: number;
}

export interface GateThresholdsResponse {
  thresholds: Record<string, GateThresholdEntry>;
}

export function mapGateThresholdsResponse(raw: unknown): GateThresholdsResponse {
  const r = asRecord(raw);
  const thresholds: Record<string, GateThresholdEntry> = {};
  const rawThresholds = asRecord(r.thresholds);
  for (const [key, val] of Object.entries(rawThresholds)) {
    const v = asRecord(val);
    thresholds[key] = {
      threshold: safeNumber(v.threshold),
      ema: safeNumber(v.ema),
      samples: safeNumber(v.samples),
    };
  }
  return { thresholds };
}

// ---------------------------------------------------------------------------
// SafeApi class
// ---------------------------------------------------------------------------

/**
 * SafeApi wraps the singleton `api` HTTP client.  Every method:
 *  - Catches network / parse errors and returns a safe empty value
 *  - Passes the raw JSON through the appropriate entity mapper
 *  - Never throws; callers receive empty arrays / safe defaults on failure
 */
export class SafeApi {
  // -------------------------------------------------------------------------
  // Health
  // -------------------------------------------------------------------------

  async getHealth(): Promise<HealthResponse> {
    try {
      const raw = await api.get<unknown>('/api/health');
      return mapHealthResponse(raw);
    } catch {
      return mapHealthResponse({});
    }
  }

  // -------------------------------------------------------------------------
  // Plans
  // -------------------------------------------------------------------------

  async getPlans(): Promise<PlanListItem[]> {
    try {
      const raw = await api.get<unknown>('/api/plans');
      return safeArray(raw, mapPlanListItem);
    } catch {
      return [];
    }
  }

  async getPlan(id: string): Promise<PlanState> {
    try {
      const raw = await api.get<unknown>(`/api/plans/${id}`);
      return mapPlanState(raw);
    } catch {
      return mapPlanState({});
    }
  }

  async getPlanTasks(planId: string): Promise<TaskState[]> {
    try {
      const raw = await api.get<unknown>(`/api/plans/${planId}/tasks`);
      return safeArray(raw, mapTaskState);
    } catch {
      return [];
    }
  }

  async getPlanGateResults(planId: string): Promise<GateResult[]> {
    try {
      const raw = await api.get<unknown>(`/api/plans/${planId}/gates`);
      return safeArray(raw, mapGateResult);
    } catch {
      return [];
    }
  }

  // -------------------------------------------------------------------------
  // Agents
  // -------------------------------------------------------------------------

  async getAgents(): Promise<AgentState[]> {
    try {
      const raw = await api.get<unknown>('/api/managed-agents');
      return safeArray(raw, mapAgentState);
    } catch {
      return [];
    }
  }

  async getAgent(id: string): Promise<AgentState | null> {
    try {
      const raw = await api.get<unknown>(`/api/agents/${id}`);
      return mapAgentState(raw);
    } catch {
      return null;
    }
  }

  // -------------------------------------------------------------------------
  // PRDs
  // -------------------------------------------------------------------------

  async getPrds(): Promise<PrdSummary[]> {
    try {
      const raw = await api.get<unknown>('/api/prds');
      return safeArray(raw, mapPrdSummary);
    } catch {
      return [];
    }
  }

  async getPrd(slug: string): Promise<PrdDetail> {
    try {
      const raw = await api.get<unknown>(`/api/prds/${slug}`);
      return mapPrdDetail(raw);
    } catch {
      return mapPrdDetail({});
    }
  }

  async getPrdStatus(): Promise<PrdCoverageStatus> {
    try {
      const raw = await api.get<unknown>('/api/prds/status');
      return mapPrdCoverageStatus(raw);
    } catch {
      return mapPrdCoverageStatus({});
    }
  }

  // -------------------------------------------------------------------------
  // Knowledge
  // -------------------------------------------------------------------------

  async queryKnowledge(
    query: string,
    opts?: { limit?: number; minTier?: string },
  ): Promise<KnowledgeQueryResult> {
    try {
      const raw = await api.post<unknown>('/api/neuro/query', {
        query,
        limit: opts?.limit ?? 10,
        min_tier: opts?.minTier,
      });
      return mapKnowledgeQueryResult(raw);
    } catch {
      return { results: [], total: 0, latencyMs: 0 };
    }
  }

  async getKnowledgeStats(): Promise<KnowledgeStatsResult> {
    try {
      const raw = await api.get<unknown>('/api/retrieval/stats');
      const r = asRecord(raw);
      const byDomain: Record<string, number> = {};
      for (const s of safeArray(r.strategies)) {
        const sr = asRecord(s);
        byDomain[safeString(sr.strategy)] = safeNumber(sr.passed);
      }
      return {
        total: safeNumber(r.total_settled),
        byTier: {},
        byDomain,
      };
    } catch {
      return { total: 0, byTier: {}, byDomain: {} };
    }
  }

  // -------------------------------------------------------------------------
  // Learning
  // -------------------------------------------------------------------------

  async getCascadeRouter(): Promise<CascadeRouterSnapshot> {
    try {
      const raw = await api.get<unknown>('/api/learn/cascade-router');
      return mapCascadeRouter(raw);
    } catch {
      return { stage: 'static', observationCount: 0, models: [] };
    }
  }

  async getEfficiency(period?: string): Promise<EfficiencyResponse> {
    try {
      const raw = await api.get<unknown>('/api/learn/efficiency');
      return mapEfficiency(raw, period);
    } catch {
      return { events: [], summary: { totalEvents: 0, avgGatePassRate: 0, totalCostUsd: 0 } };
    }
  }

  async getPlaybooks(): Promise<PlaybookEntry[]> {
    try {
      const raw = await api.get<unknown>('/api/learn/playbooks');
      const r = asRecord(raw);
      return safeArray(r.playbooks, mapPlaybookEntry);
    } catch {
      return [];
    }
  }

  async getExperiments(): Promise<ExperimentEntry[]> {
    try {
      const raw = await api.get<unknown>('/api/learn/experiments');
      const r = asRecord(raw);
      return safeArray(r.active_experiments, mapExperimentEntry);
    } catch {
      return [];
    }
  }

  async getReflexes(): Promise<ReflexesResponse> {
    try {
      const raw = await api.get<unknown>('/api/learn/reflexes');
      return mapReflexesResponse(raw);
    } catch {
      return { total: 0, rules: [] };
    }
  }

  async getGateThresholds(): Promise<GateThresholdsResponse> {
    try {
      const raw = await api.get<unknown>('/api/learn/gate-thresholds');
      return mapGateThresholdsResponse(raw);
    } catch {
      return { thresholds: {} };
    }
  }

  // -------------------------------------------------------------------------
  // Providers
  // -------------------------------------------------------------------------

  async getProviders(): Promise<ProviderState[]> {
    try {
      const raw = await api.get<unknown>('/api/providers');
      const r = asRecord(raw);
      // Server returns either an array or { providers: [...] }
      if (Array.isArray(raw)) return safeArray(raw, mapProviderState);
      return safeArray(r.providers, mapProviderState);
    } catch {
      return [];
    }
  }

  // -------------------------------------------------------------------------
  // Config
  // -------------------------------------------------------------------------

  async getConfig(): Promise<ConfigResponse> {
    try {
      const raw = await api.get<unknown>('/api/config');
      return mapConfigResponse(raw);
    } catch {
      return { config: {}, source: 'roko.toml', version: 2 };
    }
  }

  // -------------------------------------------------------------------------
  // Doctor
  // -------------------------------------------------------------------------

  async getDoctor(): Promise<DoctorResponse> {
    try {
      const raw = await api.get<unknown>('/api/doctor');
      return mapDoctorResponse(raw);
    } catch {
      return {
        healthy: false,
        workdir: '',
        checks: [],
        summary: { ok: 0, warn: 0, fail: 0, total: 0, skipped: 0 },
      };
    }
  }

  // -------------------------------------------------------------------------
  // Dashboard snapshot
  // -------------------------------------------------------------------------

  async getStatus(): Promise<DashboardSnapshot> {
    try {
      const raw = await api.get<unknown>('/api/status');
      return mapDashboardSnapshot(raw);
    } catch {
      return mapDashboardSnapshot({});
    }
  }
}

// ---------------------------------------------------------------------------
// Singleton export
// ---------------------------------------------------------------------------

export const safeApi = new SafeApi();
