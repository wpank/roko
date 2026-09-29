import { describe, it, expect, vi } from 'vitest';
import { waitForOperation, type OperationDeps } from './operation';
import type { WireAccepted, WireOperation } from '@/api/contracts';

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/** Completed WireOperation with a result slug. */
function completedOp(slug: string, id = 'op-1'): WireOperation {
  return { id, status: 'completed', result: { slug, task_count: 3 } };
}

/** Failed WireOperation with an error message. */
function failedOp(error: string, id = 'op-1'): WireOperation {
  return { id, status: 'failed', error };
}

/** Running WireOperation (no result yet). */
function runningOp(id = 'op-1'): WireOperation {
  return { id, status: 'running' };
}

/**
 * Build a fake OperationDeps.
 *
 * `now` advances by 1 000 ms on each call so elapsed time grows predictably.
 * `sleep` is a no-op (returns immediately).
 * `fetchOperation` and `planExists` default to returning null / false.
 */
function makeDeps(overrides: Partial<OperationDeps> = {}): OperationDeps {
  let t = 0;
  return {
    planExists: vi.fn().mockResolvedValue(false),
    fetchOperation: vi.fn().mockResolvedValue(null),
    sleep: vi.fn().mockResolvedValue(undefined),
    now: vi.fn().mockImplementation(() => t++ * 1_000),
    ...overrides,
  };
}

function makeAccepted(partial: Partial<WireAccepted> = {}): WireAccepted {
  return { id: 'op-1', plan_id: 'my-plan', ...partial };
}

// ---------------------------------------------------------------------------
// 1. new-plan: plan appears on the first poll
// ---------------------------------------------------------------------------

describe('waitForOperation: new-plan — plan appears on first poll', () => {
  it('resolves with {slug: plan_id} once planExists returns true', async () => {
    const deps = makeDeps({
      fetchOperation: vi.fn().mockResolvedValue(null),
      planExists: vi.fn().mockResolvedValue(true), // true immediately
    });

    const result = await waitForOperation(makeAccepted(), deps, { expect: 'new-plan' });
    expect(result).toEqual({ slug: 'my-plan' });
  });
});

// ---------------------------------------------------------------------------
// 2. new-plan: plan appears late (multiple polls)
// ---------------------------------------------------------------------------

describe('waitForOperation: new-plan — plan appears late', () => {
  it('keeps polling until planExists returns true', async () => {
    let pollCount = 0;
    const deps = makeDeps({
      fetchOperation: vi.fn().mockResolvedValue(null),
      planExists: vi.fn().mockImplementation(() => {
        pollCount++;
        return Promise.resolve(pollCount >= 4);
      }),
    });

    const result = await waitForOperation(makeAccepted(), deps, { expect: 'new-plan' });
    expect(result).toEqual({ slug: 'my-plan' });
    expect(pollCount).toBeGreaterThanOrEqual(4);
  });
});

// ---------------------------------------------------------------------------
// 3. new-plan: operation fails before plan appears
// ---------------------------------------------------------------------------

describe('waitForOperation: new-plan — operation fails first', () => {
  it('rejects with the operation error message', async () => {
    let fetchCount = 0;
    const deps = makeDeps({
      planExists: vi.fn().mockResolvedValue(false),
      fetchOperation: vi.fn().mockImplementation(() => {
        fetchCount++;
        if (fetchCount >= 2) return Promise.resolve(failedOp('generation failed: quota exceeded'));
        return Promise.resolve(null);
      }),
    });

    await expect(
      waitForOperation(makeAccepted(), deps, { expect: 'new-plan' }),
    ).rejects.toThrow('generation failed: quota exceeded');
  });

  it('uses a generic message when the error field is absent', async () => {
    const deps = makeDeps({
      planExists: vi.fn().mockResolvedValue(false),
      fetchOperation: vi.fn().mockResolvedValue({ id: 'op-1', status: 'failed' } as WireOperation),
    });

    await expect(
      waitForOperation(makeAccepted(), deps, { expect: 'new-plan' }),
    ).rejects.toThrow('op-1');
  });
});

// ---------------------------------------------------------------------------
// 4. new-plan: Rust Debug-formatted status ("Completed { … }")
// ---------------------------------------------------------------------------

describe('waitForOperation: new-plan — Rust Debug status', () => {
  it('resolves when status leading word is "Completed" (mixed case)', async () => {
    let fetchCount = 0;
    const deps = makeDeps({
      planExists: vi.fn().mockResolvedValue(false),
      fetchOperation: vi.fn().mockImplementation(() => {
        fetchCount++;
        if (fetchCount >= 2) {
          return Promise.resolve({
            id: 'op-1',
            status: 'Completed { task_count: 5 }',
            result: { slug: 'my-plan', task_count: 5 },
          } as WireOperation);
        }
        return Promise.resolve(null);
      }),
    });

    const result = await waitForOperation(makeAccepted(), deps, { expect: 'new-plan' });
    expect(result).toEqual({ slug: 'my-plan' });
  });
});

// ---------------------------------------------------------------------------
// 5. new-plan: {id}-only WireAccepted (older server) — result.slug from op
// ---------------------------------------------------------------------------

describe('waitForOperation: new-plan — accepted has only id (older server)', () => {
  it('resolves with result.slug from the operation when plan_id is absent', async () => {
    // Older server: WireAccepted has no plan_id
    const accepted: WireAccepted = { id: 'op-99' };
    let fetchCount = 0;
    const deps = makeDeps({
      planExists: vi.fn().mockResolvedValue(false),
      fetchOperation: vi.fn().mockImplementation(() => {
        fetchCount++;
        if (fetchCount >= 2) {
          return Promise.resolve(completedOp('resolved-slug', 'op-99'));
        }
        return Promise.resolve(null);
      }),
    });

    const result = await waitForOperation(accepted, deps, { expect: 'new-plan' });
    expect(result).toEqual({ slug: 'resolved-slug' });
  });
});

// ---------------------------------------------------------------------------
// 6. revision: operation completes normally
// ---------------------------------------------------------------------------

describe('waitForOperation: revision — operation completes', () => {
  it('resolves with {slug} once the operation status reaches "completed"', async () => {
    let fetchCount = 0;
    const deps = makeDeps({
      fetchOperation: vi.fn().mockImplementation(() => {
        fetchCount++;
        if (fetchCount >= 3) return Promise.resolve(completedOp('my-plan'));
        return Promise.resolve(runningOp());
      }),
    });

    const result = await waitForOperation(makeAccepted(), deps, { expect: 'revision' });
    expect(result).toEqual({ slug: 'my-plan' });
  });

  it('uses plan_id as slug fallback when result has no slug field', async () => {
    const deps = makeDeps({
      fetchOperation: vi.fn().mockResolvedValue({
        id: 'op-1',
        status: 'completed',
        result: { task_count: 2 }, // no slug
      } as WireOperation),
    });

    const result = await waitForOperation(makeAccepted(), deps, { expect: 'revision' });
    expect(result).toEqual({ slug: 'my-plan' }); // falls back to accepted.plan_id
  });
});

// ---------------------------------------------------------------------------
// 7. revision: operation fails
// ---------------------------------------------------------------------------

describe('waitForOperation: revision — operation fails', () => {
  it('rejects with the operation error message', async () => {
    let fetchCount = 0;
    const deps = makeDeps({
      fetchOperation: vi.fn().mockImplementation(() => {
        fetchCount++;
        if (fetchCount >= 2) return Promise.resolve(failedOp('revision conflict'));
        return Promise.resolve(runningOp());
      }),
    });

    await expect(
      waitForOperation(makeAccepted(), deps, { expect: 'revision' }),
    ).rejects.toThrow('revision conflict');
  });
});

// ---------------------------------------------------------------------------
// 8. revision: three consecutive null operations → untrackable
// ---------------------------------------------------------------------------

describe('waitForOperation: revision — untrackable', () => {
  it('rejects after three consecutive null (404) operations', async () => {
    const deps = makeDeps({
      fetchOperation: vi.fn().mockResolvedValue(null),
    });

    await expect(
      waitForOperation(makeAccepted(), deps, { expect: 'revision' }),
    ).rejects.toThrow('the revision cannot be tracked');

    // Exactly 3 fetches before rejection
    expect((deps.fetchOperation as ReturnType<typeof vi.fn>).mock.calls).toHaveLength(3);
  });

  it('resets the null counter when a non-null op arrives', async () => {
    // 2 nulls, then running, then 2 more nulls — should NOT reject until 3 in a row
    let fetchCount = 0;
    const deps = makeDeps({
      fetchOperation: vi.fn().mockImplementation(() => {
        fetchCount++;
        if (fetchCount === 3) return Promise.resolve(runningOp());
        if (fetchCount >= 6) return Promise.resolve(completedOp('my-plan'));
        return Promise.resolve(null);
      }),
    });

    const result = await waitForOperation(makeAccepted(), deps, { expect: 'revision' });
    expect(result).toEqual({ slug: 'my-plan' });
  });
});

// ---------------------------------------------------------------------------
// 9. timeout
// ---------------------------------------------------------------------------

describe('waitForOperation: timeout', () => {
  it('rejects with a timeout error after timeoutMs elapses', async () => {
    let time = 0;
    const deps = makeDeps({
      planExists: vi.fn().mockResolvedValue(false),
      fetchOperation: vi.fn().mockResolvedValue(null),
      sleep: vi.fn().mockImplementation(() => {
        time += 1_500;
        return Promise.resolve();
      }),
      now: vi.fn().mockImplementation(() => time),
    });

    await expect(
      waitForOperation(makeAccepted(), deps, {
        expect: 'new-plan',
        timeoutMs: 3_000,
      }),
    ).rejects.toThrow('timed out');
  });

  it('calls onPoll with elapsed time on each tick', async () => {
    let time = 0;
    const polls: number[] = [];
    const deps = makeDeps({
      planExists: vi.fn().mockResolvedValue(false),
      fetchOperation: vi.fn().mockResolvedValue(null),
      sleep: vi.fn().mockImplementation(() => {
        time += 1_000;
        return Promise.resolve();
      }),
      now: vi.fn().mockImplementation(() => time),
    });

    await expect(
      waitForOperation(makeAccepted(), deps, {
        expect: 'new-plan',
        // 2 500 ms: steps land at 1 000, 2 000, then 3 000 which exceeds 2 500
        timeoutMs: 2_500,
        onPoll: (ms) => polls.push(ms),
      }),
    ).rejects.toThrow('timed out');

    // onPoll fires before the timeout check, so the rejection tick is included
    expect(polls).toEqual([1_000, 2_000, 3_000]);
  });
});

// ---------------------------------------------------------------------------
// 10. transient fetch errors — recovered within the tolerance window
// ---------------------------------------------------------------------------

describe('waitForOperation: transient fetch errors', () => {
  it('tolerates up to three consecutive errors and resolves on the next success', async () => {
    let errorCount = 0;
    const deps = makeDeps({
      planExists: vi.fn().mockResolvedValue(false),
      fetchOperation: vi.fn().mockImplementation(() => {
        if (errorCount < 3) {
          errorCount++;
          return Promise.reject(new Error('network error'));
        }
        return Promise.resolve(completedOp('my-plan'));
      }),
    });

    const result = await waitForOperation(makeAccepted(), deps, { expect: 'new-plan' });
    expect(result).toEqual({ slug: 'my-plan' });
    expect(errorCount).toBe(3);
  });

  it('rejects on the fourth consecutive fetch error', async () => {
    let fetchCount = 0;
    const deps = makeDeps({
      planExists: vi.fn().mockResolvedValue(false),
      fetchOperation: vi.fn().mockImplementation(() => {
        fetchCount++;
        return Promise.reject(new Error(`network error ${fetchCount}`));
      }),
    });

    await expect(
      waitForOperation(makeAccepted(), deps, { expect: 'new-plan' }),
    ).rejects.toThrow('network error 4');
  });

  it('resets the error counter after a successful fetch', async () => {
    // 3 errors, then success (null), then 3 more errors, then success (completed)
    let cycle = 0;
    let errorCount = 0;
    const deps = makeDeps({
      planExists: vi.fn().mockResolvedValue(false),
      fetchOperation: vi.fn().mockImplementation(() => {
        cycle++;
        if (cycle <= 3) {
          errorCount++;
          return Promise.reject(new Error('flaky'));
        }
        if (cycle === 4) return Promise.resolve(null); // success — resets counter
        if (cycle <= 7) {
          errorCount++;
          return Promise.reject(new Error('flaky again'));
        }
        return Promise.resolve(completedOp('my-plan'));
      }),
    });

    const result = await waitForOperation(makeAccepted(), deps, { expect: 'new-plan' });
    expect(result).toEqual({ slug: 'my-plan' });
  });
});

// ---------------------------------------------------------------------------
// 11. new-plan: the plan is asked for only once the operation is unknown
// ---------------------------------------------------------------------------

describe('waitForOperation: new-plan — waits on a known operation', () => {
  it('does not poll the plan while the operation runs', async () => {
    let fetchCount = 0;
    const deps = makeDeps({
      fetchOperation: vi.fn().mockImplementation(() => {
        fetchCount++;
        return Promise.resolve(fetchCount >= 4 ? completedOp('my-plan') : runningOp());
      }),
    });

    const result = await waitForOperation(makeAccepted(), deps, { expect: 'new-plan' });
    expect(result).toEqual({ slug: 'my-plan' });
    expect(deps.fetchOperation).toHaveBeenCalledTimes(4);
    expect(deps.planExists).not.toHaveBeenCalled();
  });

  it('asks for the plan once the server no longer knows the operation', async () => {
    // Running twice, then swept (404) before a poll saw it complete.
    let fetchCount = 0;
    const deps = makeDeps({
      fetchOperation: vi.fn().mockImplementation(() => {
        fetchCount++;
        return Promise.resolve(fetchCount <= 2 ? runningOp() : null);
      }),
      planExists: vi.fn().mockResolvedValue(true),
    });

    const result = await waitForOperation(makeAccepted(), deps, { expect: 'new-plan' });
    expect(result).toEqual({ slug: 'my-plan' });
    expect(deps.planExists).toHaveBeenCalledTimes(1);
    expect(deps.planExists).toHaveBeenCalledWith('my-plan');
  });
});
