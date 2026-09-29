// @vitest-environment jsdom
/**
 * Design §5 and §11 (gap-bd33b2). The checks view lists every declared verify
 * step in order, with its output: a passed step's behind the raw-output
 * toggle, an unreached one as `·`. A failed task row shows its digest's first
 * line. A failure takes the stream unless the operator selected a task. A tool
 * row notes on its collapsed line that the server kept only the tail, and a
 * transcript left with no entries says why it is empty.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { act, cleanup, fireEvent, render } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanTask, WirePlanTasks } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { TaskList } from '@/components/stage/TaskList';
import { Checks } from '@/components/stream/Checks';
import { StreamPane } from '@/components/stream/StreamPane';
import { Transcript } from '@/components/stream/Transcript';
import { initialRunState, taskKey } from '@/lib/runState';
import { STREAM_RECORD_PREFIX, TRUNCATED_PREFIX } from '@/lib/streamRecord';
import { buildTaskRows } from '@/lib/taskRows';
import { foldEvents, renderWithClient, setStore, textOf } from '@/test/dom';

beforeEach(() => setStore(initialRunState()));
afterEach(() => cleanup());

const VERIFY = [
  { phase: 'compile', command: 'cargo build' },
  { phase: 'test', command: 'cargo test' },
  { phase: 'clippy', command: 'cargo clippy -- -D warnings' },
];
const TASKS: WirePlanTask[] = [
  { id: 'T01', title: 'Scaffold', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['compile', 'test', 'clippy'], verify: VERIFY },
  { id: 'T02', title: 'Greet', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['compile'], verify: [VERIFY[0]!] },
];
const PLAN_TASKS: WirePlanTasks = { plan_id: 'hello', task_count: 2, max_parallel: 2, tasks: TASKS };

const COMPILE_OK = '$ cargo build\n   Compiling hello v0.1.0\n    Finished `dev` profile';
const TEST_FAILED = "$ cargo test\nrunning 1 test\nthread 'tests::greets' panicked at src/lib.rs:5:9:\nassertion failed: greeting.is_empty()\n✗ exit status 101";

const gate = (task: string, name: string, passed: boolean, output: string): WireDashboardEvent => ({
  type: 'gate_result', plan_id: 'hello', task_id: task, gate: name, passed, output_text: output,
});
const frame = (o: Record<string, unknown>) =>
  STREAM_RECORD_PREFIX + JSON.stringify({ agent_id: 'a1', plan_id: 'hello', task_id: 'T01', attempt: 0, ...o });
const out = (o: Record<string, unknown>): WireDashboardEvent => ({
  type: 'agent_output', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', content: frame(o),
});

// Both tasks run; T01 then passes compile, fails test and never reaches clippy.
const BOTH_RUNNING: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
  { type: 'agent_spawned', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', role: 'implementer' },
  { type: 'task_started', plan_id: 'hello', task_id: 'T02', phase: 'implement' },
  { type: 'agent_spawned', agent_id: 'a2', plan_id: 'hello', task_id: 'T02', role: 'implementer' },
];
const T01_FAILS: WireDashboardEvent[] = [
  { type: 'agent_completed', agent_id: 'a1', plan_id: 'hello', task_id: 'T01' },
  gate('T01', 'verify[0:compile]', true, COMPILE_OK),
  gate('T01', 'verify[1:test]', false, TEST_FAILED),
  { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'failed' },
];
const T01_FAILED = [...BOTH_RUNNING, ...T01_FAILS];

function pane(selected: string | null) {
  return renderWithClient(<StreamPane planId="hello" selectedTaskId={selected} open onToggle={() => {}} />, {
    seed: [
      [queryKeys.planTasks('hello'), PLAN_TASKS],
      [queryKeys.workspace, { name: 'hello', path: '/tmp/hello', branch: null }],
    ],
  });
}

const checkRows = () => [...document.querySelectorAll('[data-region="checks"] .check-row')];
const streamTask = () => textOf(document.querySelector('.stream-bar-task'));
const tab = (name: string) => [...document.querySelectorAll('.stream-tab')].find((b) => textOf(b).startsWith(name))!;

describe('the checks view (§5)', () => {
  const checks = () => foldEvents(T01_FAILED).tasks[taskKey('hello', 'T01')]!.checks;

  it('lists every declared step in verify order, an unreached one as · with its command', () => {
    render(<Checks checks={checks()} declared={VERIFY} />);
    expect(checkRows().map((r) => r.getAttribute('data-status'))).toEqual(['passed', 'failed', 'pending']);
    const clippy = checkRows()[2]!;
    expect(textOf(clippy.querySelector('[data-glyph]'))).toBe('·');
    expect(textOf(clippy.querySelector('.check-row-label'))).toBe('clippy');
    expect(textOf(clippy.querySelector('.check-row-command'))).toBe('$ cargo clippy -- -D warnings');
    expect(clippy.querySelector('.check-raw-toggle')).toBeNull();
  });

  it('shows passed step output behind the same raw output toggle a failure uses', () => {
    render(<Checks checks={checks()} declared={VERIFY} />);
    const [compile, test] = checkRows();
    const toggle = compile!.querySelector<HTMLDetailsElement>('.check-raw-toggle');
    expect(toggle).not.toBeNull();
    expect(toggle!.open).toBe(false);
    expect(textOf(toggle!.querySelector('summary'))).toBe(textOf(test!.querySelector('.check-raw-toggle summary')));
    expect(textOf(toggle!.querySelector('.check-raw-output'))).toContain('Finished `dev` profile');
  });

  it('adds no toggle when the passed step output is only its command', () => {
    render(<Checks checks={[{ name: 'verify[0:compile]', index: 0, phase: 'compile', status: 'passed', output: '$ true' }]} declared={VERIFY} />);
    expect(checkRows()[0]!.querySelector('.check-raw-toggle')).toBeNull();
    expect(textOf(checkRows()[0]!.querySelector('.check-row-command'))).toBe('$ true');
  });
});

describe('a failed task row (§11)', () => {
  function renderRows(selected: string | null) {
    const { rows } = buildTaskRows(TASKS, foldEvents(T01_FAILED), 'hello', 60_000);
    render(<TaskList rows={rows} selectedTaskId={selected} onSelectTask={() => {}} />);
  }
  const digest = (id: string) => document.querySelectorAll(`[data-task-row="${id}"] [data-digest]`);

  it('shows the digest’s first line on the row, not only when expanded', () => {
    renderRows(null);
    expect(digest('T01')).toHaveLength(1);
    expect(textOf(digest('T01')[0]!)).toBe('src/lib.rs:5:9 assertion failed: greeting.is_empty()');
    expect(textOf(document.querySelector('[data-task-row="T01"] [data-cell="checks"]'))).toContain('✗ test');
    expect(digest('T02')).toHaveLength(0);
  });

  it('keeps one digest line when the row is expanded', () => {
    renderRows('T01');
    expect(digest('T01')).toHaveLength(1);
  });
});

describe('the stream on a failure (§11)', () => {
  it('points at a failed task while another runs, on its checks', () => {
    setStore(foldEvents(T01_FAILED));
    pane(null);
    expect(streamTask()).toBe('T01');
    expect(checkRows().map((r) => r.getAttribute('data-status'))).toEqual(['passed', 'failed', 'pending']);
  });

  it('stays on the task the operator selected', () => {
    setStore(foldEvents(T01_FAILED));
    pane('T02');
    expect(streamTask()).toBe('T02');
    expect(document.querySelector('[data-region="transcript"]')).not.toBeNull();
  });

  it('shows the failure’s checks although the operator picked a view on the task it left', () => {
    setStore(foldEvents(BOTH_RUNNING));
    pane(null);
    expect(streamTask()).toBe('T01');
    fireEvent.click(tab('transcript'));
    act(() => setStore(foldEvents([
      ...BOTH_RUNNING,
      gate('T02', 'verify[0:compile]', false, '$ cargo build\n✗ exit status 101'),
      { type: 'task_completed', plan_id: 'hello', task_id: 'T02', outcome: 'failed' },
    ])));
    expect(streamTask()).toBe('T02');
    expect(textOf(document.querySelector('[data-region="checks"] [data-exit]'))).toBe('exit status 101 · no output');
  });

  it('returns to the transcript when the failed task is retried', () => {
    setStore(foldEvents(T01_FAILED));
    pane('T01');
    expect(document.querySelector('[data-region="checks"]')).not.toBeNull();
    act(() => setStore(foldEvents([...T01_FAILED, { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' }])));
    expect(textOf(document.querySelector('[data-region="transcript"] .transcript-divider'))).toContain('attempt 2');
  });
});

describe('the transcript (§5)', () => {
  it('notes on the collapsed tool row that the server kept only the tail', () => {
    const run = foldEvents([
      ...BOTH_RUNNING,
      out({ kind: 'tool_start', payload: { tool_id: 't1', tool: 'Read' } }),
      out({ kind: 'tool_result', payload: { tool_id: 't1', output: TRUNCATED_PREFIX + 'end of a long file' } }),
      out({ kind: 'tool_start', payload: { tool_id: 't2', tool: 'Write' } }),
      out({ kind: 'tool_result', payload: { tool_id: 't2', output: 'wrote 3 lines' } }),
    ]);
    render(<Transcript transcript={run.transcripts[taskKey('hello', 'T01')]} working={null} />);
    const read = document.querySelector<HTMLDetailsElement>('[data-tool="t1"]')!;
    expect(read.open).toBe(false);
    expect(textOf(read.querySelector('summary [data-truncated]'))).toContain('server kept only the tail');
    expect(document.querySelectorAll('[data-truncated]')).toHaveLength(1);
  });

  it('says why a transcript left with no entries is empty', () => {
    const events: WireDashboardEvent[] = [
      ...BOTH_RUNNING,
      out({ kind: 'text', live: true, screened: false, payload: { text: 'draft words' } }),
      { type: 'agent_completed', agent_id: 'a1', plan_id: 'hello', task_id: 'T01' },
      { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'passed' },
    ];
    const run = foldEvents(events);
    expect(run.transcripts[taskKey('hello', 'T01')]?.entries).toEqual([]);
    setStore(run);
    pane('T01');
    expect(textOf(document.querySelector('[data-region="transcript"]'))).toBe('No transcript was kept for this task.');
  });
});
