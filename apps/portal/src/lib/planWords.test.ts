/**
 * A plan's row in the plan list says its state in roko's words: a light, a word, one line, a square per task.
 */
import { describe, expect, it } from 'vitest';
import type { WireDashboardEvent, WirePlanSummary } from '@/api/contracts';
import { buildPlanRows } from '@/lib/planRows';
import { planWords } from '@/lib/planWords';
import { initialRunState } from '@/lib/runState';
import { foldEvents } from '@/test/dom';

const T0 = 1_750_000_000_000;

const plan = (extra: Partial<WirePlanSummary> = {}): WirePlanSummary => ({
  id: 'greeting',
  title: 'Say hello from the command line',
  task_count: 3,
  tasks_done: 0,
  tasks_failed: 0,
  completed: false,
  status: 'pending',
  old_format: false,
  ...extra,
});

const START: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'greeting', title: 'Say hello', tasks_total: 3 }] },
  { type: 'plan_started', plan_id: 'greeting', tasks_total: 3 },
  { type: 'task_started', plan_id: 'greeting', task_id: 'T1', phase: 'implement' },
  { type: 'task_completed', plan_id: 'greeting', task_id: 'T1', outcome: 'passed' },
  { type: 'task_started', plan_id: 'greeting', task_id: 'T2', phase: 'implement' },
];

function row(events: WireDashboardEvent[] | null, nowMs = T0, summary = plan()) {
  const run = events ? foldEvents(events, T0, 0) : initialRunState();
  return buildPlanRows([summary], run, { filter: '', nowMs }).groups[0]!.rows[0]!;
}

describe('a plan that has not run', () => {
  it('is ready, with its task count and a waiting square per task', () => {
    const words = planWords(row(null));
    expect(words).toMatchObject({ signal: 'waiting', word: 'ready', rest: '3 tasks' });
    expect(words.squares).toEqual(['waiting', 'waiting', 'waiting']);
  });
});

describe('a running plan', () => {
  it('is running, with what is done and its clock', () => {
    const words = planWords(row(START, T0 + 9_000));
    expect(words.signal).toBe('live');
    expect(words.word).toBe('running');
    expect(words.rest).toBe('1 of 3 · 0:09');
    expect(words.squares).toEqual(['pass', 'live', 'waiting']);
  });
});

describe('a finished plan', () => {
  const done: WireDashboardEvent[] = [
    ...START,
    { type: 'task_completed', plan_id: 'greeting', task_id: 'T2', outcome: 'passed' },
    { type: 'task_started', plan_id: 'greeting', task_id: 'T3', phase: 'implement' },
    { type: 'task_completed', plan_id: 'greeting', task_id: 'T3', outcome: 'passed' },
    { type: 'plan_completed', plan_id: 'greeting', success: true },
  ];

  it('passed: says how long it took and what it spent', () => {
    const words = planWords(row(done), 0.4363);
    expect(words).toMatchObject({ signal: 'pass', word: 'passed' });
    expect(words.rest).toMatch(/^\d+(\.\d)?s · \$0\.44$/);
    expect(words.squares).toEqual(['pass', 'pass', 'pass']);
  });

  it('failed: says so in red', () => {
    const failed: WireDashboardEvent[] = [
      ...START,
      { type: 'task_completed', plan_id: 'greeting', task_id: 'T2', outcome: 'failed' },
      { type: 'plan_completed', plan_id: 'greeting', success: false },
    ];
    const words = planWords(row(failed), 0.3);
    expect(words).toMatchObject({ signal: 'fail', word: 'failed' });
    expect(words.rest).toContain('$0.30');
    expect(words.squares.filter((s) => s === 'fail')).toHaveLength(1);
  });
});

describe('a replaced plan', () => {
  it('names the plan that replaced it', () => {
    const words = planWords(row(null, T0, plan({ status: 'superseded', superseded_by: 'greeting-2' })));
    expect(words).toMatchObject({ word: 'replaced', rest: 'by greeting-2' });
  });
});
