// @vitest-environment jsdom
/**
 * Acceptance: on the whole screen, a Run all this server cannot do is an info
 * alert with the notice glyph, and a real failure is still an error.
 * Copied verbatim from plans/portal-programme/08d-portal-legibility/accept/.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, waitFor } from '@testing-library/react';
import type { WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { Workspace } from '@/components/shell/Workspace';
import { initialRunState } from '@/lib/runState';
import { renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';

vi.mock('next/navigation', () => ({
  useSearchParams: () => new URLSearchParams(window.location.search),
}));

const PLANS = [{ id: 'hello', title: 'Hello world', task_count: 1, tasks_done: 0, tasks_failed: 0, completed: false, status: 'pending', old_format: false }] as WirePlanSummary[];
const TASKS: WirePlanTasks = {
  plan_id: 'hello',
  task_count: 1,
  max_parallel: 1,
  tasks: [{ id: 'T01', title: 'Scaffold', tier: 'focused', status: 'pending', depends_on: [], files: [], completed: false, verify_phases: ['structural'] }],
};
const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };
const SEED = [
  [queryKeys.plans, PLANS],
  [queryKeys.workspace, { name: 'hello', path: '/tmp/hello', branch: null }],
  [queryKeys.planTasks('hello'), TASKS],
  [queryKeys.validation('hello'), VALID],
] as const;

beforeEach(() => {
  window.history.replaceState(null, '', '/?plan=hello');
  setStore(initialRunState());
  vi.spyOn(window, 'confirm').mockReturnValue(true);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const band = () => document.querySelector('[data-region="alert"]');

describe('request alerts on the screen', () => {
  it('shows a Run all this server lacks as information, with the notice glyph', async () => {
    stubFetch([{ method: 'POST', path: '/api/plans/execute', status: 405, statusText: 'Method Not Allowed' }]);
    renderWithClient(<Workspace />, { seed: SEED });
    fireEvent.click(document.querySelector('[data-action="run-all"]')!);
    await waitFor(() => expect(band()).not.toBeNull());
    expect(band()?.getAttribute('data-severity')).toBe('info');
    expect(textOf(band()?.querySelector('.rd-alert__glyph') ?? null)).toBe('ⓘ');
    expect(textOf(band())).toContain('This roko serve does not support running all plans yet.');
  });

  it('keeps a real failure an error', async () => {
    stubFetch([
      { method: 'POST', path: '/api/plans/execute', status: 409, body: { code: 'conflict', message: 'a plan-set run is already active' } },
    ]);
    renderWithClient(<Workspace />, { seed: SEED });
    fireEvent.click(document.querySelector('[data-action="run-all"]')!);
    await waitFor(() => expect(band()).not.toBeNull());
    expect(band()?.getAttribute('data-severity')).toBe('error');
    expect(textOf(band())).toContain('a plan-set run is already active');
  });
});
