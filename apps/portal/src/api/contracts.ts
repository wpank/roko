/**
 * Roko Portal — API Wire Contracts
 *
 * Exact TypeScript interfaces matching roko-serve HTTP response shapes.
 * Captured live from a running server on 2026-09-25.
 *
 * These are the RAW shapes from the wire, not the portal's internal types.
 * Use the transformation helpers in client.ts to convert these into the
 * portal's internal types (types.ts) where field names diverge.
 *
 * Naming convention: suffix "Response" for top-level response envelopes,
 * no suffix for nested record shapes.
 */

// ---------------------------------------------------------------------------
// Shared primitives
// ---------------------------------------------------------------------------

/**
 * Pricing record that appears in both model profiles and agent cost objects.
 * All fields may be null when pricing is unknown for the provider/model.
 */
export interface WirePricing {
  cache_read_per_m: number | null;
  cache_write_per_m: number | null;
  input_per_m: number | null;
  input_per_m_high: number | null;
  output_per_m: number | null;
  output_per_m_high: number | null;
  per_request: number | null;
}

// ---------------------------------------------------------------------------
// GET /api/health
// ---------------------------------------------------------------------------

export interface WireJwksInfo {
  age_secs: number | null;
  configured: boolean;
  fail_closed: boolean;
  fresh: boolean;
  key_count: number;
  stale: boolean;
}

export interface WireHealthProviderSummary {
  degraded: number;
  healthy: number;
  total: number;
  unhealthy: number;
}

export interface WireStateHubSnapshot {
  agents_active: number;
  cost_usd_total: number;
  episodes_total: number;
  errors_total: number;
  gates_failed: number;
  gates_passed: number;
  plans_active: number;
  tasks_active: number;
}

export interface WireStateHubInfo {
  cursor: string;
  events_retained: number;
  snapshot: WireStateHubSnapshot;
}

/**
 * GET /api/health
 *
 * Example:
 * {
 *   "active_agents": 5,
 *   "active_plans": 0,
 *   "active_runs": 0,
 *   "jwks": { "age_secs": null, "configured": true, "fail_closed": true, "fresh": false, "key_count": 0, "stale": false },
 *   "providers": { "degraded": 1, "healthy": 1, "total": 2, "unhealthy": 0 },
 *   "statehub": { "cursor": "0x1", "events_retained": 1, "snapshot": { ... } },
 *   "status": "unhealthy",
 *   "uptime_secs": 383,
 *   "version": "0.1.0"
 * }
 */
export interface WireHealthResponse {
  active_agents: number;
  active_plans: number;
  active_runs: number;
  jwks: WireJwksInfo;
  providers: WireHealthProviderSummary;
  statehub: WireStateHubInfo;
  /** "healthy" | "unhealthy" | "degraded" */
  status: string;
  uptime_secs: number;
  version: string;
}

// ---------------------------------------------------------------------------
// GET /api/plans
// ---------------------------------------------------------------------------

/**
 * Single element of the GET /api/plans array response.
 *
 * Example:
 * {
 *   "completed": false,
 *   "completed_task_count": 0,
 *   "id": "d4c25eab-42d0-435e-9e16-899212e0c9b8",
 *   "task_count": 1,
 *   "title": "Audit Test"
 * }
 */
export interface WirePlanSummary {
  completed: boolean;
  completed_task_count: number;
  id: string;
  task_count: number;
  title: string;
}

/** GET /api/plans returns a plain array. */
export type WirePlansResponse = WirePlanSummary[];

// ---------------------------------------------------------------------------
// GET /api/plans/{id}
// ---------------------------------------------------------------------------

export interface WireTaskDetail {
  completed: boolean;
  depends_on: string[];
  description: string;
  files: string[];
  id: string;
  model_hint: string | null;
  status: string;
  /** e.g. "focused" | "mechanical" | "integrative" | "architectural" */
  tier: string;
}

/**
 * GET /api/plans/{id}
 *
 * Example:
 * {
 *   "description": "Test plan",
 *   "id": "d4c25eab-42d0-435e-9e16-899212e0c9b8",
 *   "tasks": [ { "completed": false, "depends_on": [], "description": "Say hello", ... } ],
 *   "title": "Audit Test"
 * }
 */
export interface WirePlanDetailResponse {
  description: string;
  id: string;
  tasks: WireTaskDetail[];
  title: string;
}

// ---------------------------------------------------------------------------
// GET /api/plans/{id}/tasks
// ---------------------------------------------------------------------------

export interface WirePlanTasksTask {
  completed: boolean;
  depends_on: string[];
  description: string;
  files: string[];
  id: string;
  /** "pending" | "running" | "completed" | "failed" | "gating" etc. */
  status: string;
}

/**
 * GET /api/plans/{id}/tasks
 *
 * Example:
 * {
 *   "plan_id": "d4c25eab-42d0-435e-9e16-899212e0c9b8",
 *   "task_count": 1,
 *   "tasks": [ { "completed": false, "depends_on": [], ... } ]
 * }
 */
export interface WirePlanTasksResponse {
  plan_id: string;
  task_count: number;
  tasks: WirePlanTasksTask[];
}

// ---------------------------------------------------------------------------
// GET /api/managed-agents
// ---------------------------------------------------------------------------

export interface WireModelSupports {
  async: boolean;
  caching: boolean;
  citations: boolean;
  code_execution: boolean;
  embedding: boolean;
  grounding: boolean;
  mcp_tools: boolean;
  partial: boolean;
  thinking: boolean;
  tools: boolean;
  vision: boolean;
  web_search: boolean;
}

export interface WireModelProfile {
  context_window: number;
  key: string;
  max_output: number | null;
  pricing: WirePricing;
  provider: string;
  slug: string;
  supports: WireModelSupports;
  thinking_level: string | null;
  tool_format: string;
}

export interface WireAgentChatInfo {
  correlation: string;
  inline_timeout_ms: number;
  message_endpoint: string;
  stream_endpoint: string;
  streaming_supported: boolean;
}

export interface WireAgentCosts {
  burn_rate_usd_per_hour: number | null;
  cumulative_usd: number | null;
  pricing: WirePricing;
  token_burn_rate: number | null;
}

export interface WireAgentLearning {
  capabilities: string[];
  context_lift: number | null;
  episode_count: number | null;
  gate_pass_rate: number | null;
  insight_count: number | null;
  memory_freshness: number | null;
  playbook_size: number | null;
  skills: string[];
}

export interface WireAgentEndpoints {
  a2a: string | null;
  mcp: string | null;
  rest: string | null;
  websocket: string | null;
}

export interface WireAgentPerformance {
  active_tasks: number;
  completed_tasks: number;
  context_utilization: number | null;
  failed_tasks: number;
  frequency: number;
  gate_pass_rate: number | null;
  latency_ms: number | null;
  max_concurrent_jobs: number;
  past_jobs_completed: number;
  reputation: number;
  throughput: number | null;
  token_burn_rate: number | null;
}

export interface WireProviderHealth {
  consecutive_failures: number;
  error_rate: number;
  last_failure_at: string | null;
  last_success_at: string | null;
  /** "healthy" | "degraded" | "unhealthy" */
  state: string;
  total_attempts: number;
  total_successes: number;
}

/**
 * Single element of GET /api/managed-agents array.
 *
 * Example (abbreviated):
 * {
 *   "agent_id": "env-override",
 *   "capabilities": [],
 *   "card_uri": null,
 *   "chat": { "correlation": "run_id", "inline_timeout_ms": 30000, ... },
 *   "costs": { "burn_rate_usd_per_hour": null, ... },
 *   "current_task": null,
 *   "domain_tags": [],
 *   "endpoints": { "a2a": null, "mcp": null, "rest": "http://127.0.0.1:53612", "websocket": null },
 *   "heartbeat": null,
 *   "id": "env-override",
 *   "label": "env-override",
 *   "last_seen_at": 1777245495,
 *   "learning": { "capabilities": [], "context_lift": null, ... },
 *   "max_concurrent_jobs": 0,
 *   "model": "claude-sonnet",
 *   "model_profile": { "context_window": 200000, "key": "claude-sonnet", ... },
 *   "model_source": "default",
 *   "owner": "",
 *   "past_jobs_completed": 0,
 *   "performance": { "active_tasks": 0, ... },
 *   "process_id": 11827,
 *   "provider": "claude_cli",
 *   "provider_health": { "consecutive_failures": 0, ... },
 *   "registered_at": 1790320651,
 *   "reputation": 0,
 *   "role": "messaging",
 *   "skills": [],
 *   "status": "registered",
 *   "tier": null
 * }
 */
export interface WireManagedAgent {
  agent_id: string;
  capabilities: string[];
  card_uri: string | null;
  chat: WireAgentChatInfo;
  costs: WireAgentCosts;
  current_task: string | null;
  domain_tags: string[];
  endpoints: WireAgentEndpoints;
  heartbeat: number | null;
  id: string;
  label: string;
  last_seen_at: number | null;
  learning: WireAgentLearning;
  max_concurrent_jobs: number;
  model: string;
  model_profile: WireModelProfile;
  /** "default" | "agent" | "config" */
  model_source: string;
  owner: string;
  past_jobs_completed: number;
  performance: WireAgentPerformance;
  process_id: number | null;
  provider: string;
  provider_health: WireProviderHealth | null;
  registered_at: number;
  reputation: number;
  role: string;
  skills: string[];
  /** "registered" | "active" | "idle" | "stopped" */
  status: string;
  tier: string | null;
}

/** GET /api/managed-agents returns a plain array. */
export type WireManagedAgentsResponse = WireManagedAgent[];

// ---------------------------------------------------------------------------
// GET /api/providers
// ---------------------------------------------------------------------------

/**
 * Single provider entry from GET /api/providers.
 *
 * Note: `health` and `base_url` are optional — CLI-backed providers
 * (claude_cli, codex_cli) omit `base_url`, and providers with no attempts
 * may omit `health`.
 *
 * Example:
 * {
 *   "id": "cerebras",
 *   "kind": "openai_compat",
 *   "base_url": "https://api.cerebras.ai/v1",
 *   "has_api_key": true,
 *   "health": { "state": "healthy", "consecutive_failures": 1, "total_attempts": 137, "total_successes": 129 },
 *   "model_count": 3
 * }
 */
export interface WireProvider {
  id: string;
  /** "anthropic_api" | "openai_compat" | "claude_cli" | "codex_cli" | "gemini_api" | "perplexity_api" */
  kind: string;
  base_url?: string;
  has_api_key: boolean;
  health?: {
    state: string;
    consecutive_failures: number;
    total_attempts: number;
    total_successes: number;
  };
  model_count: number;
}

/**
 * GET /api/providers
 *
 * Example:
 * { "providers": [ { "id": "anthropic", "kind": "anthropic_api", ... }, ... ] }
 */
export interface WireProvidersResponse {
  providers: WireProvider[];
}

// ---------------------------------------------------------------------------
// GET /api/models
// ---------------------------------------------------------------------------

/**
 * Single model entry from GET /api/models.
 *
 * Example:
 * {
 *   "key": "cerebras-gemma",
 *   "slug": "gemma-4-31b",
 *   "provider": "cerebras",
 *   "context_window": 128000,
 *   "supports_tools": true,
 *   "supports_thinking": false,
 *   "supports_vision": false,
 *   "cost_input_per_m": 0.1,
 *   "cost_output_per_m": 0.1
 * }
 */
export interface WireModel {
  key: string;
  slug: string;
  provider: string;
  context_window: number;
  supports_tools: boolean;
  supports_thinking: boolean;
  supports_vision: boolean;
  cost_input_per_m?: number;
  cost_output_per_m?: number;
}

/**
 * GET /api/models
 *
 * Example:
 * { "models": [ { "key": "cerebras-gemma", "slug": "gemma-4-31b", ... }, ... ] }
 */
export interface WireModelsResponse {
  models: WireModel[];
}

// ---------------------------------------------------------------------------
// GET /api/config  (read-only dump — only partial shape documented here)
// ---------------------------------------------------------------------------

/**
 * GET /api/config returns the entire roko.toml config as a deep object.
 * Only the sections relevant to the portal are typed here; remaining fields
 * are captured as unknown to allow forward compatibility.
 *
 * Notable: sensitive fields (api_key, wallet_key, etc.) are replaced with "***".
 */
export interface WireConfigResponse {
  config_version: number;
  schema_version: number;
  default_model: string;
  budget: {
    max_plan_usd: number;
    max_task_usd: number;
    max_turn_usd: number;
    max_daily_usd: number;
    max_agent_lifetime_usd: number;
    max_task_retry_usd: number;
    prompt_token_budget: number;
    tier_multipliers: Record<string, number>;
  };
  routing: {
    algorithm: string;
    mode: string;
    fast_task_model: string;
    standard_task_model: string;
    complex_task_model: string;
    weights: { cost: number; latency: number; quality: number };
  };
  runner: {
    sandbox_level: string;
    dispatch_max_retries: number;
    plan_timeout_secs: number;
    warm_pool_size: number;
    dangerously_skip_permissions: boolean;
  };
  server: {
    bind: string;
    port: number;
    rate_limit_per_sec: number;
  };
  serve: {
    port: number | null;
    auto_orchestrate: boolean;
    auth: {
      enabled: boolean;
      enforcement_mode: string;
    };
  };
  conductor: {
    max_agents: number;
    max_parallel_plans: number;
    parallel_enabled: boolean;
    phase_timeout_secs: number;
  };
  gates: {
    mode: string;
    max_iterations: number;
    skip_tests: boolean;
  };
  learning: {
    replan_on_gate_failure: boolean;
    replan_gate_attempts: number;
    auto_playbook_refresh: boolean;
    dream_on_completion: boolean;
  };
  [key: string]: unknown;
}

// ---------------------------------------------------------------------------
// GET /api/episodes
// ---------------------------------------------------------------------------

export interface WireEpisodeUsage {
  cache_read_tokens: number;
  cache_write_tokens: number;
  cost_usd: number;
  cost_usd_without_cache: number;
  input_tokens: number;
  output_tokens: number;
  wall_ms: number;
}

/**
 * Single element of GET /api/episodes array.
 *
 * Example:
 * {
 *   "agent_id": "T04",
 *   "agent_template": "",
 *   "completed_at": "2026-09-23T16:50:25.030401+00:00",
 *   "duration_secs": 30.531,
 *   "episode_id": "ep_af984cc7541849fd",
 *   "failure_reason": null,
 *   "gate_verdicts": [],
 *   "headline": false,
 *   "id": "ep_af984cc7541849fd",
 *   "kind": "",
 *   "model": "claude-sonnet-4-6",
 *   "plan_id": "example-hello-world",
 *   "prompt_composition_available": false,
 *   "provider": "claude_cli",
 *   "reflection_present": false,
 *   "retry_count": 0,
 *   "role": "",
 *   "source": "episode_log",
 *   "started_at": "2026-09-23T16:50:25.030401+00:00",
 *   "status": "passed",
 *   "success": true,
 *   "task_id": "T04",
 *   "timestamp_ms": 1790182225030,
 *   "tokens_used": 1265,
 *   "trigger_kind": "",
 *   "turns": 0,
 *   "usage": { "cache_read_tokens": 0, "cost_usd": 0.35968, ... }
 * }
 */
export interface WireEpisode {
  agent_id: string;
  agent_template: string;
  completed_at: string;
  duration_secs: number;
  episode_id: string;
  failure_reason: string | null;
  gate_verdicts: string[];
  headline: boolean;
  id: string;
  kind: string;
  model: string;
  plan_id: string;
  prompt_composition_available: boolean;
  provider: string;
  reflection_present: boolean;
  retry_count: number;
  role: string;
  /** "episode_log" | "statehub" */
  source: string;
  started_at: string;
  /** "passed" | "failed" | "running" */
  status: string;
  success: boolean;
  task_id: string;
  timestamp_ms: number;
  tokens_used: number;
  trigger_kind: string;
  turns: number;
  usage: WireEpisodeUsage;
}

/** GET /api/episodes returns a plain array (may be empty). */
export type WireEpisodesResponse = WireEpisode[];

// ---------------------------------------------------------------------------
// GET /api/doctor
// ---------------------------------------------------------------------------

export interface WireDoctorCheck {
  id: string;
  message: string;
  /** "ok" | "warn" | "fail" | "skip" */
  status: string;
  /** Only present when status is "ok" for disk/path checks. */
  path?: string;
  detail?: string;
  /** Suggested fix command, only present on warn/fail. */
  fix?: string;
}

export interface WireDoctorSummary {
  fail: number;
  ok: number;
  skipped: number;
  total: number;
  warn: number;
}

/**
 * GET /api/doctor
 *
 * Example:
 * {
 *   "checks": [ { "id": "workdir", "message": "workspace directory exists", "status": "ok", "path": "/..." }, ... ],
 *   "healthy": true,
 *   "summary": { "fail": 0, "ok": 16, "skipped": 0, "total": 22, "warn": 6 },
 *   "workdir": "/Users/will/dev/nunchi/roko/roko"
 * }
 */
export interface WireDoctorResponse {
  checks: WireDoctorCheck[];
  healthy: boolean;
  summary: WireDoctorSummary;
  workdir: string;
}

// ---------------------------------------------------------------------------
// GET /api/learn/cascade-router
// ---------------------------------------------------------------------------

export interface WireCascadeConfidenceStats {
  gemini_code_execution_failures: number;
  gemini_code_execution_successes: number;
  gemini_context_window_gt_200k_requests: number;
  gemini_context_window_le_200k_requests: number;
  gemini_requests: number;
  perplexity_requests: number;
  successes: number;
  total_citations: number;
  total_cost_usd: number;
  total_gemini_cached_tokens: number;
  total_gemini_grounding_queries: number;
  total_gemini_thinking_tokens: number;
  total_search_latency_ms: number;
  trials: number;
}

export interface WireCascadeDataQuality {
  entry_count: number;
  has_real_data: boolean;
  null_cost_count: number;
}

export interface WireCascadeEvidenceFile {
  path: string;
  state?: string;
  records?: number;
  error?: string | null;
  format?: string;
  generation?: string | null;
}

export interface WireCascadeEvidence {
  runtime_feedback: {
    cascade_router: WireCascadeEvidenceFile;
    costs: WireCascadeEvidenceFile;
    efficiency: WireCascadeEvidenceFile;
    episode_paths: string[];
    episodes: number;
    executor_state: WireCascadeEvidenceFile & { error: string | null; format: string; generation: string | null };
    gate_thresholds: WireCascadeEvidenceFile;
    knowledge: WireCascadeEvidenceFile;
    provider_model_outcomes: WireCascadeEvidenceFile;
    runner_events: WireCascadeEvidenceFile;
  };
  state_hub: {
    cursor: string;
    events_retained: number;
    snapshot_recovered_from_disk: boolean;
  };
}

/**
 * GET /api/learn/cascade-router
 *
 * Large response containing LinUCB bandit state, confidence stats per model,
 * data quality metadata, and evidence paths.
 */
export interface WireCascadeRouterResponse {
  confidence_stats: Record<string, WireCascadeConfidenceStats>;
  data_quality: WireCascadeDataQuality;
  evidence: WireCascadeEvidence;
  linucb_state: {
    /** Flat row-major NxN matrices serialised as arrays of numbers. */
    a_matrices: number[][];
    b_vectors: number[][];
    models: string[];
  };
}

// ---------------------------------------------------------------------------
// GET /api/learn/efficiency
// ---------------------------------------------------------------------------

export interface WireCostTrendPoint {
  timestamp: string;
  cost_usd: number;
  cumulative_cost_usd: number;
}

/**
 * GET /api/learn/efficiency
 *
 * Example:
 * {
 *   "total_cost": 149.45,
 *   "cost_per_task": 1.245,
 *   "tokens_per_task": 239592.25,
 *   "avg_task_duration": 309811.95,
 *   "data_quality": { "has_real_data": true, "entry_count": 775, "null_cost_count": 350 },
 *   "cost_trend": [ { "timestamp": "2026-05-06T23:13:30.553104+00:00", "cost_usd": 7.08, "cumulative_cost_usd": 7.08 }, ... ]
 * }
 */
export interface WireEfficiencyResponse {
  total_cost: number;
  cost_per_task: number;
  tokens_per_task: number;
  avg_task_duration: number;
  data_quality: {
    has_real_data: boolean;
    entry_count: number;
    null_cost_count: number;
  };
  cost_trend: WireCostTrendPoint[];
}

// ---------------------------------------------------------------------------
// GET /api/learn/playbooks
// ---------------------------------------------------------------------------

export interface WirePlaybookStep {
  action_kind: string;
  description: string;
  expected_signals: string[];
  index: number;
}

/**
 * Single playbook entry from GET /api/learn/playbooks.
 *
 * Example:
 * {
 *   "created_at_ms": 1778085890866,
 *   "failure_count": 31,
 *   "goal": "For task type code, this approach works: ...",
 *   "id": "dream-playbook-f566550bf5f4f2ea",
 *   "last_used_ms": 1788518379034,
 *   "name": "Dream playbook sess_... / code / gpt-5.4-mini",
 *   "steps": [ { "action_kind": "align_context", ... }, ... ],
 *   "success_count": 116,
 *   "success_rate": 0.789,
 *   "when_pattern": null
 * }
 */
export interface WirePlaybook {
  created_at_ms: number;
  failure_count: number;
  goal: string;
  id: string;
  last_used_ms: number | null;
  name: string;
  steps: WirePlaybookStep[];
  success_count: number;
  success_rate: number;
  when_pattern: string | null;
}

/**
 * GET /api/learn/playbooks
 *
 * Example:
 * { "playbooks": [ { ... }, ... ] }
 */
export interface WirePlaybooksResponse {
  playbooks: WirePlaybook[];
}

// ---------------------------------------------------------------------------
// GET /api/learn/experiments
// ---------------------------------------------------------------------------

export interface WireExperimentVariant {
  id: string;
  name: string;
  section_name: string;
  active: boolean;
  trials: number;
  successes: number;
  success_rate: number;
}

export interface WireExperimentSignificance {
  best_variant_id: string;
  runner_up_variant_id: string;
  best_success_rate: number;
  runner_up_success_rate: number;
  success_rate_gap: number;
  z_score: number;
  p_value: number;
  alpha: number;
  meets_effect_size_threshold: boolean;
  statistically_significant: boolean;
  note: string | null;
}

/**
 * Single active experiment.
 *
 * Example:
 * {
 *   "experiment_id": "retrieval-strategy",
 *   "section_name": "retrieval-strategy",
 *   "status": "Running",
 *   "winner_id": null,
 *   "min_trials_per_variant": 5,
 *   "min_effect_size": 0.1,
 *   "total_trials": 80,
 *   "variants": [ { "id": "hybrid", "name": "hybrid", "trials": 31, "successes": 28, ... }, ... ],
 *   "significance": { "best_variant_id": "hybrid", "p_value": 0.966, "statistically_significant": false, ... }
 * }
 */
export interface WireExperiment {
  experiment_id: string;
  section_name: string;
  /** "Running" | "Concluded" */
  status: string;
  winner_id: string | null;
  min_trials_per_variant: number;
  min_effect_size: number;
  total_trials: number;
  variants: WireExperimentVariant[];
  significance: WireExperimentSignificance;
}

/**
 * GET /api/learn/experiments
 *
 * Example:
 * {
 *   "source": "/.../.roko/learn/experiments.json",
 *   "running_experiments": 1,
 *   "concluded_experiments": 0,
 *   "active_experiments": [ { ... } ]
 * }
 */
export interface WireExperimentsResponse {
  source: string;
  running_experiments: number;
  concluded_experiments: number;
  active_experiments: WireExperiment[];
}

// ---------------------------------------------------------------------------
// GET /api/learn/gate-thresholds  (reuses cascade-router evidence envelope)
// ---------------------------------------------------------------------------

/**
 * GET /api/learn/gate-thresholds returns the same evidence envelope as
 * /api/learn/cascade-router but without linucb_state or confidence_stats.
 * The gate-threshold–specific data lives in the evidence.runtime_feedback
 * object's gate_thresholds field.
 */
export type WireGateThresholdsResponse = Pick<
  WireCascadeRouterResponse,
  "data_quality" | "evidence"
>;

// ---------------------------------------------------------------------------
// GET /api/neuro/query  (POST with body)
// ---------------------------------------------------------------------------

export interface WireNeuroResult {
  content: string;
  created_at: string;
  id: string;
  /** "Heuristic" | "Insight" | "Pattern" | "Fact" */
  kind: string;
  relevance: number;
  /** "Consolidated" | "Working" | "Transient" */
  tier: string;
}

/**
 * POST /api/neuro/query
 * Body: { "query": string, "limit": number }
 *
 * Example:
 * {
 *   "results": [ { "content": "...", "created_at": "...", "id": "dream_...", "kind": "Heuristic", "relevance": 0.69, "tier": "Consolidated" }, ... ],
 *   "total": 3
 * }
 */
export interface WireNeuroQueryResponse {
  results: WireNeuroResult[];
  total: number;
}

// ---------------------------------------------------------------------------
// GET /api/retrieval/stats
// ---------------------------------------------------------------------------

export interface WireRetrievalStrategyStats {
  attempts: number;
  avg_latency_ms: number;
  passed: number;
  precision_pct: number;
  strategy: string;
}

/**
 * GET /api/retrieval/stats
 *
 * Example:
 * {
 *   "avg_latency_ms": 1.075,
 *   "miss_rate_pct": 13.75,
 *   "passed": 69,
 *   "precision_pct": 86.25,
 *   "strategies": [ { "attempts": 30, "avg_latency_ms": 1.1, "passed": 27, ... }, ... ],
 *   "total_settled": 80
 * }
 */
export interface WireRetrievalStatsResponse {
  avg_latency_ms: number;
  miss_rate_pct: number;
  passed: number;
  precision_pct: number;
  strategies: WireRetrievalStrategyStats[];
  total_settled: number;
}

// ---------------------------------------------------------------------------
// GET /api/dream/journal
// ---------------------------------------------------------------------------

export interface WireDreamPhaseReport {
  clusters_formed: number;
  duration_secs: number;
  episodes_processed: number;
  knowledge_entries_written: number;
  name: string;
  playbooks_created: number;
  /** "completed" | "running" | "skipped" */
  status: string;
  trend: unknown[];
}

/**
 * GET /api/dream/journal
 *
 * Example:
 * {
 *   "cycle_count": 9,
 *   "last_cycle": "",
 *   "phases": [
 *     { "clusters_formed": 0, "duration_secs": 0, "episodes_processed": 0, "knowledge_entries_written": 0, "name": "Hypnagogia", "playbooks_created": 0, "status": "completed", "trend": [] },
 *     ...
 *   ]
 * }
 */
export interface WireDreamJournalResponse {
  cycle_count: number;
  /** ISO timestamp of the last cycle, or empty string if no cycles yet. */
  last_cycle: string;
  phases: WireDreamPhaseReport[];
}

// ---------------------------------------------------------------------------
// GET /api/feeds
// ---------------------------------------------------------------------------

/**
 * Single feed entry from GET /api/feeds.
 *
 * Example:
 * {
 *   "id": "feed-1",
 *   "cell_id": "",
 *   "name": "signals",
 *   "kind": "raw",
 *   "access": "public",
 *   "agent_id": "system",
 *   "description": "Raw signal log (.roko/signals.jsonl)",
 *   "created_at": "2026-09-25T14:40:25.816168Z",
 *   "running": false
 * }
 */
export interface WireFeed {
  id: string;
  cell_id: string;
  name: string;
  /** "raw" | "derived" | "composite" */
  kind: string;
  /** "public" | "private" */
  access: string;
  agent_id: string;
  description: string;
  created_at: string;
  running: boolean;
}

/**
 * GET /api/feeds
 *
 * Example:
 * { "feeds": [ { "id": "feed-1", ... }, ... ], "total": 4 }
 */
export interface WireFeedsResponse {
  feeds: WireFeed[];
  total: number;
}

// ---------------------------------------------------------------------------
// GET /api/triggers
// ---------------------------------------------------------------------------

/**
 * GET /api/triggers
 *
 * Example:
 * { "triggers": [], "total": 0 }
 */
export interface WireTriggersResponse {
  triggers: unknown[];
  total: number;
}

// ---------------------------------------------------------------------------
// GET /api/signals  (returns empty array when no signals in window)
// ---------------------------------------------------------------------------

/** GET /api/signals returns a plain array (may be empty). */
export type WireSignalsResponse = unknown[];

// ---------------------------------------------------------------------------
// GET /api/statehub/events
// ---------------------------------------------------------------------------

export interface WireStateHubEvent {
  cursor: string;
  event: Record<string, unknown>;
  seq: number;
  ts_millis: number;
}

/**
 * GET /api/statehub/events?since=<seq>&limit=<n>
 *
 * Example:
 * {
 *   "after_seq": 0,
 *   "cursor": "0x1",
 *   "events": [
 *     { "cursor": "0x0", "event": { "revision": 1, "source": "missing", "type": "snapshot_rebased" }, "seq": 0, "ts_millis": 1790347225805 }
 *   ],
 *   "limit": 5
 * }
 */
export interface WireStateHubEventsResponse {
  after_seq: number;
  cursor: string;
  events: WireStateHubEvent[];
  limit: number;
}

// ---------------------------------------------------------------------------
// SSE  GET /api/events
// ---------------------------------------------------------------------------

/**
 * The /api/events SSE stream emits one line on initial connection:
 *
 *   data: {"type":"snapshot_rebased","revision":1,"source":"missing"}
 *   id: 0
 *
 * Subsequent events are emitted as the server produces state changes.
 * The `type` field discriminates the event kind.
 */
export interface WireSseSnapshotRebased {
  type: "snapshot_rebased";
  revision: number;
  /** "missing" when no snapshot has been written yet */
  source: string;
}

/** Union of all known SSE event shapes (extend as new event types are discovered). */
export type WireSseEvent = WireSseSnapshotRebased | Record<string, unknown>;

// ---------------------------------------------------------------------------
// Error shape (returned when a route does not match)
// ---------------------------------------------------------------------------

/**
 * Standard error response for unknown routes or server errors.
 *
 * Example:
 * { "error": "not_found", "message": "No route matches /api/learn" }
 */
export interface WireErrorResponse {
  error: string;
  message: string;
}
