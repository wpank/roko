import { describe, it, expect } from 'vitest';
import { GLYPHS, glyphStateForTask, progressToken } from './glyphs';
import type { GlyphState } from './glyphs';
import type { TaskStatus } from './runState';

// ── GLYPHS registry ────────────────────────────────────────────────────────────

describe('GLYPHS', () => {
  it('every state has a distinct glyph', () => {
    const glyphs = Object.values(GLYPHS).map((g) => g.glyph);
    const unique = new Set(glyphs);
    expect(unique.size).toBe(glyphs.length);
  });

  it('every token starts with var(--', () => {
    for (const [state, def] of Object.entries(GLYPHS)) {
      expect(def.token, `${state} token`).toMatch(/^var\(--/);
    }
  });

  it('every label is a non-empty string', () => {
    for (const [state, def] of Object.entries(GLYPHS)) {
      expect(def.label.length, `${state} label`).toBeGreaterThan(0);
    }
  });

  it('accepted uses the --state-accepted token (amber, not done green)', () => {
    expect(GLYPHS.accepted.token).toBe('var(--state-accepted)');
    expect(GLYPHS.accepted.token).not.toBe(GLYPHS.done.token);
  });
});

// ── glyphStateForTask ──────────────────────────────────────────────────────────

describe('glyphStateForTask', () => {
  it('maps all TaskStatus values and pending', () => {
    const cases: Array<[TaskStatus | 'pending', GlyphState]> = [
      ['active',                 'active'],
      ['passed',                 'done'],
      ['failed',                 'failed'],
      ['accepted_with_failures', 'accepted'],
      ['already_satisfied',      'satisfied'],
      ['unverified',             'unchecked'],
      ['skipped',                'skipped'],
      ['cancelled',              'skipped'],
      ['pending',                'pending'],
    ];
    for (const [input, expected] of cases) {
      expect(glyphStateForTask(input), input).toBe(expected);
    }
  });

  it('maps accepted_with_failures to accepted (never done)', () => {
    const result = glyphStateForTask('accepted_with_failures');
    expect(result).toBe('accepted');
    expect(result).not.toBe('done');
  });

  it('maps passed to done', () => {
    expect(glyphStateForTask('passed')).toBe('done');
  });

  it('maps cancelled to skipped', () => {
    expect(glyphStateForTask('cancelled')).toBe('skipped');
  });
});

// ── progressToken ──────────────────────────────────────────────────────────────

describe('progressToken', () => {
  it('returns low for fraction below 0.3', () => {
    expect(progressToken(0)).toBe('var(--progress-low)');
    expect(progressToken(0.1)).toBe('var(--progress-low)');
    expect(progressToken(0.299)).toBe('var(--progress-low)');
  });

  it('returns mid at the 0.3 lower boundary', () => {
    expect(progressToken(0.3)).toBe('var(--progress-mid)');
  });

  it('returns mid between the bands', () => {
    expect(progressToken(0.5)).toBe('var(--progress-mid)');
  });

  it('returns mid at the 0.7 upper boundary', () => {
    expect(progressToken(0.7)).toBe('var(--progress-mid)');
  });

  it('returns high for fraction above 0.7', () => {
    expect(progressToken(0.701)).toBe('var(--progress-high)');
    expect(progressToken(1)).toBe('var(--progress-high)');
  });
});
