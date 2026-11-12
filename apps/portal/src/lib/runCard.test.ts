/**
 * The run card's words for a check (gap-60f6b6): the names a run reports, from gate results and from settled tries,
 * all read as the check's own word.
 */
import { describe, expect, it } from 'vitest';
import { becauseOf, checkWord } from '@/lib/runCard';
import type { TryRun } from '@/lib/runState';

describe('checkWord', () => {
  it('reads every name a run reports for a check as its word', () => {
    expect(checkWord('verify[0:usage]')).toBe('usage');
    expect(checkWord('verify:0/usage')).toBe('usage');
    expect(checkWord('custom:usage')).toBe('usage');
    expect(checkWord('rung[compile]')).toBe('compile');
    expect(checkWord('usage')).toBe('usage');
  });
});

describe('becauseOf', () => {
  const failed = (over: Partial<TryRun>): TryRun => ({
    attempt: 1, model: 'glm-4.7', state: 'failed', failedCheck: null, detail: null, costUsd: null, durationMs: null, climbed: false,
    ...over,
  });

  it('names the check that failed a try', () => {
    expect(becauseOf(failed({ failedCheck: 'custom:usage' }))).toBe('the usage check failed');
  });

  it('falls back to the first line of what roko said, then to a plain clause', () => {
    expect(becauseOf(failed({ detail: 'timeout: the agent ran out of time\nmore' }))).toBe('the agent ran out of time');
    expect(becauseOf(failed({}))).toBe('it did not pass');
  });
});
