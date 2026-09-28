// @vitest-environment jsdom
/**
 * Acceptance: a task row names its role in the role's accent — one accent per
 * role, as in the run band — and its model beside it.
 * Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanTask } from '@/api/contracts';
import { TaskList } from '@/components/stage/TaskList';
import { buildTaskRows } from '@/lib/taskRows';
import { foldEvents, hasMissingValue, textOf } from '@/test/dom';

afterEach(() => cleanup());

const TASKS: WirePlanTask[] = [
  { id: 'T01', title: 'Scaffold', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['compile'], role: 'implementer', model_hint: 'claude-sonnet-4-6' },
  { id: 'T02', title: 'Review', tier: 'focused', status: 'pending', depends_on: ['T01'], files: [], completed: false, verify_phases: ['test'], role: 'quick-reviewer' },
  { id: 'T03', title: 'Docs', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['test'] },
];

const RUN: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'hello', tasks_total: 3 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
  { type: 'agent_spawned', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', role: 'implementer', model: 'claude-opus-4-6' },
];

function renderList() {
  const { rows } = buildTaskRows(TASKS, foldEvents(RUN), 'hello', 10_000);
  return render(<TaskList rows={rows} selectedTaskId={null} onSelectTask={() => {}} onRetry={() => {}} />);
}

const row = (id: string) => document.querySelector(`[data-task-row="${id}"]`);

describe('TaskList roles', () => {
  it('shows the dispatched role in its accent, with the model', () => {
    renderList();
    const role = row('T01')!.querySelector('[data-role]');
    expect(role?.getAttribute('data-role')).toBe('implementer');
    expect(textOf(role)).toBe('implementer');
    expect(role?.getAttribute('style') ?? '').toContain('--role-implementer');
    expect(textOf(row('T01'))).toContain('opus-4-6');
  });

  it('gives each role its own accent', () => {
    renderList();
    const role = row('T02')!.querySelector('[data-role]');
    expect(role?.getAttribute('data-role')).toBe('quick-reviewer');
    expect(role?.getAttribute('style') ?? '').toContain('--role-quick-reviewer');
  });

  it('shows no role label for a task without one', () => {
    renderList();
    expect(row('T03')!.querySelector('[data-role]')).toBeNull();
  });

  it('never shows a missing value', () => {
    const { container } = renderList();
    expect(hasMissingValue(textOf(container))).toBe(false);
  });
});
