'use client';

// ---------------------------------------------------------------------------
// AffectStrip — Daimon affect-state visualization
//
// A 4px-tall horizontal strip rendered immediately below the header. Its
// colour reflects the current PAD (Pleasure–Arousal–Dominance) affect state
// reported by the Daimon engine via the SSE stream.
//
// Colour mapping (all relative to the ROSEDUST palette):
//   pleasure [-1, 1] → hue shift
//     negative (-1) → cool blue/cyan  (hue ≈ 200°)
//     neutral  ( 0) → muted rose      (hue ≈ 345°, brand default)
//     positive (+1) → warm amber/gold  (hue ≈ 35°)
//
//   arousal [-1, 1] → saturation and brightness
//     low  (-1) → desaturated, dim
//     high (+1) → fully saturated, bright
//
//   dominance [-1, 1] → gradient spread (how far the colour bleeds inward)
//     low  (-1) → narrow band (strip barely shows colour)
//     high (+1) → full-width flat fill
//
// When no affect data is available (null), the strip renders as a hairline
// in var(--text-ghost) — present but silent.
//
// The pulse animation period scales with arousal: high arousal = faster
// pulse, low arousal = slow or none. Respects prefers-reduced-motion.
// ---------------------------------------------------------------------------

import { useMemo } from 'react';
import { useDashboardStore } from '@/stores/dashboard';
import type { AffectState } from '@/api/types';

// ---------------------------------------------------------------------------
// Keyframe injection
// ---------------------------------------------------------------------------

const PULSE_KEYFRAMES = `
@keyframes rd-affect-pulse {
  0%, 100% { opacity: 1; }
  50%       { opacity: 0.55; }
}

@media (prefers-reduced-motion: reduce) {
  .rd-affect-strip {
    animation: none !important;
  }
}
`;

// ---------------------------------------------------------------------------
// Colour derivation from PAD coordinates
// ---------------------------------------------------------------------------

/**
 * Linearly interpolate `a` → `b` by `t` (clamped 0–1).
 */
function lerp(a: number, b: number, t: number): number {
  const tc = Math.max(0, Math.min(1, t));
  return a + (b - a) * tc;
}

/**
 * Clamp `v` to the range [min, max].
 */
function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
}

/**
 * Derive a CSS colour from the PAD triple.
 *
 * Returns a CSS `hsl()` string suitable for use in a gradient stop.
 */
function padToColor(affect: AffectState, alpha = 1): string {
  const { pleasure, arousal } = affect;

  // Normalise pleasure from [-1, 1] to [0, 1].
  const p = (clamp(pleasure, -1, 1) + 1) / 2;

  // Hue:
  //   p=0 → 200° (cool blue)
  //   p=0.5 → 345° (rose, wrap via 360+345 then mod) → use 340 directly
  //   p=1 → 35° (warm amber)
  // We route through two lerp segments to hit the rose midpoint.
  let hue: number;
  if (p <= 0.5) {
    // blue → rose: 200° → 340°
    hue = lerp(200, 340, p * 2);
  } else {
    // rose → amber: 340° → 395° (≡ 35° mod 360)
    hue = lerp(340, 395, (p - 0.5) * 2);
    if (hue >= 360) hue -= 360;
  }

  // Saturation: low arousal → 20%, high arousal → 90%.
  const a = (clamp(arousal, -1, 1) + 1) / 2;
  const saturation = lerp(20, 90, a);

  // Lightness: mild modulation by arousal — low → 30%, high → 55%.
  const lightness = lerp(30, 55, a);

  return `hsla(${hue.toFixed(1)}, ${saturation.toFixed(1)}%, ${lightness.toFixed(1)}%, ${alpha})`;
}

/**
 * Derive the gradient stop positions from the dominance dimension.
 *
 * High dominance → the colour fills more of the strip horizontally.
 * Low dominance → the colour is concentrated near the edges with a dim centre.
 */
function buildGradient(affect: AffectState): string {
  const midColor = padToColor(affect, 1);
  const edgeColor = padToColor(affect, 0.7);
  const fadeColor = padToColor(affect, 0);

  // Dominance [-1, 1] normalised to [0, 1].
  const d = (clamp(affect.dominance, -1, 1) + 1) / 2;

  // How far the fade-to-transparent extends from each edge (as % of width).
  // High dominance → almost full fill (fade starts at 0%, ends at ~5%).
  // Low dominance  → narrow colour band (fade starts at 45%, ends at 55%).
  const fadeStart = lerp(45, 0, d);
  const fadeEnd = lerp(55, 100, d);

  return (
    `linear-gradient(to right,` +
    ` ${fadeColor} 0%,` +
    ` ${edgeColor} ${fadeStart.toFixed(1)}%,` +
    ` ${midColor} 50%,` +
    ` ${edgeColor} ${(100 - fadeStart).toFixed(1)}%,` +
    ` ${fadeColor} ${fadeEnd.toFixed(1)}%)`
  );
}

/**
 * Pulse animation duration (seconds) based on arousal.
 *
 * High arousal → fast pulse (0.8s), low arousal → slow (4s), very low → none.
 */
function pulseConfig(affect: AffectState): { duration: string; animate: boolean } {
  const a = (clamp(affect.arousal, -1, 1) + 1) / 2;
  if (a < 0.25) {
    return { duration: '4s', animate: false };
  }
  const duration = lerp(4, 0.8, a);
  return { duration: `${duration.toFixed(2)}s`, animate: true };
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AffectStrip() {
  const affect = useDashboardStore((s) => s.affect);

  const style = useMemo(() => {
    if (!affect) {
      return {
        background: 'var(--text-ghost)',
        animation: 'none' as const,
      };
    }

    const gradient = buildGradient(affect);
    const { duration, animate } = pulseConfig(affect);

    return {
      background: gradient,
      animation: animate
        ? `rd-affect-pulse ${duration} ease-in-out infinite`
        : 'none',
    };
  }, [affect]);

  return (
    <>
      <style>{PULSE_KEYFRAMES}</style>
      <div
        aria-hidden="true"
        className="rd-affect-strip"
        title={affect ? `affect: ${affect.word}` : undefined}
        style={{
          position: 'relative',
          width: '100%',
          height: '4px',
          zIndex: 2,
          pointerEvents: 'none',
          userSelect: 'none',
          flexShrink: 0,
          ...style,
        }}
      />
    </>
  );
}
