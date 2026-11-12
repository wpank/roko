// @vitest-environment jsdom
/**
 * Acceptance for gap-195374: the plan list. Each row is a light, the title, one status line and a square per task,
 * with no missing values; a queued plan says why it waits; run all is locked, with the reason, while a run is on.
 */
import type { ComponentProps } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary } from '@/api/contracts';
import { PlanList } from '@/components/plans/PlanList';
import { buildPlanRows } from '@/lib/planRows';
import type { PlanRowsResult } from '@/lib/planRows';
import { applyEvent, initialRunState } from '@/lib/runState';
import type { RunState } from '@/lib/runState';
import { hasMissingValue, textOf } from '@/test/dom';

afterEach(() => cleanup());

const NOW = 100_000;

/** A plan summary as today's list route sends it: no tasks_done, group or estimate. */
function listed(id: string, title: string, taskCount: number, extra: Partial<WirePlanSummary> = {}): WirePlanSummary {
  return {
    id,
    title,
    task_count: taskCount,
    tasks_failed: 0,
    completed: false,
    status: 'pending',
    old_format: false,
    ...extra,
  } as unknown as WirePlanSummary;
}

function fold(events: WireDashboardEvent[], start = 1_000): RunState {
  return events.reduce((run, e, i) => applyEvent(run, e, start + i), initialRunState());
}

const PLANS = [listed('a', 'Demo set: part A', 1), listed('b', 'Demo set: part B', 1), listed('hello', 'Hello world', 2)];

function list(result: PlanRowsResult, props: Partial<ComponentProps<typeof PlanList>> = {}) {
  return render(
    <PlanList
      result={result}
      costs={{}}
      selectedPlanId={null}
      filter=""
      onFilterChange={() => {}}
      filterInputRef={{ current: null }}
      onSelect={() => {}}
      onNewPlan={() => {}}
      onRunPlans={() => {}}
      emptySentence="No plans yet."
      {...props}
    />,
  );
}

const rows = (plans: WirePlanSummary[], run = initialRunState()) => buildPlanRows(plans, run, { filter: '', nowMs: NOW });
const row = (id: string) => document.querySelector(`[data-plan-row="${id}"]`) as HTMLButtonElement;

describe('a plan row', () => {
  it('shows its title, its state in a word and a square per task, never a missing value', () => {
    list(rows(PLANS));
    expect(textOf(row('hello').querySelector('.pt-plan-title'))).toBe('Hello world');
    expect(textOf(row('hello').querySelector('.pt-plan-line'))).toBe('ready · 2 tasks');
    expect(row('hello').querySelectorAll('.rk-strip i')).toHaveLength(2);
    expect(hasMissingValue(textOf(document.querySelector('[data-region="rail"]')))).toBe(false);
  });

  it('keeps the full title as a tooltip for a long one', () => {
    const long = 'portal-programme-backend-plan-generation-and-revision';
    list(rows([listed('long', long, 3)]));
    expect(row('long').getAttribute('title')).toBe(long);
  });

  it('gives a plan with an estimate its minutes', () => {
    list(rows([listed('quick', 'Quick', 2, { estimated_minutes: 9 }), listed('slow', 'Slow', 9, { estimated_minutes: 95 })]));
    expect(textOf(row('quick'))).toContain('ready · 2 tasks · about 9 min');
    expect(textOf(row('slow'))).toContain('about 1h 35m');
  });

  it('says why a queued plan waits', () => {
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
    list(rows(PLANS, run));
    expect(row('b').dataset.state).toBe('queued');
    expect(textOf(row('b'))).toContain('queued · after a');
    expect(row('b').getAttribute('title')).toContain('after a');
  });

  it('shows a running plan’s clock', () => {
    list(rows(PLANS, fold([{ type: 'plan_started', plan_id: 'hello', tasks_total: 2 }], NOW - 72_000)));
    expect(row('hello').dataset.signal).toBe('live');
    expect(textOf(row('hello'))).toContain('running · 0 of 2 · 1:12');
  });

  it('marks the selected row and reports clicks', () => {
    const onSelect = vi.fn();
    list(rows(PLANS), { selectedPlanId: 'hello', onSelect });
    expect(row('hello').getAttribute('aria-current')).toBe('true');
    fireEvent.click(row('hello'));
    expect(onSelect).toHaveBeenCalledWith('hello');
  });
});

describe('the list', () => {
  it('locks run all, saying why, while a run is in progress', () => {
    list(rows(PLANS), { runDisabledReason: 'A run is already in progress' });
    const runAll = screen.getByRole('button', { name: 'run all' }) as HTMLButtonElement;
    expect(runAll.disabled).toBe(true);
    expect(runAll.title).toBe('A run is already in progress');
  });

  it('runs every plan from run all otherwise', () => {
    const onRunPlans = vi.fn();
    list(rows(PLANS), { onRunPlans });
    fireEvent.click(screen.getByRole('button', { name: 'run all' }));
    expect(onRunPlans).toHaveBeenCalledWith(null, 'all 3 plans');
  });

  it('collapses a group and runs its plans from its own key', () => {
    const onRunPlans = vi.fn();
    const grouped = [listed('x1', 'First', 1, { group: 'demo' }), listed('x2', 'Second', 1, { group: 'demo' })];
    list(rows(grouped), { onRunPlans });
    fireEvent.click(screen.getByRole('button', { name: 'run' }));
    expect(onRunPlans).toHaveBeenCalledWith(['x1', 'x2'], 'the 2 plans in demo');
    fireEvent.click(document.querySelector('.pt-group-toggle')!);
    expect(document.querySelector('[data-plan-row]')).toBeNull();
  });

  it('shows the filter only past eight plans', () => {
    list(rows(PLANS));
    expect(screen.queryByRole('searchbox')).toBeNull();
    cleanup();
    list(rows(Array.from({ length: 9 }, (_, i) => listed(`p${i}`, `Plan ${i}`, 1))));
    expect(screen.getByRole('searchbox', { name: 'Filter plans' })).not.toBeNull();
  });
});
