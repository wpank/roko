// @vitest-environment jsdom
/**
 * Acceptance (1220): a task a Graph run holds for review shows its diff in the
 * portal, and approving or rejecting it posts the ReviewDecision body that
 * `submit_review` takes, which the waiting run reads. A held task in the task
 * list gets the Review action that opens the pane.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, screen, waitFor } from '@testing-library/react';
import type { WireDashboardEvent, WireTaskDiff } from '@/api/contracts';
import { ReviewPane } from '@/components/stage/ReviewPane';
import { TaskList } from '@/components/stage/TaskList';
import type { TaskRowModel } from '@/lib/taskRows';
import { initialRunState } from '@/lib/runState';
import { foldEvents, renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';

const DIFF_PATH = '/api/plans/hello/tasks/T02/diff';
const REVIEW_PATH = '/api/plans/hello/tasks/T02/review';

const DIFF: WireTaskDiff = {
  task_id: 'T02',
  file_count: 1,
  total_additions: 1,
  total_deletions: 0,
  files: [
    { path: 'feature.txt', status: 'added', additions: 1, deletions: 0, patch: '@@ -0,0 +1 @@\n+feature' },
  ],
  source: 'review_hold',
  status: 'awaiting_approval',
};

/** The server as these tests need it: the held task's diff, and its review route. */
const serve = () =>
  stubFetch([
    { path: DIFF_PATH, body: DIFF },
    { method: 'POST', path: REVIEW_PATH, body: { task_id: 'T02', status: 'approved', held: true } },
  ]);

/** The JSON bodies POSTed to the review route, in order. */
function posted(fetchMock: ReturnType<typeof stubFetch>): unknown[] {
  return fetchMock.mock.calls
    .filter(([input, init]) => init?.method === 'POST' && new URL(String(input), 'http://localhost').pathname === REVIEW_PATH)
    .map(([, init]) => JSON.parse(String(init?.body)));
}

beforeEach(() => {
  setStore(initialRunState());
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe('the review pane', () => {
  it('shows the held attempt’s files and patch', async () => {
    serve();
    renderWithClient(<ReviewPane planId="hello" taskId="T02" />);
    await waitFor(() => expect(document.querySelector('[data-file="feature.txt"]')).toBeTruthy());
    expect(textOf(document.querySelector('[data-file="feature.txt"]'))).toContain('+1 −0');
    expect(textOf(document.querySelector('[data-file="feature.txt"] pre'))).toContain('+feature');
  });

  it('approve posts { decision: "approve" }', async () => {
    const fetchMock = serve();
    const onDone = vi.fn();
    renderWithClient(<ReviewPane planId="hello" taskId="T02" onDone={onDone} />);
    fireEvent.click(document.querySelector('[data-action="approve"]')!);
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
    expect(posted(fetchMock)).toEqual([{ decision: 'approve' }]);
  });

  it('reject posts { decision: "reject", comment } with the note', async () => {
    const fetchMock = serve();
    renderWithClient(<ReviewPane planId="hello" taskId="T02" />);
    fireEvent.change(screen.getByLabelText('Review note'), {
      target: { value: 'name the file after the feature' },
    });
    fireEvent.click(document.querySelector('[data-action="reject"]')!);
    await waitFor(() => expect(posted(fetchMock)).toHaveLength(1));
    expect(posted(fetchMock)).toEqual([{ decision: 'reject', comment: 'name the file after the feature' }]);
  });
});

describe('a held task in the task list', () => {
  const HELD: WireDashboardEvent[] = [
    { type: 'plan_started', plan_id: 'hello', tasks_total: 1 },
    { type: 'task_started', plan_id: 'hello', task_id: 'T02', phase: 'graph-executing' },
    { type: 'task_phase_changed', plan_id: 'hello', task_id: 'T02', old_phase: 'graph-executing', new_phase: 'awaiting_approval' },
  ];
  const row: TaskRowModel = {
    id: 'T02', title: 'Print hello world', wave: 0, state: 'active', status: 'active', role: null, model: null,
    time: { kind: 'none', ms: null }, costUsd: null, attempts: 1, checks: [], dependsOn: [], waitingOn: [],
    blocked: null, files: [], description: null, verify: [],
  };

  it('gets a Review action that opens its pane', async () => {
    setStore(foldEvents(HELD));
    stubFetch([{ path: DIFF_PATH, body: DIFF }, { path: '/api/plans/hello/reviews', body: { plan_id: 'hello', reviews: [] } }]);
    renderWithClient(<TaskList rows={[row]} selectedTaskId={null} onSelectTask={() => {}} planId="hello" />);
    fireEvent.click(document.querySelector('[data-action="review"]')!);
    await waitFor(() => expect(document.querySelector('[data-file="feature.txt"]')).toBeTruthy());
  });
});
