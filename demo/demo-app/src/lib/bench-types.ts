/** Bench system types — mirrors Rust types in roko-serve. */

export type BenchRunKind = 'suite' | 'comparison' | 'regression';
export type AgentStrategy = 'minimal' | 'context_enriched' | 'neuro_augmented' | 'full_cascade' | 'demo';
export type BenchTaskStatusWire = 'pass' | 'fail' | 'skipped';
export type TaskStatus = BenchTaskStatusWire | 'pending' | 'running';

/** Exact serialized shape of Rust `BenchConfigOverrides`. */
export interface BenchConfigOverrides {
  model?: string;
  backend?: string;
  max_tokens?: number;
  temperature?: number;
  /** Omitted by serde when the strategy is the default `minimal`. */
  strategy?: AgentStrategy;
}

export interface BenchGateConfig {
  compile: boolean;
  test: boolean;
  clippy: boolean;
  diff: boolean;
}

export interface BenchRunConfig {
  model: string;
  backend?: string;
  provider?: string;
  temperature?: number;
  max_tokens?: number;
  timeout_secs: number;
  strategy: AgentStrategy;
  retries: number;
  system_prompt_override?: string;
  gates?: BenchGateConfig;
  max_cost_usd?: number;
  parallel?: boolean;
}

export interface BenchTask {
  id: string;
  name: string;
  prompt: string;
  expected_output?: string;
  expected_outcome?: string;
  difficulty: number; // 1-5
  tags: string[];
  timeout_secs?: number;
}

export interface BenchSuite {
  id: string;
  name: string;
  description: string;
  tasks: BenchTask[];
  task_count?: number;
  estimated_cost_usd: number;
  difficulty_range: [number, number];
}

/** Number of tasks in a suite, handling both full and summary shapes. */
export function suiteTaskCount(suite: BenchSuite): number {
  return suite.tasks?.length ?? suite.task_count ?? 0;
}

export interface BenchGateVerdict {
  gate: string;
  passed: boolean;
  message?: string;
  duration_ms?: number;
}

export interface BenchGateVerdictWire {
  gate: string;
  passed: boolean;
  detail: string;
}

export interface BenchTaskResult {
  task_id: string;
  task_name: string;
  status: TaskStatus;
  cost_usd: number;
  tokens_in: number;
  tokens_out: number;
  duration_ms: number;
  model: string;
  gate_verdicts: BenchGateVerdict[];
  error?: string;
  retries_used: number;
  output_preview?: string;
  difficulty?: number;
}

/** Exact serialized shape of Rust `BenchTaskResult`. */
export interface BenchTaskResultWire {
  task_id: string;
  task_name: string;
  status: BenchTaskStatusWire;
  duration_ms: number;
  model: string;
  tokens_in: number;
  tokens_out: number;
  cost_usd: number;
  gate_verdicts: BenchGateVerdictWire[];
  retries_used: number;
  output_preview?: string;
  error?: string;
}

export interface BenchRunSummary {
  total_tasks: number;
  passed: number;
  failed: number;
  skipped: number;
  total_cost_usd: number;
  total_tokens: number;
  total_duration_ms: number;
  pass_rate: number;
  cost_per_success_usd: number;
  avg_duration_ms: number;
}

export interface BenchRun {
  id: string;
  kind: BenchRunKind;
  config: BenchRunConfig;
  suite_id: string;
  suite_name: string;
  label?: string;
  status: 'running' | 'completed' | 'cancelled' | 'failed';
  results: BenchTaskResult[];
  summary?: BenchRunSummary;
  started_at: string;
  finished_at?: string;
}

/** Exact serialized shape returned by full bench run endpoints. */
export interface BenchRunWire {
  id: string;
  suite_id: string;
  suite_name: string;
  kind: BenchRunKind;
  config: BenchConfigOverrides;
  label?: string;
  status: 'running' | 'completed' | 'cancelled' | 'failed';
  started_at: string;
  finished_at?: string;
  results: BenchTaskResultWire[];
  summary?: BenchRunSummary;
  current_task_index: number;
  total_tasks: number;
}

/** Lightweight row returned by `GET /api/bench/runs`. */
export interface BenchRunListEntry {
  id: string;
  suite_id: string;
  suite_name: string;
  status: BenchRun['status'];
  started_at: number;
  finished_at?: number;
  label?: string;
  model?: string;
  pass_rate?: number;
  total_cost_usd?: number;
}

/** Summary returned by `GET /api/bench/suites`; fetch by id for full tasks. */
export interface BenchSuiteListEntry {
  id: string;
  name: string;
  description: string;
  task_count: number;
}

export interface BenchRunsListResponse {
  total: number;
  offset: number;
  limit: number;
  runs: BenchRunListEntry[];
}

export interface BenchSuitesResponse {
  suites: BenchSuiteListEntry[];
}

export interface BenchModelsResponse {
  models: string[];
}

export interface BenchModel {
  id: string;
  name: string;
  provider: string;
  cost_per_1k_input: number;
  cost_per_1k_output: number;
  max_tokens: number;
  context_window: number;
}

// SSE event types
export interface BenchRunStartedEvent {
  type: 'BenchRunStarted';
  bench_id: string;
  suite_id: string;
  total_tasks: number;
}

export interface BenchTaskStartedEvent {
  type: 'BenchTaskStarted';
  bench_id: string;
  task_id: string;
  task_name: string;
  task_index: number;
  total_tasks: number;
}

export interface BenchTaskCompletedEvent {
  type: 'BenchTaskCompleted';
  bench_id: string;
  task_id: string;
  result: BenchTaskResult;
}

export interface BenchProgressEvent {
  type: 'BenchProgress';
  bench_id: string;
  completed: number;
  total: number;
  cost_so_far: number;
}

export interface BenchRunCompletedEvent {
  type: 'BenchRunCompleted';
  bench_id: string;
  summary: BenchRunSummary;
}

export interface BenchLearningEvent {
  type: 'BenchLearningEvent';
  bench_id: string;
  task_id: string;
  playbooks_created: number;
  anti_patterns_created: number;
  total_playbooks: number;
  total_anti_patterns: number;
}

export interface BenchRegressionReportEvent {
  type: 'BenchRegressionReport';
  bench_id: string;
  has_regressions: boolean;
  report: unknown;
}

// ── Matrix types ──

export interface MatrixLane {
  model: string;
  backend?: string;
  strategy: AgentStrategy;
  label?: string;
  overrides: BenchConfigOverrides;
}

export interface MatrixRun {
  id: string;
  suite_id: string;
  suite_name: string;
  lane_ids: string[];
  lanes: MatrixLane[];
  status: 'running' | 'completed' | 'cancelled' | 'partial_failure';
  started_at: string;
  finished_at?: string;
  label?: string;
}

export interface ConfigPreset {
  id: string;
  label: string;
  strategy: AgentStrategy;
  temperature?: number;
  maxTokens?: number;
  description: string;
}

// Matrix SSE events

export interface MatrixRunStartedEvent {
  type: 'MatrixRunStarted';
  matrix_id: string;
  suite_id: string;
  lane_ids: string[];
  total_lanes: number;
}

export interface MatrixLaneCompletedEvent {
  type: 'MatrixLaneCompleted';
  matrix_id: string;
  lane_id: string;
  pass_rate: number;
  cost_usd: number;
}

export interface MatrixRunCompletedEvent {
  type: 'MatrixRunCompleted';
  matrix_id: string;
  summary: MatrixLaneSummary[];
}

export interface MatrixLaneSummary {
  lane_id: string;
  pass_rate: number;
  cost_usd: number;
}

// ── SWE-bench types ──

export interface SweDataset {
  id: string;
  name: string;
  total_instances: number;
  description?: string;
}

export interface SweInstance {
  instance_id: string;
  repo: string;
  resolved: boolean;
  duration_ms: number;
  error?: string;
}

export interface SweRun {
  id: string;
  dataset: string;
  status: 'running' | 'completed' | 'cancelled';
  agent_mode: string;
  total_instances: number;
  resolved: number;
  instances: SweInstance[];
  started_at: string;
  finished_at?: string;
}

export interface SweRunStartedEvent {
  type: 'SweRunStarted';
  run_id: string;
  dataset: string;
  total_instances: number;
}

export interface SweInstanceCompletedEvent {
  type: 'SweInstanceCompleted';
  run_id: string;
  instance_id: string;
  resolved: boolean;
  duration_ms: number;
}

export interface SweRunCompletedEvent {
  type: 'SweRunCompleted';
  run_id: string;
  resolved: number;
  total: number;
  pass_rate: number;
}

// ── Pareto types ──

export interface ParetoFrontierPoint {
  run_id: string;
  label?: string;
  model?: string;
  provider?: string;
  cost_usd: number;
  total_cost_usd?: number;
  pass_rate: number;
  duration_ms?: number;
}

export interface ParetoFrontierResponse {
  points: ParetoFrontierPoint[];
  frontier?: ParetoFrontierPoint[];
  generated_at?: string;
}

export interface BenchAgentOutputEvent {
  type: 'BenchAgentOutput';
  bench_id: string;
  task_id: string;
  agent_id: string;
  content: string;
  done: boolean;
  tool_calls?: unknown[];
  reasoning?: string;
}

export interface BenchGateVerdictEvent {
  type: 'BenchGateVerdict';
  bench_id: string;
  task_id: string;
  gate: string;
  passed: boolean;
  message?: string;
  duration_ms: number;
}

export interface BenchTokenVelocityEvent {
  type: 'BenchTokenVelocity';
  bench_id: string;
  task_id: string;
  tokens_per_second: number;
  tokens_in: number;
  tokens_out: number;
  duration_ms: number;
}

export type BenchSSEEvent =
  | BenchRunStartedEvent
  | BenchTaskStartedEvent
  | BenchTaskCompletedEvent
  | BenchProgressEvent
  | BenchRunCompletedEvent
  | BenchRegressionReportEvent
  | BenchLearningEvent
  | BenchAgentOutputEvent
  | BenchGateVerdictEvent
  | BenchTokenVelocityEvent
  | MatrixRunStartedEvent
  | MatrixLaneCompletedEvent
  | MatrixRunCompletedEvent;

export type BenchSSEEventWire =
  | Exclude<BenchSSEEvent, BenchTaskCompletedEvent>
  | { type: 'BenchTaskCompleted'; bench_id: string; task_id: string; result: BenchTaskResultWire };

export interface MatrixLaneRequestWire {
  model: string;
  backend?: string;
  strategy?: AgentStrategy;
  label?: string;
  overrides: BenchConfigOverrides;
}

export interface StartMatrixRequestWire {
  suite_id: string;
  lanes: MatrixLaneRequestWire[];
}

export interface StartMatrixResponseWire {
  matrix_id: string;
  lane_ids: string[];
}

const MAX_BENCH_STRING = 256_000;
const MAX_BENCH_ITEMS = 10_000;
const MAX_BENCH_JSON = 1_000_000;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function isString(value: unknown, max = MAX_BENCH_STRING): value is string {
  return typeof value === 'string' && value.length <= max;
}

function isOptionalString(value: unknown): value is string | undefined {
  return value === undefined || isString(value);
}

function isNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

function isUint(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}

function isBool(value: unknown): value is boolean {
  return typeof value === 'boolean';
}

function isArrayOf<T>(value: unknown, validate: (item: unknown) => item is T): value is T[] {
  return Array.isArray(value)
    && value.length <= MAX_BENCH_ITEMS
    && value.every((item) => validate(item));
}

function isBoundedJson(value: unknown): boolean {
  try {
    const encoded = JSON.stringify(value);
    return encoded !== undefined && encoded.length <= MAX_BENCH_JSON;
  } catch {
    return false;
  }
}

function isStrategy(value: unknown): value is AgentStrategy {
  return value === 'minimal' || value === 'context_enriched' || value === 'neuro_augmented'
    || value === 'full_cascade' || value === 'demo';
}

function isRunStatus(value: unknown): value is BenchRunWire['status'] {
  return value === 'running' || value === 'completed' || value === 'cancelled' || value === 'failed';
}

function isRunKind(value: unknown): value is BenchRunKind {
  return value === 'suite' || value === 'comparison' || value === 'regression';
}

function isTaskStatus(value: unknown): value is BenchTaskStatusWire {
  return value === 'pass' || value === 'fail' || value === 'skipped';
}

function isBenchGateVerdictWire(value: unknown): value is BenchGateVerdictWire {
  return isRecord(value) && isString(value.gate) && isBool(value.passed) && isString(value.detail);
}

function isBenchSummary(value: unknown): value is BenchRunSummary {
  if (!isRecord(value)) return false;
  return ['total_tasks', 'passed', 'failed', 'skipped', 'total_duration_ms', 'total_tokens']
    .every((field) => isUint(value[field]))
    && ['total_cost_usd', 'pass_rate', 'cost_per_success_usd', 'avg_duration_ms']
      .every((field) => isNumber(value[field]));
}

function isBenchTaskResultWire(value: unknown): value is BenchTaskResultWire {
  if (!isRecord(value)) return false;
  return isString(value.task_id)
    && isString(value.task_name)
    && isTaskStatus(value.status)
    && isUint(value.duration_ms)
    && isString(value.model)
    && isUint(value.tokens_in)
    && isUint(value.tokens_out)
    && isNumber(value.cost_usd)
    && isArrayOf(value.gate_verdicts, isBenchGateVerdictWire)
    && isUint(value.retries_used)
    && isOptionalString(value.output_preview)
    && isOptionalString(value.error);
}

function isBenchOverrides(value: unknown): value is BenchConfigOverrides {
  if (!isRecord(value)) return false;
  return isOptionalString(value.model)
    && isOptionalString(value.backend)
    && (value.max_tokens === undefined || isUint(value.max_tokens))
    && (value.temperature === undefined || isNumber(value.temperature))
    && (value.strategy === undefined || isStrategy(value.strategy));
}

function isBenchRunListEntry(value: unknown): value is BenchRunListEntry {
  if (!isRecord(value)) return false;
  return isString(value.id)
    && isString(value.suite_id)
    && isString(value.suite_name)
    && isRunStatus(value.status)
    && isUint(value.started_at)
    && (value.finished_at === undefined || isUint(value.finished_at))
    && isOptionalString(value.label)
    && isOptionalString(value.model)
    && (value.pass_rate === undefined || isNumber(value.pass_rate))
    && (value.total_cost_usd === undefined || isNumber(value.total_cost_usd));
}

export function parseBenchRunsListResponse(value: unknown): BenchRunsListResponse | null {
  if (!isRecord(value)
    || !isUint(value.total)
    || !isUint(value.offset)
    || !isUint(value.limit)
    || !isArrayOf(value.runs, isBenchRunListEntry)) {
    return null;
  }
  return value as unknown as BenchRunsListResponse;
}

function adaptTaskResult(result: BenchTaskResultWire): BenchTaskResult {
  return {
    ...result,
    gate_verdicts: result.gate_verdicts.map((gate) => ({
      gate: gate.gate,
      passed: gate.passed,
      message: gate.detail || undefined,
    })),
  };
}

/** Validate and adapt a full Rust `BenchRun` response for UI consumers. */
export function adaptBenchRun(value: unknown): BenchRun | null {
  if (!isRecord(value)
    || !isString(value.id)
    || !isString(value.suite_id)
    || !isString(value.suite_name)
    || !isRunKind(value.kind)
    || !isBenchOverrides(value.config)
    || !isOptionalString(value.label)
    || !isRunStatus(value.status)
    || !isString(value.started_at)
    || !isOptionalString(value.finished_at)
    || !isArrayOf(value.results, isBenchTaskResultWire)
    || (value.summary !== undefined && !isBenchSummary(value.summary))
    || !isUint(value.current_task_index)
    || !isUint(value.total_tasks)) {
    return null;
  }

  const wire = value as unknown as BenchRunWire;
  return {
    id: wire.id,
    suite_id: wire.suite_id,
    suite_name: wire.suite_name,
    kind: wire.kind,
    label: wire.label,
    status: wire.status,
    started_at: wire.started_at,
    finished_at: wire.finished_at,
    results: wire.results.map(adaptTaskResult),
    summary: wire.summary,
    config: {
      model: wire.config.model ?? 'unknown',
      backend: wire.config.backend,
      provider: wire.config.backend,
      temperature: wire.config.temperature,
      max_tokens: wire.config.max_tokens,
      strategy: wire.config.strategy ?? 'minimal',
      timeout_secs: 0,
      retries: 0,
    },
  };
}

/** Validate and adapt `{ runs: BenchRun[] }` from the compare endpoint. */
export function adaptBenchRunEnvelope(value: unknown): BenchRun[] | null {
  if (!isRecord(value) || !Array.isArray(value.runs) || value.runs.length > MAX_BENCH_ITEMS) {
    return null;
  }
  const runs = value.runs.map(adaptBenchRun);
  return runs.every((run) => run !== null) ? runs as BenchRun[] : null;
}

function isMatrixSummary(value: unknown): value is MatrixLaneSummary {
  return isRecord(value) && isString(value.lane_id)
    && isNumber(value.pass_rate) && isNumber(value.cost_usd);
}

/** Strictly validate and adapt one dedicated `/api/bench/events` frame. */
export function parseBenchSSEEvent(raw: unknown): BenchSSEEvent | null {
  if (!isRecord(raw) || !isBoundedJson(raw) || !isString(raw.type, 64)) return null;

  switch (raw.type) {
    case 'BenchRunStarted':
      return isString(raw.bench_id) && isString(raw.suite_id) && isUint(raw.total_tasks)
        ? raw as unknown as BenchRunStartedEvent : null;
    case 'BenchTaskStarted':
      return isString(raw.bench_id) && isString(raw.task_id) && isString(raw.task_name)
        && isUint(raw.task_index) && isUint(raw.total_tasks)
        ? raw as unknown as BenchTaskStartedEvent : null;
    case 'BenchTaskCompleted':
      if (!isString(raw.bench_id) || !isString(raw.task_id) || !isBenchTaskResultWire(raw.result)) {
        return null;
      }
      return {
        type: 'BenchTaskCompleted',
        bench_id: raw.bench_id,
        task_id: raw.task_id,
        result: adaptTaskResult(raw.result),
      };
    case 'BenchLearningEvent':
      return isString(raw.bench_id) && isString(raw.task_id)
        && ['playbooks_created', 'anti_patterns_created', 'total_playbooks', 'total_anti_patterns']
          .every((field) => isUint(raw[field]))
        ? raw as unknown as BenchLearningEvent : null;
    case 'BenchProgress':
      return isString(raw.bench_id) && isUint(raw.completed) && isUint(raw.total)
        && isNumber(raw.cost_so_far) ? raw as unknown as BenchProgressEvent : null;
    case 'BenchRunCompleted':
      return isString(raw.bench_id) && isBenchSummary(raw.summary)
        ? raw as unknown as BenchRunCompletedEvent : null;
    case 'BenchRegressionReport':
      return isString(raw.bench_id) && isBool(raw.has_regressions) && isBoundedJson(raw.report)
        ? raw as unknown as BenchRegressionReportEvent : null;
    case 'MatrixRunStarted':
      return isString(raw.matrix_id) && isString(raw.suite_id)
        && isArrayOf(raw.lane_ids, isString) && isUint(raw.total_lanes)
        ? raw as unknown as MatrixRunStartedEvent : null;
    case 'MatrixLaneCompleted':
      return isString(raw.matrix_id) && isString(raw.lane_id)
        && isNumber(raw.pass_rate) && isNumber(raw.cost_usd)
        ? raw as unknown as MatrixLaneCompletedEvent : null;
    case 'MatrixRunCompleted':
      return isString(raw.matrix_id) && isArrayOf(raw.summary, isMatrixSummary)
        ? raw as unknown as MatrixRunCompletedEvent : null;
    case 'BenchGateVerdict':
      return isString(raw.bench_id) && isString(raw.task_id) && isString(raw.gate)
        && isBool(raw.passed) && isOptionalString(raw.message) && isUint(raw.duration_ms)
        ? raw as unknown as BenchGateVerdictEvent : null;
    case 'BenchTokenVelocity':
      return isString(raw.bench_id) && isString(raw.task_id)
        && isNumber(raw.tokens_per_second) && isUint(raw.tokens_in)
        && isUint(raw.tokens_out) && isUint(raw.duration_ms)
        ? raw as unknown as BenchTokenVelocityEvent : null;
    case 'BenchAgentOutput':
      return isString(raw.bench_id) && isString(raw.task_id) && isString(raw.agent_id)
        && isString(raw.content) && isBool(raw.done)
        && (raw.tool_calls === undefined || isArrayOf(raw.tool_calls, (item): item is unknown => isBoundedJson(item)))
        && isOptionalString(raw.reasoning)
        ? raw as unknown as BenchAgentOutputEvent : null;
    default:
      return null;
  }
}
