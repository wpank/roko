/**
 * Acceptance: request errors — a route this roko serve lacks is named as such,
 * client errors are never retried, and no alert shows a raw HTTP status line.
 * Copied verbatim from plans/portal-programme/08b-portal-polish/accept/.
 */
import { describe, expect, it } from 'vitest';
import { ApiError } from '@/api/client';
import { createQueryClient } from '@/api/queryClient';
import { SIGN_IN_HINT } from '@/lib/bootstrap';
import {
  describeRequestError,
  isMissingRoute,
  shouldRetryQuery,
  unsupportedMessage,
} from '@/lib/apiErrors';

const routeMissing = new ApiError(404, 'Not Found', {
  error: 'not_found',
  message: 'No route matches /api/plans/hello/source',
});
const emptyNotFound = new ApiError(404, 'Not Found', null);
const planNotFound = new ApiError(404, 'Not Found', {
  code: 'not_found',
  message: "plan 'ghost' not found",
});
const methodNotAllowed = new ApiError(405, 'Method Not Allowed', null);

describe('isMissingRoute', () => {
  it('treats 405 as a missing route', () => {
    expect(isMissingRoute(methodNotAllowed)).toBe(true);
  });

  it('treats a bare 404 and the router fallback as a missing route', () => {
    expect(isMissingRoute(emptyNotFound)).toBe(true);
    expect(isMissingRoute(routeMissing)).toBe(true);
  });

  it('does not treat a resource that was not found as a missing route', () => {
    expect(isMissingRoute(planNotFound)).toBe(false);
  });

  it('is false for other statuses and non-API errors', () => {
    expect(isMissingRoute(new ApiError(500, 'Internal Server Error', null))).toBe(false);
    expect(isMissingRoute(new TypeError('fetch failed'))).toBe(false);
    expect(isMissingRoute('404')).toBe(false);
  });
});

describe('shouldRetryQuery', () => {
  it('never retries a client error', () => {
    for (const status of [400, 401, 404, 405, 409, 422]) {
      expect(shouldRetryQuery(0, new ApiError(status, '', null))).toBe(false);
    }
  });

  it('retries server and network errors twice', () => {
    const serverError = new ApiError(503, 'Service Unavailable', null);
    expect(shouldRetryQuery(0, serverError)).toBe(true);
    expect(shouldRetryQuery(1, serverError)).toBe(true);
    expect(shouldRetryQuery(2, serverError)).toBe(false);
    expect(shouldRetryQuery(1, new TypeError('fetch failed'))).toBe(true);
    expect(shouldRetryQuery(2, new TypeError('fetch failed'))).toBe(false);
  });

  it('is the default retry policy of the portal query client', () => {
    expect(createQueryClient().getDefaultOptions().queries?.retry).toBe(shouldRetryQuery);
  });
});

describe('describeRequestError', () => {
  it('names the feature a missing route belongs to', () => {
    expect(unsupportedMessage('running all plans')).toBe(
      'This roko serve does not support running all plans yet.',
    );
    expect(describeRequestError(methodNotAllowed, 'running all plans')).toBe(
      'This roko serve does not support running all plans yet.',
    );
    expect(describeRequestError(routeMissing, 'editing plans')).toBe(
      'This roko serve does not support editing plans yet.',
    );
  });

  it('sends a 401 to the sign-in hint', () => {
    expect(describeRequestError(new ApiError(401, 'Unauthorized', null), 'running plans')).toBe(
      SIGN_IN_HINT,
    );
  });

  it("uses the server's message when it sent one", () => {
    const conflict = new ApiError(409, 'Conflict', {
      code: 'conflict',
      message: 'a plan-set run is already active',
    });
    expect(describeRequestError(conflict, 'running plans')).toBe('a plan-set run is already active');
    expect(describeRequestError(planNotFound, 'running plans')).toBe("plan 'ghost' not found");
    expect(describeRequestError(new ApiError(400, 'Bad Request', 'slug is required'), 'x')).toBe(
      'slug is required',
    );
  });

  it('never shows a raw HTTP status line', () => {
    const text = describeRequestError(new ApiError(502, 'Bad Gateway', null), 'running plans');
    expect(text).toBe('Request failed with status 502.');
    expect(text).not.toMatch(/HTTP \d{3}/);
  });

  it('falls back to the error message for other errors', () => {
    expect(describeRequestError(new TypeError('fetch failed'), 'running plans')).toBe('fetch failed');
    expect(describeRequestError('offline', 'running plans')).toBe('offline');
  });
});
