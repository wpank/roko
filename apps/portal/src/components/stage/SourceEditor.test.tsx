// @vitest-environment jsdom
/**
 * A rejected save lists each diagnostic with its rule id, and a task id that
 * jumps to that task's `[[task]]` line (design §4a, gap-cb9274).
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, screen, waitFor } from '@testing-library/react';
import type { WireDiagnostic } from '@/api/contracts';
import { SourceEditor, findTaskLine } from '@/components/stage/SourceEditor';
import { renderWithClient, stubFetch, textOf } from '@/test/dom';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

/** Three tasks, each `id` spelled differently. */
const TOML = [
  '[meta]',
  'plan = "hello"',
  '',
  '[[task]]',
  'id = "T01"',
  'title = "Scaffold the project"',
  '',
  '[[task.verify]]',
  'phase = "structural"',
  'command = "test -f hello/Cargo.toml"',
  '',
  '[[task]]',
  'title = "Print hello world"',
  "id='T02'",
  'depends_on = ["T99"]',
  '',
  '[[task]]   # the last one',
  '  id   =   "T03"',
  '',
].join('\n');

/** Offset of the n-th (0-based) `[[task]]` line in TOML. */
function taskLine(n: number): number {
  let at = -1;
  for (let i = 0; i <= n; i++) at = TOML.indexOf('[[task]]', at + 1);
  return at;
}

const DIAGNOSTICS: WireDiagnostic[] = [
  { severity: 'error', rule_id: 'PLAN_005', task_id: 'T02', message: "task 'T02' depends on unknown task 'T99'" },
  { severity: 'warning', rule_id: 'PLAN_031', task_id: 'T01', message: "task 'T01' reads 'docs/missing.md', which does not exist" },
  { severity: 'warning', rule_id: 'PLAN_020', message: 'the plan has no estimate' },
];

describe('findTaskLine', () => {
  it('finds a task whatever the spacing and quotes of its id', () => {
    expect(findTaskLine(TOML, 'T01')).toBe(taskLine(0));
    expect(findTaskLine(TOML, 'T02')).toBe(taskLine(1));
    expect(findTaskLine(TOML, 'T03')).toBe(taskLine(2));
  });

  it('ignores ids outside a task’s own table, and unknown tasks', () => {
    expect(findTaskLine(TOML, 'T99')).toBeNull();
    expect(findTaskLine('[[task]]\nid = "T01"\n\n[task.context]\nid = "T04"\n', 'T04')).toBeNull();
    expect(findTaskLine('[meta]\nid = "T05"\n', 'T05')).toBeNull();
  });
});

describe('SourceEditor diagnostics', () => {
  async function rejectSave() {
    stubFetch([
      { path: '/api/plans/hello/source', body: { id: 'hello', path: 'plans/hello/tasks.toml', toml: TOML } },
      {
        method: 'PUT',
        path: '/api/plans/hello/source',
        status: 422,
        body: {
          code: 'invalid_plan',
          message: DIAGNOSTICS[0]!.message,
          errors: ["PLAN_005: task 'T02' depends on unknown task 'T99'"],
          warnings: [],
          diagnostics: DIAGNOSTICS,
        },
      },
    ]);
    renderWithClient(<SourceEditor planId="hello" running={false} onClose={() => {}} />);
    const editor = (await screen.findByLabelText('Plan source TOML')) as HTMLTextAreaElement;
    await waitFor(() => expect(editor.value).toBe(TOML));
    fireEvent.click(document.querySelector('[data-action="save-source"]')!);
    await waitFor(() => expect(document.querySelectorAll('[data-diagnostic]')).toHaveLength(3));
    return editor;
  }

  it('shows the rule id and the task of each diagnostic after a rejected save', async () => {
    await rejectSave();
    const rows = [...document.querySelectorAll('[data-diagnostic]')];
    expect(rows.map((row) => textOf(row.querySelector('[data-rule-id]')))).toEqual([
      'PLAN_005',
      'PLAN_031',
      'PLAN_020',
    ]);
    expect(textOf(rows[0]!)).toBe("error PLAN_005 T02: task 'T02' depends on unknown task 'T99'");
    const link = rows[0]!.querySelector('[data-task-link]')!;
    expect(link.tagName).toBe('BUTTON');
    expect(link.getAttribute('data-task-link')).toBe('T02');
    expect(rows[2]!.querySelector('[data-task-link]')).toBeNull();
  });

  it('moves the caret to the first task named once the save is rejected', async () => {
    const editor = await rejectSave();
    await waitFor(() => expect(editor.selectionStart).toBe(taskLine(1)));
    expect(document.activeElement).toBe(editor);
  });

  it('clicking a task id moves the caret to that task’s [[task]] line', async () => {
    const editor = await rejectSave();
    fireEvent.click(document.querySelector('[data-task-link="T01"]')!);
    expect(document.activeElement).toBe(editor);
    expect(editor.selectionStart).toBe(taskLine(0));
    expect(editor.selectionEnd).toBe(taskLine(0));

    fireEvent.click(document.querySelector('[data-task-link="T02"]')!);
    expect(editor.selectionStart).toBe(taskLine(1));
    expect(editor.value).toBe(TOML);
  });
});
