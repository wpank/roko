// @vitest-environment jsdom
/**
 * Acceptance for gap-195374: the portal in roko's look, and its shell. An empty project says what roko is; the top
 * bar names the project and says the one most urgent thing; each plan says its state in a word; the theme switch
 * keeps the person's pick; signed out, the page shows one way in and nothing else.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { DotMatrix } from '@/components/kit/DotMatrix';
import { Workspace } from '@/components/shell/Workspace';
import { initialRunState } from '@/lib/runState';
import { THEME_KEY } from '@/lib/theme';
import { foldEvents, renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';

vi.mock('next/navigation', () => ({
  useSearchParams: () => new URLSearchParams(window.location.search),
}));

const T0 = 1_780_000_000_000;
const WORKSPACE = { name: 'greeting-app', path: '/tmp/greeting-app', branch: 'main' };

const summary = (id: string, title: string, tasks: number): WirePlanSummary =>
  ({
    id,
    title,
    task_count: tasks,
    tasks_done: 0,
    tasks_failed: 0,
    completed: false,
    status: 'pending',
    old_format: false,
  }) as WirePlanSummary;

const GREETING = summary('greeting', 'Say hello from the command line', 3);
const BROKEN = summary('broken', 'Read settings from a file', 1);

const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };

/** The plan list, the project, and each plan's tasks and validation, as the server would answer. */
const seed = (plans: WirePlanSummary[]) => [
  [queryKeys.plans, plans] as const,
  [queryKeys.workspace, WORKSPACE] as const,
  ...plans.flatMap((p) => [
    [queryKeys.planTasks(p.id), { plan_id: p.id, task_count: 0, tasks: [] } satisfies WirePlanTasks] as const,
    [queryKeys.validation(p.id), VALID] as const,
  ]),
];

const RUNNING: WireDashboardEvent[] = [
  { type: 'plan_set_loaded', plans: [{ plan_id: 'greeting', title: GREETING.title, tasks_total: 3 }] },
  { type: 'plan_started', plan_id: 'greeting', tasks_total: 3 },
  { type: 'task_started', plan_id: 'greeting', task_id: 'T1', phase: 'implement' },
];

beforeEach(() => {
  window.history.replaceState(null, '', '/');
  setStore(initialRunState());
  stubFetch([]);
  localStorage.clear();
  document.documentElement.removeAttribute('data-theme');
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const header = () => document.querySelector('header[data-region="header"]')!;
const chip = () => header().querySelector('.rk-chip') as HTMLButtonElement;
const planRow = (id: string) => document.querySelector(`[data-plan-row="${id}"]`) as HTMLButtonElement;

describe('an empty project', () => {
  it('an empty project says what roko is and asks for a request', () => {
    renderWithClient(<Workspace />, { seed: seed([]) });
    const intro = document.querySelector('[data-region="intro"]')!;
    expect(textOf(intro)).toContain('Ask for a change.');
    expect(textOf(intro)).toContain('Get checked work.');
    expect(textOf(intro)).toMatch(/plan of small tasks, each with a check/);

    expect(screen.getByLabelText('What should roko do in greeting-app?').hasAttribute('data-prompt')).toBe(true);
    const write = document.querySelector('[data-action="submit-prompt"]')!;
    expect(write.hasAttribute('data-you')).toBe(true);
    expect(textOf(write)).toContain('write plan');

    const steps = screen.getByRole('list', { name: 'How it works' });
    expect(steps.querySelectorAll('li')).toHaveLength(4);
    expect(textOf(document.querySelector('[data-region="rail"]'))).toContain('No plans yet.');
    expect(document.querySelector('[data-action="run-all"]')).toBeNull();
  });

  it('has exactly one orange key', () => {
    renderWithClient(<Workspace />, { seed: seed([]) });
    expect(document.querySelectorAll('[data-you]')).toHaveLength(1);
  });
});

describe('the top bar', () => {
  it('names the project and its branch, beside roko’s wordmark and an idle chip', () => {
    renderWithClient(<Workspace />, { seed: seed([GREETING]) });
    expect(textOf(header().querySelector('[data-slot="workspace"]'))).toBe('greeting-app');
    expect(textOf(header())).toContain('main');
    expect(header().querySelector('svg[aria-label="roko"]')).not.toBeNull();
    expect(chip().dataset.signal).toBe('off');
    expect(textOf(chip())).toBe('idle');
    expect(chip().disabled).toBe(true);
  });

  it('says what runs, with its time on glass, and opens that plan on a click', () => {
    vi.useFakeTimers();
    vi.setSystemTime(T0 + 9_000);
    window.history.replaceState(null, '', '/?plan=broken');
    setStore(foldEvents(RUNNING, T0, 0));
    renderWithClient(<Workspace />, { seed: seed([GREETING, BROKEN]) });
    expect(planRow('broken').getAttribute('aria-current')).toBe('true');
    expect(chip().dataset.slot).toBe('run');
    expect(textOf(chip())).toBe('1 running');
    expect(chip().querySelector('svg[aria-label="9 seconds"]')).not.toBeNull();
    act(() => {
      fireEvent.click(chip());
    });
    expect(planRow('greeting').getAttribute('aria-current')).toBe('true');
  });
});

describe('the plan list', () => {
  it('says each plan’s state in a word, with a square per task', () => {
    vi.useFakeTimers();
    vi.setSystemTime(T0 + 9_000);
    setStore(foldEvents(RUNNING, T0, 0));
    renderWithClient(<Workspace />, { seed: seed([GREETING, BROKEN]) });
    expect(planRow('greeting').dataset.signal).toBe('live');
    expect(textOf(planRow('greeting'))).toContain('running · 0 of 3 · 0:09');
    expect(planRow('greeting').querySelectorAll('.rk-strip i')).toHaveLength(3);
    expect(planRow('broken').dataset.signal).toBe('waiting');
    expect(textOf(planRow('broken'))).toContain('ready · 1 task');
  });

  it('offers run all once there are two plans, and new plan always', () => {
    const { unmount } = renderWithClient(<Workspace />, { seed: seed([GREETING]) });
    expect(document.querySelector('[data-action="run-all"]')).toBeNull();
    expect(document.querySelector('[data-action="new-plan"]')).not.toBeNull();
    unmount();
    renderWithClient(<Workspace />, { seed: seed([GREETING, BROKEN]) });
    expect(document.querySelector('[data-action="run-all"]')).not.toBeNull();
  });

  it('opens the request box under its own heading for a new plan', () => {
    renderWithClient(<Workspace />, { seed: seed([GREETING, BROKEN]) });
    act(() => {
      fireEvent.click(document.querySelector('[data-action="new-plan"]')!);
    });
    expect(screen.getByRole('heading', { name: 'New plan' })).not.toBeNull();
    expect(document.querySelector('[data-prompt]')).not.toBeNull();
    expect(textOf(document.querySelector('[data-region="stage"]'))).toContain(
      'roko writes a plan first: small tasks, each with a check. Nothing runs until you say so.',
    );
  });
});

describe('day and night', () => {
  it('the switch picks night, keeps it, and sets it on the page', () => {
    renderWithClient(<Workspace />, { seed: seed([GREETING]) });
    const sw = screen.getByRole('switch', { name: 'Night theme' });
    act(() => {
      fireEvent.click(sw);
    });
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    expect(localStorage.getItem(THEME_KEY)).toBe('dark');
    expect(sw.getAttribute('aria-checked')).toBe('true');
  });
});

describe('signed out', () => {
  it('shows one way in and nothing that pretends to work', () => {
    setStore(initialRunState(), { connection: 'unauthorized' });
    renderWithClient(<Workspace />, { seed: seed([GREETING]) });
    const page = document.querySelector('[data-region="sign-in"]')!;
    expect(textOf(page)).toContain('Open roko with its sign-in link');
    expect(textOf(page)).toContain('#token=…');
    expect(document.querySelector('[data-region="rail"]')).toBeNull();
    expect(document.querySelector('[data-prompt]')).toBeNull();
    expect(textOf(chip())).toBe('signed out');
  });
});

describe('a reading on the glass', () => {
  it('is dots for the eye and plain words for a screen reader', () => {
    render(<DotMatrix text="4:05" label="4 minutes 5 seconds" tone="live" />);
    const svg = screen.getByRole('img', { name: '4 minutes 5 seconds' });
    expect(svg.textContent).toBe('');
    expect(svg.closest('[data-tone="live"]')).not.toBeNull();
  });
});
