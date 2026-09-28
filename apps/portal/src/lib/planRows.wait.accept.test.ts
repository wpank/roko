/**
 * Acceptance: rail rows say why a queued plan waits, and never show a missing
 * count. Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { describe, expect, it } from 'vitest';
import type { WireDashboardEvent, WirePlanSetEntry, WirePlanSummary } from '@/api/contracts';
import { buildPlanRows } from '@/lib/planRows';
import type { PlanRowModel } from '@/lib/planRows';
import { applyEvent, initialRunState } from '@/lib/runState';
import type { RunState } from '@/lib/runState';

const NOW = 10_000;

function disk(id: string, extra: Partial<WirePlanSummary> = {}): WirePlanSummary {
  return {
    id,
    title: `Plan ${id}`,
    task_count: 2,
    tasks_done: 0,
    tasks_failed: 0,
    completed: false,
    status: 'pending',
    old_format: false,
    ...extra,
  };
}

function fold(events: WireDashboardEvent[]): RunState {
  return events.reduce((run, e, i) => applyEvent(run, e, 1_000 + i), initialRunState());
}

function row(rows: ReturnType<typeof buildPlanRows>, id: string): PlanRowModel {
  const found = rows.groups.flatMap((g) => g.rows).find((r) => r.id === id);
  if (!found) throw new Error(`no row ${id}`);
  return found;
}

const SET: WirePlanSetEntry[] = [
  { plan_id: '01-a', tasks_total: 2, wave: 0, depends_on: [], conflicts_with: [] },
  { plan_id: '02-b', tasks_total: 2, wave: 1, depends_on: ['01-a'], conflicts_with: [] },
  { plan_id: '03-c', tasks_total: 2, wave: 0, depends_on: [], conflicts_with: [] },
];
const DISKS = [disk('01-a'), disk('02-b'), disk('03-c')];

describe('queued rows', () => {
  it('say which prerequisite they wait for', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
    ]);
    const r = row(buildPlanRows(DISKS, run, { filter: '', nowMs: NOW }), '02-b');
    expect(r.state).toBe('queued');
    expect(r.queuePosition).toBe(2);
    expect(r.waitReason).toBe('after 01-a');
  });

  it('say they wait for a free slot when nothing blocks them', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
    ]);
    const r = row(buildPlanRows(DISKS, run, { filter: '', nowMs: NOW }), '03-c');
    expect(r.queuePosition).toBe(3);
    expect(r.waitReason).toBe('waiting for a free slot');
  });

  it('say when a failed prerequisite blocks them', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
      { type: 'plan_completed', plan_id: '01-a', success: false },
    ]);
    const r = row(buildPlanRows(DISKS, run, { filter: '', nowMs: NOW }), '02-b');
    expect(r.state).toBe('queued');
    expect(r.waitReason).toBe('blocked: 01-a failed');
  });

  it('have no reason once running or finished', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
      { type: 'plan_started', plan_id: '03-c', tasks_total: 2 },
      { type: 'plan_completed', plan_id: '03-c', success: true },
    ]);
    const rows = buildPlanRows(DISKS, run, { filter: '', nowMs: NOW });
    expect(row(rows, '01-a').waitReason).toBeNull();
    expect(row(rows, '03-c').waitReason).toBeNull();
  });

  it('lose position and reason once the run reports an outcome', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: SET },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
      { type: 'run_completed', outcome: 'cancelled', duration_ms: 500 },
    ]);
    const r = row(buildPlanRows(DISKS, run, { filter: '', nowMs: NOW }), '03-c');
    expect(r.state).toBe('pending');
    expect(r.queuePosition).toBeNull();
    expect(r.waitReason).toBeNull();
  });

  it('are not queued once every member has finished, though the server keeps the set', () => {
    const run = fold([
      { type: 'plan_set_loaded', plans: [SET[0]!] },
      { type: 'plan_started', plan_id: '01-a', tasks_total: 2 },
      { type: 'plan_completed', plan_id: '01-a', success: true },
    ]);
    const rows = buildPlanRows(DISKS, run, { filter: '', nowMs: NOW });
    expect(row(rows, '01-a').state).toBe('done');
    expect(rows.groups.flatMap((g) => g.rows).some((r) => r.state === 'queued')).toBe(false);
  });
});

describe('counts when the plan list omits tasks_done', () => {
  const partial = (extra: Partial<WirePlanSummary>) => {
    const d = disk('x', extra) as Partial<WirePlanSummary>;
    delete d.tasks_done;
    return d as WirePlanSummary;
  };

  it('count nothing done for an unfinished plan', () => {
    const r = row(buildPlanRows([partial({})], initialRunState(), { filter: '', nowMs: NOW }), 'x');
    expect(r.done).toBe(0);
    expect(r.total).toBe(2);
    expect(Number.isNaN(r.fraction)).toBe(false);
  });

  it('count every task done for a completed plan', () => {
    const r = row(
      buildPlanRows([partial({ completed: true, status: 'completed' })], initialRunState(), {
        filter: '',
        nowMs: NOW,
      }),
      'x',
    );
    expect(r.done).toBe(2);
    expect(r.fraction).toBe(1);
  });
});
