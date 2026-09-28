// @vitest-environment jsdom
/**
 * Acceptance: an open editor blocks Run only while it holds unsaved text; the
 * plan title leads the type scale; Run and Cancel use the button tokens whose
 * text passes contrast.
 * Copied verbatim from plans/portal-programme/08d-portal-legibility/accept/.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, screen, waitFor } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { PlanView } from '@/components/stage/PlanView';
import { initialRunState } from '@/lib/runState';
import { foldEvents, renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';

const PLAN: WirePlanSummary = {
  id: 'hello',
  title: 'Hello world',
  task_count: 2,
  tasks_done: 0,
  tasks_failed: 0,
  completed: false,
  status: 'pending',
  old_format: false,
};
const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 2,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: 'Scaffold', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural'] },
    { id: 'T02', title: 'Print', tier: 'focused', status: 'pending', depends_on: ['T01'], files: [], completed: false, verify_phases: ['structural'] },
  ],
};
const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };
const SET: WireDashboardEvent = { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 2 }] };
const DONE: WireDashboardEvent[] = [
  SET,
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
  { type: 'plan_completed', plan_id: 'hello', success: true },
];
const TOML = '[meta]\nplan = "hello"\n';

beforeEach(() => setStore(initialRunState()));
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function renderView() {
  return renderWithClient(
    <PlanView plan={PLAN} selectedTaskId={null} onSelectTask={() => {}} onRequestError={() => {}} />,
    { seed: [[queryKeys.planTasks('hello'), TASKS], [queryKeys.validation('hello'), VALID]] },
  );
}

const primary = () =>
  document.querySelector('[data-action="run"], [data-action="run-again"], [data-action="retry"], [data-action="cancel"]') as HTMLButtonElement;

describe('the editor and Run', () => {
  it('leaves Run again available when the editor only shows the not-supported notice', async () => {
    stubFetch([{ path: '/api/plans/hello/source', status: 404 }]);
    setStore(foldEvents(DONE));
    renderView();
    fireEvent.click(document.querySelector('[data-action="edit"]')!);
    await waitFor(() => expect(document.querySelector('[data-notice="unsupported"]')).not.toBeNull());
    expect(primary().getAttribute('data-action')).toBe('run-again');
    expect(primary().disabled).toBe(false);
    expect(document.querySelector('[data-reason]')).toBeNull();
  });

  it('blocks Run, with the reason, while the editor holds unsaved text', async () => {
    stubFetch([{ path: '/api/plans/hello/source', body: { id: 'hello', path: 'plans/hello/tasks.toml', toml: TOML } }]);
    renderView();
    fireEvent.click(document.querySelector('[data-action="edit"]')!);
    const editor = (await screen.findByLabelText('Plan source TOML')) as HTMLTextAreaElement;
    await waitFor(() => expect(editor.value).toBe(TOML));
    expect(primary().disabled).toBe(false);
    fireEvent.change(editor, { target: { value: TOML + '# edit\n' } });
    expect(primary().disabled).toBe(true);
    expect(textOf(document.querySelector('[data-reason]'))).toBe('Save or discard your edits first');
  });

  it('frees Run again when the edited text is discarded', async () => {
    stubFetch([{ path: '/api/plans/hello/source', body: { id: 'hello', path: 'plans/hello/tasks.toml', toml: TOML } }]);
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    renderView();
    fireEvent.click(document.querySelector('[data-action="edit"]')!);
    const editor = (await screen.findByLabelText('Plan source TOML')) as HTMLTextAreaElement;
    await waitFor(() => expect(editor.value).toBe(TOML));
    fireEvent.change(editor, { target: { value: TOML + '# edit\n' } });
    fireEvent.click(document.querySelector('[data-action="edit"]')!);
    expect(primary().disabled).toBe(false);
  });
});

describe('the plan header', () => {
  it('sets the title in the title style and the id as meta', () => {
    renderView();
    const title = document.querySelector('h2');
    expect(textOf(title)).toBe('Hello world');
    expect(title?.classList.contains('rd-title')).toBe(true);
    expect(title?.className).not.toMatch(/\btext-(xs|sm)\b/);
    const id = [...document.querySelectorAll('.rd-meta')].find((e) => textOf(e) === 'hello');
    expect(id).toBeDefined();
  });

  it('draws Run with the primary button tokens and Cancel with the danger fill', () => {
    renderView();
    expect(primary().className).toContain('bg-button-primary-bg');
    expect(primary().className).toContain('text-button-primary-fg');
    cleanup();
    setStore(foldEvents([SET, { type: 'plan_started', plan_id: 'hello', tasks_total: 2 }]));
    renderView();
    expect(primary().getAttribute('data-action')).toBe('cancel');
    expect(primary().className).toContain('bg-danger-fill');
    expect(primary().className).toContain('text-danger-fill-fg');
  });

  it('draws the validation badge at the meta size or larger', () => {
    renderView();
    const badge = screen.getByText('valid ✓');
    expect(badge.className).not.toMatch(/text-\[(9|10|11)px\]/);
  });
});
