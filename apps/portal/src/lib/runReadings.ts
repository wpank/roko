/**
 * A plan run's TIME and SPENT, one definition for every reading of the run on the page: the run card's glass, the
 * top bar's clock, the plan's row in the list and the header's "started" (gap-c1726f, bug-b6d5cb).
 */

import type { RunState } from '@/lib/runState';

/** When roko accepted each run, by plan id: the plans' `last_run.started_at`, which only roko serve knows. */
export type RunStarts = Readonly<Record<string, number>>;

export interface RunClock {
  /** When the clock started: when roko accepted the run, else when the plan's tasks began. */
  startedAtMs: number;
  /** The run's time so far or, once it has ended, its whole time. */
  ms: number;
  /** Whether the plan's run has ended. */
  ended: boolean;
}

/**
 * TIME: from when roko accepted the run (`acceptedAtMs`, else when the plan's tasks began) to its end. Once ended,
 * it is the run's own duration as roko counted it (`run_completed.duration_ms`) when the run held this plan alone.
 * Null when the plan hasn't started.
 */
export function runClock(run: RunState, planId: string, acceptedAtMs: number | null, nowMs: number): RunClock | null {
  const live = run.plans[planId];
  if (!live || live.startedAtMs === null) return null;
  const startedAtMs = acceptedAtMs ?? live.startedAtMs;
  if (live.finishedAtMs === null) return { startedAtMs, ms: nowMs - startedAtMs, ended: false };
  const aloneInRun = run.planSet
    ? run.planSet.planIds.length === 1 && run.planSet.planIds[0] === planId
    : run.run.startedAtMs !== null && run.run.startedAtMs === live.startedAtMs;
  const ms = aloneInRun && run.run.durationMs !== null ? run.run.durationMs : live.finishedAtMs - startedAtMs;
  return { startedAtMs, ms, ended: true };
}

/**
 * SPENT: what the plan's run has cost so far, each call priced as it returns: its settled tries, its running tries'
 * calls so far and, on a resume, what the run's earlier processes spent.
 */
export function runSpentUsd(run: RunState, planId: string): number {
  const live = run.plans[planId];
  if (!live) return 0;
  const liveCalls = Object.values(run.tasks).reduce(
    (sum, task) => sum + (task.planId === planId ? (task.liveCall?.usd ?? 0) : 0),
    0,
  );
  return live.costUsd + liveCalls + (live.carriedUsd ?? 0);
}
