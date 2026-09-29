// @vitest-environment jsdom
/**
 * Green means verified (design §6 rule 1, gap-63e0b6): a running plan's rail
 * row and the header's run glyph turn amber once a task was accepted despite
 * failing checks, or once no task waits to be dispatched and only checks remain.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Header } from '@/components/shell/Header';
import { PlanRow } from '@/components/rail/PlanRow';
import { buildPlanRows } from '@/lib/planRows';
import { foldEvents, renderWithClient, setStore } from '@/test/dom';

afterEach(() => cleanup());

const PLAN = {
  id: 'hello',
  title: 'Hello world',
  task_count: 3,
  tasks_done: 0,
  tasks_failed: 0,
  completed: false,
  status: 'pending',
  old_format: false,
} as WirePlanSummary;

const START: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 3 }] },
  { type: 'plan_started', plan_id: 'hello', tasks_total: 3 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
];
const passed = (id: string): WireDashboardEvent[] => [
  { type: 'task_completed', plan_id: 'hello', task_id: id, outcome: 'passed' },
];
const started = (id: string): WireDashboardEvent[] => [
  { type: 'task_started', plan_id: 'hello', task_id: id, phase: 'implement' },
];

/** The header's run glyph and the plan's rail glyph, after `events`. */
function glyphs(events: WireDashboardEvent[]): { header: HTMLElement; row: HTMLElement } {
  const run = foldEvents(events);
  setStore(run);
  renderWithClient(
    <Header runningPlanIds={['hello']} selectedPlanId={null} onSelectPlan={() => {}} onCancelRun={() => {}} />,
    { seed: [[queryKeys.workspace, { name: 'ws', path: '/tmp/ws' }]] },
  );
  const row = buildPlanRows([PLAN], run, { filter: '', nowMs: 0 }).groups[0]!.rows[0]!;
  render(<PlanRow row={row} selected={false} onSelect={() => {}} />);
  return {
    header: document.querySelector('[data-slot="run"] [data-glyph]') as HTMLElement,
    row: document.querySelector('[data-plan-row="hello"] [data-glyph]') as HTMLElement,
  };
}

const AMBER = 'var(--state-accepted)';

describe('a running plan', () => {
  it('stays the running colour while a task waits to be dispatched', () => {
    const { header, row } = glyphs([...START, ...passed('T01'), ...started('T02')]);
    for (const glyph of [header, row]) {
      expect(glyph.getAttribute('data-glyph')).toBe('active');
      expect(glyph.style.color).toBe('var(--state-active)');
    }
  });

  it('header is amber once every task is dispatched, and so is its rail row', () => {
    const { header, row } = glyphs([...START, ...passed('T01'), ...started('T02'), ...started('T03')]);
    for (const glyph of [header, row]) {
      expect(glyph.getAttribute('data-glyph')).toBe('unverified');
      expect(glyph.style.color).toBe(AMBER);
      expect(glyph.getAttribute('aria-label')).toBe('running, not verified');
      expect(glyph.classList.contains('rd-pulse')).toBe(true);
    }
  });

  it('header is amber once a task was accepted despite failing checks, and so is its rail row', () => {
    const { header, row } = glyphs([
      ...START,
      { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'accepted_with_failures' },
      ...started('T02'),
    ]);
    for (const glyph of [header, row]) {
      expect(glyph.getAttribute('data-glyph')).toBe('unverified');
      expect(glyph.style.color).toBe(AMBER);
    }
  });

  it('header is amber while any running plan is, though another is not', () => {
    setStore(
      foldEvents([
        ...START,
        ...passed('T01'),
        ...started('T02'),
        ...started('T03'),
        { type: 'plan_started', plan_id: 'other', tasks_total: 2 },
        { type: 'task_started', plan_id: 'other', task_id: 'T01', phase: 'implement' },
      ]),
    );
    renderWithClient(
      <Header runningPlanIds={['hello', 'other']} selectedPlanId={null} onSelectPlan={() => {}} onCancelRun={() => {}} />,
      { seed: [[queryKeys.workspace, { name: 'ws', path: '/tmp/ws' }]] },
    );
    const glyph = document.querySelector('[data-slot="run"] [data-glyph]') as HTMLElement;
    expect(glyph.getAttribute('data-glyph')).toBe('unverified');
  });
});
