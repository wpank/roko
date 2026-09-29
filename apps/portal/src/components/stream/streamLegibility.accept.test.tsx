// @vitest-environment jsdom
/**
 * Acceptance: the stream (transcript, checks, pane) dims text with the muted
 * and faint tokens — which pass contrast — never with opacity, and draws
 * nothing smaller than the meta size.
 * Copied verbatim from plans/portal-programme/08d-portal-legibility/accept/.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanTasks } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Checks } from '@/components/stream/Checks';
import { StreamPane } from '@/components/stream/StreamPane';
import { Transcript } from '@/components/stream/Transcript';
import { taskKey } from '@/lib/runState';
import type { CheckRun } from '@/lib/runState';
import { STREAM_RECORD_PREFIX, TRUNCATED_PREFIX } from '@/lib/streamRecord';
import { foldEvents, renderWithClient, setStore } from '@/test/dom';

afterEach(() => cleanup());

const frame = (kind: string, payload: Record<string, unknown>, extra: Record<string, unknown> = {}) =>
  STREAM_RECORD_PREFIX + JSON.stringify({ kind, agent_id: 'hello/T01', plan_id: 'hello', task_id: 'T01', attempt: 0, payload, ...extra });
const out = (content: string): WireDashboardEvent => ({ type: 'agent_output', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', content });

const EVENTS: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'hello', tasks_total: 1 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
  { type: 'agent_spawned', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', role: 'implementer' },
  out(frame('reasoning', { text: 'thinking about the file' })),
  out(frame('text', { text: 'I will write it.' })),
  out(frame('tool_start', { tool_id: 't1', tool: 'Write' })),
  out(frame('tool_result', { tool_id: 't1', output: TRUNCATED_PREFIX + 'tail of output' })),
  out(STREAM_RECORD_PREFIX + '{broken'),
  { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'failed' },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
  out(frame('tool_start', { tool_id: 't2', tool: 'Bash', target: 'cargo test' }, { live: true })),
  out(frame('text', { text: 'draft' }, { live: true, screened: false })),
];

const CHECKS: CheckRun[] = [
  {
    name: 'verify[0:compile]',
    index: 0,
    phase: 'compile',
    status: 'failed',
    output: '$ cargo build\nerror[E0425]: cannot find value `x` in this scope\n  --> src/main.rs:2:5\nwarning: unused import\n  --> src/lib.rs:1:1\nsome unparsed line',
  },
  { name: 'verify[1:test]', index: 1, phase: 'test', status: 'passed', output: '$ cargo test\nok' },
];

/** Every inline style that dims text with opacity or shrinks it below the meta size. */
function offenders(root: ParentNode): string[] {
  const bad: string[] = [];
  for (const el of root.querySelectorAll<HTMLElement>('[style]')) {
    const opacity = el.style.opacity;
    if (opacity !== '' && Number(opacity) < 1) bad.push(`opacity ${opacity}: ${el.outerHTML.slice(0, 80)}`);
    const size = el.style.fontSize;
    const em = /^(\d*\.?\d+)em$/.exec(size);
    const px = /^(\d*\.?\d+)px$/.exec(size);
    if ((em && Number(em[1]) < 1) || (px && Number(px[1]) < 12)) bad.push(`font-size ${size}: ${el.outerHTML.slice(0, 80)}`);
  }
  for (const el of root.querySelectorAll<HTMLElement>('[class]')) {
    const tiny = /text-\[(\d+)px\]/.exec(el.className);
    if (tiny && Number(tiny[1]) < 12) bad.push(`class ${tiny[0]}`);
  }
  return bad;
}

describe('stream legibility', () => {
  it('the transcript dims nothing with opacity and shrinks nothing below meta', () => {
    const run = foldEvents(EVENTS);
    const { container } = render(<Transcript transcript={run.transcripts[taskKey('hello', 'T01')]} working={{ sinceMs: Date.now() - 4_000 }} />);
    expect(container.querySelector('[data-region="transcript"]')).not.toBeNull();
    expect(offenders(container)).toEqual([]);
  });

  it('the checks view does the same', () => {
    const { container } = render(<Checks checks={CHECKS} />);
    expect(offenders(container)).toEqual([]);
  });

  it('the stream pane bar and badges do the same', () => {
    const tasks: WirePlanTasks = {
      plan_id: 'hello',
      task_count: 1,
      max_parallel: 1,
      tasks: [{ id: 'T01', title: 'Scaffold', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['compile'] }],
    };
    setStore(foldEvents(EVENTS));
    const { container } = renderWithClient(<StreamPane planId="hello" selectedTaskId="T01" open onToggle={() => {}} />, {
      seed: [
        [queryKeys.planTasks('hello'), tasks],
        [queryKeys.workspace, { name: 'hello', path: '/tmp/hello', branch: null }],
      ],
    });
    expect(offenders(container)).toEqual([]);
  });
});
