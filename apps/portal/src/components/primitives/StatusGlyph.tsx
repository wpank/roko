'use client';

import React from 'react';
import { GLYPHS } from '@/lib/glyphs';
import type { GlyphState } from '@/lib/glyphs';

export interface StatusGlyphProps {
  state: GlyphState;
  /** Optional override for the tooltip text (aria-label always uses the registry label). */
  title?: string;
}

/**
 * StatusGlyph — renders a single state glyph in the correct token colour.
 *
 * Reads glyph, token, and label from the GLYPHS registry in @/lib/glyphs —
 * the single source of truth. No component should define its own state glyphs.
 *
 * aria-label: always the registry label (e.g. "accepted").
 * title:      defaults to the label; overridable for richer tooltip text.
 * color:      inline style from the CSS token (e.g. var(--state-accepted)).
 */
export function StatusGlyph({ state, title }: StatusGlyphProps) {
  const def = GLYPHS[state];
  return (
    <span
      aria-label={def.label}
      title={title ?? def.label}
      style={{ color: def.token }}
    >
      {def.glyph}
    </span>
  );
}
