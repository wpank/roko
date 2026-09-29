// @vitest-environment jsdom
/**
 * Acceptance: keys and selection follow design §9 — Esc closes the editor or
 * prompt (the ✦ Revise prompt too), an unknown id clears silently, and with no
 * plan on load the first running plan is selected; the selection never jumps
 * on its own afterwards, however many plans start.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Workspace } from '@/components/shell/Workspace';
import { initialRunState } from '@/lib/runState';
import { foldEvents, renderWithClient, setStore, stubFetch } from '@/test/dom';

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
  { id: 'hello', title: 'Hello world' },
  { id: 'notes', title: 'Notes app' },
].map((p) => ({ ...p, task_count: 2, tasks_failed: 0, completed: false, status: 'pending', old_format: false }) as WirePlanSummary);

const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 2,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: 'Scaffold the hello project', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural'] },
    { id: 'T02', title: 'Print hello world', tier: 'focused', status: 'pending', depends_on: ['T01'], files: [], completed: false, verify_phases: ['structural'] },
  ],
};

const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };

const SEED = [
  [queryKeys.plans, PLANS],
  [queryKeys.workspace, { name: 'ws', path: '/tmp/ws', branch: 'main' }],
  [queryKeys.planTasks('hello'), TASKS],
  [queryKeys.validation('hello'), VALID],
] as const;

const HELLO_RUNNING: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'hello', title: 'Hello world', tasks_total: 2 }] },
  { type: 'plan_started', plan_id: 'hello', tasks_total: 2 },
];

beforeEach(() => {
  window.history.replaceState(null, '', '/?plan=hello');
  setStore(initialRunState());
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

/** Press `key` where the operator is: the focused element, else the page. */
const press = (key: string) => fireEvent.keyDown(document.activeElement ?? document.body, { key });
const prompt = () => document.querySelector('[data-prompt]');
const search = () => window.location.search;

describe('Esc', () => {
  it('Esc closes the revise prompt', () => {
    stubFetch([]);
    renderWithClient(<Workspace />, { seed: SEED });
    fireEvent.click(document.querySelector('[data-action="revise"]')!);
    expect(prompt()).not.toBeNull();
    expect(document.activeElement).toBe(prompt());

    press('Escape');
    expect(prompt()).toBeNull();
    expect(document.querySelectorAll('[data-task-row]')).toHaveLength(2);
  });

  it('closes the generate prompt, but not while it waits for the generator', () => {
    // The generate request never answers, so the prompt keeps waiting.
    vi.stubGlobal('fetch', vi.fn(() => new Promise(() => {})));
    renderWithClient(<Workspace />, { seed: SEED });
    press('n');
    expect(prompt()).not.toBeNull();
    press('Escape');
    expect(prompt()).toBeNull();

    press('n');
    fireEvent.change(prompt()!, { target: { value: 'a rust app that prints hello world' } });
    fireEvent.click(document.querySelector('[data-action="submit-prompt"]')!);
    expect((prompt() as HTMLTextAreaElement).disabled).toBe(true);
    press('Escape');
    expect(prompt()).not.toBeNull();
  });
});

describe('selection', () => {
  it('clears an unknown task id silently and keeps a known one', () => {
    stubFetch([]);
    window.history.replaceState(null, '', '/?plan=hello&task=T99');
    const { unmount } = renderWithClient(<Workspace />, { seed: SEED });
    expect(search()).toBe('?plan=hello');
    unmount();

    window.history.replaceState(null, '', '/?plan=hello&task=T02');
    renderWithClient(<Workspace />, { seed: SEED });
    expect(search()).toBe('?plan=hello&task=T02');
  });

  it('selects the first running plan when there is no plan on load', () => {
    stubFetch([]);
    window.history.replaceState(null, '', '/');
    setStore(foldEvents(HELLO_RUNNING));
    renderWithClient(<Workspace />, { seed: SEED });
    expect(search()).toBe('?plan=hello');
  });

  it('waits for the run snapshot before choosing a running plan', () => {
    stubFetch([]);
    window.history.replaceState(null, '', '/');
    setStore(foldEvents(HELLO_RUNNING), { connection: 'connecting' });
    renderWithClient(<Workspace />, { seed: SEED });
    expect(search()).toBe('');

    act(() => setStore(foldEvents(HELLO_RUNNING)));
    expect(search()).toBe('?plan=hello');
  });

  it('never selects a plan that starts after load', () => {
    stubFetch([]);
    window.history.replaceState(null, '', '/');
    renderWithClient(<Workspace />, { seed: SEED });
    expect(search()).toBe('');

    act(() => setStore(foldEvents(HELLO_RUNNING)));
    expect(search()).toBe('');
    expect(document.querySelector('[data-slot="run"]')).not.toBeNull();
  });
});
