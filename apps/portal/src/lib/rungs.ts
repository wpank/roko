/**
 * rungs.ts — verify-ladder helpers for the run band's CHECKS cell and the
 * stream's checks view.
 *
 * A task's authored verify steps form a "ladder". The event stream reports
 * each step by name (`verify[i:phase]`) as it starts and ends. buildRungs
 * folds CheckRun events onto the declared ladder so the operator always sees
 * the full list of steps, reached or not.
 */

import type { CheckRun, RunState } from './runState';
import { taskKey } from './runState';

// ── Types ──────────────────────────────────────────────────────────────────────

export interface Rung {
  index: number | null;
  label: string;
  state: 'passed' | 'failed' | 'running' | 'pending';
  /** The step's result so far; null while it has not been reached. */
  check: CheckRun | null;
}

// ── buildRungs ─────────────────────────────────────────────────────────────────

/**
 * Fold a task's CheckRun array onto its declared verify ladder.
 *
 * When `declared` is provided:
 *   - One rung per declared step i, labelled by `declared[i].phase` (or
 *     `verify[i]` when phase is empty), taking the state of the check with
 *     index === i, or 'pending' when no such check exists yet.
 *   - Checks whose index is null or falls beyond the declared list are
 *     appended after the declared rungs, in their original order.
 *
 * When `declared` is null:
 *   - One rung per check, in the order they appear in `checks`.
 */
export function buildRungs(
  checks: readonly CheckRun[],
  declared: readonly { phase: string }[] | null,
): Rung[] {
  if (declared === null) {
    return checks.map((c) => ({
      index: c.index,
      label: labelFor(c.phase, c.index, c.name),
      state: c.status,
      check: c,
    }));
  }

  // Partition checks: those that map onto a declared slot vs. extras.
  const byIndex = new Map<number, CheckRun>();
  const extras: CheckRun[] = [];

  for (const c of checks) {
    if (c.index !== null && c.index < declared.length) {
      byIndex.set(c.index, c);
    } else {
      extras.push(c);
    }
  }

  const rungs: Rung[] = [];

  // One rung per declared step.
  for (let i = 0; i < declared.length; i++) {
    const decl = declared[i]!;
    const check = byIndex.get(i) ?? null;
    rungs.push({
      index: i,
      label: decl.phase || `verify[${i}]`,
      state: check ? check.status : 'pending',
      check,
    });
  }

  // Append checks beyond the declared list or without an index, in order.
  for (const c of extras) {
    rungs.push({
      index: c.index,
      label: labelFor(c.phase, c.index, c.name),
      state: c.status,
      check: c,
    });
  }

  return rungs;
}

/** Derive a display label for a check rung. */
function labelFor(phase: string, index: number | null, name: string): string {
  if (phase) return phase;
  if (index !== null) return `verify[${index}]`;
  return name;
}

// ── checkFocusTask ─────────────────────────────────────────────────────────────

/**
 * Resolve which task the CHECKS cell should focus on.
 *
 * Priority order:
 *  1. The explicitly selected task, when `run.tasks` knows it.
 *  2. The first active task of the selected plan (sorted by taskId).
 *  3. The first active task across running plans, in `runningPlanIds` order
 *     then by taskId.
 *  4. null — nothing to show.
 */
export function checkFocusTask(
  run: RunState,
  selected: { planId: string | null; taskId: string | null },
  runningPlanIds: readonly string[],
): { planId: string; taskId: string } | null {
  // 1. Selected task known in run.tasks.
  if (selected.planId !== null && selected.taskId !== null) {
    const key = taskKey(selected.planId, selected.taskId);
    if (run.tasks[key]) {
      return { planId: selected.planId, taskId: selected.taskId };
    }
  }

  // 2. First active task of the selected plan.
  if (selected.planId !== null) {
    const active = activeTasksForPlan(run, selected.planId);
    if (active.length > 0) {
      const t = active[0]!;
      return { planId: t.planId, taskId: t.taskId };
    }
  }

  // 3. First active task across running plans, in runningPlanIds order then taskId.
  for (const planId of runningPlanIds) {
    const active = activeTasksForPlan(run, planId);
    if (active.length > 0) {
      const t = active[0]!;
      return { planId: t.planId, taskId: t.taskId };
    }
  }

  // 4. Nothing to show.
  return null;
}

/** Return active tasks for a given plan, sorted by taskId. */
function activeTasksForPlan(
  run: RunState,
  planId: string,
): Array<{ planId: string; taskId: string }> {
  return Object.values(run.tasks)
    .filter((t) => t.planId === planId && t.status === 'active')
    .sort((a, b) => a.taskId.localeCompare(b.taskId));
}
