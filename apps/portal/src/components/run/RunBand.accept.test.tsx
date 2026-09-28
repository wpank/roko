// @vitest-environment jsdom
/**
 * Acceptance: the run band is a real band — AGENTS, CHECKS and BURN cells with
 * titles — whose clocks move between events and which never shows a
 * placeholder. Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanTasks } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { RunBand } from '@/components/run/RunBand';
import { initialRunState } from '@/lib/runState';
import { foldEvents, hasMissingValue, renderWithClient, setStore, textOf } from '@/test/dom';

const T0 = 1_760_000_000_000;

const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 2,
  max_parallel: 3,
  tasks: [
    {
      id: 'T01',
      title: 'Scaffold the hello project',
      tier: 'focused',
      status: 'pending',
      depends_on: [],
      files: ['Cargo.toml'],
      completed: false,
      verify_phases: ['structural', 'compile'],
      verify: [
        { phase: 'structural', command: 'test -f Cargo.toml' },
        { phase: 'compile', command: 'cargo build' },
      ],
    },
    {
      id: 'T02',
      title: 'Print hello world',
      tier: 'focused',
      status: 'pending',
      depends_on: ['T01'],
      files: ['src/main.rs'],
      completed: false,
      verify_phases: ['test'],
    },
  ],
};

const RUNNING: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 2 }] },
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', title: 'Scaffold the hello project', phase: 'implement' },
  { type: 'agent_spawned', agent_id: 'a0', plan_id: 'hello', task_id: 'T01', role: 'implementer', model: 'claude-sonnet-4-6' },
  { type: 'agent_completed', agent_id: 'a0', plan_id: 'hello', task_id: 'T01' },
  { type: 'agent_spawned', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', role: 'implementer', model: 'claude-sonnet-4-6' },
  { type: 'agent_heartbeat', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', elapsed_ms: 0 },
  { type: 'gate_rung_started', plan_id: 'hello', task_id: 'T01', rung_name: 'verify[0:structural]' },
  { type: 'gate_result', plan_id: 'hello', task_id: 'T01', gate: 'verify[0:structural]', passed: true, output_text: '$ test -f Cargo.toml' },
];

const TOKENS: WireDashboardEvent[] = [
  { type: 'efficiency_event', plan_id: 'hello', task_id: 'T01', metric: 'input_tokens', value: 1_200 },
  { type: 'efficiency_event', plan_id: 'hello', task_id: 'T01', metric: 'output_tokens', value: 300 },
];

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(T0);
  setStore(initialRunState());
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

/** Every event at T0 − 5 s: the agent has been working for five seconds. */
const at5s = (events: WireDashboardEvent[]) => foldEvents(events, T0 - 5_000, 0);

function renderBand(runningPlanIds: string[]) {
  return renderWithClient(
    <RunBand runningPlanIds={runningPlanIds} selection={{ plan: 'hello', task: null }} />,
    { seed: [[queryKeys.planTasks('hello'), TASKS]] },
  );
}

const band = () => document.querySelector('[data-region="run-band"]');
const cell = (name: string) => document.querySelector(`[data-cell="${name}"]`);
const title = (name: string) => textOf(cell(name)?.querySelector('.rd-band__title') ?? null);

describe('RunBand', () => {
  it('takes no space while nothing runs', () => {
    const { container } = renderBand([]);
    expect(container.innerHTML).toBe('');
  });

  it('is a styled band of three titled cells in order', () => {
    setStore(at5s(RUNNING));
    renderBand(['hello']);
    expect(band()?.classList.contains('rd-band')).toBe(true);
    const cells = [...band()!.querySelectorAll(':scope > [data-cell]')].map((c) => c.getAttribute('data-cell'));
    expect(cells).toEqual(['burn', 'agents', 'checks']);
    for (const name of ['agents', 'checks', 'burn']) {
      expect(cell(name)?.classList.contains('rd-band__cell')).toBe(true);
    }
    expect(title('agents')).toMatch(/^AGENTS/);
    expect(title('checks')).toBe('CHECKS · T01');
    expect(title('burn')).toMatch(/^BURN/);
  });

  it('lists the working agent with its plan, task, model and clock', () => {
    setStore(at5s(RUNNING));
    renderBand(['hello']);
    const row = cell('agents')!.querySelector('[data-agent="a1"]');
    expect(row).not.toBeNull();
    expect(row!.querySelector('[data-role="implementer"]')).not.toBeNull();
    const text = textOf(row);
    expect(text).toContain('hello · T01');
    expect(text).toContain('sonnet-4-6');
    expect(text).toContain('5s');
  });

  it('counts finished agents and open slots', () => {
    setStore(at5s(RUNNING));
    renderBand(['hello']);
    const text = textOf(cell('agents'));
    expect(text).toContain('+1 finished');
    expect(text).toContain('idle ×2');
  });

  it('keeps the agent clock moving between events', () => {
    setStore(at5s(RUNNING));
    renderBand(['hello']);
    act(() => {
      vi.advanceTimersByTime(2_000);
    });
    expect(textOf(cell('agents')!.querySelector('[data-agent="a1"]'))).toContain('7s');
  });

  it('shows the focus task and its whole verify ladder', () => {
    setStore(at5s(RUNNING));
    renderBand(['hello']);
    const checks = cell('checks')!;
    expect(textOf(checks)).toContain('Scaffold the hello project');
    const rungs = [...checks.querySelectorAll('[data-rung]')].map((r) => [r.getAttribute('data-rung'), textOf(r)]);
    expect(rungs).toEqual([
      ['passed', '✓ structural'],
      ['pending', '· compile'],
    ]);
  });

  it('shows the token total without placeholders before any usage arrives', () => {
    setStore(at5s(RUNNING));
    renderBand(['hello']);
    const text = textOf(cell('burn'));
    expect(text).toContain('0 tok');
    expect(text).not.toContain('/min');
    expect(text).not.toMatch(/·\s*·|·\s*$/);
  });

  it('shows the burn rate and each role’s share once tokens arrive', () => {
    setStore(at5s([...RUNNING, ...TOKENS]));
    renderBand(['hello']);
    const burn = cell('burn')!;
    expect(textOf(burn)).toContain('1.5k tok');
    expect(textOf(burn)).toContain('/min');
    expect(textOf(burn.querySelector('[data-role-share="implementer"]'))).toContain('implementer 100%');
  });

  it('never shows a missing value', () => {
    setStore(at5s([...RUNNING, ...TOKENS]));
    renderBand(['hello']);
    expect(hasMissingValue(textOf(band()))).toBe(false);
  });
});
