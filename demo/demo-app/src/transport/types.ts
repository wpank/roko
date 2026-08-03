// -- Execution sub-events (nested inside ServerEvent::Execution) ----
// Field names match the server's serde(rename_all = "snake_case") contract.
export type ExecutionEvent =
  | { type: 'plan_started' }
  | { type: 'task_started'; task_id: string; title: string; phase: string }
  | { type: 'task_phase_changed'; task_id: string; old_phase: string; new_phase: string }
  | { type: 'gate_result'; task_id: string; gate: string; passed: boolean; message: string }
  | { type: 'task_completed'; task_id: string; outcome: string }
  | { type: 'plan_completed'; outcome: string; stats: Record<string, unknown> }
  | { type: 'replan_triggered'; task_id: string; strategy: string }
  | { type: 'watcher_alert'; watcher: string; message: string };

// -- Top-level ServerEvent discriminated union ----------------------
// 50 variants. Field names use the server's snake_case wire shape verbatim.
// PascalCase type tags (Bench*, Matrix*, Swe*) use serde(rename) on the server
// and are preserved as-is.
export type ServerEvent =
  // Plan execution
  | { type: 'plan_started'; plan_id: string }
  | { type: 'plan_completed'; plan_id: string; success: boolean }
  | { type: 'phase_transition'; plan_id: string; from: string; to: string }
  | { type: 'execution'; plan_id: string; event: ExecutionEvent }
  | { type: 'episode'; plan_id: string; task_id: string; passed: boolean }
  | { type: 'efficiency_event'; plan_id: string; task_id: string;
       metric: string; value: number }
  // Task lifecycle (dashboard-facing)
  | { type: 'task_started'; plan_id: string; task_id: string;
       description: string }
  | { type: 'task_completed'; plan_id: string; task_id: string;
       success: boolean }
  | { type: 'task_failed'; plan_id: string; task_id: string; error: string }
  // Agent lifecycle
  | { type: 'agent_spawned'; agent_id: string; role: string; model: string }
  | { type: 'agent_output'; agent_id: string; run_id?: string;
       content: string; done: boolean;
       metadata?: Record<string, unknown> }
  | { type: 'agent_trace'; agent_id: string; run_id?: string;
       content: string; tool_calls?: unknown[];
       reasoning?: string; usage?: Record<string, unknown>; done?: boolean }
  | { type: 'agent_started'; agent_id: string }
  | { type: 'agent_stopped'; agent_id: string; reason: string }
  // Gate results
  | { type: 'gate_result'; plan_id: string; task_id: string;
       gate: string; rung: number; passed: boolean }
  // Inference tracking
  | { type: 'inference_started'; request_id: string; model: string;
       agent_id: string; auto_routed: boolean }
  | { type: 'inference_completed'; request_id: string; model: string;
       agent_id: string; input_tokens: number; output_tokens: number;
       cost_usd: number; duration_ms: number }
  | { type: 'inference_failed'; request_id: string; model: string;
       agent_id: string; error: string }
  // One-shot runs
  | { type: 'run_started'; run_id: string; prompt_preview: string }
  | { type: 'run_completed'; run_id: string; success: boolean }
  // Knowledge
  | { type: 'knowledge_ingested'; run_id: string; entry_id: string; topic: string;
       source_agent: string }
  | { type: 'knowledge_consumed'; run_id: string; entry_id: string; topic: string;
       consuming_agent: string }
  // Generic operations
  | { type: 'operation_started'; op_id: string; kind: string }
  | { type: 'operation_completed'; op_id: string; kind: string;
       success: boolean }
  // Somatic / affect
  | { type: 'somatic_marker_fired'; plan_id: string; task_id: string;
       valence: number; intensity: number; source_episodes: string[];
       strategy_param: string }
  // Deployment lifecycle
  | { type: 'deployment_created'; id: string; name: string }
  | { type: 'deployment_ready'; id: string; url: string }
  | { type: 'deployment_failed'; id: string; reason: string }
  | { type: 'deployment_torn_down'; id: string }
  // Job marketplace
  | { type: 'job_created'; job: Record<string, unknown> }
  | { type: 'job_posted_to_candidate'; job_id: string; agent_id: string;
       reward: string }
  | { type: 'job_updated'; job: Record<string, unknown> }
  | { type: 'job_transitioned'; job_id: string; from: string; to: string;
       assigned_to?: string }
  | { type: 'job_execution_started'; job_id: string; job_type: string;
       agent_id: string }
  | { type: 'job_progress'; job_id: string; percent: number;
       message: string }
  | { type: 'job_agent_output'; job_id: string; agent_id: string;
       content: string; done: boolean }
  | { type: 'job_submitted'; job_id: string; agent_id: string }
  | { type: 'job_evaluated'; job_id: string; accepted: boolean;
       feedback: string }
  | { type: 'job_state_changed'; job_id: string; from: string;
       to: string }
  // Worker
  | { type: 'worker_task_started'; deployment_id: string;
       task_id: string }
  | { type: 'worker_task_completed'; deployment_id: string;
       task_id: string; success: boolean }
  // Chain triage
  | { type: 'chain_triage_result'; job_id: string; event_count: number;
       anomaly_count: number; summary: string }
  // Heartbeat
  | { type: 'heartbeat_received'; sender_id: string;
       active_tasks: number; active_agents: number }
  | { type: 'heartbeat'; agent_id: string; block_number?: number }
  // Config / strategy reload
  | { type: 'config_reloaded'; applied_sections: string[];
       restart_required: string[] }
  | { type: 'strategy_reloaded'; goals_count: number;
       tactics_count: number }
  // Vision loop
  | { type: 'vision_loop_iteration'; run_id: string; iteration: number;
       score: number; notes: string }
  | { type: 'vision_loop_completed'; run_id: string; iterations: number;
       best_score: number; stop_reason: string }
  // Webhook
  | { type: 'webhook_received'; signal: Record<string, unknown> }
  // Bench (PascalCase type tags -- server uses #[serde(rename)])
  | { type: 'BenchRunStarted'; bench_id: string; suite_id: string;
       total_tasks: number }
  | { type: 'BenchTaskStarted'; bench_id: string; task_id: string;
       task_name: string; task_index: number; total_tasks: number }
  | { type: 'BenchTaskCompleted'; bench_id: string; task_id: string;
       result: Record<string, unknown> }
  | { type: 'BenchLearningEvent'; bench_id: string; task_id: string;
       playbooks_created: number; anti_patterns_created: number;
       total_playbooks: number; total_anti_patterns: number }
  | { type: 'BenchProgress'; bench_id: string; completed: number;
       total: number; cost_so_far: number }
  | { type: 'BenchRunCompleted'; bench_id: string;
       summary: Record<string, unknown> }
  | { type: 'BenchGateVerdict'; bench_id: string; task_id: string;
       gate: string; passed: boolean; message?: string;
       duration_ms: number }
  | { type: 'BenchTokenVelocity'; bench_id: string; task_id: string;
       tokens_per_second: number; tokens_in: number; tokens_out: number;
       duration_ms: number }
  | { type: 'BenchAgentOutput'; bench_id: string; task_id: string;
       agent_id: string; content: string; done: boolean;
       tool_calls?: unknown[]; reasoning?: string }
  // Matrix bench
  | { type: 'MatrixRunStarted'; matrix_id: string; suite_id: string;
       lane_ids: string[]; total_lanes: number }
  | { type: 'MatrixLaneCompleted'; matrix_id: string; lane_id: string;
       pass_rate: number; cost_usd: number }
  | { type: 'MatrixRunCompleted'; matrix_id: string;
       summary: Record<string, unknown>[] }
  // SWE-bench
  | { type: 'SweRunStarted'; run_id: string; dataset: string;
       total_instances: number }
  | { type: 'SweInstanceCompleted'; run_id: string; instance_id: string;
       resolved: boolean; duration_ms: number }
  | { type: 'SweRunCompleted'; run_id: string; resolved: number;
       total: number; pass_rate: number }
  // ISFR (interest-free secured rate)
  | { type: 'isfr_rate_computed'; composite_bps: number; lending_bps: number;
       structured_bps: number; funding_bps: number; staking_bps: number;
       confidence_bps: number; source_count: number; timestamp_ms: number }
  | { type: 'isfr_source_health_changed'; source_id: string;
       health: 'live' | 'stale' | 'offline'; last_rate_bps: number | null }
  | { type: 'isfr_keeper_state_changed'; running: boolean }
  // Chain (block watcher)
  | { type: 'chain_block'; number: number; hash: string; parent_hash: string;
       timestamp: number; gas_used: number; gas_limit: number; tx_count: number;
       base_fee_per_gas: number | null }
  | { type: 'chain_tx'; block_number: number; tx_hash: string; from: string;
       to: string | null; value_wei: string; gas_used: number;
       method_sig: string | null; success: boolean }
  | { type: 'chain_contract_event'; block_number: number; tx_hash: string;
       log_index: number; contract: string; event_name: string;
       decoded: Record<string, unknown> }
  // Feed agents
  | { type: 'feed_tick'; agent_id: string; feed_id: string; topic: string;
       payload: unknown; timestamp_ms: number }
  | { type: 'feed_agent_online'; agent_id: string; name: string; feed_count: number }
  | { type: 'feed_agent_offline'; agent_id: string }
  // System
  | { type: 'server_shutdown' }
  | { type: 'error'; message: string };

/**
 * Parse a raw JSON object (from SSE `e.data`) into a typed ServerEvent.
 * The wire shape is already snake_case — no casing conversion is performed.
 * Returns null if the `type` field is missing.
 */
export function parseServerEvent(
  raw: Record<string, unknown>,
): ServerEvent | null {
  const type = typeof raw.type === 'string'
    ? raw.type
    : typeof raw.kind === 'string'
      ? raw.kind
      : null;
  if (!type) return null;

  // Flatten nested `data` payload (some SSE frames wrap fields under `data`).
  const nested = raw.data !== null && typeof raw.data === 'object' && !Array.isArray(raw.data)
    ? raw.data as Record<string, unknown>
    : {};
  const merged = { ...nested, ...raw, type };
  return merged as unknown as ServerEvent;
}
