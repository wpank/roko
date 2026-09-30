/**
 * glyphs.ts — single source of truth for state glyphs, tokens, and labels.
 *
 * Every widget that renders task/plan status must import from here.
 * No other file should define its own state-glyph vocabulary.
 */

import type { TaskStatus } from '@/lib/runState';

// ── Types ──────────────────────────────────────────────────────────────────────

export type GlyphState =
  | 'done'
  | 'active'
  | 'unverified'
  | 'accepted'
  | 'unchecked'
  | 'failed'
  | 'queued'
  | 'pending'
  | 'skipped';

export interface GlyphDef {
  /** Unicode glyph character */
  glyph: string;
  /** CSS custom property reference, e.g. var(--state-done) */
  token: string;
  /** Human-readable label for aria-label and tooltips */
  label: string;
}

// ── GLYPHS registry ────────────────────────────────────────────────────────────

/**
 * Canonical glyph/token/label for every state.
 *
 * Tokens map 1-to-1 with the CSS custom properties in tokens.css:
 *   --state-done, --state-active, --state-accepted (amber, NOT green),
 *   --state-failed, --state-queued, --state-pending, --state-skipped.
 * `unverified` is a running plan in amber: a task was accepted despite failing
 * checks or finished unchecked, or every task is dispatched and only checks
 * remain. `unchecked` is a task that finished without a verify step judging it:
 * amber too, since green means verified.
 */
export const GLYPHS: Record<GlyphState, GlyphDef> = {
  done:     { glyph: '✓', token: 'var(--state-done)',     label: 'done'     },
  active:   { glyph: '►', token: 'var(--state-active)',   label: 'active'   },
  unverified: { glyph: '▷', token: 'var(--state-accepted)', label: 'running, not verified' },
  accepted: { glyph: '⚠', token: 'var(--state-accepted)', label: 'accepted' },
  unchecked: { glyph: '?', token: 'var(--state-accepted)', label: 'done, not verified' },
  failed:   { glyph: '✗', token: 'var(--state-failed)',   label: 'failed'   },
  queued:   { glyph: '◌', token: 'var(--state-queued)',   label: 'queued'   },
  pending:  { glyph: '·', token: 'var(--state-pending)',  label: 'pending'  },
  skipped:  { glyph: '⊘', token: 'var(--state-skipped)',  label: 'skipped'  },
};

// ── glyphStateForTask ──────────────────────────────────────────────────────────

/**
 * Map a TaskStatus (or the synthetic 'pending' state) to a GlyphState.
 *
 * Mapping:
 *   active               → active
 *   passed               → done
 *   failed               → failed
 *   accepted_with_failures → accepted  (NEVER done — amber, not green)
 *   unverified           → unchecked (NEVER done — amber, not green)
 *   skipped              → skipped
 *   cancelled            → skipped
 *   pending              → pending
 */
export function glyphStateForTask(status: TaskStatus | 'pending'): GlyphState {
  switch (status) {
    case 'active':
      return 'active';
    case 'passed':
      return 'done';
    case 'failed':
      return 'failed';
    case 'accepted_with_failures':
      // Must map to accepted (amber), not done (green). See tokens.css §1.
      return 'accepted';
    case 'unverified':
      return 'unchecked';
    case 'skipped':
      return 'skipped';
    case 'cancelled':
      return 'skipped';
    case 'pending':
      return 'pending';
  }
}

// ── glyphStateForCheck ─────────────────────────────────────────────────────────

/**
 * Map a verify step's state to a GlyphState: passed → done, running → active,
 * failed → failed, pending (not reached) → pending.
 */
export function glyphStateForCheck(state: 'passed' | 'running' | 'failed' | 'pending'): GlyphState {
  switch (state) {
    case 'passed':
      return 'done';
    case 'running':
      return 'active';
    case 'failed':
      return 'failed';
    case 'pending':
      return 'pending';
  }
}

// ── progressToken ──────────────────────────────────────────────────────────────

/**
 * Return the CSS token for a progress fraction.
 *
 * Bands (mirrors tokens.css §3):
 *   fraction < 0.3        → var(--progress-low)   (running colour — indigo)
 *   0.3 ≤ fraction ≤ 0.7  → var(--progress-mid)   (running colour — indigo)
 *   fraction > 0.7        → var(--progress-high)  (running colour — indigo)
 *
 * All three bands are the running colour. Finished bars use state tokens
 * (--state-done, --state-accepted, --state-failed) via GLYPHS[state].token.
 */
export function progressToken(fraction: number): string {
  if (fraction < 0.3) return 'var(--progress-low)';
  if (fraction <= 0.7) return 'var(--progress-mid)';
  return 'var(--progress-high)';
}
