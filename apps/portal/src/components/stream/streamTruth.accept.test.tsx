// @vitest-environment jsdom
/**
 * Acceptance: the stream tells the truth about the focused task. "agent
 * working" shows only while that task's agent is itself running — not while
 * its checks run after the agent completed — and an empty transcript says why
 * it is empty: not started, no output yet, or none kept (a finished or
 * replayed task). The transcript and checks are set at row size.
 * Copied verbatim from plans/portal-programme/08e-portal-refine/accept/.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanTasks } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Checks } from '@/components/stream/Checks';
import { StreamPane } from '@/components/stream/StreamPane';
import { Transcript } from '@/components/stream/Transcript';
import { initialRunState } from '@/lib/runState';
import type { CheckRun } from '@/lib/runState';
import { STREAM_RECORD_PREFIX } from '@/lib/streamRecord';
import { foldEvents, renderWithClient, setStore, textOf } from '@/test/dom';

beforeEach(() => setStore(initialRunState()));
afterEach(() => cleanup());

const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 2,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: 'Scaffold', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['compile'] },
    { id: 'T02', title: 'Greet', tier: 'focused', status: 'pending', depends_on: ['T01'], files: [], completed: false, verify_phases: ['compile'] },
  ],
};

const text = (t: string): WireDashboardEvent => ({
  type: 'agent_output',
  agent_id: 'a1',
  plan_id: 'hello',
  task_id: 'T01',
  content: STREAM_RECORD_PREFIX + JSON.stringify({ kind: 'text', agent_id: 'hello/T01', plan_id: 'hello', task_id: 'T01', attempt: 0, payload: { text: t } }),
});

const STARTED: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', title: 'Scaffold', phase: 'implement' },
  { type: 'agent_spawned', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', role: 'implementer' },
];
const AGENT_DONE: WireDashboardEvent[] = [
  ...STARTED,
  text('I wrote src/main.rs.'),
  { type: 'agent_completed', agent_id: 'a1', plan_id: 'hello', task_id: 'T01' },
  { type: 'gate_rung_started', plan_id: 'hello', task_id: 'T01', rung_name: 'verify[0:compile]' },
];
const FINISHED_SILENT: WireDashboardEvent[] = [
  ...STARTED,
  { type: 'agent_completed', agent_id: 'a1', plan_id: 'hello', task_id: 'T01' },
  { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'passed' },
];

function pane(events: WireDashboardEvent[], selected: string | null) {
  setStore(foldEvents(events));
  return renderWithClient(<StreamPane planId="hello" selectedTaskId={selected} open onToggle={() => {}} />, {
    seed: [
      [queryKeys.planTasks('hello'), TASKS],
      [queryKeys.workspace, { name: 'hello', path: '/tmp/hello', branch: null }],
    ],
  });
}

const transcriptText = (root: ParentNode) => textOf(root.querySelector('[data-region="transcript"]')!);

describe('truthful stream', () => {
  it('shows agent working while the agent runs', () => {
    const { container } = pane([...STARTED, text('Reading the plan.')], 'T01');
    expect(transcriptText(container)).toContain('Reading the plan.');
    expect(transcriptText(container)).toContain('agent working');
  });

  it('drops agent working once the agent completed, though its checks still run', () => {
    const { container } = pane(AGENT_DONE, 'T01');
    expect(transcriptText(container)).toContain('I wrote src/main.rs.');
    expect(textOf(container)).not.toContain('agent working');
  });

  it('says a finished task kept no transcript, not that output is still coming', () => {
    const { container } = pane(FINISHED_SILENT, 'T01');
    expect(transcriptText(container)).toBe('No transcript was kept for this task.');
    expect(textOf(container)).not.toContain('agent working');
  });

  it('says a task that has not started has not started', () => {
    const { container } = pane(STARTED, 'T02');
    expect(transcriptText(container)).toBe('This task has not started.');
  });

  it('keeps the waiting line for a running task with no output yet', () => {
    const { container } = render(<Transcript transcript={undefined} working={null} taskStatus="active" />);
    expect(transcriptText(container)).toBe('The agent has not produced output yet.');
  });

  it('sets the transcript and the checks at row size', () => {
    const run = foldEvents([...STARTED, text('hi')]);
    const checks: CheckRun[] = [{ name: 'verify[0:compile]', index: 0, phase: 'compile', status: 'passed', output: '$ tsc\nok' }];
    const t = render(<Transcript transcript={run.transcripts['hello/T01']} working={null} />);
    expect(t.container.querySelector('[data-region="transcript"]')!.classList.contains('rd-stream-body')).toBe(true);
    const empty = render(<Transcript transcript={undefined} working={null} taskStatus="pending" />);
    expect(empty.container.querySelector('[data-region="transcript"]')!.classList.contains('rd-stream-body')).toBe(true);
    const c = render(<Checks checks={checks} />);
    expect(c.container.querySelector('[data-region="checks"]')!.classList.contains('rd-stream-body')).toBe(true);
  });
});
