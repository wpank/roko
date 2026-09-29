// @vitest-environment jsdom
/**
 * Acceptance: the stream pane's empty sentence tells the truth about a queued
 * plan and never prints a placeholder duration.
 * Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanTasks } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { StreamPane } from '@/components/stream/StreamPane';
import { describeEmpty } from '@/lib/emptyState';
import type { EmptyStateInput } from '@/lib/emptyState';
import { initialRunState } from '@/lib/runState';
import { foldEvents, renderWithClient, setStore, textOf } from '@/test/dom';

const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 2,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: 'Scaffold', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['compile'] },
    { id: 'T02', title: 'Print', tier: 'focused', status: 'pending', depends_on: ['T01'], files: [], completed: false, verify_phases: ['test'] },
  ],
};

const SET: WireDashboardEvent[] = [
  {
    type: 'plan_set_loaded',
    plans: [
      { plan_id: 'a', tasks_total: 1, wave: 0, depends_on: [], conflicts_with: [] },
      { plan_id: 'hello', tasks_total: 2, wave: 1, depends_on: ['a'], conflicts_with: [] },
    ],
  },
  { type: 'plan_started', plan_id: 'a', tasks_total: 1 },
];

const PLAN: NonNullable<EmptyStateInput['plan']> = {
  id: 'hello',
  phase: 'pending',
  tasksTotal: 2,
  tasksDone: 0,
  tasksActive: 0,
  tasksAccepted: 0,
};

const input = (plan: Partial<NonNullable<EmptyStateInput['plan']>>): EmptyStateInput => ({
  workspace: 'hello',
  connection: 'connected',
  planCount: 2,
  plan: { ...PLAN, ...plan },
});

beforeEach(() => setStore(initialRunState()));
afterEach(() => cleanup());

describe('describeEmpty', () => {
  it('says what a queued plan waits for', () => {
    expect(describeEmpty(input({ phase: 'pending', waitReason: 'after a' }))).toBe('Queued — after a.');
  });

  it('says a queued plan waits to start when the reason is unknown', () => {
    expect(describeEmpty(input({ phase: 'pending' }))).toBe('Queued — waiting to start.');
  });

  it('leaves out a duration it does not know', () => {
    expect(describeEmpty(input({ phase: 'completed', tasksTotal: 3, tasksDone: 3 }))).toBe(
      'Finished — 3 of 3 verified.',
    );
  });

  it('keeps a duration it knows', () => {
    expect(
      describeEmpty(input({ phase: 'completed', tasksTotal: 3, tasksDone: 3, durationMs: 252_000 })),
    ).toBe('Finished in 4m12s — 3 of 3 verified.');
  });

  it('still reports a cancelled plan', () => {
    expect(describeEmpty(input({ phase: 'cancelled' }))).toBe('Plan was cancelled.');
  });
});

describe('StreamPane', () => {
  const seed = [
    [queryKeys.planTasks('hello'), TASKS],
    [queryKeys.workspace, { name: 'hello', path: '/tmp/hello', branch: null }],
  ] as const;

  it('explains why a queued plan has no output yet', () => {
    setStore(foldEvents(SET));
    renderWithClient(<StreamPane planId="hello" selectedTaskId={null} open onToggle={() => {}} />, { seed });
    expect(textOf(document.querySelector('[data-region="stream"]'))).toContain('Queued — after a.');
  });

  it('calls a member that never started in a finished run ready, not cancelled', () => {
    setStore(foldEvents([...SET, { type: 'run_completed', outcome: 'failed', duration_ms: 500 }]));
    renderWithClient(<StreamPane planId="hello" selectedTaskId={null} open onToggle={() => {}} />, { seed });
    const text = textOf(document.querySelector('[data-region="stream"]'));
    expect(text).toContain('Ready — 2 tasks');
    expect(text).not.toContain('cancelled');
  });
});
