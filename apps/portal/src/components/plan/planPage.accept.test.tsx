// @vitest-environment jsdom
/**
 * Acceptance for gap-60f6b6: a plan's page is roko's run card at full width. The glass holds the four readings, the
 * plan is drawn as steps of pads, one sentence (or a line per running task) says where it stands, and at most one
 * key is orange. A pad opens its task: its checks with their commands and each try with how it ended.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, screen } from '@testing-library/react';
import type { WireDashboardEvent } from '@/api/contracts';
import { initialRunState } from '@/lib/runState';
import { STREAM_RECORD_PREFIX } from '@/lib/streamRecord';
import { setStore, textOf } from '@/test/dom';
import { planSummary, planTasks, renderPlanPage } from '@/test/plan';

const T0 = 1_780_000_000_000;
const PLAN = planSummary('greeting', 'Say hello from the command line', 3);
const TASKS = planTasks('greeting', [
  ['T1', 'Write the greeting module', []],
  ['T2', 'Add the command-line entry point', ['T1']],
  ['T3', 'Document how to run it', ['T1']],
]);

const spawned = (task: string, attempt = 1): WireDashboardEvent => ({
  type: 'agent_spawned', agent_id: `greeting/${task}`, plan_id: 'greeting', task_id: task, attempt, role: 'implementer', model: 'glm-4.7',
});
const started = (task: string): WireDashboardEvent => ({ type: 'task_started', plan_id: 'greeting', task_id: task, phase: 'implement' });
const gate = (task: string, name: string, passed: boolean): WireDashboardEvent => ({ type: 'gate_result', plan_id: 'greeting', task_id: task, gate: name, passed });
const done = (task: string, outcome = 'passed'): WireDashboardEvent => ({ type: 'task_completed', plan_id: 'greeting', task_id: task, outcome });
const writing = (task: string, target: string): WireDashboardEvent => ({
  type: 'agent_output',
  agent_id: `greeting/${task}`,
  plan_id: 'greeting',
  task_id: task,
  content:
    STREAM_RECORD_PREFIX +
    JSON.stringify({ kind: 'tool_start', live: true, agent_id: `greeting/${task}`, plan_id: 'greeting', task_id: task, attempt: 0, payload: { tool_id: `t-${task}`, tool: 'write_file', target } }),
});
const settled = (task: string, attempt: number, verdict: string, extra: Partial<Extract<WireDashboardEvent, { type: 'attempt_settled' }>> = {}): WireDashboardEvent => ({
  type: 'attempt_settled', plan_id: 'greeting', task_id: task, attempt, verdict, model: 'glm-4.7', ...extra,
});

const T1_PASSED: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'greeting', tasks_total: 3 },
  started('T1'), spawned('T1'), gate('T1', 'verify[0:build]', true), gate('T1', 'rung[compile]', true),
  settled('T1', 1, 'passed', { cost_usd: 0.1007, duration_ms: 7_100 }), done('T1'),
];
const RUNNING: WireDashboardEvent[] = [
  ...T1_PASSED,
  started('T2'), spawned('T2'), writing('T2', 'cli.py'),
  started('T3'), spawned('T3'), gate('T3', 'verify[0:build]', false),
  settled('T3', 1, 'gate_failed', { failed_check: 'verify:0/build', cost_usd: 0.1342, duration_ms: 4_900 }),
  spawned('T3', 2),
];
const PASSED: WireDashboardEvent[] = [
  ...RUNNING,
  gate('T2', 'verify[0:build]', true), gate('T2', 'rung[compile]', true), settled('T2', 1, 'passed', { cost_usd: 0.1007 }), done('T2'),
  gate('T3', 'verify[0:build]', true), gate('T3', 'rung[compile]', true), settled('T3', 2, 'passed', { cost_usd: 0.1007, duration_ms: 6_100 }), done('T3'),
  { type: 'plan_check_finished', plan_id: 'greeting', passed: true, name: 'runs' },
  { type: 'plan_completed', plan_id: 'greeting', success: true },
];

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(T0 + 9_000);
  setStore(initialRunState());
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const plan = () => document.querySelector('[data-region="plan"]')!;
const reading = (name: string) => document.querySelector(`[data-reading="${name}"]`)!;
const readingLabel = (name: string) => reading(name).querySelector('svg')!.getAttribute('aria-label');
const orange = () => [...document.querySelectorAll('[data-region="status-line"] [data-you]')];

describe('before a run', () => {
  it('sits unlit on the glass, draws the plan as steps, and makes run plan the one orange key', () => {
    renderPlanPage(PLAN, TASKS, [], { startMs: T0 });
    expect(textOf(plan().querySelector('.rk-pill'))).toBe('ready');
    expect(readingLabel('tasks')).toBe('tasks: 0 of 3 tasks done');
    // Each task's own check and the project's compile check.
    expect(readingLabel('checks')).toBe('checks: 0 of 6 checks passed');
    expect(readingLabel('spent')).toBe('spent: $0.00 of $3.00');
    expect(textOf(reading('spent'))).toContain('of $3.00');

    const steps = [...document.querySelectorAll('[data-step-col]')].map((col) =>
      [...col.querySelectorAll('[data-pad]')].map((p) => p.getAttribute('data-pad')),
    );
    expect(steps).toEqual([['T1'], ['T2', 'T3']]);
    expect(textOf(document.querySelector('[data-pad="T2"]'))).toContain('02');
    expect(textOf(document.querySelector('[data-pad="T2"]'))).toContain('not run');

    expect(textOf(document.querySelector('[data-region="status-line"] .pt-sentence'))).toContain(
      'roko works on its own branch, so your files change only when you merge.',
    );
    expect(orange().map((k) => k.getAttribute('data-action'))).toEqual(['run']);
    expect(textOf(orange()[0]!)).toContain('run plan');
  });
});

describe('while it runs', () => {
  it('lights the readings and says what each running task does now, with stop as a plain key', () => {
    renderPlanPage(PLAN, TASKS, RUNNING, { startMs: T0 });
    expect(textOf(plan().querySelector('.rk-pill'))).toBe('running');
    expect(readingLabel('tasks')).toBe('tasks: 1 of 3 tasks done');
    expect(reading('time').querySelector('[data-tone="live"]')).not.toBeNull();

    const now = [...document.querySelectorAll('[data-now-task]')].map((li) => textOf(li));
    expect(now[0]).toContain('Task 2');
    expect(now[0]).toContain('writing cli.py · glm-4.7');
    expect(now[1]).toContain('Task 3');
    expect(now[1]).toContain('try 2, after the build check failed');

    expect(textOf(document.querySelector('[data-pad="T3"]'))).toContain('try 2');
    expect([...document.querySelectorAll('[data-pad="T3"] .rk-pips i')].map((i) => i.getAttribute('data-state'))).toEqual(['fail', 'live']);

    const stop = document.querySelector('[data-action="cancel"]')!;
    expect(textOf(stop)).toBe('stop');
    expect(orange()).toHaveLength(0);
  });
});

describe('after a run', () => {
  it('says every task passed, and keeps each task’s tries, model and spend in the details', () => {
    renderPlanPage(PLAN, TASKS, PASSED, { startMs: T0 });
    expect(textOf(plan().querySelector('.rk-pill'))).toBe('passed');
    expect(textOf(document.querySelector('[data-region="run-summary"]'))).toBe(
      'All 3 tasks passed their checks, and so did the plan’s own check.',
    );
    expect(readingLabel('checks')).toBe('checks: 6 of 6 checks passed');
    const rows = [...document.querySelectorAll('[data-fold="plan"] tbody tr')].map((tr) => textOf(tr));
    expect(rows[2]).toContain('03 Document how to run it');
    expect(rows[2]).toContain('glm-4.7');
    expect(rows[2]).toMatch(/^03 Document how to run it2glm-4\.7/);
    expect(textOf(document.querySelector('[data-fold="plan"]'))).toContain('plan checkpassed');
  });

  it('names the failed task, its tries and the check that failed it, and makes resume the orange key', () => {
    renderPlanPage(
      PLAN,
      TASKS,
      [
        { type: 'plan_started', plan_id: 'greeting', tasks_total: 3 },
        started('T1'), spawned('T1'), gate('T1', 'verify[0:build]', false),
        settled('T1', 1, 'gate_failed', { failed_check: 'verify:0/build' }), spawned('T1', 2), gate('T1', 'verify[0:build]', false),
        settled('T1', 2, 'gate_failed', { failed_check: 'verify:0/build' }), done('T1', 'failed'),
        { type: 'plan_completed', plan_id: 'greeting', success: false },
      ],
      { startMs: T0 },
    );
    expect(textOf(plan().querySelector('.rk-pill'))).toBe('failed');
    expect(textOf(document.querySelector('[data-region="run-summary"]'))).toBe(
      'Task 1 failed after 2 tries: the build check failed.',
    );
    expect(orange().map((k) => k.getAttribute('data-action'))).toEqual(['retry']);
    expect(textOf(orange()[0]!)).toContain('resume');
  });
});

describe('in a plan set', () => {
  const SET: WireDashboardEvent[] = [
    {
      type: 'plan_set_loaded',
      plans: [
        { plan_id: 'farewell', tasks_total: 1, wave: 0, depends_on: [], conflicts_with: [] },
        { plan_id: 'greeting', tasks_total: 3, wave: 1, depends_on: ['farewell'], conflicts_with: [] },
      ],
    },
    { type: 'plan_started', plan_id: 'farewell', tasks_total: 1 },
  ];

  it('says what a queued plan waits for', () => {
    renderPlanPage(PLAN, TASKS, SET, { startMs: T0 });
    expect(textOf(plan().querySelector('.rk-pill'))).toBe('queued');
    expect(textOf(document.querySelector('[data-region="run-summary"]'))).toBe('Queued: it starts after farewell.');
  });

  it('calls a plan that a finished set never started ready, not stopped', () => {
    renderPlanPage(PLAN, TASKS, [...SET, { type: 'run_completed', outcome: 'failed', duration_ms: 500 }], { startMs: T0 });
    expect(textOf(plan().querySelector('.rk-pill'))).toBe('ready');
    expect(textOf(document.querySelector('[data-region="run-summary"]'))).toContain('roko works on its own branch');
    expect(readingLabel('tasks')).toBe('tasks: 0 of 3 tasks done');
  });
});

describe('a task, opened', () => {
  it('lists its checks with their commands, and each try with its model, spend and ending', () => {
    renderPlanPage(PLAN, TASKS, PASSED, { startMs: T0, selected: 'T3' });
    const detail = document.querySelector('[data-region="task-detail"]')!;
    expect(textOf(detail.querySelector('h2'))).toBe('Document how to run it');
    expect(textOf(detail)).toContain('build · make t3');
    expect(textOf(detail)).toContain('compile · your project’s check, run on every task');
    const tries = [...detail.querySelectorAll('[data-try]')].map((li) => textOf(li));
    expect(tries[0]).toBe('✗Try 1 · glm-4.7 · $0.13 · 4.9s · the build check failed');
    expect(tries[1]).toBe('✓Try 2 · glm-4.7 · $0.10 · 6.1s · passed');
    expect(document.querySelector('[data-pad="T3"]')!.getAttribute('aria-pressed')).toBe('true');
  });

  it('says why a skipped task never ran, by the number of the task it needed', () => {
    renderPlanPage(
      PLAN,
      TASKS,
      [
        { type: 'plan_started', plan_id: 'greeting', tasks_total: 3 },
        started('T1'), spawned('T1'), gate('T1', 'verify[0:build]', false),
        settled('T1', 1, 'gate_failed', { failed_check: 'verify:0/build' }), done('T1', 'failed'),
        { type: 'task_blocked', plan_id: 'greeting', task_id: 'T2', blocked_by: 'T1', reason: 'T1 failed' },
        { type: 'plan_completed', plan_id: 'greeting', success: false },
      ],
      { startMs: T0, selected: 'T2' },
    );
    const detail = document.querySelector('[data-region="task-detail"]')!;
    expect(textOf(detail)).toContain('It didn’t run, because task 1, which it needs, didn’t pass.');
    expect(textOf(detail)).not.toContain('T1');
  });

  it('closes when its pad is clicked again', () => {
    const onSelectTask = vi.fn();
    renderPlanPage(PLAN, TASKS, PASSED, { startMs: T0, selected: 'T3', onSelectTask });
    fireEvent.click(document.querySelector('[data-pad="T3"]')!);
    expect(onSelectTask).toHaveBeenCalledWith(null);
    fireEvent.click(screen.getByRole('button', { name: 'Close this task' }));
    expect(onSelectTask).toHaveBeenLastCalledWith(null);
  });
});
