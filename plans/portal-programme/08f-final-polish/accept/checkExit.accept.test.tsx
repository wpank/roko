// @vitest-environment jsdom
/**
 * Acceptance: a failed check says why. The runner now ends a failed step's
 * output with how it ended (`✗ exit status 1`, `✗ timed out after 600000 ms`)
 * after the tail of what it printed; the checks view shows that ending, and
 * "no output" when the command printed nothing. The stream's count badges are
 * the neutral and failed badge classes, not white on a state colour.
 * Copied verbatim from plans/portal-programme/08f-final-polish/accept/.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanTasks } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Checks } from '@/components/stream/Checks';
import { StreamPane } from '@/components/stream/StreamPane';
import { digestOutput } from '@/lib/checks';
import { initialRunState } from '@/lib/runState';
import type { CheckRun } from '@/lib/runState';
import { STREAM_RECORD_PREFIX } from '@/lib/streamRecord';
import { foldEvents, renderWithClient, setStore, textOf } from '@/test/dom';

beforeEach(() => setStore(initialRunState()));
afterEach(() => cleanup());

const failed = (output: string): CheckRun => ({ name: 'verify[0:structural]', index: 0, phase: 'structural', status: 'failed', output });
const exitLine = () => document.querySelector('[data-exit]');

describe('the digest', () => {
  it('reads how a failed step ended from its closing ✗ line', () => {
    const d = digestOutput('$ test -f MISSING.md\n✗ exit status 1');
    expect(d.command).toBe('test -f MISSING.md');
    expect(d.exit).toBe('exit status 1');
    expect(d.unparsed).toEqual([]);
  });

  it('keeps the output tail and drops only the closing line', () => {
    const d = digestOutput('$ cargo build\n   Compiling hello\n---stderr---\nerror: could not compile `hello`\n✗ exit status 101\n');
    expect(d.exit).toBe('exit status 101');
    expect(d.unparsed).toEqual(['   Compiling hello', '---stderr---', 'error: could not compile `hello`']);
  });

  it('reads no ending from output that has none', () => {
    expect(digestOutput('$ true').exit).toBeNull();
    expect(digestOutput('$ x\n✗ not the last line\nok').exit).toBeNull();
  });
});

describe('the checks view', () => {
  it('says how a silent failure ended', () => {
    render(<Checks checks={[failed('$ test -f MISSING.md\n✗ exit status 1')]} />);
    expect(textOf(exitLine())).toBe('exit status 1 · no output');
  });

  it('shows what the command printed, then how it ended', () => {
    render(<Checks checks={[failed('$ make\nmake: *** No rule to make target `all`.\n✗ exit status 2')]} />);
    expect(textOf(document.querySelector('.check-digest-unparsed'))).toContain('No rule to make target');
    expect(textOf(exitLine())).toBe('exit status 2');
  });

  it('says no output for an older server that sent only the command', () => {
    render(<Checks checks={[failed('$ test -f MISSING.md')]} />);
    expect(textOf(exitLine())).toBe('no output');
  });

  it('adds nothing to a passed step', () => {
    render(<Checks checks={[{ ...failed('$ true'), status: 'passed' }]} />);
    expect(exitLine()).toBeNull();
  });
});

describe('the stream badges', () => {
  const TASKS: WirePlanTasks = {
    plan_id: 'hello',
    task_count: 1,
    max_parallel: 1,
    tasks: [{ id: 'T01', title: 'Scaffold', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural'] }],
  };
  const text = (t: string): WireDashboardEvent => ({
    type: 'agent_output',
    agent_id: 'a1',
    plan_id: 'hello',
    task_id: 'T01',
    content: STREAM_RECORD_PREFIX + JSON.stringify({ kind: 'text', agent_id: 'hello/T01', plan_id: 'hello', task_id: 'T01', attempt: 0, payload: { text: t } }),
  });
  const EVENTS: WireDashboardEvent[] = [
    { type: 'plan_started', plan_id: 'hello', tasks_total: 1 },
    { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
    { type: 'agent_spawned', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', role: 'implementer' },
    text('I wrote README.md.'),
    { type: 'gate_result', plan_id: 'hello', task_id: 'T01', gate: 'verify[0:structural]', passed: false, output_text: '$ test -f MISSING.md\n✗ exit status 1' },
    { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'failed' },
  ];

  it('draw a count in the badge classes, never white on a state colour', () => {
    setStore(foldEvents(EVENTS));
    renderWithClient(<StreamPane planId="hello" selectedTaskId="T01" open onToggle={() => {}} />, {
      seed: [
        [queryKeys.planTasks('hello'), TASKS],
        [queryKeys.workspace, { name: 'hello', path: '/tmp/hello', branch: null }],
      ],
    });
    // The failure switched the pane to checks, so the transcript tab counts its news.
    const news = document.querySelector<HTMLElement>('.stream-badge')!;
    expect(news.classList.contains('rd-badge')).toBe(true);
    expect(news.classList.contains('rd-badge--failed')).toBe(false);
    expect(news.style.background + news.style.backgroundColor + news.style.color).toBe('');

    fireEvent.click([...document.querySelectorAll('.stream-tab')].find((b) => textOf(b).startsWith('transcript'))!);
    const failures = document.querySelector<HTMLElement>('.stream-badge')!;
    expect(textOf(failures)).toBe('1');
    expect(failures.classList.contains('rd-badge')).toBe(true);
    expect(failures.classList.contains('rd-badge--failed')).toBe(true);
    expect(failures.style.background + failures.style.backgroundColor + failures.style.color).toBe('');
  });
});
