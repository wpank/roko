// @vitest-environment jsdom
/**
 * Acceptance: rail rows — a readable name beside fixed number columns, no
 * missing values, and the reason a queued plan waits; Run all is locked while
 * a set runs. Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary } from '@/api/contracts';
import { PlanRail } from '@/components/rail/PlanRail';
import { PlanRow } from '@/components/rail/PlanRow';
import { buildPlanRows } from '@/lib/planRows';
import type { PlanRowModel, PlanRowsResult } from '@/lib/planRows';
import { applyEvent, initialRunState } from '@/lib/runState';
import type { RunState } from '@/lib/runState';
import { hasMissingValue, textOf } from '@/test/dom';

afterEach(() => cleanup());

const NOW = 100_000;

/** A plan summary as today's list route sends it: no tasks_done, group or estimate. */
function listed(id: string, title: string, taskCount: number): WirePlanSummary {
  return {
    id,
    title,
    task_count: taskCount,
    tasks_failed: 0,
    completed: false,
    status: 'pending',
    old_format: false,
  } as unknown as WirePlanSummary;
}

function fold(events: WireDashboardEvent[], start = 1_000): RunState {
  return events.reduce((run, e, i) => applyEvent(run, e, start + i), initialRunState());
}

function rowsFor(plans: WirePlanSummary[], run: RunState): PlanRowsResult {
  return buildPlanRows(plans, run, { filter: '', nowMs: NOW });
}

function only(result: PlanRowsResult, id: string): PlanRowModel {
  return result.groups.flatMap((g) => g.rows).find((r) => r.id === id)!;
}

const PLANS = [
  listed('a', 'Demo set: part A', 1),
  listed('b', 'Demo set: part B', 1),
  listed('hello', 'Hello world', 2),
];

describe('PlanRow', () => {
  it('renders glyph, name, count, bar and time cells', () => {
    const row = only(rowsFor(PLANS, initialRunState()), 'hello');
    const { container } = render(<PlanRow row={row} selected={false} onSelect={() => {}} />);
    const button = container.querySelector('button[data-plan-row="hello"]')!;
    expect(button.classList.contains('rd-plan-row')).toBe(true);
    expect(textOf(button.querySelector('[data-name]'))).toBe('Hello world');
    expect(textOf(button.querySelector('[data-count]'))).toBe('0/2');
    expect(button.querySelector('[data-time]')).not.toBeNull();
  });

  it('never renders a missing value, even when the list omits tasks_done', () => {
    const result = rowsFor(PLANS, initialRunState());
    for (const row of result.groups.flatMap((g) => g.rows)) {
      const { container } = render(<PlanRow row={row} selected={false} onSelect={() => {}} />);
      expect(hasMissingValue(textOf(container))).toBe(false);
      cleanup();
    }
  });

  it('middle-ellipsises a long name to fit its column and keeps the full title as a tooltip', () => {
    const long = listed('long', 'portal-programme-backend-plan-generation-and-revision', 3);
    const row = only(rowsFor([long], initialRunState()), 'long');
    const { container } = render(<PlanRow row={row} selected={false} onSelect={() => {}} />);
    const name = textOf(container.querySelector('[data-name]'));
    expect(name.length).toBeLessThanOrEqual(24);
    expect(name).toContain('…');
    expect(container.querySelector('button')!.getAttribute('title')).toContain(
      'portal-programme-backend-plan-generation-and-revision',
    );
  });

  it('shows a queued plan’s position and why it waits', () => {
    const run = fold([
      {
        type: 'plan_set_loaded',
        plans: [
          { plan_id: 'a', tasks_total: 1, wave: 0, depends_on: [], conflicts_with: [] },
          { plan_id: 'b', tasks_total: 1, wave: 1, depends_on: ['a'], conflicts_with: [] },
        ],
      },
      { type: 'plan_started', plan_id: 'a', tasks_total: 1 },
    ]);
    const row = only(rowsFor(PLANS, run), 'b');
    const { container } = render(<PlanRow row={row} selected={false} onSelect={() => {}} />);
    expect(container.querySelector('button')!.getAttribute('data-state')).toBe('queued');
    expect(textOf(container.querySelector('[data-queue]'))).toBe('#2');
    expect(textOf(container.querySelector('[data-wait]'))).toBe('after a');
    expect(container.querySelector('button')!.getAttribute('title')).toContain('after a');
  });

  it('shows no wait line for a plan that is not queued', () => {
    const row = only(rowsFor(PLANS, initialRunState()), 'a');
    const { container } = render(<PlanRow row={row} selected={false} onSelect={() => {}} />);
    expect(container.querySelector('[data-wait]')).toBeNull();
    expect(container.querySelector('[data-queue]')).toBeNull();
  });

  it('marks the selected row and reports clicks', () => {
    const onSelect = vi.fn();
    const row = only(rowsFor(PLANS, initialRunState()), 'hello');
    const { container } = render(<PlanRow row={row} selected onSelect={onSelect} />);
    const button = container.querySelector('button')!;
    expect(button.getAttribute('aria-current')).toBe('true');
    fireEvent.click(button);
    expect(onSelect).toHaveBeenCalledWith('hello');
  });

  it('shows an estimate in whole minutes', () => {
    const plans = [
      { ...listed('quick', 'Quick', 2), estimated_minutes: 9 },
      { ...listed('slow', 'Slow', 9), estimated_minutes: 95 },
    ];
    const result = rowsFor(plans, initialRunState());
    for (const [id, label] of [['quick', '~9m'], ['slow', '~1h35m']] as const) {
      const { container } = render(<PlanRow row={only(result, id)} selected={false} onSelect={() => {}} />);
      expect(textOf(container.querySelector('[data-time]'))).toBe(label);
      cleanup();
    }
  });

  it('shows elapsed time for a running plan', () => {
    const run = fold([{ type: 'plan_started', plan_id: 'hello', tasks_total: 2 }], NOW - 72_000);
    const row = only(rowsFor(PLANS, run), 'hello');
    const { container } = render(<PlanRow row={row} selected={false} onSelect={() => {}} />);
    expect(textOf(container.querySelector('[data-time]'))).toBe('1m12s');
  });
});

describe('PlanRail', () => {
  const props = {
    selectedPlanId: null,
    filter: '',
    onFilterChange: () => {},
    filterInputRef: { current: null },
    onSelect: () => {},
    onNewPlan: () => {},
    onRunPlans: () => {},
    emptySentence: 'No plans yet.',
  };

  it('locks Run all, with the reason, while a run is in progress', () => {
    const { container } = render(
      <PlanRail {...props} result={rowsFor(PLANS, initialRunState())} runDisabledReason="A run is already in progress" />,
    );
    const runAll = container.querySelector('[data-action="run-all"]') as HTMLButtonElement;
    expect(runAll.disabled).toBe(true);
    expect(runAll.getAttribute('title')).toBe('A run is already in progress');
  });

  it('offers Run all otherwise', () => {
    const onRunPlans = vi.fn();
    const { container } = render(
      <PlanRail {...props} onRunPlans={onRunPlans} result={rowsFor(PLANS, initialRunState())} runDisabledReason={null} />,
    );
    const runAll = container.querySelector('[data-action="run-all"]') as HTMLButtonElement;
    expect(runAll.disabled).toBe(false);
    fireEvent.click(runAll);
    expect(onRunPlans).toHaveBeenCalledWith(null, 'all 3 plans');
  });

  it('renders every row without missing values', () => {
    const { container } = render(
      <PlanRail {...props} result={rowsFor(PLANS, initialRunState())} runDisabledReason={null} />,
    );
    expect(container.querySelectorAll('[data-plan-row]')).toHaveLength(3);
    expect(hasMissingValue(textOf(container))).toBe(false);
  });
});
