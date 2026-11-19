/**
 * plan.tsx — render a plan's page (PlanPage) in a jsdom test, with the server's answers a page needs seeded: its
 * tasks, its validation, the project's checks and its cap.
 */

import type { WireDashboardEvent, WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { PlanPage } from '@/components/plan/PlanPage';
import type { Seed } from '@/test/dom';
import { foldEvents, renderWithClient, setStore } from '@/test/dom';

export const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };

/** A plan summary as the list route sends it. */
export function planSummary(id: string, title: string, taskCount: number, extra: Partial<WirePlanSummary> = {}): WirePlanSummary {
  return {
    id,
    title,
    task_count: taskCount,
    tasks_done: 0,
    tasks_failed: 0,
    completed: false,
    status: 'pending',
    old_format: false,
    ...extra,
  };
}

/** A plan's tasks route: `[id, title, depends_on]` per task, each with one declared check. */
export function planTasks(planId: string, tasks: ReadonlyArray<readonly [string, string, readonly string[]]>): WirePlanTasks {
  return {
    plan_id: planId,
    task_count: tasks.length,
    max_parallel: 2,
    tasks: tasks.map(([id, title, deps]) => ({
      id,
      title,
      tier: 'focused',
      status: 'pending',
      depends_on: [...deps],
      files: [],
      completed: false,
      verify_phases: ['build'],
      verify: [{ phase: 'build', command: `make ${id.toLowerCase()}` }],
    })),
  };
}

/**
 * Render `plan`'s page with `events` folded into the store. `seed` adds or replaces answers; by default the plan
 * is valid, the project has a `compile` check and the cap is $3.00.
 */
export function renderPlanPage(
  plan: WirePlanSummary,
  tasks: WirePlanTasks,
  events: readonly WireDashboardEvent[] = [],
  opts: { selected?: string | null; seed?: Seed; onSelectTask?: (id: string | null) => void; startMs?: number } = {},
) {
  setStore(foldEvents(events, opts.startMs ?? 1_000));
  return renderWithClient(
    <PlanPage
      plan={plan}
      selectedTaskId={opts.selected ?? null}
      onSelectTask={opts.onSelectTask ?? (() => {})}
      onRequestError={() => {}}
    />,
    {
      seed: [
        [queryKeys.planTasks(plan.id), tasks],
        [queryKeys.validation(plan.id), VALID],
        [queryKeys.gateRungs, ['compile']],
        [queryKeys.planCosts(plan.id), 3],
        ...(opts.seed ?? []),
      ],
    },
  );
}
