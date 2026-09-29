/**
 * operation.ts — wait for a plan-generation or plan-revision operation to settle.
 *
 * POST /api/plans/generate and POST /api/plans/{id}/revise both answer 202 with a
 * WireAccepted body. This module polls GET /api/operations/{id} until the server
 * reports a terminal status, using injected deps so callers keep full control over
 * timing and I/O.
 *
 * Rules:
 * - Never call Date.now() or setTimeout() directly — use the injected deps.
 * - Never diff the plan list to detect creation — that race is policy-forbidden.
 * - Never ask for a new plan while its operation runs: GET /api/plans/{id}
 *   answers 404 until the plan is written, and each 404 is a console error.
 */

import type { WireAccepted, WireOperation } from '@/api/contracts';

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface OperationDeps {
  /**
   * Returns true if GET /api/plans/{id} is 200 (the plan has been created).
   * Asked only once the server no longer knows the operation.
   */
  planExists(id: string): Promise<boolean>;
  /** Returns the operation record, or null when the server replies 404. */
  fetchOperation(id: string): Promise<WireOperation | null>;
  /** Sleep for ms milliseconds (injected — do not call setTimeout directly). */
  sleep(ms: number): Promise<void>;
  /** Return the current wall-clock timestamp in ms (injected — do not call Date.now()). */
  now(): number;
}

export interface WaitForOperationOpts {
  /**
   * 'new-plan': the plan does not exist yet; the operation reports the end, and
   *   once the server no longer knows the operation, planExists() becoming true
   *   is the signal instead.
   * 'revision': the plan already exists; only the operation can signal completion.
   */
  expect: 'new-plan' | 'revision';
  /** Polling interval in ms. Default 1000. */
  intervalMs?: number;
  /** Timeout in ms. Default 600 000. */
  timeoutMs?: number;
  /** Called after each poll tick with the elapsed time since waitForOperation was called. */
  onPoll?(elapsedMs: number): void;
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/**
 * Extract the leading word of a status string, normalised to lower-case.
 *
 * Handles both plain strings ("completed") and Rust Debug output where enum
 * variants are rendered with their payload ("Completed { task_count: 5 }").
 * The first word before any space, `{`, or `(` is the discriminant.
 */
function leadingWord(status: string): string {
  return status.trim().split(/[\s{(]/)[0]!.toLowerCase();
}

// ---------------------------------------------------------------------------
// waitForOperation
// ---------------------------------------------------------------------------

/**
 * Wait for an async plan operation to complete and return the plan slug.
 *
 * Polls deps.fetchOperation every intervalMs milliseconds (default 1 000).
 *
 * Resolution rules, in both `expect` modes:
 *   • A 'failed' operation rejects with its error message (or a generic
 *     fallback when the field is absent).
 *   • A 'completed' operation resolves `{slug}` from result.slug, else plan_id.
 *   • A running operation is left to finish; nothing else is requested.
 *
 * A null operation (404) means the server no longer knows it: finished
 * operations are swept within a minute, and a restart forgets them all.
 *   • 'new-plan': the plan appearing is the only signal left, so resolves
 *     `{slug: plan_id}` once planExists(plan_id) is true.
 *   • 'revision': the plan already exists, so three consecutive null
 *     operations reject with "the revision cannot be tracked".
 *
 * Error-tolerance rules:
 *   • Up to three consecutive fetchOperation errors are swallowed; the fourth
 *     consecutive error is re-thrown as-is.
 *   • The promise rejects after timeoutMs (default 600 000 ms).
 */
export async function waitForOperation(
  accepted: WireAccepted,
  deps: OperationDeps,
  opts: WaitForOperationOpts,
): Promise<{ slug: string }> {
  const { expect, intervalMs = 1_000, timeoutMs = 600_000, onPoll } = opts;

  const startMs = deps.now();

  // For 'new-plan', planId is the plan's slug; fall back to the operation id for
  // older servers that only include `id` in WireAccepted (no `plan_id`).
  const planId = accepted.plan_id ?? accepted.id;

  let consecutiveFetchErrors = 0;
  let lastFetchError: unknown;
  // Tracks consecutive null (404) operations — only meaningful for 'revision'.
  let consecutiveNullOps = 0;

  for (;;) {
    await deps.sleep(intervalMs);

    const elapsed = deps.now() - startMs;
    onPoll?.(elapsed);

    if (elapsed >= timeoutMs) {
      throw new Error(`waitForOperation: timed out after ${elapsed}ms`);
    }

    // ── Fetch the operation record ──────────────────────────────────────────
    let op: WireOperation | null;
    try {
      op = await deps.fetchOperation(accepted.id);
      consecutiveFetchErrors = 0;
      lastFetchError = undefined;
    } catch (err) {
      consecutiveFetchErrors++;
      lastFetchError = err;
      if (consecutiveFetchErrors >= 4) {
        throw lastFetchError;
      }
      continue;
    }

    // ── Unknown operation (404): swept, or lost in a restart ────────────────
    if (op === null) {
      if (expect === 'new-plan') {
        // The plan appearing is the only signal left.
        if (await deps.planExists(planId)) {
          return { slug: planId };
        }
      } else if (++consecutiveNullOps >= 3) {
        // 'revision': the plan already exists; only the operation knows when it's done.
        throw new Error('the revision cannot be tracked');
      }
      continue;
    }

    // ── Known operation: wait for it to report the end ──────────────────────
    consecutiveNullOps = 0;
    const word = leadingWord(op.status);

    if (word === 'failed') {
      throw new Error(op.error ?? `Operation ${accepted.id} failed`);
    }

    if (word === 'completed') {
      // result.slug is populated on current servers; fall back to planId
      // for servers that don't include it.
      return { slug: op.result?.slug ?? planId };
    }
  }
}
