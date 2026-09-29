// @vitest-environment jsdom
/**
 * Acceptance: the task table is a grid — every row renders the same eight
 * cells in the same order, empty or not, so time, cost and checks line up
 * whatever a row carries; values sit on the type scale.
 * Copied verbatim from plans/portal-programme/08e-portal-refine/accept/.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanTask } from '@/api/contracts';
import { TaskList } from '@/components/stage/TaskList';
import { buildTaskRows } from '@/lib/taskRows';
import { foldEvents, textOf } from '@/test/dom';

afterEach(() => cleanup());

const TASKS: WirePlanTask[] = [
  { id: 'T01', title: 'Scaffold the hello project', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural', 'compile'], role: 'implementer' },
  { id: 'T02', title: 'Print hello world', tier: 'focused', status: 'pending', depends_on: ['T01'], files: [], completed: false, verify_phases: ['test'] },
];

const RUN: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
  { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'failed' },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
  { type: 'agent_spawned', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', role: 'implementer', model: 'claude-sonnet-4-6' },
  { type: 'efficiency_event', plan_id: 'hello', task_id: 'T01', metric: 'cost_usd', value: 0.04 },
  { type: 'gate_result', plan_id: 'hello', task_id: 'T01', gate: 'verify[0:structural]', passed: true, output_text: '$ true' },
  { type: 'gate_result', plan_id: 'hello', task_id: 'T01', gate: 'verify[1:compile]', passed: true, output_text: '$ true' },
  { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'passed' },
];

const ORDER = ['glyph', 'id', 'title', 'role', 'time', 'cost', 'attempts', 'checks'];

function renderRows() {
  const { rows } = buildTaskRows(TASKS, foldEvents(RUN, 1_000, 3_000), 'hello', 60_000);
  render(<TaskList rows={rows} selectedTaskId={null} onSelectTask={() => {}} onRetry={() => {}} />);
}

const grid = (id: string) => document.querySelector(`[data-task-row="${id}"] .rd-task-row`)!;
const cells = (id: string) => [...grid(id).children].map((c) => c.getAttribute('data-cell'));
const cell = (id: string, name: string) => grid(id).querySelector(`:scope > [data-cell="${name}"]`);

describe('task table grid', () => {
  it('gives every row the same eight cells in the same order', () => {
    renderRows();
    expect(cells('T01')).toEqual(ORDER);
    expect(cells('T02')).toEqual(ORDER);
  });

  it('keeps empty cells in place so columns line up', () => {
    renderRows();
    for (const name of ['role', 'time', 'cost', 'attempts', 'checks']) {
      expect(cell('T02', name)).not.toBeNull();
      expect(textOf(cell('T02', name))).toBe('');
    }
  });

  it('fills the cells of a row that ran', () => {
    renderRows();
    expect(textOf(cell('T01', 'id'))).toBe('T01');
    expect(textOf(cell('T01', 'title'))).toBe('Scaffold the hello project');
    expect(textOf(cell('T01', 'role'))).toContain('sonnet-4-6');
    expect(textOf(cell('T01', 'time'))).toMatch(/^\d+s$/);
    expect(textOf(cell('T01', 'cost'))).toBe('$0.04');
    expect(textOf(cell('T01', 'attempts'))).toBe('↻2');
    expect(textOf(cell('T01', 'checks'))).toBe('✓ structural ✓ compile');
  });

  it('sets titles at row size and the rest at meta size, numbers aligned right', () => {
    renderRows();
    expect(cell('T01', 'title')?.classList.contains('rd-row')).toBe(true);
    expect(cell('T01', 'id')?.classList.contains('rd-meta')).toBe(true);
    for (const name of ['time', 'cost', 'attempts']) {
      expect(cell('T01', name)?.classList.contains('rd-task-row__num')).toBe(true);
    }
    for (const name of ['role', 'checks']) {
      expect(cell('T01', name)?.classList.contains('rd-task-row__meta')).toBe(true);
    }
  });

  it('hides no cell at narrow widths — the grid keeps every track', () => {
    renderRows();
    for (const name of ORDER) {
      expect(cell('T01', name)?.className ?? '').not.toMatch(/\bhidden\b/);
    }
  });
});
