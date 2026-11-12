/**
 * A plan in the plan list, in roko's words: its signal, the word for its state, its one status line, and a square
 * per task for its strip.
 */

import type { Signal } from '@/components/kit/Led';
import type { PlanRowModel } from '@/lib/planRows';
import { clock, count, duration, money } from '@/lib/words';

export interface PlanWords {
  signal: Signal;
  /** The state's word: ready, running, passed, failed, … */
  word: string;
  /** What follows the word on the status line, or '' when nothing does. */
  rest: string;
  /** One square per task: passed, running, failed, yours, or waiting. */
  squares: Signal[];
}

/** The signal and word for each state a plan row can be in. */
const STATE_WORDS: Record<PlanRowModel['state'], { signal: Signal; word: string }> = {
  pending: { signal: 'waiting', word: 'ready' },
  queued: { signal: 'waiting', word: 'queued' },
  active: { signal: 'live', word: 'running' },
  unverified: { signal: 'live', word: 'running' },
  done: { signal: 'pass', word: 'passed' },
  preexisting: { signal: 'pass', word: 'passed' },
  satisfied: { signal: 'pass', word: 'passed' },
  accepted: { signal: 'still', word: 'not checked' },
  unchecked: { signal: 'still', word: 'not checked' },
  marked: { signal: 'still', word: 'marked done' },
  failed: { signal: 'fail', word: 'failed' },
  interrupted: { signal: 'you', word: 'interrupted' },
  skipped: { signal: 'still', word: 'stopped' },
};

/** The plan's tasks as squares, in the order passed, yours, failed, running, waiting. */
export function planSquares(row: PlanRowModel): Signal[] {
  const squares: Signal[] = [];
  const signalOf = { done: 'pass', accepted: 'you', failed: 'fail', active: 'live' } as const;
  for (const segment of row.segments) {
    const n = Math.round(segment.share * row.total);
    for (let i = 0; i < n && squares.length < row.total; i += 1) squares.push(signalOf[segment.state]);
  }
  while (squares.length < row.total) squares.push('waiting');
  return squares;
}

/** An estimate in whole minutes, rounded up: `about 9 min`, `about 1h 35m`. */
function about(ms: number): string {
  const minutes = Math.max(1, Math.ceil(ms / 60_000));
  return minutes < 60 ? `about ${minutes} min` : `about ${duration(minutes * 60_000)}`;
}

/** `row` in words, with `costUsd`, what its latest run has spent. */
export function planWords(row: PlanRowModel, costUsd = 0): PlanWords {
  const { signal, word } = STATE_WORDS[row.state];
  const tasks = count(row.total, 'task');
  const progress = `${row.done} of ${row.total}`;
  const spent = costUsd > 0 ? money(costUsd) : null;
  const took = row.time.ms !== null && row.time.kind === 'actual' ? duration(row.time.ms) : null;
  let rest: string[];
  switch (row.state) {
    case 'pending':
      rest = row.done > 0 ? [`${progress} done`] : [tasks];
      if (row.time.kind === 'estimate' && row.time.ms !== null) rest.push(about(row.time.ms));
      break;
    case 'queued':
      rest = [row.waitReason ?? (row.queuePosition !== null ? `#${row.queuePosition}` : tasks)];
      break;
    case 'active':
    case 'unverified':
      rest = [progress, row.time.ms !== null && row.time.kind === 'elapsed' ? clock(row.time.ms) : null].filter(
        (part): part is string => part !== null,
      );
      break;
    case 'skipped':
      // A superseded plan is skipped too: it was replaced, not stopped.
      if (row.supersededBy !== null) {
        return { signal, word: 'replaced', rest: `by ${row.supersededBy}`, squares: planSquares(row) };
      }
      rest = [progress];
      break;
    case 'interrupted':
      rest = [progress];
      break;
    case 'marked':
      rest = [];
      break;
    default:
      rest = [took, spent].filter((part): part is string => part !== null);
  }
  return { signal, word, rest: rest.join(' · '), squares: planSquares(row) };
}
