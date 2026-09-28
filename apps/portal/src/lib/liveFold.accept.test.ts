/**
 * Acceptance: the run-state fold keeps live tool steps, holds unscreened
 * content only until that agent and attempt's screened transcript arrives (or
 * the agent or the run ends), and does the same for a snapshot's task outputs.
 * Copied verbatim from plans/portal-programme/08c-portal-live-steps/accept/.
 */
import { describe, expect, it } from 'vitest';
import type { WireDashboardEvent, WireDashboardSnapshot } from '@/api/contracts';
import { applyEvent, fromSnapshot, initialRunState, taskKey } from '@/lib/runState';
import type { RunState } from '@/lib/runState';
import { STREAM_RECORD_PREFIX } from '@/lib/streamRecord';

const KEY = taskKey('hello', 'T01');
const frame = (o: Record<string, unknown>) => STREAM_RECORD_PREFIX + JSON.stringify(o);
const env = (attempt: number) => ({ agent_id: 'hello/T01', plan_id: 'hello', task_id: 'T01', attempt });

const stepLine = (toolId: string, tool: string, target: string, attempt = 0) =>
  frame({ kind: 'tool_start', live: true, ...env(attempt), payload: { tool_id: toolId, tool, target } });
const unscreenedLine = (kind: string, payload: Record<string, unknown>, attempt = 0) =>
  frame({ kind, live: true, screened: false, ...env(attempt), payload });
const screenedLine = (kind: string, payload: Record<string, unknown>, attempt = 0) =>
  frame({ kind, ...env(attempt), payload });

const output = (content: string, agent = 'a1', attempt?: number): WireDashboardEvent => ({
  type: 'agent_output',
  agent_id: agent,
  plan_id: 'hello',
  task_id: 'T01',
  ...(attempt === undefined ? {} : { attempt }),
  content,
});

function fold(events: WireDashboardEvent[]): RunState {
  const start: WireDashboardEvent[] = [
    { type: 'plan_started', plan_id: 'hello', tasks_total: 1 },
    { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
    { type: 'agent_spawned', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', role: 'implementer' },
  ];
  return [...start, ...events].reduce((run, e, i) => applyEvent(run, e, 1_000 + i), initialRunState());
}

const kinds = (run: RunState) => (run.transcripts[KEY]?.entries ?? []).map((e) => e.kind);

describe('live tool steps', () => {
  it('append a step entry with the tool, target, agent and attempt', () => {
    const run = fold([output(stepLine('toolu_1', 'Write', 'apps/x.ts'))]);
    expect(run.transcripts[KEY]?.entries).toEqual([
      { kind: 'step', toolId: 'toolu_1', tool: 'Write', target: 'apps/x.ts', agentId: 'a1', attempt: 0 },
    ]);
  });

  it('take the attempt from the event before the envelope', () => {
    const run = fold([output(stepLine('toolu_1', 'Write', 'apps/x.ts', 0), 'a1', 2)]);
    expect(run.transcripts[KEY]?.entries[0]).toMatchObject({ kind: 'step', attempt: 2 });
  });

  it('survive the screened transcript, the agent’s end and the run’s end', () => {
    const run = fold([
      output(stepLine('toolu_1', 'Write', 'apps/x.ts')),
      output(screenedLine('text', { text: 'Done.' })),
      { type: 'agent_completed', agent_id: 'a1', plan_id: 'hello', task_id: 'T01' },
      { type: 'run_completed', outcome: 'succeeded', duration_ms: 100 },
    ]);
    expect(kinds(run)).toEqual(['step', 'text']);
  });
});

describe('unscreened live content', () => {
  it('appends an unscreened entry holding the record and any tool input', () => {
    const run = fold([
      output(unscreenedLine('text', { text: 'Writing…' })),
      output(unscreenedLine('tool_start', { tool_id: 'toolu_1', tool: 'Write', input: '{"file_path":"apps/x.ts"}' })),
    ]);
    expect(run.transcripts[KEY]?.entries).toEqual([
      { kind: 'unscreened', record: { kind: 'text', text: 'Writing…' }, input: null, agentId: 'a1', attempt: 0 },
      {
        kind: 'unscreened',
        record: { kind: 'tool_start', toolId: 'toolu_1', tool: 'Write' },
        input: '{"file_path":"apps/x.ts"}',
        agentId: 'a1',
        attempt: 0,
      },
    ]);
  });

  it('goes when that agent and attempt’s first screened record arrives, and the steps stay', () => {
    const run = fold([
      output(stepLine('toolu_1', 'Write', 'apps/x.ts')),
      output(unscreenedLine('text', { text: 'Writing…' })),
      output(unscreenedLine('tool_result', { tool_id: 'toolu_1', output: 'ok' })),
      output(screenedLine('text', { text: 'I wrote apps/x.ts.' })),
    ]);
    expect(kinds(run)).toEqual(['step', 'text']);
  });

  it('stays when the screened record belongs to another attempt or agent', () => {
    let run = fold([output(unscreenedLine('text', { text: 'retry draft' }, 1), 'a1', 1)]);
    run = applyEvent(run, output(screenedLine('text', { text: 'late' }, 0), 'a1', 0), 9_000);
    run = applyEvent(run, output(screenedLine('text', { text: 'other' }), 'a2', 1), 9_001);
    expect(kinds(run)).toEqual(['unscreened', 'text', 'text']);
  });

  it('goes when its agent completes', () => {
    const run = fold([
      output(stepLine('toolu_1', 'Write', 'apps/x.ts')),
      output(unscreenedLine('text', { text: 'Writing…' })),
      { type: 'agent_completed', agent_id: 'a1', plan_id: 'hello', task_id: 'T01' },
    ]);
    expect(kinds(run)).toEqual(['step']);
  });

  it('goes when the run completes', () => {
    const run = fold([
      output(unscreenedLine('reasoning', { text: 'thinking' })),
      { type: 'run_completed', outcome: 'cancelled', duration_ms: 100 },
    ]);
    expect(kinds(run)).toEqual([]);
  });

  it('ignores a live record that is neither a step nor unscreened', () => {
    const run = fold([output(frame({ kind: 'text', live: true, ...env(0), payload: { text: 'stray' } }))]);
    expect(kinds(run)).toEqual([]);
  });
});

describe('a snapshot’s task outputs', () => {
  function snapshot(taskOutcome: string | null): WireDashboardSnapshot {
    return {
      plans: {
        hello: { plan_id: 'hello', phase: 'started', active: taskOutcome === null, tasks_total: 1, tasks_done: 0, tasks_failed: 0 },
      },
      tasks: { [KEY]: { task_id: 'T01', title: 'Scaffold', plan_id: 'hello', phase: 'implement', outcome: taskOutcome } },
      agents: {},
      gates: [],
      errors: [],
      stats: { cost_usd_total: 0, total_input_tokens: 0, total_output_tokens: 0 },
      task_outputs: {
        T01: [
          stepLine('toolu_1', 'Write', 'apps/x.ts'),
          unscreenedLine('text', { text: 'Writing…' }),
        ],
      },
    } as unknown as WireDashboardSnapshot;
  }

  it('keep steps and unscreened content while the task runs', () => {
    expect(fromSnapshot(snapshot(null), 5_000).transcripts[KEY]?.entries.map((e) => e.kind)).toEqual([
      'step',
      'unscreened',
    ]);
  });

  it('keep only the steps once the task has finished', () => {
    expect(fromSnapshot(snapshot('passed'), 5_000).transcripts[KEY]?.entries.map((e) => e.kind)).toEqual([
      'step',
    ]);
  });
});
