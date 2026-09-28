import type { RunState } from '@/lib/runState';

/**
 * True only while the plan-set exists, the run has not yet completed
 * (outcome is null), and at least one member plan is still pending or running.
 *
 * The server keeps `planSet` in its snapshot after a run ends, so
 * `run.planSet !== null` alone does not mean a run is active.
 * A member with no live record in `run.plans` does not keep the set active.
 */
export function planSetActive(run: RunState): boolean {
  if (run.planSet === null) return false;
  if (run.run.outcome !== null) return false;

  return run.planSet.planIds.some((id) => {
    const plan = run.plans[id];
    if (!plan) return false; // no live record — does not keep the set active
    return plan.phase === 'pending' || plan.phase === 'running';
  });
}

/**
 * Returns the 1-based position of `planId` in the active plan-set queue,
 * or null if:
 *  - the set is inactive,
 *  - the plan is not a member,
 *  - or the plan is not pending (e.g. already running or completed).
 */
export function queuePosition(run: RunState, planId: string): number | null {
  if (!planSetActive(run)) return null;
  if (!run.planSet) return null;

  const idx = run.planSet.planIds.indexOf(planId);
  if (idx === -1) return null;

  const plan = run.plans[planId];
  if (!plan || plan.phase !== 'pending') return null;

  return idx + 1; // 1-based
}

/**
 * Returns a human-readable explanation for why `planId` is waiting in the
 * queue, or null if the plan is not queued (not pending in an active set,
 * not a member, or the set is inactive).
 *
 * Priority order (uses `run.planSet.members?.[planId]`):
 *  1. A dependency that failed or was cancelled → "blocked: <id> failed/cancelled"
 *  2. Dependencies not yet completed → "after <id>, <id>" (depends_on order)
 *  3. Conflicting plan that is running → "after <id>, <id> (shared files)"
 *  4. Fallback → "waiting for a free slot"
 */
export function waitReason(run: RunState, planId: string): string | null {
  if (queuePosition(run, planId) === null) return null;
  if (!run.planSet) return null;

  const member = run.planSet.members?.[planId];

  if (!member) {
    // No scheduling facts from the server
    return 'waiting for a free slot';
  }

  const { dependsOn, conflictsWith } = member;

  // 1. Check for failed/cancelled dependencies (in depends_on order)
  for (const depId of dependsOn) {
    const dep = run.plans[depId];
    if (dep?.phase === 'failed') return `blocked: ${depId} failed`;
    if (dep?.phase === 'cancelled') return `blocked: ${depId} cancelled`;
  }

  // 2. Check for unfinished dependencies (in depends_on order)
  const unfinishedDeps = dependsOn.filter((depId) => {
    const dep = run.plans[depId];
    return !dep || dep.phase !== 'completed';
  });

  if (unfinishedDeps.length > 0) {
    return `after ${unfinishedDeps.join(', ')}`;
  }

  // 3. Check for running conflicting plans
  const runningConflicts = conflictsWith.filter((conflictId) => {
    const conflict = run.plans[conflictId];
    return conflict?.phase === 'running';
  });

  if (runningConflicts.length > 0) {
    return `after ${runningConflicts.join(', ')} (shared files)`;
  }

  // 4. Default — no specific reason why this slot is blocked
  return 'waiting for a free slot';
}
