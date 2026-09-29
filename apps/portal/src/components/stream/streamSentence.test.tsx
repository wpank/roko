// @vitest-environment jsdom
/**
 * With no task to follow, the stream says what the system is doing (design §7;
 * gap-6ff814): a plan that never ran is Ready with its task and wave counts,
 * and with plans present but none selected it asks for one instead of saying
 * the workspace has no plans.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent } from '@testing-library/react';
import type { WirePlanSummary, WirePlanTasks } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Workspace } from '@/components/shell/Workspace';
import { StreamPane } from '@/components/stream/StreamPane';
import { initialRunState } from '@/lib/runState';
import { renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';

vi.mock('next/navigation', () => ({
  useSearchParams: () => new URLSearchParams(window.location.search),
}));

const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 2,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: 'Scaffold', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['compile'] },
    { id: 'T02', title: 'Print', tier: 'focused', status: 'pending', depends_on: ['T01'], files: [], completed: false, verify_phases: ['test'] },
  ],
};

const PLANS = ['hello', 'other'].map(
  (id) => ({ id, title: id, task_count: 2, tasks_failed: 0, completed: false, status: 'pending', old_format: false }) as WirePlanSummary,
);

const WORKSPACE = [queryKeys.workspace, { name: 'hello', path: '/tmp/hello', branch: null }] as const;

beforeEach(() => {
  window.history.replaceState(null, '', '/');
  setStore(initialRunState());
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const sentence = () => textOf(document.querySelector('.stream-empty'));

function pane(planId: string | null, planCount?: number) {
  return renderWithClient(
    <StreamPane planId={planId} selectedTaskId={null} open onToggle={() => {}} planCount={planCount} />,
    { seed: [[queryKeys.planTasks('hello'), TASKS], WORKSPACE] },
  );
}

describe('the stream with no task to follow', () => {
  it('says a plan that never ran is ready, with its waves', () => {
    pane('hello');
    expect(sentence()).toBe('Ready — 2 tasks in 2 waves.');
  });

  it('asks for a plan when plans exist but none is selected', () => {
    pane(null, 2);
    expect(sentence()).toBe('Select a plan, or press n to describe a new one.');
  });

  it('says the workspace has no plans only when it has none', () => {
    pane(null, 0);
    expect(sentence()).toBe('No plans in hello yet. Describe what you want to build.');
  });

  it('gets the plan count from the workspace', () => {
    stubFetch([]);
    renderWithClient(<Workspace />, {
      seed: [[queryKeys.plans, PLANS], WORKSPACE, [queryKeys.planTasks('hello'), TASKS]],
    });
    fireEvent.keyDown(window, { key: 'o' });
    expect(sentence()).toBe('Select a plan, or press n to describe a new one.');
  });
});
