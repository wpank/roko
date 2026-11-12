/**
 * The API client reads every response it gets, even an empty 204: Chrome reports a response nobody read as an
 * aborted request, which the CI browser check counts as a failure (gap-60f6b6).
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { api } from '@/api/client';

afterEach(() => vi.unstubAllGlobals());

describe('an empty 204', () => {
  it('is read, and comes back as undefined', async () => {
    const response = new Response(null, { status: 204 });
    const read = vi.spyOn(response, 'arrayBuffer');
    vi.stubGlobal('fetch', vi.fn(async () => response));
    await expect(api.post('/api/plans/greeting/cancel')).resolves.toBeUndefined();
    expect(read).toHaveBeenCalledOnce();
  });
});
