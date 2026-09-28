// @vitest-environment jsdom
/**
 * Acceptance: against a roko serve that lacks a route, the editor and the
 * validation badge say so at once instead of retrying behind a spinner.
 * Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, screen } from '@testing-library/react';
import { createQueryClient } from '@/api/queryClient';
import { SourceEditor } from '@/components/stage/SourceEditor';
import { ValidationBadge } from '@/components/stage/ValidationBadge';
import { renderWithClient, stubFetch } from '@/test/dom';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe('missing routes', () => {
  it('the editor reports a missing source route without retrying', async () => {
    const fetchMock = stubFetch([
      {
        path: '/api/plans/hello/source',
        status: 404,
        body: { error: 'not_found', message: 'No route matches /api/plans/hello/source' },
      },
    ]);
    renderWithClient(<SourceEditor planId="hello" running={false} onClose={() => {}} />, {
      client: createQueryClient(),
    });
    expect(
      await screen.findByText('This roko serve does not support editing plans yet.', {}, { timeout: 600 }),
    ).toBeTruthy();
    expect(screen.queryByText('Loading source…')).toBeNull();
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('the validation badge reports a missing validate route without retrying', async () => {
    const fetchMock = stubFetch([
      { method: 'POST', path: '/api/plans/hello/validate', status: 404 },
    ]);
    renderWithClient(<ValidationBadge planId="hello" onSelectTask={() => {}} />, {
      client: createQueryClient(),
    });
    expect(await screen.findByText('validation unavailable', {}, { timeout: 600 })).toBeTruthy();
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('a 405 on validate reads the same way', async () => {
    stubFetch([
      { method: 'POST', path: '/api/plans/hello/validate', status: 405, statusText: 'Method Not Allowed' },
    ]);
    renderWithClient(<ValidationBadge planId="hello" onSelectTask={() => {}} />, {
      client: createQueryClient(),
    });
    expect(await screen.findByText('validation unavailable', {}, { timeout: 600 })).toBeTruthy();
  });
});
