/**
 * Exact wire contract emitted by roko-serve's `/api/events` route.
 *
 * Keep this union in sync with `roko_core::DashboardEvent`. The endpoint does
 * not emit the larger internal `roko_serve::ServerEvent` enum; bench events,
 * inference events, and config reload events therefore do not belong here.
 */

export type InboxCategory =
  | 'gate_verdict'
  | 'agent_question'
  | 'budget_alert'
  | 'task_completion'
  | 'structural_change'
  | 'security_event'
  | 'knowledge_event'
  | 'system_event';

export type UrgencyLevel = 'notify' | 'question' | 'review';

export interface DiagnosisSummary {
  id: string;
  ts: string;
  severity: 'info' | 'warn' | 'alert';
  subject: string;
  detail: string;
  suggested_action: string | null;
  intervention_taken: string | null;
}

export interface ExperimentWinnerSummary {
  experiment_id: string;
  parameter: string;
  winner: string;
  winner_variant_id: string;
  win_rate: number;
  sample_size: number;
  ci_lower: number;
  ci_upper: number;
  confidence: number;
}

export interface CFactorBucket {
  start: string;
  samples: number;
  avg: number;
  p50: number;
  p95: number;
}

export interface EfficiencyBucket {
  start: string;
  turns: number;
  tokens_in: number;
  tokens_out: number;
  cost_usd_cents: number;
  latency_ms_avg: number;
}

export interface MarketplaceJob {
  id: string;
  title: string;
  description: string;
  job_type: string;
  status: string;
  state?: string;
  posted_by: string;
  assigned_to: string;
  priority: string;
  created_at: string;
  updated_at: string;
  tags: string[];
  reward: string;
  plan_id: string;
  submission?: unknown;
  evaluation?: unknown;
  auto_execute: boolean;
}

export interface PrdSummary {
  slug: string;
  title: string;
  status: string;
  plan_count: number;
  task_total: number;
  task_done: number;
  task_failed: number;
}

export interface TaskSummary {
  id: string;
  title: string;
  status: string;
  agent: string;
}

export interface KnowledgeBrowseEntry {
  id: string;
  kind: string;
  content_preview: string;
  confidence: number;
  tier: string;
  tags: string[];
  created_at: string;
  frozen: boolean;
}

export interface DashboardPlanState {
  plan_id: string;
  phase: string;
  tasks_total: number;
  tasks_done: number;
  tasks_failed: number;
  active: boolean;
}

export interface DashboardTaskState {
  task_id: string;
  title: string;
  plan_id: string;
  phase: string;
  outcome: string | null;
}

export interface DashboardAgentState {
  agent_id: string;
  role: string;
  active: boolean;
  output_bytes: number;
  model: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  cost_usd: number;
  current_task: string;
  current_plan: string;
  attempt: number;
  spawned_at_ms: number;
  last_event_at_ms: number;
}

export interface DashboardGateVerdict {
  plan_id: string;
  task_id: string;
  gate: string;
  passed: boolean;
  ts_millis: number;
}

export interface DashboardEpisodeSummary {
  episode_id: string;
  agent_id: string;
  role: string;
  passed: boolean;
  ts_millis: number;
}

export interface DashboardAgentTopology {
  nodes: Array<{
    id: string;
    address: string;
    insights_posted: number;
    confirmations_given: number;
    challenges_given: number;
    total_weight: number;
  }>;
  edges: Array<{
    from: string;
    to: string;
    weight: number;
    type: string;
  }>;
  timestamp: number;
}

export interface DashboardTrendBuckets {
  bucket_size_secs: number;
  slots: Array<{ start: string; pass: number; fail: number }>;
}

export interface DashboardFailureEntry {
  ts: string;
  plan_id: string;
  task_id: string;
  gate: string;
  summary: string;
  artifacts: string | null;
}

export interface DashboardErrorEntry {
  message: string;
  ts_millis: number;
}

export interface DashboardEventLogEntry {
  timestamp_ms: number;
  event_type: string;
  plan_id: string;
  task_id: string;
  message: string;
}

export interface DashboardInboxItem {
  item_id: string;
  category: InboxCategory;
  urgency: UrgencyLevel;
  summary: string;
  received_at_ms: number;
  defer_until: string | null;
}

export interface DashboardSnapshotStats {
  plans_active: number;
  plans_completed: number;
  plans_failed: number;
  tasks_active: number;
  tasks_completed: number;
  tasks_failed: number;
  agents_active: number;
  gates_passed: number;
  gates_failed: number;
  errors_total: number;
  episodes_total: number;
  cost_usd_total: number;
}

/** Full materialized state carried by a named SSE `gap` frame. */
export interface DashboardSnapshot {
  plans: Record<string, DashboardPlanState>;
  tasks: Record<string, DashboardTaskState>;
  agents: Record<string, DashboardAgentState>;
  gates: DashboardGateVerdict[];
  diagnoses: DiagnosisSummary[];
  experiment_winners: ExperimentWinnerSummary[];
  agent_topology: DashboardAgentTopology;
  efficiency_trend: EfficiencyBucket[];
  cfactor_trend: CFactorBucket[];
  gate_trends: Record<string, DashboardTrendBuckets>;
  gate_recent_failures: DashboardFailureEntry[];
  episodes: DashboardEpisodeSummary[];
  errors: DashboardErrorEntry[];
  event_log: DashboardEventLogEntry[];
  task_outputs: Record<string, string[]>;
  cascade_router_json: string;
  gate_thresholds_json: string;
  marketplace_jobs: MarketplaceJob[];
  atelier_prds: PrdSummary[];
  atelier_tasks: Record<string, TaskSummary[]>;
  knowledge_entries: KnowledgeBrowseEntry[];
  payment_count: number;
  total_payment_korai: number;
  payments_by_protocol: Record<string, number>;
  settlement_count: number;
  inbox_items: Record<string, DashboardInboxItem>;
  inbox_resolved_ids: string[];
  inbox_pending_count: number;
  stats: DashboardSnapshotStats;
}

export interface DashboardGapPayload {
  missed_events: number;
  last_materialized_seq: number;
  snapshot: DashboardSnapshot;
}

export type DashboardEvent =
  | { type: 'plan_started'; plan_id: string }
  | { type: 'plan_completed'; plan_id: string; success: boolean }
  | { type: 'task_started'; plan_id: string; task_id: string; title: string; phase: string }
  | { type: 'task_completed'; plan_id: string; task_id: string; outcome: string }
  | { type: 'task_phase_changed'; plan_id: string; task_id: string; old_phase: string; new_phase: string }
  | { type: 'agent_spawned'; agent_id: string; plan_id: string; task_id: string; attempt: number; role: string; model: string }
  | { type: 'agent_output'; agent_id: string; plan_id: string; task_id: string; attempt: number; content: string }
  | { type: 'gate_result'; plan_id: string; task_id: string; gate: string; passed: boolean }
  | { type: 'phase_transition'; plan_id: string; from: string; to: string }
  | { type: 'efficiency_event'; plan_id: string; task_id: string; metric: string; value: number }
  | { type: 'diagnosis'; summary: DiagnosisSummary }
  | { type: 'experiment_winners_updated'; winners: ExperimentWinnerSummary[] }
  | { type: 'c_factor_trend_updated'; buckets: CFactorBucket[] }
  | { type: 'projection_updated'; projection_id: string; version: number; source_lens: string }
  | { type: 'episode_recorded'; agent_id: string; role: string; episode_id: string; passed: boolean }
  | { type: 'task_output_appended'; task_id: string; lines: string[] }
  | { type: 'event_log_entry'; timestamp_ms: number; event_type: string; plan_id: string; task_id: string; message: string }
  | { type: 'cascade_router_updated'; snapshot_json: string }
  | { type: 'gate_thresholds_updated'; snapshot_json: string }
  | { type: 'agent_completed'; agent_id: string; plan_id: string; task_id: string; attempt: number }
  | { type: 'marketplace_jobs_updated'; jobs: MarketplaceJob[] }
  | { type: 'atelier_prds_updated'; prds: PrdSummary[]; tasks: Record<string, TaskSummary[]> }
  | { type: 'knowledge_entries_updated'; entries: KnowledgeBrowseEntry[] }
  | { type: 'job_execution_started'; job_id: string; job_type: string; agent_id: string }
  | { type: 'job_progress'; job_id: string; percent: number; message: string }
  | { type: 'efficiency_trend_updated'; buckets: EfficiencyBucket[] }
  | { type: 'chain_block'; number: number; hash: string; parent_hash: string; timestamp: number; gas_used: number; gas_limit: number; tx_count: number; base_fee_per_gas: number | null }
  | { type: 'chain_tx'; block_number: number; tx_hash: string; from: string; to: string | null; value_wei: string; gas_used: number; method_sig: string | null; success: boolean }
  | { type: 'chain_contract_event'; block_number: number; tx_hash: string; log_index: number; contract: string; event_name: string; decoded: unknown }
  | { type: 'feed_tick'; agent_id: string; feed_id: string; topic: string; payload: unknown; timestamp_ms: number }
  | { type: 'feed_agent_online'; agent_id: string; name: string; feed_count: number }
  | { type: 'feed_agent_offline'; agent_id: string }
  | { type: 'payment_received'; feed_id: string; protocol: string; amount_korai: number; payer: string; payee: string }
  | { type: 'settlement_completed'; protocol: string; batch_size: number; total_korai: number }
  | { type: 'inbox_item_received'; item_id: string; category: InboxCategory; urgency: UrgencyLevel; summary: string }
  | { type: 'inbox_approve'; item_id: string }
  | { type: 'inbox_reject'; item_id: string; reason: string }
  | { type: 'inbox_defer'; item_id: string; defer_until: string }
  | { type: 'inbox_dismiss'; item_id: string }
  | { type: 'error'; message: string };

export const DASHBOARD_EVENT_TYPES = [
  'plan_started',
  'plan_completed',
  'task_started',
  'task_completed',
  'task_phase_changed',
  'agent_spawned',
  'agent_output',
  'gate_result',
  'phase_transition',
  'efficiency_event',
  'diagnosis',
  'experiment_winners_updated',
  'c_factor_trend_updated',
  'projection_updated',
  'episode_recorded',
  'task_output_appended',
  'event_log_entry',
  'cascade_router_updated',
  'gate_thresholds_updated',
  'agent_completed',
  'marketplace_jobs_updated',
  'atelier_prds_updated',
  'knowledge_entries_updated',
  'job_execution_started',
  'job_progress',
  'efficiency_trend_updated',
  'chain_block',
  'chain_tx',
  'chain_contract_event',
  'feed_tick',
  'feed_agent_online',
  'feed_agent_offline',
  'payment_received',
  'settlement_completed',
  'inbox_item_received',
  'inbox_approve',
  'inbox_reject',
  'inbox_defer',
  'inbox_dismiss',
  'error',
] as const satisfies readonly DashboardEvent['type'][];

const MAX_WIRE_STRING = 256_000;
const MAX_WIRE_ITEMS = 10_000;
const MAX_WIRE_JSON = 1_000_000;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function isString(value: unknown, max = MAX_WIRE_STRING): value is string {
  return typeof value === 'string' && value.length <= max;
}

function isBoolean(value: unknown): value is boolean {
  return typeof value === 'boolean';
}

function isNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

function isUint(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}

function isNullableString(value: unknown): value is string | null {
  return value === null || isString(value);
}

function isOptionalString(value: unknown): value is string | undefined {
  return value === undefined || isString(value);
}

function isArrayOf<T>(
  value: unknown,
  validate: (item: unknown) => item is T,
  max = MAX_WIRE_ITEMS,
): value is T[] {
  return Array.isArray(value) && value.length <= max && value.every((item) => validate(item));
}

function isBoundedJson(value: unknown): boolean {
  try {
    const encoded = JSON.stringify(value);
    return encoded !== undefined && encoded.length <= MAX_WIRE_JSON;
  } catch {
    return false;
  }
}

function hasStrings(record: Record<string, unknown>, fields: readonly string[]): boolean {
  return fields.every((field) => isString(record[field]));
}

function hasUints(record: Record<string, unknown>, fields: readonly string[]): boolean {
  return fields.every((field) => isUint(record[field]));
}

function isDiagnosis(value: unknown): value is DiagnosisSummary {
  if (!isRecord(value)) return false;
  return hasStrings(value, ['id', 'ts', 'subject', 'detail'])
    && (value.severity === 'info' || value.severity === 'warn' || value.severity === 'alert')
    && isNullableString(value.suggested_action)
    && isNullableString(value.intervention_taken);
}

function isExperimentWinner(value: unknown): value is ExperimentWinnerSummary {
  if (!isRecord(value)) return false;
  return hasStrings(value, ['experiment_id', 'parameter', 'winner', 'winner_variant_id'])
    && ['win_rate', 'ci_lower', 'ci_upper', 'confidence'].every((field) => isNumber(value[field]))
    && isUint(value.sample_size);
}

function isCFactorBucket(value: unknown): value is CFactorBucket {
  if (!isRecord(value)) return false;
  return isString(value.start) && isUint(value.samples)
    && isNumber(value.avg) && isNumber(value.p50) && isNumber(value.p95);
}

function isEfficiencyBucket(value: unknown): value is EfficiencyBucket {
  if (!isRecord(value)) return false;
  return isString(value.start)
    && hasUints(value, ['turns', 'tokens_in', 'tokens_out', 'cost_usd_cents'])
    && isNumber(value.latency_ms_avg);
}

function isMarketplaceJob(value: unknown): value is MarketplaceJob {
  if (!isRecord(value)) return false;
  return hasStrings(value, [
    'id', 'title', 'description', 'job_type', 'status', 'posted_by', 'assigned_to',
    'priority', 'created_at', 'updated_at', 'reward', 'plan_id',
  ])
    && isOptionalString(value.state)
    && isArrayOf(value.tags, isString)
    && (value.submission === undefined || isBoundedJson(value.submission))
    && (value.evaluation === undefined || isBoundedJson(value.evaluation))
    && isBoolean(value.auto_execute);
}

function isPrdSummary(value: unknown): value is PrdSummary {
  if (!isRecord(value)) return false;
  return hasStrings(value, ['slug', 'title', 'status'])
    && hasUints(value, ['plan_count', 'task_total', 'task_done', 'task_failed']);
}

function isTaskSummary(value: unknown): value is TaskSummary {
  return isRecord(value) && hasStrings(value, ['id', 'title', 'status', 'agent']);
}

function isKnowledgeEntry(value: unknown): value is KnowledgeBrowseEntry {
  if (!isRecord(value)) return false;
  return hasStrings(value, ['id', 'kind', 'content_preview', 'tier', 'created_at'])
    && isNumber(value.confidence)
    && isArrayOf(value.tags, isString)
    && isBoolean(value.frozen);
}

function isRecordOf<T>(
  value: unknown,
  validate: (item: unknown) => item is T,
): value is Record<string, T> {
  if (!isRecord(value)) return false;
  const entries = Object.entries(value);
  return entries.length <= MAX_WIRE_ITEMS
    && entries.every(([key, item]) => isString(key, 4_096) && validate(item));
}

function mergedPayload(raw: Record<string, unknown>): Record<string, unknown> {
  const nested = isRecord(raw.data) ? raw.data : {};
  return { ...nested, ...raw };
}

/** Parse one `/api/events` payload with per-variant bounded validation. */
export function parseDashboardEvent(raw: Record<string, unknown>): DashboardEvent | null {
  const event = mergedPayload(raw);
  if (!isBoundedJson(event)) return null;
  const type = event.type;
  if (!isString(type, 64)) return null;

  let valid = false;
  switch (type) {
    case 'plan_started':
      valid = hasStrings(event, ['plan_id']);
      break;
    case 'plan_completed':
      valid = hasStrings(event, ['plan_id']) && isBoolean(event.success);
      break;
    case 'task_started':
      valid = hasStrings(event, ['plan_id', 'task_id', 'title', 'phase']);
      break;
    case 'task_completed':
      valid = hasStrings(event, ['plan_id', 'task_id', 'outcome']);
      break;
    case 'task_phase_changed':
      valid = hasStrings(event, ['plan_id', 'task_id', 'old_phase', 'new_phase']);
      break;
    case 'agent_spawned':
      valid = hasStrings(event, ['agent_id', 'plan_id', 'task_id', 'role', 'model'])
        && isUint(event.attempt);
      break;
    case 'agent_output':
      valid = hasStrings(event, ['agent_id', 'plan_id', 'task_id', 'content'])
        && isUint(event.attempt);
      break;
    case 'gate_result':
      valid = hasStrings(event, ['plan_id', 'task_id', 'gate']) && isBoolean(event.passed);
      break;
    case 'phase_transition':
      valid = hasStrings(event, ['plan_id', 'from', 'to']);
      break;
    case 'efficiency_event':
      valid = hasStrings(event, ['plan_id', 'task_id', 'metric']) && isNumber(event.value);
      break;
    case 'diagnosis':
      valid = isDiagnosis(event.summary);
      break;
    case 'experiment_winners_updated':
      valid = isArrayOf(event.winners, isExperimentWinner);
      break;
    case 'c_factor_trend_updated':
      valid = isArrayOf(event.buckets, isCFactorBucket);
      break;
    case 'projection_updated':
      valid = hasStrings(event, ['projection_id', 'source_lens']) && isUint(event.version);
      break;
    case 'episode_recorded':
      valid = hasStrings(event, ['agent_id', 'role', 'episode_id']) && isBoolean(event.passed);
      break;
    case 'task_output_appended':
      valid = isString(event.task_id) && isArrayOf(event.lines, isString, 1_000);
      break;
    case 'event_log_entry':
      valid = isUint(event.timestamp_ms)
        && hasStrings(event, ['event_type', 'plan_id', 'task_id', 'message']);
      break;
    case 'cascade_router_updated':
    case 'gate_thresholds_updated':
      valid = isString(event.snapshot_json, MAX_WIRE_JSON);
      break;
    case 'agent_completed':
      valid = hasStrings(event, ['agent_id', 'plan_id', 'task_id']) && isUint(event.attempt);
      break;
    case 'marketplace_jobs_updated':
      valid = isArrayOf(event.jobs, isMarketplaceJob);
      break;
    case 'atelier_prds_updated':
      valid = isArrayOf(event.prds, isPrdSummary)
        && isRecordOf(event.tasks, (tasks): tasks is TaskSummary[] => isArrayOf(tasks, isTaskSummary));
      break;
    case 'knowledge_entries_updated':
      valid = isArrayOf(event.entries, isKnowledgeEntry);
      break;
    case 'job_execution_started':
      valid = hasStrings(event, ['job_id', 'job_type', 'agent_id']);
      break;
    case 'job_progress':
      valid = hasStrings(event, ['job_id', 'message']) && isUint(event.percent) && event.percent <= 100;
      break;
    case 'efficiency_trend_updated':
      valid = isArrayOf(event.buckets, isEfficiencyBucket);
      break;
    case 'chain_block':
      valid = hasStrings(event, ['hash', 'parent_hash'])
        && hasUints(event, ['number', 'timestamp', 'gas_used', 'gas_limit', 'tx_count'])
        && (event.base_fee_per_gas === null || isUint(event.base_fee_per_gas));
      break;
    case 'chain_tx':
      valid = hasStrings(event, ['tx_hash', 'from', 'value_wei'])
        && isUint(event.block_number) && isUint(event.gas_used)
        && isNullableString(event.to) && isNullableString(event.method_sig)
        && isBoolean(event.success);
      break;
    case 'chain_contract_event':
      valid = hasStrings(event, ['tx_hash', 'contract', 'event_name'])
        && isUint(event.block_number) && isUint(event.log_index) && isBoundedJson(event.decoded);
      break;
    case 'feed_tick':
      valid = hasStrings(event, ['agent_id', 'feed_id', 'topic'])
        && typeof event.timestamp_ms === 'number'
        && Number.isSafeInteger(event.timestamp_ms)
        && isBoundedJson(event.payload);
      break;
    case 'feed_agent_online':
      valid = hasStrings(event, ['agent_id', 'name']) && isUint(event.feed_count);
      break;
    case 'feed_agent_offline':
      valid = hasStrings(event, ['agent_id']);
      break;
    case 'payment_received':
      valid = hasStrings(event, ['feed_id', 'protocol', 'payer', 'payee']) && isNumber(event.amount_korai);
      break;
    case 'settlement_completed':
      valid = isString(event.protocol) && isUint(event.batch_size) && isNumber(event.total_korai);
      break;
    case 'inbox_item_received':
      valid = hasStrings(event, ['item_id', 'summary'])
        && isInboxCategory(event.category)
        && isUrgency(event.urgency);
      break;
    case 'inbox_approve':
    case 'inbox_dismiss':
      valid = isString(event.item_id);
      break;
    case 'inbox_reject':
      valid = hasStrings(event, ['item_id', 'reason']);
      break;
    case 'inbox_defer':
      valid = hasStrings(event, ['item_id', 'defer_until']);
      break;
    case 'error':
      valid = isString(event.message);
      break;
    default:
      return null;
  }

  return valid ? event as DashboardEvent : null;
}

function isPlanState(value: unknown): value is DashboardPlanState {
  if (!isRecord(value)) return false;
  return hasStrings(value, ['plan_id', 'phase'])
    && hasUints(value, ['tasks_total', 'tasks_done', 'tasks_failed'])
    && isBoolean(value.active);
}

function isTaskState(value: unknown): value is DashboardTaskState {
  if (!isRecord(value)) return false;
  return hasStrings(value, ['task_id', 'title', 'plan_id', 'phase'])
    && isNullableString(value.outcome);
}

function isAgentState(value: unknown): value is DashboardAgentState {
  if (!isRecord(value)) return false;
  return hasStrings(value, ['agent_id', 'role', 'model', 'current_task', 'current_plan'])
    && isBoolean(value.active)
    && hasUints(value, [
      'output_bytes', 'input_tokens', 'output_tokens', 'cache_read_tokens', 'cache_write_tokens',
      'attempt', 'spawned_at_ms', 'last_event_at_ms',
    ])
    && isNumber(value.cost_usd);
}

function isGateVerdict(value: unknown): value is DashboardGateVerdict {
  if (!isRecord(value)) return false;
  return hasStrings(value, ['plan_id', 'task_id', 'gate'])
    && isBoolean(value.passed)
    && isUint(value.ts_millis);
}

function isEpisodeSummary(value: unknown): value is DashboardEpisodeSummary {
  return isRecord(value)
    && hasStrings(value, ['episode_id', 'agent_id', 'role'])
    && isBoolean(value.passed)
    && isUint(value.ts_millis);
}

function isAgentTopology(value: unknown): value is DashboardAgentTopology {
  if (!isRecord(value)) return false;
  const isNode = (node: unknown): node is DashboardAgentTopology['nodes'][number] => {
    if (!isRecord(node)) return false;
    return hasStrings(node, ['id', 'address'])
      && hasUints(node, ['insights_posted', 'confirmations_given', 'challenges_given'])
      && isNumber(node.total_weight);
  };
  const isEdge = (edge: unknown): edge is DashboardAgentTopology['edges'][number] => {
    if (!isRecord(edge)) return false;
    return hasStrings(edge, ['from', 'to', 'type']) && isUint(edge.weight);
  };
  return isArrayOf(value.nodes, isNode)
    && isArrayOf(value.edges, isEdge)
    && isUint(value.timestamp);
}

function isTrendBuckets(value: unknown): value is DashboardTrendBuckets {
  if (!isRecord(value) || !isUint(value.bucket_size_secs)) return false;
  return isArrayOf(value.slots, (slot): slot is DashboardTrendBuckets['slots'][number] => (
    isRecord(slot) && isString(slot.start) && isUint(slot.pass) && isUint(slot.fail)
  ));
}

function isFailureEntry(value: unknown): value is DashboardFailureEntry {
  if (!isRecord(value)) return false;
  return hasStrings(value, ['ts', 'plan_id', 'task_id', 'gate', 'summary'])
    && isNullableString(value.artifacts);
}

function isErrorEntry(value: unknown): value is DashboardErrorEntry {
  return isRecord(value) && isString(value.message) && isUint(value.ts_millis);
}

function isEventLogEntry(value: unknown): value is DashboardEventLogEntry {
  if (!isRecord(value)) return false;
  return isUint(value.timestamp_ms)
    && hasStrings(value, ['event_type', 'plan_id', 'task_id', 'message']);
}

function isInboxCategory(value: unknown): value is InboxCategory {
  return typeof value === 'string' && [
    'gate_verdict',
    'agent_question',
    'budget_alert',
    'task_completion',
    'structural_change',
    'security_event',
    'knowledge_event',
    'system_event',
  ].includes(value);
}

function isUrgency(value: unknown): value is UrgencyLevel {
  return value === 'notify' || value === 'question' || value === 'review';
}

function isInboxItem(value: unknown): value is DashboardInboxItem {
  if (!isRecord(value)) return false;
  return hasStrings(value, ['item_id', 'summary'])
    && isInboxCategory(value.category)
    && isUrgency(value.urgency)
    && isUint(value.received_at_ms)
    && isNullableString(value.defer_until);
}

function isSnapshotStats(value: unknown): value is DashboardSnapshotStats {
  if (!isRecord(value)) return false;
  return hasUints(value, [
    'plans_active', 'plans_completed', 'plans_failed', 'tasks_active', 'tasks_completed',
    'tasks_failed', 'agents_active', 'gates_passed', 'gates_failed', 'errors_total',
    'episodes_total',
  ]) && isNumber(value.cost_usd_total);
}

function isDashboardSnapshot(value: unknown): value is DashboardSnapshot {
  if (!isRecord(value) || !isBoundedJson(value)) return false;
  return isRecordOf(value.plans, isPlanState)
    && isRecordOf(value.tasks, isTaskState)
    && isRecordOf(value.agents, isAgentState)
    && isArrayOf(value.gates, isGateVerdict)
    && isArrayOf(value.diagnoses, isDiagnosis)
    && isArrayOf(value.experiment_winners, isExperimentWinner)
    && isAgentTopology(value.agent_topology)
    && isArrayOf(value.efficiency_trend, isEfficiencyBucket)
    && isArrayOf(value.cfactor_trend, isCFactorBucket)
    && isRecordOf(value.gate_trends, isTrendBuckets)
    && isArrayOf(value.gate_recent_failures, isFailureEntry)
    && isArrayOf(value.episodes, isEpisodeSummary)
    && isArrayOf(value.errors, isErrorEntry)
    && isArrayOf(value.event_log, isEventLogEntry)
    && isRecordOf(value.task_outputs, (lines): lines is string[] => isArrayOf(lines, isString, 1_000))
    && isString(value.cascade_router_json, MAX_WIRE_JSON)
    && isString(value.gate_thresholds_json, MAX_WIRE_JSON)
    && isArrayOf(value.marketplace_jobs, isMarketplaceJob)
    && isArrayOf(value.atelier_prds, isPrdSummary)
    && isRecordOf(value.atelier_tasks, (tasks): tasks is TaskSummary[] => isArrayOf(tasks, isTaskSummary))
    && isArrayOf(value.knowledge_entries, isKnowledgeEntry)
    && isUint(value.payment_count)
    && isNumber(value.total_payment_korai)
    && isRecordOf(value.payments_by_protocol, isUint)
    && isUint(value.settlement_count)
    && isRecordOf(value.inbox_items, isInboxItem)
    && isArrayOf(value.inbox_resolved_ids, isString)
    && isUint(value.inbox_pending_count)
    && isSnapshotStats(value.stats);
}

/** Parse and bound-check a named `/api/events` replay-gap frame. */
export function parseDashboardGap(raw: Record<string, unknown>): DashboardGapPayload | null {
  const payload = mergedPayload(raw);
  if (payload.type !== 'gap'
    || !isUint(payload.missed_events)
    || !isUint(payload.last_materialized_seq)
    || !isDashboardSnapshot(payload.snapshot)) {
    return null;
  }
  return {
    missed_events: payload.missed_events,
    last_materialized_seq: payload.last_materialized_seq,
    snapshot: payload.snapshot,
  };
}
