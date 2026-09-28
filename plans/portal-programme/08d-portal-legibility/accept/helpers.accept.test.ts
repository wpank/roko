/**
 * Acceptance: one time formatter for every time cell, the short model slug,
 * and a human sentence instead of "request body must be valid JSON".
 * Copied verbatim from plans/portal-programme/08d-portal-legibility/accept/.
 */
import { describe, expect, it } from 'vitest';
import { ApiError } from '@/api/client';
import { describeRequestError, OUTDATED_SERVER } from '@/lib/apiErrors';
import { formatSpan, shortModel } from '@/lib/formatters';

describe('formatSpan', () => {
  it('shows an estimate in whole minutes', () => {
    expect(formatSpan({ kind: 'estimate', ms: 9 * 60_000 })).toBe('~9m');
    expect(formatSpan({ kind: 'estimate', ms: 95 * 60_000 })).toBe('~1h35m');
    expect(formatSpan({ kind: 'estimate', ms: 20_000 })).toBe('~1m');
  });

  it('shows elapsed and actual time the same compact way', () => {
    expect(formatSpan({ kind: 'elapsed', ms: 12_000 })).toBe('12s');
    expect(formatSpan({ kind: 'actual', ms: 72_000 })).toBe('1m12s');
    expect(formatSpan({ kind: 'actual', ms: 100 })).toBe('0s');
    expect(formatSpan({ kind: 'elapsed', ms: 3_900_000 })).toBe('1h05m');
  });

  it('returns null when the time is unknown, never a placeholder', () => {
    expect(formatSpan({ kind: 'none', ms: null })).toBeNull();
    expect(formatSpan({ kind: 'elapsed', ms: null })).toBeNull();
    expect(formatSpan({ kind: 'actual', ms: -5 })).toBeNull();
  });
});

describe('shortModel', () => {
  it('drops the vendor prefix, provider path and date suffix', () => {
    expect(shortModel('claude-sonnet-4-6')).toBe('sonnet-4-6');
    expect(shortModel('claude-opus-4-6')).toBe('opus-4-6');
    expect(shortModel('claude-sonnet-4-20250514')).toBe('sonnet-4');
    expect(shortModel('openai/gpt-5.1-codex')).toBe('gpt-5.1-codex');
  });

  it('leaves other names alone', () => {
    expect(shortModel('gpt-5.1-codex')).toBe('gpt-5.1-codex');
    expect(shortModel('kimi-k2')).toBe('kimi-k2');
    expect(shortModel('')).toBe('');
  });
});

describe('describeRequestError for an older server', () => {
  it('says the server is older instead of quoting the JSON parser', () => {
    const err = new ApiError(400, 'Bad Request', {
      code: 'invalid_json',
      details: { reason: 'missing field `slug` at line 1 column 47' },
      message: 'request body must be valid JSON',
    });
    const text = describeRequestError(err, 'generating plans');
    expect(text).toBe(OUTDATED_SERVER);
    expect(text).toMatch(/older/);
    expect(text).not.toMatch(/JSON/);
  });

  it('still passes other server messages through', () => {
    const err = new ApiError(400, 'Bad Request', { code: 'bad_request', message: 'prompt is empty' });
    expect(describeRequestError(err, 'generating plans')).toBe('prompt is empty');
  });
});
