// @vitest-environment jsdom
/**
 * A disabled button says why in its title (bug-5e71d0). The title shows only
 * while the button takes hover, so no portal rule may take pointer events from
 * a disabled button; it shows the not-allowed cursor instead.
 */
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { PlanRail } from '@/components/rail/PlanRail';
import { PlanView } from '@/components/stage/PlanView';
import { buildPlanRows } from '@/lib/planRows';
import { initialRunState } from '@/lib/runState';
import { foldEvents, renderWithClient, setStore } from '@/test/dom';

// jsdom ignores rules inside @layer, so lift the portal's button rules (and
// the pointer-events utility) out of theirs.
const globals = readFileSync(fileURLToPath(import.meta.url).replace(/components\/[^/]+$/, 'styles/globals.css'), 'utf8');
const BUTTON_RULES = [...globals.matchAll(/^(?:button[^{\n]*|\.pointer-events-none) \{[^}]*\}/gm)].map((m) => m[0]);

beforeAll(() => {
  const style = document.createElement('style');
  style.textContent = BUTTON_RULES.join('\n');
  document.head.append(style);
});

afterEach(() => cleanup());

/** The button is disabled, carries `reason` as its description, and can be hovered to show it. */
function expectReasonShown(name: string, reason: string): void {
  const button = screen.getByRole('button', { name, description: reason }) as HTMLButtonElement;
  expect(button.disabled).toBe(true);
  const style = getComputedStyle(button);
  expect(style.pointerEvents).not.toBe('none');
  expect(style.cursor).toBe('not-allowed');
}

const PLAN = {
  id: 'hello',
  title: 'Hello world',
  group: 'demo',
  task_count: 2,
  tasks_done: 0,
  tasks_failed: 0,
  completed: false,
  status: 'pending',
  old_format: false,
} as WirePlanSummary;

const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 2,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: 'Scaffold', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: [] },
    { id: 'T02', title: 'Print hello', tier: 'focused', status: 'pending', depends_on: ['T01'], files: [], completed: false, verify_phases: [] },
  ],
};

const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };

const RUNNING: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 2 }] },
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
];

describe('a disabled button', () => {
  it('disabled reason is visible on the rail’s Run all and group ▶ while a run is active', () => {
    render(
      <PlanRail
        result={buildPlanRows([PLAN], initialRunState(), { filter: '', nowMs: 0 })}
        selectedPlanId={null}
        filter=""
        onFilterChange={() => {}}
        filterInputRef={{ current: null }}
        onSelect={() => {}}
        onNewPlan={() => {}}
        onRunPlans={() => {}}
        emptySentence=""
        runDisabledReason="A run is already in progress"
      />,
    );
    expectReasonShown('▶ Run all', 'A run is already in progress');
    expectReasonShown('▶', 'A run is already in progress');
  });

  it('disabled reason is visible on ✦ Revise and ✎ Edit while the plan runs', () => {
    setStore(foldEvents(RUNNING));
    renderWithClient(
      <PlanView plan={PLAN} selectedTaskId={null} onSelectTask={() => {}} onRequestError={() => {}} />,
      { seed: [[queryKeys.planTasks('hello'), TASKS], [queryKeys.validation('hello'), VALID]] },
    );
    expectReasonShown('✦ Revise', 'Cannot revise while the plan is running');
    expectReasonShown('✎ Edit', 'Cannot edit while the plan is running');
  });
});
