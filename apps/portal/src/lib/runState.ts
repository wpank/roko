/**
 * runState.ts — pure fold that assembles live RunState from dashboard events.
 *
 * `applyEvent` mirrors `DashboardSnapshot::apply_with_ts` in
 * `crates/roko-core/src/dashboard_snapshot.rs`.
 *
 * Rules:
 * - Never mutate the input state — always return new objects.
 * - Never call Date.now() — time is passed in as `nowMs`.
 */

import type { WireDashboardEvent, WireDashboardSnapshot } from '@/api/contracts';
import { TASK_OUTCOME_ACCEPTED_WITH_FAILURES } from '@/api/contracts';
import { decodeStreamRecord } from '@/lib/streamRecord';
import type { TranscriptEntry, AttemptDivider } from '@/lib/streamRecord';

// ── Constants ──────────────────────────────────────────────────────────────────

export const MAX_TRANSCRIPT_ENTRIES = 2000;
export const MAX_ERRORS = 50;
export const MAX_USAGE_SAMPLES = 600;

// ── Types ──────────────────────────────────────────────────────────────────────

export type TaskStatus =
  | 'active'
  | 'passed'
  | 'failed'
  | 'accepted_with_failures'
  | 'skipped'
  | 'cancelled';

export type PlanPhase = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface CheckRun {
  name: string;
  index: number | null;
  phase: string;
  status: 'running' | 'passed' | 'failed';
  output: string;
}

export interface TaskRun {
  planId: string;
  taskId: string;
  title: string;
  status: TaskStatus;
  phase: string;
  attempts: number;
  startedAtMs: number | null;
  finishedAtMs: number | null;
  agentId: string | null;
  role: string | null;
  model: string | null;
  costUsd: number;
  inputTokens: number;
  outputTokens: number;
  checks: CheckRun[];
}

export interface PlanRun {
  planId: string;
  title: string | null;
  phase: PlanPhase;
  tasksTotal: number;
  tasksDone: number;
  tasksFailed: number;
  tasksAccepted: number;
  startedAtMs: number | null;
  finishedAtMs: number | null;
  etaMinutes: number | null;
  costUsd: number;
}

export interface AgentRun {
  agentId: string;
  planId: string | null;
  taskId: string | null;
  role: string;
  model: string;
  active: boolean;
  spawnedAtMs: number | null;
  costUsd: number;
  inputTokens: number;
  outputTokens: number;
}

export interface Transcript {
  entries: TranscriptEntry[];
  dropped: number;
}

export interface RunState {
  planSet: { planIds: string[]; tasksTotal: number; loadedAtMs: number } | null;
  plans: Record<string, PlanRun>;
  tasks: Record<string, TaskRun>;
  agents: Record<string, AgentRun>;
  transcripts: Record<string, Transcript>;
  errors: { message: string; atMs: number }[];
  run: { startedAtMs: number | null; durationMs: number | null; outcome: string | null };
  totals: { costUsd: number; inputTokens: number; outputTokens: number };
  usage: { atMs: number; tokens: number }[];
}

// ── Terminal statuses ──────────────────────────────────────────────────────────

const TERMINAL: Set<TaskStatus> = new Set([
  'passed',
  'failed',
  'accepted_with_failures',
  'skipped',
  'cancelled',
]);

// ── Helpers ────────────────────────────────────────────────────────────────────

/**
 * Compound key for the `tasks` and `transcripts` records.
 * e.g. taskKey('plan1', 'task1') → 'plan1/task1'
 */
export function taskKey(planId: string, taskId: string): string {
  return `${planId}/${taskId}`;
}

/**
 * Parse a structured index/phase out of a gate rung name.
 *
 * "verify[3:compile]" → { index: 3, phase: 'compile' }
 * "verify[2]"         → { index: 2, phase: '' }
 * "anything else"     → { index: null, phase: 'anything else' }
 */
export function parseCheckName(name: string): { index: number | null; phase: string } {
  const m = /\[(\d+)(?::([^\]]*))?\]$/.exec(name);
  if (!m) return { index: null, phase: name };
  return { index: parseInt(m[1]!, 10), phase: m[2] ?? '' };
}

/**
 * Classify a task outcome string into a TaskStatus.
 * Mirrors `classify_task_outcome` in the Rust source.
 * The accepted_with_failures check runs first because the string contains "fail".
 */
function classifyOutcome(outcome: string): TaskStatus {
  if (outcome === TASK_OUTCOME_ACCEPTED_WITH_FAILURES) return 'accepted_with_failures';
  const lo = outcome.toLowerCase();
  if (lo.includes('fail') || lo.includes('error')) return 'failed';
  if (lo === 'skipped' || lo === 'condition-skipped' || lo === 'unknown') return 'skipped';
  return 'passed';
}

/** Append a TranscriptEntry to a Transcript, capping at MAX_TRANSCRIPT_ENTRIES. */
function appendTranscript(t: Transcript, entry: TranscriptEntry): Transcript {
  if (t.entries.length < MAX_TRANSCRIPT_ENTRIES) {
    return { entries: [...t.entries, entry], dropped: t.dropped };
  }
  // Drop oldest entry and count it
  return {
    entries: [...t.entries.slice(1), entry],
    dropped: t.dropped + 1,
  };
}

/** Sort checks by index, nulls last. Stable relative to original order for ties. */
function sortChecks(checks: CheckRun[]): CheckRun[] {
  return [...checks].sort((a, b) => {
    if (a.index === null && b.index === null) return 0;
    if (a.index === null) return 1;
    if (b.index === null) return -1;
    return a.index - b.index;
  });
}

/**
 * Upsert a check by name, applying `update` to the existing or a fresh blank
 * check, then re-sort the array.
 */
function upsertCheck(
  checks: CheckRun[],
  name: string,
  update: (check: CheckRun) => CheckRun,
): CheckRun[] {
  const { index, phase } = parseCheckName(name);
  const idx = checks.findIndex((c) => c.name === name);
  if (idx >= 0) {
    const updated = [...checks];
    updated[idx] = update(checks[idx]!);
    return sortChecks(updated);
  }
  const blank: CheckRun = { name, index, phase, status: 'running', output: '' };
  return sortChecks([...checks, update(blank)]);
}

// ── initialRunState ────────────────────────────────────────────────────────────

/** Return a blank RunState with zero counters. */
export function initialRunState(): RunState {
  return {
    planSet: null,
    plans: {},
    tasks: {},
    agents: {},
    transcripts: {},
    errors: [],
    run: { startedAtMs: null, durationMs: null, outcome: null },
    totals: { costUsd: 0, inputTokens: 0, outputTokens: 0 },
    usage: [],
  };
}

// ── applyEvent ─────────────────────────────────────────────────────────────────

/**
 * Pure fold — returns a new RunState with the event applied.
 * Never mutates `state` or any nested object.
 */
export function applyEvent(
  state: RunState,
  event: WireDashboardEvent,
  nowMs: number,
): RunState {
  switch (event.type) {
    // ── plan_set_loaded ────────────────────────────────────────────────────────
    case 'plan_set_loaded': {
      const planIds = event.plans.map((p) => p.plan_id);
      const tasksTotal = event.plans.reduce((s, p) => s + (p.tasks_total ?? 0), 0);

      const newPlans: Record<string, PlanRun> = { ...state.plans };
      for (const entry of event.plans) {
        const existing = newPlans[entry.plan_id];
        if (existing?.phase === 'running') {
          // Keep the running plan; only fill in the title if missing
          if (entry.title && !existing.title) {
            newPlans[entry.plan_id] = { ...existing, title: entry.title };
          }
        } else {
          newPlans[entry.plan_id] = {
            planId: entry.plan_id,
            title: entry.title ?? null,
            phase: 'pending',
            tasksTotal: entry.tasks_total ?? 0,
            tasksDone: 0,
            tasksFailed: 0,
            tasksAccepted: 0,
            startedAtMs: null,
            finishedAtMs: null,
            etaMinutes: null,
            costUsd: 0,
          };
        }
      }

      return {
        ...state,
        planSet: { planIds, tasksTotal, loadedAtMs: nowMs },
        plans: newPlans,
        run: { startedAtMs: nowMs, durationMs: null, outcome: null },
      };
    }

    // ── plan_started ───────────────────────────────────────────────────────────
    case 'plan_started': {
      const existing = state.plans[event.plan_id];
      const tasksTotal = Math.max(existing?.tasksTotal ?? 0, event.tasks_total ?? 0);
      const plan: PlanRun = existing
        ? { ...existing, phase: 'running', tasksTotal, startedAtMs: nowMs }
        : {
            planId: event.plan_id,
            title: null,
            phase: 'running',
            tasksTotal,
            tasksDone: 0,
            tasksFailed: 0,
            tasksAccepted: 0,
            startedAtMs: nowMs,
            finishedAtMs: null,
            etaMinutes: null,
            costUsd: 0,
          };

      // Only reset run when the plan is not already part of the planSet
      const inPlanSet = state.planSet?.planIds.includes(event.plan_id) ?? false;
      const newRun = inPlanSet
        ? state.run
        : { startedAtMs: nowMs, durationMs: null, outcome: null };

      return {
        ...state,
        plans: { ...state.plans, [event.plan_id]: plan },
        run: newRun,
      };
    }

    // ── plan_completed ─────────────────────────────────────────────────────────
    case 'plan_completed': {
      const existing = state.plans[event.plan_id];
      if (!existing) return state;
      return {
        ...state,
        plans: {
          ...state.plans,
          [event.plan_id]: {
            ...existing,
            phase: event.success ? 'completed' : 'failed',
            finishedAtMs: nowMs,
          },
        },
      };
    }

    // ── run_completed ──────────────────────────────────────────────────────────
    case 'run_completed': {
      // durationMs/outcome set only when durationMs is currently null
      const newRun =
        state.run.durationMs === null
          ? { ...state.run, durationMs: event.duration_ms, outcome: event.outcome }
          : state.run;

      // Running plans → terminal phase based on outcome
      const planPhaseForOutcome = (outcome: string): PlanPhase => {
        if (outcome === 'succeeded') return 'completed';
        if (outcome === 'cancelled') return 'cancelled';
        return 'failed';
      };

      const newPlans: Record<string, PlanRun> = {};
      for (const [id, plan] of Object.entries(state.plans)) {
        if (plan.phase === 'running') {
          newPlans[id] = {
            ...plan,
            phase: planPhaseForOutcome(event.outcome),
            finishedAtMs: nowMs,
          };
        } else {
          newPlans[id] = plan;
        }
      }

      // Active tasks → cancelled
      const newTasks: Record<string, TaskRun> = {};
      for (const [key, task] of Object.entries(state.tasks)) {
        newTasks[key] = task.status === 'active'
          ? { ...task, status: 'cancelled', finishedAtMs: nowMs }
          : task;
      }

      // Every agent → inactive
      const newAgents: Record<string, AgentRun> = {};
      for (const [id, agent] of Object.entries(state.agents)) {
        newAgents[id] = agent.active ? { ...agent, active: false } : agent;
      }

      return { ...state, run: newRun, plans: newPlans, tasks: newTasks, agents: newAgents };
    }

    // ── task_started ───────────────────────────────────────────────────────────
    case 'task_started': {
      const key = taskKey(event.plan_id, event.task_id);
      const existing = state.tasks[key];
      const isRetry = existing !== undefined && TERMINAL.has(existing.status);

      const attempts = isRetry ? existing.attempts + 1 : 1;
      // Preserve title when the event's title is empty
      const title =
        event.title && event.title.length > 0 ? event.title : (existing?.title ?? '');

      const task: TaskRun = {
        planId: event.plan_id,
        taskId: event.task_id,
        title,
        status: 'active',
        phase: event.phase,
        attempts,
        startedAtMs: nowMs,
        finishedAtMs: null,
        agentId: existing?.agentId ?? null,
        role: existing?.role ?? null,
        model: existing?.model ?? null,
        costUsd: existing?.costUsd ?? 0,
        inputTokens: existing?.inputTokens ?? 0,
        outputTokens: existing?.outputTokens ?? 0,
        // Clear checks on retry; preserve for first start (handles phase changes)
        checks: isRetry ? [] : (existing?.checks ?? []),
      };

      // Append a divider to the transcript on retry
      let newTranscripts = state.transcripts;
      if (isRetry) {
        const divider: AttemptDivider = { kind: 'divider', attempt: attempts };
        const transcript = state.transcripts[key] ?? { entries: [], dropped: 0 };
        newTranscripts = {
          ...state.transcripts,
          [key]: appendTranscript(transcript, divider),
        };
      }

      // plan.tasksTotal = max(tasksTotal, count of known tasks for this plan)
      const plan = state.plans[event.plan_id];
      const planTaskCount = Object.values({ ...state.tasks, [key]: task }).filter(
        (t) => t.planId === event.plan_id,
      ).length;
      const newPlans = plan
        ? {
            ...state.plans,
            [event.plan_id]: {
              ...plan,
              tasksTotal: Math.max(plan.tasksTotal, planTaskCount),
            },
          }
        : state.plans;

      return {
        ...state,
        tasks: { ...state.tasks, [key]: task },
        transcripts: newTranscripts,
        plans: newPlans,
      };
    }

    // ── task_completed ─────────────────────────────────────────────────────────
    case 'task_completed': {
      const key = taskKey(event.plan_id, event.task_id);
      const existing = state.tasks[key];

      // Ignore unknown tasks or already-terminal tasks (idempotent)
      if (!existing || TERMINAL.has(existing.status)) return state;

      const status = classifyOutcome(event.outcome);
      const task: TaskRun = { ...existing, status, finishedAtMs: nowMs };

      // Update plan counters
      const plan = state.plans[event.plan_id];
      let newPlans = state.plans;
      if (plan) {
        let { tasksDone, tasksFailed, tasksAccepted } = plan;
        if (status === 'accepted_with_failures') {
          tasksDone += 1;
          tasksAccepted += 1;
        } else if (status === 'failed') {
          tasksFailed += 1;
        } else {
          // passed or skipped both count as done
          tasksDone += 1;
        }
        newPlans = {
          ...state.plans,
          [event.plan_id]: { ...plan, tasksDone, tasksFailed, tasksAccepted },
        };
      }

      return { ...state, tasks: { ...state.tasks, [key]: task }, plans: newPlans };
    }

    // ── task_phase_changed ─────────────────────────────────────────────────────
    case 'task_phase_changed': {
      const key = taskKey(event.plan_id, event.task_id);
      const existing = state.tasks[key];
      if (!existing) return state;
      return {
        ...state,
        tasks: { ...state.tasks, [key]: { ...existing, phase: event.new_phase } },
      };
    }

    // ── agent_spawned ──────────────────────────────────────────────────────────
    case 'agent_spawned': {
      const role = event.role || event.model || 'impl';
      const model = event.model ?? '';
      const existing = state.agents[event.agent_id];
      const agent: AgentRun = {
        agentId: event.agent_id,
        planId: event.plan_id ?? null,
        taskId: event.task_id ?? null,
        role,
        model,
        active: true,
        spawnedAtMs: existing?.spawnedAtMs ?? null,
        costUsd: existing?.costUsd ?? 0,
        inputTokens: existing?.inputTokens ?? 0,
        outputTokens: existing?.outputTokens ?? 0,
      };

      // Link the task's agentId/role/model on spawn
      let newTasks = state.tasks;
      if (event.plan_id && event.task_id) {
        const key = taskKey(event.plan_id, event.task_id);
        const task = state.tasks[key];
        if (task) {
          newTasks = {
            ...state.tasks,
            [key]: { ...task, agentId: event.agent_id, role, model },
          };
        }
      }

      return {
        ...state,
        agents: { ...state.agents, [event.agent_id]: agent },
        tasks: newTasks,
      };
    }

    // ── agent_completed ────────────────────────────────────────────────────────
    case 'agent_completed': {
      const existing = state.agents[event.agent_id];
      if (!existing) return state;
      return {
        ...state,
        agents: { ...state.agents, [event.agent_id]: { ...existing, active: false } },
      };
    }

    // ── agent_heartbeat ────────────────────────────────────────────────────────
    case 'agent_heartbeat': {
      const existing = state.agents[event.agent_id];
      // Fill spawnedAtMs only when it is not yet known
      const spawnedAtMs =
        existing?.spawnedAtMs != null ? existing.spawnedAtMs : nowMs - event.elapsed_ms;

      const agent: AgentRun = existing
        ? { ...existing, spawnedAtMs }
        : {
            agentId: event.agent_id,
            planId: event.plan_id,
            taskId: event.task_id,
            role: 'impl',
            model: '',
            active: true,
            spawnedAtMs,
            costUsd: 0,
            inputTokens: 0,
            outputTokens: 0,
          };

      return { ...state, agents: { ...state.agents, [event.agent_id]: agent } };
    }

    // ── agent_output ───────────────────────────────────────────────────────────
    case 'agent_output': {
      const entry = decodeStreamRecord(event.content);

      // Resolve the transcript key: prefer event's plan/task, then agent's current context
      let key: string | null = null;
      if (event.plan_id && event.task_id) {
        key = taskKey(event.plan_id, event.task_id);
      } else {
        const agent = state.agents[event.agent_id];
        if (agent?.planId && agent?.taskId) {
          key = taskKey(agent.planId, agent.taskId);
        }
      }

      // Drop if no context
      if (!key) return state;

      const transcript = state.transcripts[key] ?? { entries: [], dropped: 0 };
      return {
        ...state,
        transcripts: {
          ...state.transcripts,
          [key]: appendTranscript(transcript, entry),
        },
      };
    }

    // ── gate_rung_started ──────────────────────────────────────────────────────
    case 'gate_rung_started': {
      const key = taskKey(event.plan_id, event.task_id);
      const task = state.tasks[key];
      if (!task) return state;
      const { index, phase } = parseCheckName(event.rung_name);
      const check: CheckRun = { name: event.rung_name, index, phase, status: 'running', output: '' };
      const checks = upsertCheck(task.checks, event.rung_name, () => check);
      return { ...state, tasks: { ...state.tasks, [key]: { ...task, checks } } };
    }

    // ── gate_output_line ───────────────────────────────────────────────────────
    case 'gate_output_line': {
      const key = taskKey(event.plan_id, event.task_id);
      const task = state.tasks[key];
      if (!task) return state;
      const checks = upsertCheck(task.checks, event.gate, (c) => ({
        ...c,
        output: c.output + event.line + '\n',
      }));
      return { ...state, tasks: { ...state.tasks, [key]: { ...task, checks } } };
    }

    // ── gate_result ────────────────────────────────────────────────────────────
    case 'gate_result': {
      const key = taskKey(event.plan_id, event.task_id);
      const task = state.tasks[key];
      if (!task) return state;
      const checks = upsertCheck(task.checks, event.gate, (c) => ({
        ...c,
        status: event.passed ? ('passed' as const) : ('failed' as const),
        output: event.output_text != null ? event.output_text : c.output,
      }));
      return { ...state, tasks: { ...state.tasks, [key]: { ...task, checks } } };
    }

    // ── efficiency_event ───────────────────────────────────────────────────────
    case 'efficiency_event': {
      const { metric, value, plan_id, task_id } = event;

      // Only handle the three metrics we track
      if (metric !== 'input_tokens' && metric !== 'output_tokens' && metric !== 'cost_usd') {
        return state;
      }

      const isTokens = metric === 'input_tokens' || metric === 'output_tokens';
      const isCost = metric === 'cost_usd';

      // ── task
      const key = taskKey(plan_id, task_id);
      const task = state.tasks[key];
      let newTasks = state.tasks;
      let agentId: string | null = null;
      if (task) {
        agentId = task.agentId;
        newTasks = {
          ...state.tasks,
          [key]: {
            ...task,
            costUsd: isCost ? task.costUsd + value : task.costUsd,
            inputTokens:
              metric === 'input_tokens' ? task.inputTokens + value : task.inputTokens,
            outputTokens:
              metric === 'output_tokens' ? task.outputTokens + value : task.outputTokens,
          },
        };
      }

      // ── agent (the one working the task)
      let newAgents = state.agents;
      if (agentId) {
        const agent = state.agents[agentId];
        if (agent) {
          newAgents = {
            ...state.agents,
            [agentId]: {
              ...agent,
              costUsd: isCost ? agent.costUsd + value : agent.costUsd,
              inputTokens:
                metric === 'input_tokens' ? agent.inputTokens + value : agent.inputTokens,
              outputTokens:
                metric === 'output_tokens' ? agent.outputTokens + value : agent.outputTokens,
            },
          };
        }
      }

      // ── plan (cost only)
      const plan = state.plans[plan_id];
      let newPlans = state.plans;
      if (plan && isCost) {
        newPlans = {
          ...state.plans,
          [plan_id]: { ...plan, costUsd: plan.costUsd + value },
        };
      }

      // ── totals
      const newTotals = {
        costUsd: isCost ? state.totals.costUsd + value : state.totals.costUsd,
        inputTokens:
          metric === 'input_tokens' ? state.totals.inputTokens + value : state.totals.inputTokens,
        outputTokens:
          metric === 'output_tokens'
            ? state.totals.outputTokens + value
            : state.totals.outputTokens,
      };

      // ── usage ring (token metrics only)
      let newUsage = state.usage;
      if (isTokens) {
        const entry = { atMs: nowMs, tokens: value };
        const next = [...state.usage, entry];
        newUsage = next.length > MAX_USAGE_SAMPLES ? next.slice(-MAX_USAGE_SAMPLES) : next;
      }

      return {
        ...state,
        tasks: newTasks,
        agents: newAgents,
        plans: newPlans,
        totals: newTotals,
        usage: newUsage,
      };
    }

    // ── critical_path_eta_updated ──────────────────────────────────────────────
    case 'critical_path_eta_updated': {
      const plan = state.plans[event.plan_id];
      if (!plan) return state;
      return {
        ...state,
        plans: {
          ...state.plans,
          [event.plan_id]: { ...plan, etaMinutes: event.eta_minutes },
        },
      };
    }

    // ── error ──────────────────────────────────────────────────────────────────
    case 'error': {
      const next = [...state.errors, { message: event.message, atMs: nowMs }];
      return {
        ...state,
        errors: next.length > MAX_ERRORS ? next.slice(-MAX_ERRORS) : next,
      };
    }

    // ── snapshot_rebased and unknown ───────────────────────────────────────────
    case 'snapshot_rebased':
      return state;

    default:
      // Unknown event type — return unchanged
      return state;
  }
}

// ── fromSnapshot ───────────────────────────────────────────────────────────────

/**
 * Map a materialized `WireDashboardSnapshot` to a `RunState`.
 *
 * Mirrors the projection done by the Rust DashboardSnapshot fields.
 * `usage` starts empty — the burn-rate ring is only built from live events.
 */
export function fromSnapshot(snapshot: WireDashboardSnapshot, nowMs: number): RunState {
  void nowMs; // only used by callers who may pass it for consistency; unused here

  // ── Map plan phase ────────────────────────────────────────────────────────
  function mapPlanPhase(phase: string, active: boolean): PlanPhase {
    if (active) return 'running';
    switch (phase) {
      case 'pending':
        return 'pending';
      case 'completed':
      case 'complete':
        return 'completed';
      case 'failed':
        return 'failed';
      case 'cancelled':
        return 'cancelled';
      default:
        // Unknown phase and not active → pending
        return 'pending';
    }
  }

  // ── Plans ─────────────────────────────────────────────────────────────────
  const plans: Record<string, PlanRun> = {};
  for (const [id, p] of Object.entries(snapshot.plans)) {
    plans[id] = {
      planId: id,
      title: null,
      phase: mapPlanPhase(p.phase, p.active),
      tasksTotal: p.tasks_total,
      tasksDone: p.tasks_done,
      tasksFailed: p.tasks_failed,
      tasksAccepted: 0,
      startedAtMs: null,
      finishedAtMs: null,
      etaMinutes: null,
      costUsd: 0,
    };
  }

  // ── plan_set ──────────────────────────────────────────────────────────────
  const planSet = snapshot.plan_set
    ? {
        planIds: snapshot.plan_set.plans.map((p) => p.plan_id),
        tasksTotal: snapshot.plan_set.tasks_total,
        loadedAtMs: snapshot.plan_set.loaded_at_ms,
      }
    : null;

  // ── Build task → plan mapping (for ambiguity detection in task_outputs) ───
  // task_outputs is keyed by task_id only; skip if one task_id belongs to multiple plans
  const taskIdToPlanIds = new Map<string, string[]>();
  for (const t of Object.values(snapshot.tasks)) {
    const ids = taskIdToPlanIds.get(t.task_id) ?? [];
    ids.push(t.plan_id);
    taskIdToPlanIds.set(t.task_id, ids);
  }

  // ── Tasks ─────────────────────────────────────────────────────────────────
  const tasks: Record<string, TaskRun> = {};
  for (const t of Object.values(snapshot.tasks)) {
    const key = taskKey(t.plan_id, t.task_id);
    const status: TaskStatus = t.outcome === null ? 'active' : classifyOutcome(t.outcome);
    tasks[key] = {
      planId: t.plan_id,
      taskId: t.task_id,
      title: t.title ?? '',
      status,
      phase: t.phase,
      attempts: 1,
      startedAtMs: null,
      finishedAtMs: null,
      agentId: null, // linked below from agents
      role: null,
      model: null,
      costUsd: 0,
      inputTokens: 0,
      outputTokens: 0,
      checks: [],
    };
  }

  // ── Agents + link agentId on tasks ───────────────────────────────────────
  const agents: Record<string, AgentRun> = {};
  for (const [id, a] of Object.entries(snapshot.agents)) {
    agents[id] = {
      agentId: id,
      planId: a.current_plan || null,
      taskId: a.current_task || null,
      role: a.role || a.model || 'impl',
      model: a.model,
      active: a.active,
      spawnedAtMs: a.spawned_at_ms === 0 ? null : a.spawned_at_ms,
      costUsd: a.cost_usd,
      inputTokens: a.input_tokens,
      outputTokens: a.output_tokens,
    };

    if (a.current_plan && a.current_task) {
      const key = taskKey(a.current_plan, a.current_task);
      const task = tasks[key];
      if (task) {
        tasks[key] = {
          ...task,
          agentId: id,
          role: a.role || a.model || 'impl',
          model: a.model,
        };
      }
    }
  }

  // ── Gate output map: `plan/task/gate` → joined output ────────────────────
  const gateOutputMap = new Map<string, string>();
  for (const tgo of snapshot.task_gate_outputs ?? []) {
    const k = `${taskKey(tgo.plan_id, tgo.task_id)}/${tgo.gate}`;
    gateOutputMap.set(k, tgo.lines.join('\n'));
  }

  // ── Latest gate verdict per (task_key, gate) — later entries win ─────────
  const latestGate = new Map<
    string,
    { plan_id: string; task_id: string; gate: string; passed: boolean }
  >();
  for (const g of snapshot.gates) {
    const k = `${taskKey(g.plan_id, g.task_id)}/${g.gate}`;
    latestGate.set(k, g);
  }

  // Apply checks to tasks
  for (const [mapKey, verdict] of latestGate) {
    const tKey = taskKey(verdict.plan_id, verdict.task_id);
    const task = tasks[tKey];
    if (!task) continue;
    const output = gateOutputMap.get(mapKey) ?? '';
    const { index, phase } = parseCheckName(verdict.gate);
    const check: CheckRun = {
      name: verdict.gate,
      index,
      phase,
      status: verdict.passed ? 'passed' : 'failed',
      output,
    };
    tasks[tKey] = {
      ...task,
      checks: sortChecks([
        ...task.checks.filter((c) => c.name !== verdict.gate),
        check,
      ]),
    };
  }

  // ── Transcripts from task_outputs ─────────────────────────────────────────
  const transcripts: Record<string, Transcript> = {};
  for (const [taskId, lines] of Object.entries(snapshot.task_outputs ?? {})) {
    const planIds = taskIdToPlanIds.get(taskId) ?? [];
    if (planIds.length !== 1) continue; // ambiguous — skip
    const planId = planIds[0]!;
    const key = taskKey(planId, taskId);
    let t: Transcript = { entries: [], dropped: 0 };
    for (const line of lines) {
      t = appendTranscript(t, decodeStreamRecord(line));
    }
    transcripts[key] = t;
  }

  // ── Errors ────────────────────────────────────────────────────────────────
  const errors = snapshot.errors.map((e) => ({ message: e.message, atMs: e.ts_millis }));

  // ── Totals ────────────────────────────────────────────────────────────────
  const totals = {
    costUsd: snapshot.stats.cost_usd_total,
    inputTokens: snapshot.stats.total_input_tokens,
    outputTokens: snapshot.stats.total_output_tokens,
  };

  // ── Run ───────────────────────────────────────────────────────────────────
  const run = {
    startedAtMs: planSet?.loadedAtMs ?? null,
    durationMs: snapshot.run_duration_ms ?? null,
    outcome: snapshot.run_outcome ?? null,
  };

  // ── ETA → the running plan, when exactly one is running ───────────────────
  if (snapshot.critical_path_eta_minutes != null) {
    const runningIds = Object.keys(plans).filter((id) => plans[id]!.phase === 'running');
    if (runningIds.length === 1) {
      const id = runningIds[0]!;
      plans[id] = { ...plans[id]!, etaMinutes: snapshot.critical_path_eta_minutes };
    }
  }

  return {
    planSet,
    plans,
    tasks,
    agents,
    transcripts,
    errors,
    run,
    totals,
    usage: [], // usage ring starts empty — built from live events only
  };
}
