// @vitest-environment jsdom
/**
 * Acceptance: the run band is half as tall, so each cell opens with one head
 * line — its title and a summary beside it — and spends no line on a footer.
 * Copied verbatim from plans/portal-programme/08e-portal-refine/accept/.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanTasks } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { RunBand } from '@/components/run/RunBand';
import { initialRunState } from '@/lib/runState';
import { foldEvents, renderWithClient, setStore, textOf } from '@/test/dom';

const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 1,
  max_parallel: 2,
  tasks: [
    { id: 'T01', title: 'Scaffold', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural'], verify: [{ phase: 'structural', command: 'true' }] },
  ],
};

const EVENTS: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 1 }] },
  { type: 'plan_started', plan_id: 'hello', tasks_total: 1 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', title: 'Scaffold', phase: 'implement' },
  { type: 'agent_spawned', agent_id: 'a0', plan_id: 'hello', task_id: 'T01', role: 'implementer', model: 'claude-sonnet-4-6' },
  { type: 'agent_completed', agent_id: 'a0', plan_id: 'hello', task_id: 'T01' },
  { type: 'agent_spawned', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', role: 'implementer', model: 'claude-sonnet-4-6' },
  { type: 'efficiency_event', plan_id: 'hello', task_id: 'T01', metric: 'input_tokens', value: 1_200 },
];

beforeEach(() => setStore(initialRunState()));
afterEach(() => cleanup());

function renderBand() {
  setStore(foldEvents(EVENTS));
  renderWithClient(<RunBand runningPlanIds={['hello']} selection={{ plan: 'hello', task: null }} />, {
    seed: [[queryKeys.planTasks('hello'), TASKS]],
  });
}

const cell = (name: string) => document.querySelector(`[data-cell="${name}"]`)!;
const head = (name: string) => cell(name).firstElementChild as HTMLElement;

describe('compact run band', () => {
  it('opens every cell with a head line holding its title', () => {
    renderBand();
    for (const name of ['burn', 'agents', 'checks']) {
      expect(head(name).classList.contains('rd-band__head')).toBe(true);
      expect(head(name).querySelector('.rd-band__title')).not.toBeNull();
    }
  });

  it('puts the token total beside BURN', () => {
    renderBand();
    expect(textOf(head('burn'))).toMatch(/^BURN.*1\.2k tok/);
  });

  it('puts finished agents and open slots beside AGENTS, not on a footer line', () => {
    renderBand();
    expect(textOf(head('agents'))).toContain('+1 finished');
    expect(textOf(head('agents'))).toContain('idle ×1');
    expect(cell('agents').querySelector('.rd-band__footer')).toBeNull();
  });

  it('puts the focus task beside CHECKS', () => {
    renderBand();
    expect(textOf(head('checks'))).toMatch(/^CHECKS · T01.*Scaffold · hello/);
  });

  it('keeps two lines under each head and counts the rest in the note', () => {
    const tasks: WirePlanTasks = {
      ...TASKS,
      task_count: 3,
      max_parallel: 3,
      tasks: ['T01', 'T02', 'T03'].map((id) => ({ ...TASKS.tasks[0]!, id, title: `Task ${id}` })),
    };
    const roles = ['implementer', 'strategist', 'quick-reviewer'];
    const events: WireDashboardEvent[] = [{ type: 'plan_started', plan_id: 'hello', tasks_total: 3 }];
    roles.forEach((role, i) => {
      const task_id = `T0${i + 1}`;
      events.push(
        { type: 'task_started', plan_id: 'hello', task_id, title: `Task ${task_id}`, phase: 'implement' },
        { type: 'agent_spawned', agent_id: `a${i}`, plan_id: 'hello', task_id, role, model: 'claude-sonnet-4-6' },
        { type: 'efficiency_event', plan_id: 'hello', task_id, metric: 'input_tokens', value: 3_000 - i * 1_000 },
      );
    });
    setStore(foldEvents(events));
    renderWithClient(<RunBand runningPlanIds={['hello']} selection={{ plan: 'hello', task: null }} />, {
      seed: [[queryKeys.planTasks('hello'), tasks]],
    });
    expect(cell('agents').querySelectorAll('[data-agent]')).toHaveLength(2);
    expect(textOf(head('agents'))).toContain('+1 more');
    expect(cell('burn').querySelectorAll('[data-role-share]')).toHaveLength(2);
    expect(textOf(head('burn'))).toContain('+1 more');
  });
});
