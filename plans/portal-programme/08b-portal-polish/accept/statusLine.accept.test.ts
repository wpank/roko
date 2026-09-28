/**
 * Acceptance: the plan status line lists only the fields it knows — no
 * placeholder dots. Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { describe, expect, it } from 'vitest';
import { statusFields } from '@/lib/statusLine';
import type { StatusLineInput } from '@/lib/statusLine';

const BASE: StatusLineInput = {
  hasRun: false,
  running: false,
  taskCount: 3,
  waveCount: 3,
  estimatedMinutes: 9,
  parallel: 1,
  tasksDone: 0,
  tasksTotal: 3,
  elapsedMs: null,
  etaMinutes: null,
  costUsd: 0,
  busyAgents: 0,
  maxParallel: null,
};

const texts = (input: Partial<StatusLineInput>) =>
  statusFields({ ...BASE, ...input }).map((f) => f.text);

describe('statusFields before a run', () => {
  it('lists tasks, waves, estimate and parallelism', () => {
    const fields = statusFields(BASE);
    expect(fields.map((f) => f.key)).toEqual(['tasks', 'waves', 'estimate', 'parallel']);
    expect(fields.map((f) => f.text)).toEqual(['3 tasks', '3 waves', '~9 min', 'parallel 1']);
  });

  it('uses the singular for one task and one wave', () => {
    expect(texts({ taskCount: 1, waveCount: 1 })).toEqual(['1 task', '1 wave', '~9 min', 'parallel 1']);
  });

  it('omits what it does not know', () => {
    expect(texts({ taskCount: 2, waveCount: 0, estimatedMinutes: null, parallel: null })).toEqual([
      '2 tasks',
    ]);
  });

  it('omits a zero estimate', () => {
    expect(texts({ estimatedMinutes: 0 })).toEqual(['3 tasks', '3 waves', 'parallel 1']);
  });
});

describe('statusFields once run', () => {
  const RUNNING: Partial<StatusLineInput> = {
    hasRun: true,
    running: true,
    tasksDone: 2,
    tasksTotal: 3,
    elapsedMs: 72_000,
    etaMinutes: 2,
    costUsd: 0.21,
    busyAgents: 1,
    maxParallel: 1,
  };

  it('lists progress, elapsed, estimate, cost and agents while running', () => {
    const fields = statusFields({ ...BASE, ...RUNNING });
    expect(fields.map((f) => f.key)).toEqual(['progress', 'elapsed', 'eta', 'cost', 'agents']);
    expect(fields.map((f) => f.text)).toEqual(['2/3', '1m12s', '~2m', '$0.21', 'agents 1/1']);
  });

  it('drops the estimate and the agents once finished', () => {
    expect(
      texts({ ...RUNNING, running: false, busyAgents: 0, tasksDone: 3, elapsedMs: 16_700, costUsd: 0 }),
    ).toEqual(['3/3', '16s']);
  });

  it('omits elapsed when the start is unknown', () => {
    expect(texts({ ...RUNNING, elapsedMs: null })).toEqual(['2/3', '~2m', '$0.21', 'agents 1/1']);
  });

  it('omits a zero cost', () => {
    expect(texts({ ...RUNNING, costUsd: 0 })).toEqual(['2/3', '1m12s', '~2m', 'agents 1/1']);
  });

  it('shows the agent count alone when the limit is unknown', () => {
    expect(texts({ ...RUNNING, maxParallel: null, busyAgents: 2 })).toContain('agents 2');
  });

  it('shows a zero elapsed time', () => {
    expect(texts({ ...RUNNING, elapsedMs: 0 })).toContain('0s');
  });
});

describe('statusFields never shows a placeholder', () => {
  it('has no empty, dot-only or missing-value field for any combination', () => {
    const options: Partial<StatusLineInput>[] = [];
    for (const hasRun of [false, true])
      for (const running of [false, true])
        for (const known of [false, true])
          options.push({
            hasRun,
            running,
            waveCount: known ? 2 : 0,
            estimatedMinutes: known ? 5 : null,
            parallel: known ? 2 : null,
            elapsedMs: known ? 1_000 : null,
            etaMinutes: known ? 1 : null,
            costUsd: known ? 0.5 : 0,
            busyAgents: known ? 1 : 0,
            maxParallel: known ? 2 : null,
          });
    for (const option of options) {
      for (const text of texts(option)) {
        expect(text.trim()).not.toBe('');
        expect(text.trim()).not.toBe('·');
        expect(text).not.toMatch(/undefined|NaN|null/);
      }
    }
  });
});
