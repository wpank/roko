/**
 * agentRoster.ts — pure fold that builds the AGENTS cell of the run band.
 *
 * A plan-set run executes independent plans concurrently, so the roster spans
 * every running plan and each row names its plan.
 *
 * Rules:
 * - Never mutate the input state.
 * - Never call Date.now() — time is passed in as `nowMs`.
 */

import type { RunState, AgentRun } from '@/lib/runState';

// ── Public interfaces ──────────────────────────────────────────────────────────

export interface RosterRow {
  agentId: string;
  planId: string | null;
  taskId: string | null;
  role: string;
  model: string;
  /** nowMs − spawnedAtMs; null when spawnedAtMs is unknown. */
  elapsedMs: number | null;
  /** inputTokens + outputTokens. */
  tokens: number;
}

export interface Roster {
  /** Active agents of running plans, ordered by plan → task (numeric) → agentId. */
  rows: RosterRow[];
  /** Inactive agents of those same running plans (counted, not listed). */
  finished: number;
  /**
   * Open parallel slots: sum over running plans with a known max_parallel of
   * max(0, maxParallel − activeAgents).  null when no running plan has a
   * known limit.
   */
  idle: number | null;
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/** Locale-aware numeric compare (task-2 < task-10). */
function numericCompare(a: string, b: string): number {
  return a.localeCompare(b, undefined, { numeric: true });
}

// ── buildRoster ────────────────────────────────────────────────────────────────

/**
 * Build the agent roster spanning all running plans.
 *
 * @param run              - Live RunState assembled from SSE events.
 * @param opts.nowMs       - Current wall-clock timestamp (do not call Date.now here).
 * @param opts.runningPlanIds - Ordered list of currently running plan ids; defines
 *                           the primary sort key for rows.
 * @param opts.maxParallelByPlan - Known concurrency caps, keyed by plan id.
 *                           `undefined` means the limit is unknown for that plan.
 */
export function buildRoster(
  run: RunState,
  opts: {
    nowMs: number;
    runningPlanIds: readonly string[];
    maxParallelByPlan: Readonly<Record<string, number | undefined>>;
  },
): Roster {
  const { nowMs, runningPlanIds, maxParallelByPlan } = opts;

  // Build a plan-id → position lookup for the primary sort key.
  const planOrder = new Map<string, number>();
  runningPlanIds.forEach((id, idx) => planOrder.set(id, idx));

  const runningSet = new Set(runningPlanIds);

  // Partition agents that belong to running plans into active / finished.
  const planAgents: AgentRun[] = Object.values(run.agents).filter(
    (a) => a.planId !== null && runningSet.has(a.planId),
  );

  const activeAgents = planAgents.filter((a) => a.active);
  const finishedAgents = planAgents.filter((a) => !a.active);

  // Count active agents per plan (used for idle calculation).
  const activeByPlan = new Map<string, number>();
  for (const a of activeAgents) {
    if (a.planId !== null) {
      activeByPlan.set(a.planId, (activeByPlan.get(a.planId) ?? 0) + 1);
    }
  }

  // Sort: plan order → taskId (numeric-aware, null sorts first as '') → agentId.
  const sorted = [...activeAgents].sort((a, b) => {
    const pa = a.planId !== null ? (planOrder.get(a.planId) ?? Infinity) : Infinity;
    const pb = b.planId !== null ? (planOrder.get(b.planId) ?? Infinity) : Infinity;
    if (pa !== pb) return pa - pb;

    const taskCmp = numericCompare(a.taskId ?? '', b.taskId ?? '');
    if (taskCmp !== 0) return taskCmp;

    return numericCompare(a.agentId, b.agentId);
  });

  // Build rows.
  const rows: RosterRow[] = sorted.map((a) => ({
    agentId: a.agentId,
    planId: a.planId,
    taskId: a.taskId,
    role: a.role,
    model: a.model,
    elapsedMs: a.spawnedAtMs !== null ? nowMs - a.spawnedAtMs : null,
    tokens: a.inputTokens + a.outputTokens,
  }));

  // idle: sum max(0, maxParallel − active) for plans with a known limit.
  // Stays null until at least one running plan has a known limit.
  let idle: number | null = null;
  for (const planId of runningPlanIds) {
    const maxParallel = maxParallelByPlan[planId];
    if (maxParallel === undefined) continue;
    const active = activeByPlan.get(planId) ?? 0;
    idle = (idle ?? 0) + Math.max(0, maxParallel - active);
  }

  return {
    rows,
    finished: finishedAgents.length,
    idle,
  };
}
