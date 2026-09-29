// @vitest-environment jsdom
/**
 * A task row's check chips take their glyph and colour from the shared step
 * mapping in lib/glyphs.ts, as the stream's checks and the run band do: passed
 * ✓ green, failed ✗ red, running ► indigo and pulsing.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanTask } from '@/api/contracts';
import { TaskList } from '@/components/stage/TaskList';
import { buildTaskRows } from '@/lib/taskRows';
import { foldEvents } from '@/test/dom';

afterEach(() => cleanup());

const TASKS: WirePlanTask[] = [
  { id: 'T01', title: 'Print hello world', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural', 'compile'] },
  { id: 'T02', title: 'Add an integration test', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['test'] },
];

const RUN: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
  { type: 'gate_result', plan_id: 'hello', task_id: 'T01', gate: 'verify[0:structural]', passed: true, output_text: '$ true' },
  { type: 'gate_result', plan_id: 'hello', task_id: 'T01', gate: 'verify[1:compile]', passed: false, output_text: '$ cargo build\n✗ exit status 1' },
  { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'failed' },
  { type: 'task_started', plan_id: 'hello', task_id: 'T02', phase: 'implement' },
  { type: 'gate_rung_started', plan_id: 'hello', task_id: 'T02', rung_name: 'verify[0:test]' },
];

function chips(id: string): string[] {
  const cell = document.querySelector(`[data-task-row="${id}"] [data-cell="checks"]`)!;
  return [...cell.querySelectorAll(':scope > span')].map((chip) => chip.outerHTML);
}

describe('check chips', () => {
  it('draw passed, failed and running steps with their glyph, colour and tooltip', () => {
    const { rows } = buildTaskRows(TASKS, foldEvents(RUN, 1_000, 1_000), 'hello', 60_000);
    render(<TaskList rows={rows} selectedTaskId={null} onSelectTask={() => {}} />);
    expect(chips('T01')).toEqual([
      '<span class="inline-block text-xs font-mono" title="verify[0:structural]: passed">'
        + '<span data-glyph="done" aria-label="done" title="verify[0:structural]: passed" style="color: var(--state-done);">✓</span>'
        + ' <span style="color: var(--state-done);">structural</span></span>',
      '<span class="inline-block text-xs font-mono" title="verify[1:compile]: failed">'
        + '<span data-glyph="failed" aria-label="failed" title="verify[1:compile]: failed" style="color: var(--state-failed);">✗</span>'
        + ' <span style="color: var(--state-failed);">compile</span></span>',
    ]);
    expect(chips('T02')).toEqual([
      '<span class="inline-block text-xs font-mono" title="verify[0:test]: running">'
        + '<span data-glyph="active" aria-label="active" title="verify[0:test]: running" style="color: var(--state-active);" class="rd-pulse">►</span>'
        + ' <span style="color: var(--state-active);">test</span></span>',
    ]);
  });
});
