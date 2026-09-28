// @vitest-environment jsdom
/**
 * Acceptance: the running glyph — wherever it appears — carries the gentle
 * pulse (`.rd-pulse`, still under reduced motion); no other state moves.
 * Copied verbatim from plans/portal-programme/08e-portal-refine/accept/.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/react';
import { StatusGlyph } from '@/components/primitives/StatusGlyph';
import type { GlyphState } from '@/lib/glyphs';

afterEach(() => cleanup());

const glyph = () => document.querySelector('[data-glyph]') as HTMLElement;

describe('StatusGlyph', () => {
  it('pulses the running glyph', () => {
    render(<StatusGlyph state="active" />);
    expect(glyph().getAttribute('data-glyph')).toBe('active');
    expect(glyph().classList.contains('rd-pulse')).toBe(true);
    expect(glyph().style.color).toBe('var(--state-active)');
  });

  it('keeps every other state still', () => {
    const still: GlyphState[] = ['done', 'accepted', 'failed', 'queued', 'pending', 'skipped'];
    for (const state of still) {
      render(<StatusGlyph state={state} />);
      expect(glyph().getAttribute('data-glyph')).toBe(state);
      expect(glyph().classList.contains('rd-pulse')).toBe(false);
      cleanup();
    }
  });

  it('keeps its label and tooltip', () => {
    render(<StatusGlyph state="active" title="T01 is running" />);
    expect(glyph().getAttribute('aria-label')).toBe('active');
    expect(glyph().getAttribute('title')).toBe('T01 is running');
  });
});
