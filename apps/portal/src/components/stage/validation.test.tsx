// @vitest-environment jsdom
/**
 * The validation badge, Run and the alert follow the server's report (ask P-3,
 * bug-48494b): the portal validates the saved plan without a body, an invalid
 * plan shows its errors and cannot run, and a failed validation request never
 * reads as valid.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, screen, waitFor } from '@testing-library/react';
import type { WirePlanSummary, WirePlanTasks, WireValidation } from '@/api/contracts';
import { createQueryClient } from '@/api/queryClient';
import { queryKeys } from '@/api/queries';
import { Workspace } from '@/components/shell/Workspace';
import { PlanView } from '@/components/stage/PlanView';
import { ValidationBadge, validationCounts } from '@/components/stage/ValidationBadge';
import { OUTDATED_SERVER } from '@/lib/apiErrors';
import { initialRunState } from '@/lib/runState';
import { renderWithClient, setStore, stubFetch, textOf } from '@/test/dom';
import type { FetchRoute } from '@/test/dom';

vi.mock('next/navigation', () => ({
  useSearchParams: () => new URLSearchParams(window.location.search),
}));

const VALIDATE = '/api/plans/live-b/validate';

const PLAN: WirePlanSummary = {
  id: 'live-b',
  title: 'Live B',
  task_count: 1,
  tasks_done: 0,
  tasks_failed: 0,
  completed: false,
  status: 'ready',
  old_format: false,
};

const TASKS: WirePlanTasks = {
  plan_id: 'live-b',
  task_count: 1,
  max_parallel: 1,
  tasks: [
    { id: 'T01', title: 'Fixture task T01', tier: 'focused', status: 'pending', depends_on: ['T99'], files: [], completed: false, verify_phases: ['structural'] },
  ],
};

const VALID: WireValidation = { valid: true, errors: [], warnings: [], diagnostics: [] };

/** The server's report for live-b edited to `depends_on = ["T99"]`. */
const INVALID: WireValidation = {
  valid: false,
  errors: ["PLAN_005: task 'T01' depends on unknown task 'T99'"],
  warnings: [],
  diagnostics: [
    { severity: 'error', rule_id: 'PLAN_005', task_id: 'T01', message: "task 'T01' depends on unknown task 'T99'" },
  ],
};

beforeEach(() => {
  setStore(initialRunState());
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

function renderBadge(opts: { client?: ReturnType<typeof createQueryClient> } = {}) {
  return renderWithClient(<ValidationBadge planId="live-b" onSelectTask={() => {}} />, opts);
}

describe('validation badge', () => {
  it('validates the saved plan without a request body', async () => {
    const fetchMock = stubFetch([{ method: 'POST', path: VALIDATE, body: VALID }]);
    renderBadge({ client: createQueryClient() });
    expect(await screen.findByText('valid ✓')).toBeTruthy();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const init = fetchMock.mock.calls[0]![1] as RequestInit;
    expect(init.method).toBe('POST');
    expect(init.body).toBeUndefined();
  });

  it('shows the error count of an invalid plan', async () => {
    stubFetch([{ method: 'POST', path: VALIDATE, body: INVALID }]);
    renderBadge();
    expect(await screen.findByText('1 error')).toBeTruthy();
    expect(textOf(document.body)).not.toContain('valid ✓');
  });

  it('reads an older server’s counts from the diagnostics', async () => {
    stubFetch([
      {
        method: 'POST',
        path: VALIDATE,
        body: {
          valid: false,
          errors: 1,
          warnings: 1,
          diagnostics: [
            ...INVALID.diagnostics,
            { severity: 'warning', rule_id: 'PLAN_031', task_id: 'T01', message: "task 'T01' reads a missing file" },
          ],
        },
      },
    ]);
    renderBadge();
    expect(await screen.findByText('1 error')).toBeTruthy();
  });

  const failures: Array<[string, FetchRoute | null, string]> = [
    [
      'an older server rejects the body (400)',
      {
        method: 'POST',
        path: VALIDATE,
        status: 400,
        body: {
          code: 'invalid_json',
          message: 'request body must be valid JSON',
          details: { reason: 'missing field `toml` at line 1 column 2' },
        },
      },
      OUTDATED_SERVER,
    ],
    [
      'the plan is gone (404)',
      { method: 'POST', path: VALIDATE, status: 404, body: { code: 'not_found', message: "plan 'live-b' not found" } },
      "plan 'live-b' not found",
    ],
    [
      'the server fails (500)',
      { method: 'POST', path: VALIDATE, status: 500, body: { code: 'internal_error', message: 'validate source for plan live-b: disk full' } },
      'validate source for plan live-b: disk full',
    ],
    ['the server is unreachable', null, `unexpected request: POST ${VALIDATE}`],
  ];

  it.each(failures)(
    'does not claim valid when the validation request fails: %s',
    async (_case, route, reason) => {
      stubFetch(route ? [route] : []);
      renderBadge();
      const failed = await screen.findByText('validation failed');
      expect(failed.getAttribute('title')).toBe(reason);
      expect(textOf(document.body)).not.toContain('valid ✓');
    },
  );
});

describe('validationCounts', () => {
  it('counts errors and warnings from the diagnostics', () => {
    expect(validationCounts(INVALID)).toEqual({ errors: 1, warnings: 0 });
    expect(validationCounts(VALID)).toEqual({ errors: 0, warnings: 0 });
    expect(validationCounts(undefined)).toEqual({ errors: 0, warnings: 0 });
  });

  it('counts a report the server did not call valid as at least one error', () => {
    expect(validationCounts({ valid: false, errors: [], warnings: [], diagnostics: [] })).toEqual({
      errors: 1,
      warnings: 0,
    });
    expect(validationCounts({ diagnostics: [] } as unknown as WireValidation).errors).toBe(1);
  });
});

describe('an invalid plan', () => {
  it('disables Run, with the reason', async () => {
    stubFetch([{ method: 'POST', path: VALIDATE, body: INVALID }]);
    renderWithClient(
      <PlanView plan={PLAN} selectedTaskId={null} onSelectTask={() => {}} onRequestError={() => {}} />,
      { seed: [[queryKeys.planTasks('live-b'), TASKS]] },
    );
    await waitFor(() =>
      expect(textOf(document.querySelector('[data-region="status-line"]'))).toContain('1 error'),
    );
    const run = document.querySelector('[data-action="run"]') as HTMLButtonElement;
    expect(run.disabled).toBe(true);
    expect(textOf(document.querySelector('[data-reason]'))).toBe('Fix 1 validation error first');
  });

  it('raises the validation alert with its error count', () => {
    window.history.replaceState(null, '', '/?plan=live-b');
    stubFetch([]);
    renderWithClient(<Workspace />, {
      seed: [
        [queryKeys.plans, [PLAN]],
        [queryKeys.workspace, { name: 'live-ws', path: '/tmp/live-ws', branch: 'main' }],
        [queryKeys.planTasks('live-b'), TASKS],
        [queryKeys.validation('live-b'), INVALID],
      ],
    });
    expect(textOf(document.querySelector('[data-region="alert"]'))).toContain('1 validation error');
    expect(textOf(document.querySelector('[data-region="alert"]'))).not.toContain('1 validation errors');
  });
});
