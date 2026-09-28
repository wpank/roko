import { describe, it, expect } from 'vitest';
import { formatCost, formatTokens, formatDuration, middleEllipsis, compactDuration } from './formatters';

describe('formatCost', () => {
  it('formats values >= $0.01 with two decimal places', () => {
    expect(formatCost(1.234)).toBe('$1.23');
  });

  it('formats values below $0.01 with four decimal places', () => {
    expect(formatCost(0.00123)).toBe('$0.0012');
  });

  it('formats zero as $0.00', () => {
    expect(formatCost(0)).toBe('$0.00');
  });
});

describe('formatTokens', () => {
  it('renders small counts as integers', () => {
    expect(formatTokens(42)).toBe('42');
  });

  it('renders thousands with one decimal and k suffix', () => {
    expect(formatTokens(1234)).toBe('1.2k');
  });

  it('renders millions with one decimal and M suffix', () => {
    expect(formatTokens(1_500_000)).toBe('1.5M');
  });
});

describe('formatDuration', () => {
  it('formats sub-minute durations in seconds', () => {
    expect(formatDuration(1234)).toBe('1.2s');
  });

  it('formats minute-range durations as Xm Ys', () => {
    expect(formatDuration(150_000)).toBe('2m 30s');
  });

  it('formats hour-range durations as Xh Ym', () => {
    expect(formatDuration(4_500_000)).toBe('1h 15m');
  });
});

describe('middleEllipsis', () => {
  it('returns text unchanged when it fits within max', () => {
    expect(middleEllipsis('short', 10)).toBe('short');
  });

  it('returns text unchanged when length equals max', () => {
    expect(middleEllipsis('exactly10c', 10)).toBe('exactly10c');
  });

  it('truncates in the middle and preserves the tail', () => {
    // 29 chars → 20: head=9, tail=10
    expect(middleEllipsis('portal-plan-implementer-shell', 20)).toBe('portal-pl…nter-shell');
  });

  it('tail gets the extra character when available space is odd', () => {
    // max=10 → available=9, head=4, tail=5
    expect(middleEllipsis('abcdefghijk', 10)).toBe('abcd…ghijk');
  });

  it('splits evenly when available space is even', () => {
    // max=9 → available=8, head=4, tail=4
    expect(middleEllipsis('abcdefghijk', 9)).toBe('abcd…hijk');
  });

  it('returns first max chars when max < 3', () => {
    expect(middleEllipsis('hello', 2)).toBe('he');
    expect(middleEllipsis('hello', 0)).toBe('');
  });
});

describe('compactDuration', () => {
  it('returns placeholder for null', () => {
    expect(compactDuration(null)).toBe('·');
  });

  it('returns placeholder for undefined', () => {
    expect(compactDuration(undefined)).toBe('·');
  });

  it('returns placeholder for negative values', () => {
    expect(compactDuration(-1)).toBe('·');
  });

  it('formats sub-minute durations as Xs', () => {
    expect(compactDuration(45_000)).toBe('45s');
  });

  it('formats minute-range durations as XmYs', () => {
    expect(compactDuration(134_000)).toBe('2m14s');
  });

  it('formats hour-range durations as XhYYm with zero-padded minutes', () => {
    expect(compactDuration(3_900_000)).toBe('1h05m');
  });

  it('formats whole hours with 00m', () => {
    expect(compactDuration(3_600_000)).toBe('1h00m');
  });
});
