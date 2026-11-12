/**
 * The dot-matrix face: digits keep five columns so a number never jitters; punctuation takes only what it lights;
 * a glyph that changed is marked to roll in.
 */
import { describe, expect, it } from 'vitest';
import { DOT_SIZES, dotReading, glyph } from '@/components/kit/dots';

describe('the face', () => {
  it('gives every digit and letter five columns', () => {
    for (const ch of '0123456789ABCXYZ') expect(glyph(ch).width).toBe(5);
  });

  it('gives a colon and a dot only the columns they light', () => {
    expect(glyph(':').width).toBe(2);
    expect(glyph('.').width).toBe(2);
  });

  it('draws lower case as capitals, and anything it lacks as a middle dot', () => {
    expect(glyph('a')).toBe(glyph('A'));
    expect(glyph('€')).toBe(glyph('·'));
  });
});

describe('a reading', () => {
  it('is as wide as its glyphs and one column between each', () => {
    const { dot, pitch } = DOT_SIZES.M;
    // "4:05": 5 + 1 + 2 + 1 + 5 + 1 + 5 columns.
    expect(dotReading('4:05', 'M').width).toBeCloseTo(20 * pitch - (pitch - dot), 2);
    expect(dotReading('4:05', 'M').height).toBeCloseTo(7 * pitch - (pitch - dot), 2);
  });

  it('marks only the glyphs that changed to roll in', () => {
    const lit = dotReading('4:06', 'S', '4:05').lit;
    expect(lit.map((g) => g.roll)).toEqual([false, false, false, true]);
  });

  it('rolls nothing on its first render', () => {
    expect(dotReading('4:05', 'S').lit.every((g) => !g.roll)).toBe(true);
  });
});
