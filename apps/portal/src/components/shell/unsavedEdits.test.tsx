// @vitest-environment jsdom
/**
 * The editor's unsaved text is never bypassed or dropped silently (design
 * §4.1, §4a; bug-0522e8): no run starts past it — not the `r` key, not Run
 * all — and every way of leaving the editor (the Edit toggle, Revise, another
 * plan, a new plan) asks first. One plan's text never lands in another plan.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { MockInstance } from 'vitest';
import { act, cleanup, fireEvent, screen, waitFor } from '@testing-library/react';
import type { WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Workspace } from '@/components/shell/Workspace';
import { initialRunState } from '@/lib/runState';
import { renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';

// As in Next 15, replaceState re-renders every reader of the search params.
vi.mock('next/navigation', async () => {
  const React = await import('react');
  const listeners = new Set<() => void>();
  const replaceState = window.history.replaceState.bind(window.history);
  window.history.replaceState = (...args: Parameters<History['replaceState']>) => {
    replaceState(...args);
    listeners.forEach((notify) => notify());
  };
  const subscribe = (notify: () => void) => {
    listeners.add(notify);
    return () => listeners.delete(notify);
  };
  return {
    useSearchParams: () => {
      const search = React.useSyncExternalStore(subscribe, () => window.location.search);
      return React.useMemo(() => new URLSearchParams(search), [search]);
    },
  };
});

const PLANS = [
  { id: 'hello', title: 'Hello world', task_count: 1 },
  { id: 'other', title: 'Other plan', task_count: 1 },
].map((p) => ({ ...p, tasks_failed: 0, completed: false, status: 'pending', old_format: false }) as WirePlanSummary);

const tasks = (plan: string): WirePlanTasks => ({
  plan_id: plan,
  task_count: 1,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: `The ${plan} task`, tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural'] },
  ],
});

const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };
const toml = (plan: string) => `[meta]\nplan = "${plan}"\n`;
const EDITED = toml('hello') + '# unsaved\n';

let fetchMock: ReturnType<typeof stubFetch>;
let confirm: MockInstance<(message?: string) => boolean>;

beforeEach(() => {
  window.history.replaceState(null, '', '/?plan=hello');
  setStore(initialRunState());
  fetchMock = stubFetch([
    { path: '/api/plans', body: PLANS },
    ...['hello', 'other'].map((plan) => ({
      path: `/api/plans/${plan}/source`,
      body: { id: plan, path: `plans/${plan}/tasks.toml`, toml: toml(plan) },
    })),
    { method: 'POST', path: '/api/plans/hello/execute', status: 202, body: { id: 'op-run', plan_id: 'hello' } },
  ]);
  confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const action = (name: string) => document.querySelector(`[data-action="${name}"]`) as HTMLButtonElement | null;
const editor = () => screen.queryByLabelText('Plan source TOML') as HTMLTextAreaElement | null;
const planInUrl = () => new URLSearchParams(window.location.search).get('plan');
const runsStarted = () =>
  fetchMock.mock.calls.filter(([url, init]) => String(url).endsWith('/api/plans/hello/execute') && init?.method === 'POST')
    .length;
/** Give a run the key may have started time to reach fetch. */
const settle = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

/** Render the workspace on plan hello and open its editor, with the saved text loaded. */
async function openEditor() {
  renderWithClient(<Workspace />, {
    seed: [
      [queryKeys.plans, PLANS],
      [queryKeys.workspace, { name: 'ws', path: '/tmp/ws', branch: 'main' }],
      ...['hello', 'other'].flatMap((plan) => [
        [queryKeys.planTasks(plan), tasks(plan)] as const,
        [queryKeys.validation(plan), VALID] as const,
      ]),
    ],
  });
  fireEvent.click(action('edit')!);
  const el = (await screen.findByLabelText('Plan source TOML')) as HTMLTextAreaElement;
  await waitFor(() => expect(el.value).toBe(toml('hello')));
  return el;
}

/** Open the editor and type text that is not saved. */
async function editUnsaved() {
  fireEvent.change(await openEditor(), { target: { value: EDITED } });
}

describe('protects unsaved edits', () => {
  it('from the r key: no run starts while the editor holds unsaved text', async () => {
    await editUnsaved();
    fireEvent.keyDown(window, { key: 'r' });
    await settle();
    expect(runsStarted()).toBe(0);
    expect(action('run')!.disabled).toBe(true);
    expect(textOf(document.querySelector('[data-reason]'))).toBe('Save or discard your edits first');

    confirm.mockReturnValue(true);
    fireEvent.click(action('edit')!);
    fireEvent.keyDown(window, { key: 'r' });
    await waitFor(() => expect(runsStarted()).toBe(1));
  });

  it('from the Edit toggle: it asks, and keeps the text unless told to discard it', async () => {
    await editUnsaved();
    fireEvent.click(action('edit')!);
    expect(confirm).toHaveBeenCalledWith('Discard unsaved edits?');
    expect(editor()!.value).toBe(EDITED);
    expect(action('run')!.disabled).toBe(true);

    confirm.mockReturnValue(true);
    fireEvent.click(action('edit')!);
    expect(editor()).toBeNull();
    expect(action('run')!.disabled).toBe(false);
  });

  it('from the Revise button: it asks before the prompt replaces the editor', async () => {
    await editUnsaved();
    fireEvent.click(action('revise')!);
    expect(confirm).toHaveBeenCalledWith('Discard unsaved edits?');
    expect(editor()!.value).toBe(EDITED);
    expect(document.querySelector('[data-prompt]')).toBeNull();

    confirm.mockReturnValue(true);
    fireEvent.click(action('revise')!);
    expect(editor()).toBeNull();
    expect(document.querySelector('[data-prompt]')).not.toBeNull();
  });

  it('from another plan: it asks, and never carries the text into that plan', async () => {
    await editUnsaved();
    fireEvent.click(document.querySelector('[data-plan-row="other"]')!);
    fireEvent.keyDown(window, { key: 'ArrowDown' });
    expect(confirm).toHaveBeenCalledTimes(2);
    expect(planInUrl()).toBe('hello');
    expect(editor()!.value).toBe(EDITED);

    confirm.mockReturnValue(true);
    fireEvent.click(document.querySelector('[data-plan-row="other"]')!);
    expect(planInUrl()).toBe('other');
    expect(editor()).toBeNull();
    expect(textOf(document.querySelector('[data-region="stage"]'))).toContain('The other task');

    fireEvent.click(action('edit')!);
    await waitFor(() => expect(editor()?.value).toBe(toml('other')));
  });

  it('from a new plan: it asks before the generate field replaces the editor', async () => {
    await editUnsaved();
    fireEvent.keyDown(window, { key: 'n' });
    expect(confirm).toHaveBeenCalledTimes(1);
    expect(document.querySelector('[data-prompt]')).toBeNull();
    expect(editor()!.value).toBe(EDITED);

    confirm.mockReturnValue(true);
    fireEvent.click(action('new-plan')!);
    expect(document.querySelector('[data-prompt]')).not.toBeNull();
    expect(editor()).toBeNull();
  });

  it('from Run all: it waits until the text is saved or discarded', async () => {
    await editUnsaved();
    expect(action('run-all')!.disabled).toBe(true);

    confirm.mockReturnValue(true);
    fireEvent.click(action('edit')!);
    expect(action('run-all')!.disabled).toBe(false);
  });

  it('asks nothing while the editor text is saved', async () => {
    await openEditor();
    fireEvent.click(document.querySelector('[data-plan-row="other"]')!);
    expect(planInUrl()).toBe('other');
    fireEvent.click(action('edit')!);
    await screen.findByLabelText('Plan source TOML');
    fireEvent.click(action('edit')!);
    fireEvent.keyDown(window, { key: 'n' });
    expect(document.querySelector('[data-prompt]')).not.toBeNull();
    expect(confirm).not.toHaveBeenCalled();
  });
});
