// @vitest-environment jsdom
/**
 * Acceptance: the run band lines up with the rail — BURN, the compact cell,
 * sits above it — never contradicts itself ("no agent working yet" beside
 * "+1 finished"), spaces its check chips and names models by their short slug.
 * Copied verbatim from plans/portal-programme/08d-portal-legibility/accept/.
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
  task_count: 2,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: 'Scaffold', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural'], verify: [{ phase: 'structural', command: 'true' }, { phase: 'compile', command: 'true' }] },
    { id: 'T02', title: 'Print', tier: 'focused', status: 'pending', depends_on: ['T01'], files: [], completed: false, verify_phases: ['compile'] },
  ],
};

const STARTED: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 2 }] },
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', title: 'Scaffold', phase: 'implement' },
];
const SPAWN: WireDashboardEvent = { type: 'agent_spawned', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', role: 'implementer', model: 'claude-sonnet-4-6' };
const DONE_AGENT: WireDashboardEvent = { type: 'agent_completed', agent_id: 'a1', plan_id: 'hello', task_id: 'T01' };
const PASSED: WireDashboardEvent = { type: 'gate_result', plan_id: 'hello', task_id: 'T01', gate: 'verify[0:structural]', passed: true, output_text: '$ true' };

beforeEach(() => setStore(initialRunState()));
afterEach(() => cleanup());

function renderBand(events: WireDashboardEvent[]) {
  setStore(foldEvents(events));
  return renderWithClient(<RunBand runningPlanIds={['hello']} selection={{ plan: 'hello', task: null }} />, {
    seed: [[queryKeys.planTasks('hello'), TASKS]],
  });
}

const agents = () => textOf(document.querySelector('[data-cell="agents"]'));

describe('layout', () => {
  it('puts BURN above the rail and AGENTS and CHECKS over the stage', () => {
    renderBand([...STARTED, SPAWN]);
    const band = document.querySelector('[data-region="run-band"]')!;
    const cells = [...band.querySelectorAll(':scope > [data-cell]')].map((c) => c.getAttribute('data-cell'));
    expect(cells).toEqual(['burn', 'agents', 'checks']);
  });
});

describe('AGENTS', () => {
  it('says no agent is working only while none has run', () => {
    renderBand(STARTED);
    expect(agents()).toContain('no agent working yet');
    expect(agents()).not.toContain('finished');
  });

  it('does not claim no agent is working beside "+1 finished"', () => {
    renderBand([...STARTED, SPAWN, DONE_AGENT]);
    expect(agents()).toContain('+1 finished');
    expect(agents()).not.toContain('no agent working yet');
  });

  it('names the model by its short slug, the full name as a tooltip', () => {
    renderBand([...STARTED, SPAWN]);
    const row = document.querySelector('[data-agent="a1"]');
    expect(textOf(row)).toContain('sonnet-4-6');
    expect(textOf(row)).not.toContain('claude-');
    expect(row?.querySelector('[title="claude-sonnet-4-6"]')).not.toBeNull();
  });
});

describe('CHECKS', () => {
  it('spaces each chip’s glyph from its phase', () => {
    renderBand([...STARTED, SPAWN, PASSED]);
    const rungs = [...document.querySelectorAll('[data-rung]')].map((r) => textOf(r));
    expect(rungs).toEqual(['✓ structural', '· compile']);
  });
});
