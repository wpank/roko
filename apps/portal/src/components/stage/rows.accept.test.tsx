// @vitest-environment jsdom
/**
 * Acceptance: task and rail rows — no placeholder dots, one time format, the
 * short model slug, spaced check chips, and the type scale on ids, titles and
 * section labels.
 * Copied verbatim from plans/portal-programme/08d-portal-legibility/accept/.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary, WirePlanTask } from '@/api/contracts';
import { PlanRow } from '@/components/rail/PlanRow';
import { TaskList } from '@/components/stage/TaskList';
import { WaveStrip } from '@/components/stage/WaveStrip';
import { buildPlanRows } from '@/lib/planRows';
import { initialRunState } from '@/lib/runState';
import { buildTaskRows } from '@/lib/taskRows';
import { foldEvents, textOf } from '@/test/dom';

afterEach(() => cleanup());

const TASKS: WirePlanTask[] = [
  { id: 'T01', title: 'Scaffold the hello project', description: 'Create the cargo project.', tier: 'focused', status: 'pending', depends_on: [], files: ['Cargo.toml'], completed: false, verify_phases: ['structural'], verify: [{ phase: 'structural', command: 'test -f Cargo.toml' }], role: 'implementer' },
  { id: 'T02', title: 'Print hello world', tier: 'focused', status: 'pending', depends_on: ['T01'], files: ['src/main.rs'], completed: false, verify_phases: ['compile'], role: 'implementer' },
];

const RUN: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
  { type: 'agent_spawned', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', role: 'implementer', model: 'claude-sonnet-4-20250514' },
  { type: 'gate_result', plan_id: 'hello', task_id: 'T01', gate: 'verify[0:structural]', passed: true, output_text: '$ test -f Cargo.toml' },
  { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'passed' },
];

function renderTasks(selected: string | null = null) {
  const run = foldEvents(RUN, 1_000, 6_000);
  const { rows, waves } = buildTaskRows(TASKS, run, 'hello', 60_000);
  render(
    <>
      <WaveStrip waves={waves} rows={rows} maxParallel={1} selectedTaskId={null} onSelectTask={() => {}} />
      <TaskList rows={rows} selectedTaskId={selected} onSelectTask={() => {}} onRetry={() => {}} />
    </>,
  );
}

const row = (id: string) => document.querySelector(`[data-task-row="${id}"]`)!;

describe('task rows', () => {
  it('leave time and cost empty for a task that has not run — no placeholder dots', () => {
    renderTasks();
    expect(textOf(row('T02').querySelector('[data-cell="time"]'))).toBe('');
    expect(textOf(row('T02').querySelector('[data-cell="cost"]'))).toBe('');
  });

  it('show a finished task’s time in the one compact format', () => {
    renderTasks();
    expect(textOf(row('T01').querySelector('[data-cell="time"]'))).toMatch(/^\d+s$|^\d+m\d+s$/);
  });

  it('show the short model slug, with the full name as its tooltip', () => {
    renderTasks();
    expect(textOf(row('T01'))).toContain('sonnet-4');
    expect(textOf(row('T01'))).not.toContain('claude-');
    expect(row('T01').querySelector('[title="claude-sonnet-4-20250514"]')).not.toBeNull();
  });

  it('space the check chip glyph from its phase', () => {
    renderTasks();
    expect(textOf(row('T01'))).toContain('✓ structural');
  });

  it('set ids as meta, titles as rows and detail labels as section labels', () => {
    renderTasks('T01');
    expect(row('T01').querySelector('.rd-meta')?.textContent).toBe('T01');
    expect(row('T01').querySelector('.rd-row')?.textContent).toBe('Scaffold the hello project');
    const labels = [...row('T01').querySelectorAll('.rd-section')].map((e) => textOf(e).toLowerCase());
    expect(labels).toEqual(expect.arrayContaining(['files', 'verify']));
  });

  it('draw wave labels at the meta size', () => {
    renderTasks();
    const label = [...document.querySelectorAll('[data-wave] span')].find((e) => textOf(e).startsWith('W1'));
    expect(label?.className).toContain('rd-meta');
    expect(label?.className).not.toMatch(/text-\[(9|10|11)px\]/);
  });
});

describe('rail rows', () => {
  const plan = (extra: Partial<WirePlanSummary>): WirePlanSummary => ({
    id: 'p',
    title: 'Plan',
    task_count: 2,
    tasks_done: 0,
    tasks_failed: 0,
    completed: false,
    status: 'pending',
    old_format: false,
    ...extra,
  });
  const railRow = (p: WirePlanSummary) => buildPlanRows([p], initialRunState(), { filter: '', nowMs: 0 }).groups[0]!.rows[0]!;

  it('leave the time empty when it is unknown — no placeholder dot', () => {
    render(<PlanRow row={railRow(plan({}))} selected={false} onSelect={() => {}} />);
    expect(textOf(document.querySelector('[data-time]'))).toBe('');
  });

  it('use the same formatter as the task rows for estimates', () => {
    render(<PlanRow row={railRow(plan({ estimated_minutes: 65 }))} selected={false} onSelect={() => {}} />);
    expect(textOf(document.querySelector('[data-time]'))).toBe('~1h05m');
  });

  it('fit the name in 20 characters', () => {
    render(<PlanRow row={railRow(plan({ title: 'Backend plan execution and run control' }))} selected={false} onSelect={() => {}} />);
    expect(textOf(document.querySelector('[data-name]')).length).toBeLessThanOrEqual(20);
  });
});
