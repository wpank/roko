/**
 * apiErrors — shared helpers for classifying and describing API errors.
 *
 * Keep this module pure (no React, no side-effects) so it can be used from
 * both components and tests without a DOM environment.
 */

import { ApiError } from '@/api/client';
import { SIGN_IN_HINT } from '@/lib/bootstrap';

// ---------------------------------------------------------------------------
// isMissingRoute
// ---------------------------------------------------------------------------

/**
 * Returns true when `err` signals that the roko-serve instance does not have
 * the requested route at all (as opposed to a resource that was not found).
 *
 * Rules:
 *  - 405 Method Not Allowed is always a missing route.
 *  - 404 is a missing route UNLESS the body carries the server's own error
 *    object (a `code` string field) whose message does not start with the
 *    router fallback prefix "No route matches".
 *  - Everything else is false.
 */
export function isMissingRoute(err: unknown): boolean {
  if (!(err instanceof ApiError)) return false;

  if (err.status === 405) return true;

  if (err.status === 404) {
    const body = err.body;
    // A body that is an object with a string `code` is the server's own
    // resource-not-found shape.  Exception: if the message starts with
    // "No route matches" it is the router fallback — still a missing route.
    if (body !== null && typeof body === 'object') {
      const b = body as Record<string, unknown>;
      if (typeof b['code'] === 'string') {
        const msg = typeof b['message'] === 'string' ? b['message'] : '';
        return msg.startsWith('No route matches');
      }
    }
    // Empty / null body or a body without a `code` string → missing route.
    return true;
  }

  return false;
}

// ---------------------------------------------------------------------------
// shouldRetryQuery
// ---------------------------------------------------------------------------

/**
 * React Query `retry` callback.
 *
 * - Never retries 4xx responses — they will not fix themselves.
 * - Retries server errors and network failures up to 2 times (failureCount
 *   starts at 0 after the first failure, so `< 2` means up to two retries).
 */
export function shouldRetryQuery(failureCount: number, err: unknown): boolean {
  if (err instanceof ApiError && err.status >= 400 && err.status <= 499) {
    return false;
  }
  return failureCount < 2;
}

// ---------------------------------------------------------------------------
// unsupportedMessage / describeRequestError
// ---------------------------------------------------------------------------

/**
 * Returns the human-readable "this server does not support X yet" string.
 */
export function unsupportedMessage(action: string): string {
  return `This roko serve does not support ${action} yet.`;
}

/**
 * Converts an arbitrary caught error into a one-sentence operator-facing
 * string.  Never shows the raw `ApiError` message ("HTTP 405 …").
 *
 * Priority order:
 *  1. 401 Unauthorized → SIGN_IN_HINT
 *  2. Missing route (404/405 without a resource body) → unsupportedMessage
 *  3. ApiError with a string `body.message` → that message
 *  4. ApiError with a non-empty string body → that string
 *  5. Any other ApiError → "Request failed with status N."
 *  6. Any other Error → err.message
 *  7. Anything else → String(err)
 */
export function describeRequestError(err: unknown, action: string): string {
  if (err instanceof ApiError) {
    if (err.status === 401) return SIGN_IN_HINT;

    if (isMissingRoute(err)) return unsupportedMessage(action);

    // Check for a server-supplied message in the body.
    const body = err.body;
    if (body !== null && typeof body === 'object') {
      const b = body as Record<string, unknown>;
      if (typeof b['message'] === 'string' && b['message'].length > 0) {
        return b['message'];
      }
    }
    if (typeof body === 'string' && body.length > 0) {
      return body;
    }

    return `Request failed with status ${err.status}.`;
  }

  if (err instanceof Error) return err.message;

  return String(err);
}
