/**
 * Roko Portal — Wire Contracts
 *
 * TypeScript types that exactly mirror what roko-serve sends over the wire.
 * Field names are snake_case, matching serde's default serialisation.
 *
 * Rust authority:
 *   crates/roko-core/src/dashboard_snapshot.rs   — DashboardEvent, DashboardSnapshot, *State, *Stats
 *   crates/roko-serve/src/plan_types.rs           — PlanSummaryDto, PlanTaskDto, PlanTasksDto
 *   crates/roko-serve/src/projection_contract.rs  — state_frame (WireStateHubSnapshotResponse)
 *   crates/roko-serve/src/routes/sse.rs           — gap payload (WireGapPayload)
 *   crates/roko-serve/src/routes/plans/merge.rs   — task reviews and diffs (WireReviews, WireTaskDiff)
 *
 * Types only — no runtime code except the TASK_OUTCOME_* constants.
 */

// ---------------------------------------------------------------------------
// Task outcome constant (mirrors the Rust string literal used as outcome)
// ---------------------------------------------------------------------------

/** Outcome string emitted when a task completed despite gate warnings. */
export const TASK_OUTCOME_ACCEPTED_WITH_FAILURES = 'accepted_with_failures' as const;

/** Outcome string emitted when a task completed without a verify step judging it. */
export const TASK_OUTCOME_UNVERIFIED = 'unverified' as const;

/**
 * Outcome string emitted when a task passed its verify steps apart from tests that also failed on
 * the plan run's start commit (gap-161be1). Counted as passed; its own outcome keeps those
 * failures in view.
 */
export const TASK_OUTCOME_PASSED_WITH_PREEXISTING_FAILURES = 'passed_with_preexisting_failures' as const;

/**
 * Outcome string emitted when a task's work was already there: its attempt changed nothing,
 * and its verify steps passed on the tree as it was.
 */
export const TASK_OUTCOME_ALREADY_SATISFIED = 'already_satisfied' as const;

/**
 * Outcome string of a task that will not run: a task it depends on failed, or it did not start.
 * Counted as neither done nor failed.
 */
export const TASK_OUTCOME_BLOCKED = 'blocked' as const;

/**
 * Outcome string of a task that was still running when its run ended other than by cancellation
 * (bug-60ccba). Counted as failed: it did not finish.
 */
export const TASK_OUTCOME_INTERRUPTED = 'interrupted' as const;

// ---------------------------------------------------------------------------
// Dashboard events
// ---------------------------------------------------------------------------

/**
 * Entry in the plan-set index.
 *
 * Mirrors PlanSetEntry in crates/roko-core/src/dashboard_snapshot.rs.
 * wave: scheduling wave (0 = no prerequisites in the set).
 * depends_on: plan_ids in this set that must succeed before this one starts.
 * conflicts_with: plan_ids that this plan never runs beside (same output files).
 * Older servers omit wave/depends_on/conflicts_with; consumers default to 0/[]/[].
 */
export interface WirePlanSetEntry {
  plan_id: string;
  title?: string;
  tasks_total?: number;
  wave?: number;
  depends_on?: string[];
  conflicts_with?: string[];
}

/**
 * Union of all DashboardEvent variants.
 * Discriminated on the `type` field (serde tag = "type", rename_all = "snake_case").
 */
export type WireDashboardEvent =
  | { type: 'plan_set_loaded'; plans: WirePlanSetEntry[] }
  | { type: 'plan_started'; plan_id: string; tasks_total?: number }
  | { type: 'plan_completed'; plan_id: string; success: boolean }
  | {
      type: 'run_completed';
      outcome: string;
      duration_ms: number;
      cleanup_degraded?: boolean;
      surviving_agent_ids?: string[];
    }
  | { type: 'task_started'; plan_id: string; task_id: string; title?: string; phase: string }
  | { type: 'task_completed'; plan_id: string; task_id: string; outcome: string }
  | {
      type: 'task_blocked';
      plan_id: string;
      task_id: string;
      title?: string;
      /** The failed task that blocked this one; absent when it did not start for another reason. */
      blocked_by?: string;
      reason?: string;
    }
  | {
      type: 'task_phase_changed';
      plan_id: string;
      task_id: string;
      old_phase: string;
      new_phase: string;
    }
  | {
      type: 'agent_spawned';
      agent_id: string;
      plan_id?: string;
      task_id?: string;
      attempt?: number;
      role: string;
      model?: string;
      provider?: string;
    }
  | {
      type: 'agent_output';
      agent_id: string;
      plan_id?: string;
      task_id?: string;
      attempt?: number;
      content: string;
    }
  | { type: 'agent_completed'; agent_id: string; plan_id?: string; task_id?: string; attempt?: number }
  | { type: 'agent_heartbeat'; agent_id: string; plan_id: string; task_id: string; elapsed_ms: number }
  | { type: 'gate_rung_started'; plan_id: string; task_id: string; rung_name: string }
  | { type: 'gate_output_line'; plan_id: string; task_id: string; gate: string; line: string }
  | {
      type: 'gate_result';
      plan_id: string;
      task_id: string;
      gate: string;
      passed: boolean;
      output_text?: string | null;
    }
  | { type: 'efficiency_event'; plan_id: string; task_id: string; metric: string; value: number }
  | { type: 'critical_path_eta_updated'; plan_id: string; eta_minutes: number | null }
  | { type: 'snapshot_rebased'; revision: number; source?: string }
  | { type: 'error'; message: string };

/**
 * A dashboard event as `/api/events` sends it: the server stamps each data frame with the time its
 * hub published the event (gap-8a1fb3). Older servers send no stamp.
 */
export type WireDashboardFrame = WireDashboardEvent & { ts_millis?: number };

// ---------------------------------------------------------------------------
// Dashboard snapshot — fields the portal reads
// ---------------------------------------------------------------------------

/**
 * Live state of one plan (PlanDisplayState / PlanState in Rust). The run
 * fields describe the plan's latest run; older servers omit them.
 */
export interface WirePlanDisplayState {
  plan_id: string;
  phase: string;
  tasks_total: number;
  /**
   * Tasks that finished without failing: passed, already satisfied, accepted with failures,
   * unverified or skipped.
   */
  tasks_done: number;
  tasks_failed: number;
  /** Tasks accepted although their verification failed. */
  tasks_accepted_with_failures?: number;
  /** Tasks that passed their verify steps. */
  tasks_passed?: number;
  /** Tasks whose work was already there; their verify steps passed on the unchanged tree. */
  tasks_already_satisfied?: number;
  /** Tasks that completed without a verify step judging them. */
  tasks_unverified?: number;
  /** Tasks that never ran. */
  tasks_skipped?: number;
  active: boolean;
  /** When the run started (Unix ms); null until it starts. */
  started_at_ms?: number | null;
  /** When the run ended (Unix ms); null until it ends. */
  finished_at_ms?: number | null;
  /** What the run has cost so far, in USD. */
  cost_usd?: number;
}

/** Live state of one task (TaskState in Rust). */
export interface WireTaskState {
  task_id: string;
  /** May be empty when the title was not supplied at TaskStarted time. */
  title?: string;
  plan_id: string;
  phase: string;
  outcome: string | null;
  /** The failed task that blocked this one, when it is blocked. Older servers omit it. */
  blocked_by?: string;
  /** Why the task will not run, when it is blocked. Older servers omit it. */
  blocked_reason?: string;
}

/** Live state of one agent (AgentState in Rust). */
export interface WireAgentState {
  agent_id: string;
  role: string;
  active: boolean;
  model: string;
  provider: string;
  input_tokens: number;
  output_tokens: number;
  cost_usd: number;
  current_task: string;
  current_plan: string;
  attempt: number;
  spawned_at_ms: number;
  elapsed_ms: number;
}

/** A single gate verdict in the ring buffer (GateVerdictView in Rust). */
export interface WireGateVerdictView {
  plan_id: string;
  task_id: string;
  gate: string;
  passed: boolean;
  ts_millis: number;
}

/** Gate outputs aggregated per task (optional — may not be present on older servers). */
export interface WireTaskGateOutput {
  plan_id: string;
  task_id: string;
  gate: string;
  passed: boolean;
  lines: string[];
}

/** A recent error entry (ErrorEntry in Rust). */
export interface WireErrorEntry {
  message: string;
  ts_millis: number;
}

/**
 * Aggregate counters (SnapshotStats in Rust).
 * Only the fields the portal reads are listed; additional fields may be present.
 */
export interface WireSnapshotStats {
  cost_usd_total: number;
  total_input_tokens: number;
  total_output_tokens: number;
  plans_active?: number;
  plans_completed?: number;
  plans_failed?: number;
  tasks_active?: number;
  tasks_completed?: number;
  tasks_failed?: number;
  agents_active?: number;
  gates_passed?: number;
  gates_failed?: number;
  errors_total?: number;
  episodes_total?: number;
}

/**
 * Plan-set index entry, when present (DashboardSnapshot.plan_set — engine work
 * in progress; may be absent on the current server).
 */
export interface WirePlanSet {
  plans: WirePlanSetEntry[];
  tasks_total: number;
  loaded_at_ms: number;
}

/**
 * The materialized dashboard snapshot (DashboardSnapshot in Rust).
 * Only the fields the portal reads are typed here.
 */
export interface WireDashboardSnapshot {
  /** Active and recently completed plans, keyed by plan_id. */
  plans: Record<string, WirePlanDisplayState>;
  /** Plan-set index, present once the plan-set is loaded. */
  plan_set?: WirePlanSet | null;
  /** Active tasks, keyed by "plan_id/task_id". */
  tasks: Record<string, WireTaskState>;
  /** Active and recently completed agents, keyed by agent_id. */
  agents: Record<string, WireAgentState>;
  /** Recent gate verdicts (ring of last 256). */
  gates: WireGateVerdictView[];
  /** Gate outputs aggregated per task (may be absent on older servers). */
  task_gate_outputs?: WireTaskGateOutput[];
  /** Live task output lines, keyed by task_id. */
  task_outputs?: Record<string, string[]>;
  /** Recent errors (ring of last 64). */
  errors: WireErrorEntry[];
  /** Aggregate counters. */
  stats: WireSnapshotStats;
  /** Wall-clock duration of the last completed runner invocation, in ms. */
  run_duration_ms?: number;
  /** Terminal outcome string for the last runner invocation. */
  run_outcome?: string;
  /** Remaining ETA minutes from the critical-path computation. */
  critical_path_eta_minutes?: number;
}

// ---------------------------------------------------------------------------
// StateHub snapshot response — GET /api/statehub/snapshot
// ---------------------------------------------------------------------------

/**
 * Response body from GET /api/statehub/snapshot.
 * Built by RuntimeProjectionSet.state_frame() in projection_contract.rs.
 */
export interface WireStateHubSnapshotResponse {
  /** Monotonic event-bus cursor in lowercase hex, e.g. "0x1f". */
  cursor: string;
  /** The materialized dashboard snapshot. */
  state: WireDashboardSnapshot;
  /** True when the snapshot was recovered from disk rather than built from live events. */
  recovered?: boolean;
}

// ---------------------------------------------------------------------------
// SSE gap payload — emitted when the client missed events
// ---------------------------------------------------------------------------

/**
 * Payload emitted on the SSE stream when the client's cursor is too old
 * and missed events cannot be replayed.
 * Defined in crates/roko-serve/src/routes/sse.rs.
 */
export interface WireGapPayload {
  missed_events: number;
  last_materialized_seq: number;
  snapshot: WireDashboardSnapshot;
}

// ---------------------------------------------------------------------------
// Plan list — GET /api/plans
// ---------------------------------------------------------------------------

/**
 * Wire shape for a plan summary (PlanSummaryDto in plan_types.rs).
 * Optional fields are absent on older servers that have not yet added them.
 */
export interface WirePlanSummary {
  id: string;
  title: string;
  task_count: number;
  tasks_done?: number;
  tasks_failed: number;
  completed: boolean;
  status: string;
  superseded_by?: string;
  old_format: boolean;
  last_error?: string;
  /** Optional grouping label (engine work in progress). */
  group?: string;
  /** Estimated duration in minutes (engine work in progress). */
  estimated_minutes?: number;
}

// ---------------------------------------------------------------------------
// Plan tasks — GET /api/plans/{id}/tasks
// ---------------------------------------------------------------------------

/** A single verify step within a task. */
export interface WireVerifyStep {
  phase: string;
  command: string;
  fail_msg?: string;
  timeout_ms?: number;
}

/**
 * Wire shape for one task (PlanTaskDto in plan_types.rs).
 * Optional fields are absent on older servers.
 */
export interface WirePlanTask {
  id: string;
  title: string;
  description?: string;
  role?: string;
  tier: string;
  status: string;
  depends_on: string[];
  files: string[];
  completed: boolean;
  verify_phases: string[];
  verify?: WireVerifyStep[];
  estimated_minutes?: number;
  model_hint?: string;
}

/**
 * Wire envelope for the tasks route (PlanTasksDto in plan_types.rs).
 * Optional fields are absent on older servers.
 */
export interface WirePlanTasks {
  plan_id: string;
  task_count: number;
  tasks: WirePlanTask[];
  title?: string;
  max_parallel?: number;
}

// ---------------------------------------------------------------------------
// Plan source — raw TOML for the editor
// ---------------------------------------------------------------------------

/** Raw plan source file as served to the editor. */
export interface WirePlanSource {
  id: string;
  path: string;
  toml: string;
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/** A single diagnostic message from plan validation. */
export interface WireDiagnostic {
  severity: 'error' | 'warning';
  rule_id: string;
  plan_id?: string;
  task_id?: string;
  message: string;
}

/** Response body from the plan validation endpoint. */
export interface WireValidation {
  valid: boolean;
  errors: string[];
  warnings: string[];
  diagnostics: WireDiagnostic[];
}

/** Response body (200) from saving a plan source. */
export interface WireSourceSaved {
  saved: boolean;
  errors: string[];
  warnings: string[];
  diagnostics: WireDiagnostic[];
}

/** Response body (422) when a saved plan fails validation. */
export interface WireInvalidPlan {
  code: string;
  message: string;
  errors: string[];
  warnings: string[];
  diagnostics: WireDiagnostic[];
}

// ---------------------------------------------------------------------------
// Operations and run control
// ---------------------------------------------------------------------------

/** Status of an async operation (e.g. plan import, run start). */
export interface WireOperation {
  id: string;
  kind?: string;
  status: string;
  result?: { slug?: string; task_count?: number } | null;
  error?: string | null;
}

/**
 * Response body from a generate/run acceptance endpoint.
 * - plan_id:            the newly created plan's identifier
 * - order:             plan execution order for a multi-plan run
 * - resume:            whether this resumes a prior run
 * - max_parallel_plans: effective concurrency limit for the run
 */
export interface WireAccepted {
  id: string;
  plan_id?: string;
  order?: string[];
  resume?: boolean;
  max_parallel_plans?: number;
}

/** Response body from GET /api/status. */
export interface WireStatus {
  workdir: string;
  git_branch?: string | null;
}

// ---------------------------------------------------------------------------
// Task reviews — a Graph run holds a verified attempt for approval
// ---------------------------------------------------------------------------

/** One task of GET /api/plans/{id}/reviews; a held Graph attempt has status 'awaiting_approval'. */
export interface WireReview {
  task_id: string;
  description?: string | null;
  status: string;
  attempt_key?: string | null;
  diff_summary: string;
  files_changed: string[];
}

/** Response body from GET /api/plans/{id}/reviews. */
export interface WireReviews {
  plan_id: string;
  reviews: WireReview[];
}

/** One changed file of a task's diff. */
export interface WireDiffFile {
  path: string;
  status: string;
  additions: number;
  deletions: number;
  patch: string;
}

/** Response body from GET /api/plans/{id}/tasks/{task_id}/diff ('review_hold' for a held attempt). */
export interface WireTaskDiff {
  task_id: string;
  file_count: number;
  total_additions: number;
  total_deletions: number;
  files: WireDiffFile[];
  source?: string;
  status?: string;
}

/** Request body of POST /api/plans/{id}/tasks/{task_id}/review (ReviewDecision). */
export interface WireReviewDecision {
  decision: 'approve' | 'reject' | 'skip';
  comment?: string;
}
