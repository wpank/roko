/**
 * The status chip in the top bar: the one most urgent thing in the workspace (the design system's §4.9), first
 * match wins. A click opens the plan it is about.
 *
 * 1. roko serve isn't answering, or this page isn't signed in: a hollow light and `off`.
 * 2. A task waits for the person's review: an orange light and `1 waiting`.
 * 3. A run failed and the person hasn't opened its plan since: a red light and `1 failed`.
 * 4. Plans run: a blue light, the newest run's time on glass, and `1 running`.
 * 5. Nothing is going on: a hollow light and `idle`.
 */

import type { Signal } from '@/components/kit/Led';
import type { ConnectionStatus } from '@/lib/bootstrap';
import type { RunState } from '@/lib/runState';
import { clock, clockLabel } from '@/lib/words';

export interface Chip {
  signal: Signal;
  word: string;
  /** A reading for the chip's small glass, or null. */
  reading: { text: string; label: string } | null;
  /** The plan a click opens, or null when there is none. */
  planId: string | null;
  /** Whether plans run now. */
  running: boolean;
}

/** The phase of a task whose verified try waits for a review (roko-cli's AWAITING_APPROVAL_PHASE). */
const AWAITING_APPROVAL = 'awaiting_approval';

/**
 * Failed runs the person has opened since they failed: plan id → the failure's `finishedAtMs`. A plan that
 * fails again later counts as unopened.
 */
export type SeenFailures = Readonly<Record<string, number | null>>;

export function statusChip(
  run: RunState,
  connection: ConnectionStatus,
  opts: { nowMs: number; seenFailures: SeenFailures },
): Chip {
  const idle = { reading: null, planId: null, running: false };
  if (connection === 'unauthorized') return { signal: 'off', word: 'signed out', ...idle };
  if (connection === 'disconnected' || connection === 'error') return { signal: 'off', word: 'off', ...idle };

  const plans = Object.values(run.plans);
  const running = plans
    .filter((p) => p.phase === 'running')
    .sort((a, b) => (b.startedAtMs ?? 0) - (a.startedAtMs ?? 0));

  const waiting = Object.values(run.tasks).filter((t) => t.phase === AWAITING_APPROVAL);
  if (waiting.length > 0) {
    return {
      signal: 'you',
      word: `${waiting.length} waiting`,
      reading: null,
      planId: waiting[0]!.planId,
      running: running.length > 0,
    };
  }

  const failed = plans.filter(
    (p) => p.phase === 'failed' && !(p.planId in opts.seenFailures && opts.seenFailures[p.planId] === p.finishedAtMs),
  );
  if (failed.length > 0) {
    return {
      signal: 'fail',
      word: `${failed.length} failed`,
      reading: null,
      planId: failed[0]!.planId,
      running: running.length > 0,
    };
  }

  const newest = running[0];
  if (newest) {
    const elapsed = newest.startedAtMs !== null ? opts.nowMs - newest.startedAtMs : null;
    return {
      signal: 'live',
      word: `${running.length} running`,
      reading: elapsed !== null ? { text: clock(elapsed), label: clockLabel(elapsed) } : null,
      planId: newest.planId,
      running: true,
    };
  }

  if (connection === 'connecting') return { signal: 'off', word: 'connecting', ...idle };
  return { signal: 'off', word: 'idle', ...idle };
}
