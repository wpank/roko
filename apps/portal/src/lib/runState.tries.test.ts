/**
 * Tries (gap-60f6b6): a Graph task's second try arrives as a new agent_spawned with attempt 2, not as a second
 * task_started, so the run state counts tries from agent_spawned and settles each from attempt_settled.
 */
import { describe, expect, it } from 'vitest';
import type { WireDashboardEvent, WireDashboardSnapshot } from '@/api/contracts';
import { fromSnapshot } from '@/lib/runState';
import { foldEvents } from '@/test/dom';

const KEY = 'greeting/T3';

const START: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'greeting', tasks_total: 3 },
  { type: 'task_started', plan_id: 'greeting', task_id: 'T3', title: 'Document how to run it', phase: 'implement' },
  { type: 'agent_spawned', agent_id: 'greeting/T3', plan_id: 'greeting', task_id: 'T3', attempt: 1, role: 'implementer', model: 'glm-4.7' },
];

const FIRST_TRY_FAILS: WireDashboardEvent[] = [
  ...START,
  { type: 'agent_output', agent_id: 'greeting/T3', plan_id: 'greeting', task_id: 'T3', attempt: 1, content: 'Wrote README.md.' },
  { type: 'gate_result', plan_id: 'greeting', task_id: 'T3', gate: 'verify[0:usage]', passed: false },
  {
    type: 'attempt_settled',
    plan_id: 'greeting',
    task_id: 'T3',
    attempt: 1,
    verdict: 'gate_failed',
    failed_check: 'verify:0/usage',
    detail: 'verify[0:usage] exit code: 1',
    model: 'glm-4.7',
    cost_usd: 0.1342,
    duration_ms: 4_932,
    climbed: false,
  },
];

const SECOND_TRY_PASSES: WireDashboardEvent[] = [
  ...FIRST_TRY_FAILS,
  { type: 'agent_spawned', agent_id: 'greeting/T3', plan_id: 'greeting', task_id: 'T3', attempt: 2, role: 'implementer', model: 'glm-4.7' },
  { type: 'gate_result', plan_id: 'greeting', task_id: 'T3', gate: 'verify[0:usage]', passed: true },
  {
    type: 'attempt_settled',
    plan_id: 'greeting',
    task_id: 'T3',
    attempt: 2,
    verdict: 'passed',
    model: 'glm-4.7',
    cost_usd: 0.1007,
    duration_ms: 6_100,
  },
  { type: 'task_completed', plan_id: 'greeting', task_id: 'T3', outcome: 'passed' },
];

describe('tries', () => {
  it('counts a second try from agent_spawned', () => {
    const task = foldEvents(SECOND_TRY_PASSES).tasks[KEY]!;
    expect(task.attempts).toBe(2);
    expect(task.tries).toEqual([
      {
        attempt: 1,
        model: 'glm-4.7',
        state: 'failed',
        failedCheck: 'verify:0/usage',
        detail: 'verify[0:usage] exit code: 1',
        costUsd: 0.1342,
        durationMs: 4_932,
        climbed: false,
      },
      {
        attempt: 2,
        model: 'glm-4.7',
        state: 'passed',
        failedCheck: null,
        detail: null,
        costUsd: 0.1007,
        durationMs: 6_100,
        climbed: false,
      },
    ]);
  });

  it('counts a try as failed once the next one starts, before its settlement arrives', () => {
    const task = foldEvents([
      ...START,
      { type: 'agent_spawned', agent_id: 'greeting/T3', plan_id: 'greeting', task_id: 'T3', attempt: 2, role: 'implementer', model: 'gpt-5.4-mini' },
    ]).tasks[KEY]!;
    expect(task.tries?.map((t) => [t.attempt, t.state, t.model])).toEqual([
      [1, 'failed', 'glm-4.7'],
      [2, 'running', 'gpt-5.4-mini'],
    ]);
  });

  it('draws a divider in the transcript where the second try starts', () => {
    const run = foldEvents([
      ...FIRST_TRY_FAILS,
      { type: 'agent_spawned', agent_id: 'greeting/T3', plan_id: 'greeting', task_id: 'T3', attempt: 2, role: 'implementer', model: 'glm-4.7' },
    ]);
    const entries = run.transcripts[KEY]!.entries;
    expect(entries[entries.length - 1]).toEqual({ kind: 'divider', attempt: 2 });
  });

  it('runs a new try’s checks afresh, and restarts its clock', () => {
    const run = foldEvents([
      ...FIRST_TRY_FAILS,
      { type: 'agent_spawned', agent_id: 'greeting/T3', plan_id: 'greeting', task_id: 'T3', attempt: 2, role: 'implementer', model: 'glm-4.7' },
    ]);
    expect(run.tasks[KEY]!.checks).toEqual([]);
    const first = foldEvents(START).agents['greeting/T3']!.spawnedAtMs!;
    expect(run.agents['greeting/T3']!.spawnedAtMs).toBeGreaterThan(first);
  });

  it('starts the try of an agent announced before its task', () => {
    const task = foldEvents([
      { type: 'plan_started', plan_id: 'greeting', tasks_total: 3 },
      { type: 'agent_spawned', agent_id: 'greeting/T3', plan_id: 'greeting', task_id: 'T3', attempt: 1, role: 'implementer', model: 'glm-4.7' },
      { type: 'task_started', plan_id: 'greeting', task_id: 'T3', phase: 'implement' },
    ]).tasks[KEY]!;
    expect(task.tries?.map((t) => [t.attempt, t.state])).toEqual([[1, 'running']]);
  });

  it('ends the running try with its task', () => {
    const task = foldEvents([...START, { type: 'task_completed', plan_id: 'greeting', task_id: 'T3', outcome: 'failed' }]).tasks[KEY]!;
    expect(task.tries?.map((t) => t.state)).toEqual(['failed']);
  });

  it('keeps the count after a reload, from the snapshot agent’s attempt', () => {
    const snapshot = {
      plans: { greeting: { plan_id: 'greeting', phase: 'completed', tasks_total: 3, tasks_done: 3, tasks_failed: 0, active: false } },
      tasks: { [KEY]: { task_id: 'T3', plan_id: 'greeting', phase: 'completed', outcome: 'passed', title: 'Document how to run it' } },
      agents: {
        'greeting/T3': {
          agent_id: 'greeting/T3', role: 'implementer', active: false, model: 'glm-4.7', provider: 'fake-model',
          input_tokens: 0, output_tokens: 0, cost_usd: 0, current_task: 'T3', current_plan: 'greeting',
          attempt: 2, spawned_at_ms: 1_000, elapsed_ms: 0,
        },
      },
      gates: [],
      errors: [],
      stats: { cost_usd_total: 0, total_input_tokens: 0, total_output_tokens: 0 },
    } as WireDashboardSnapshot;
    const task = fromSnapshot(snapshot, 9_000).tasks[KEY]!;
    expect(task.attempts).toBe(2);
    expect(task.tries?.map((t) => [t.attempt, t.state])).toEqual([
      [1, 'failed'],
      [2, 'passed'],
    ]);
  });
});

describe('the plan check', () => {
  it('is kept once it ran', () => {
    const checked = foldEvents([
      { type: 'plan_started', plan_id: 'greeting', tasks_total: 3 },
      { type: 'plan_check_finished', plan_id: 'greeting', passed: true, name: 'runs', step: 1, steps: 1 },
    ]);
    expect(checked.plans.greeting!.planCheck).toEqual({ passed: true, name: 'runs' });
  });
});
