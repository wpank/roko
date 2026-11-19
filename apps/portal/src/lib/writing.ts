/**
 * A plan being written, in roko's words (gap-f8c57a): its glass, what a stopped write says, its row in the plan
 * list, and the events that change what the plan list shows.
 */

import type { WireDashboardEvent, WirePlanSummary, WirePlanWriting } from '@/api/contracts';
import type { Reading } from '@/lib/runCard';
import { clock, clockLabel, money } from '@/lib/words';

/**
 * The writing view's glass: TIME, and SPENT against the write's cap, as a run's glass shows them. `capUsd` is null
 * when nothing caps the write, and undefined while roko serve hasn't said.
 */
export function writeReadings(
  elapsedMs: number,
  costUsd: number,
  capUsd: number | null | undefined,
): { time: Reading; spent: Reading } {
  const share = capUsd && capUsd > 0 ? costUsd / capUsd : 0;
  return {
    time: { text: clock(elapsedMs), label: clockLabel(elapsedMs), tone: 'live', note: 'usually under a minute' },
    spent: {
      text: money(costUsd),
      label: capUsd ? `${money(costUsd)} of ${money(capUsd)}` : `${money(costUsd)} spent`,
      tone: 'live',
      note: capUsd ? `of ${money(capUsd)}` : capUsd === null ? 'no cap set' : '',
      meter: { lit: Math.min(20, Math.ceil(share * 20 - 1e-9)), tone: share >= 0.8 ? 'you' : 'live' },
    },
  };
}

/** What a stopped write says, with what its settled planner calls cost: "Stopped. Writing the plan spent $0.04." */
export function stoppedSentence(costUsd: number | null): string {
  return costUsd === null ? 'Stopped before roko wrote the plan.' : `Stopped. Writing the plan spent ${money(costUsd)}.`;
}

/** The plan list's rows for plans being written, oldest first, apart from the plans on disk. */
export function splitWrites(listed: readonly WirePlanSummary[] | undefined): {
  plans: WirePlanSummary[] | undefined;
  writes: (WirePlanSummary & { writing: WirePlanWriting })[];
} {
  if (!listed) return { plans: undefined, writes: [] };
  const writes = listed.filter((row): row is WirePlanSummary & { writing: WirePlanWriting } => row.writing != null);
  return { plans: listed.filter((row) => row.writing == null), writes };
}

/** A writing row's status line, "writing the plan · $0.04": its word, then what the write has spent. */
export function writingWords(writing: WirePlanWriting): { word: string; rest: string } {
  return { word: 'writing the plan', rest: money(writing.cost_usd ?? 0) };
}

/** Whether `event` says a plan write started or ended, which changes the plan list's rows. */
export function isWriteEvent(event: WireDashboardEvent): boolean {
  return event.type === 'event_log_entry' && /^plan_(generate|revise)\./.test(event.event_type);
}
