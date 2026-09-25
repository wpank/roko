'use client';

// ---------------------------------------------------------------------------
// GrainLayer — CSS-only film grain overlay
//
// Uses a tiny SVG feTurbulence tile baked into a data URI, repeated across
// the viewport. Each animation frame shifts the tile position by a random-
// ish offset, producing the appearance of living analogue film grain.
//
// Opacity is intentionally kept at 0.04 — it should be felt rather than
// seen. The mix-blend-mode: overlay means it brightens light areas slightly
// and darkens dark areas, mimicking the grain of physical film.
//
// prefers-reduced-motion: pauses the animation entirely.
// ---------------------------------------------------------------------------

const GRAIN_KEYFRAMES = `
@keyframes rd-grain-shift {
  0%   { background-position: 0 0 }
  10%  { background-position: -13px  7px }
  20%  { background-position:  22px -4px }
  30%  { background-position:  -6px 16px }
  40%  { background-position:  18px -9px }
  50%  { background-position: -21px  3px }
  60%  { background-position:   9px 20px }
  70%  { background-position: -15px -5px }
  80%  { background-position:  25px 12px }
  90%  { background-position:  -4px -18px }
  100% { background-position:   0 0 }
}

@media (prefers-reduced-motion: reduce) {
  .rd-grain-layer {
    animation: none !important;
    background-position: 0 0 !important;
  }
}
`;

// 128×128 SVG noise tile. feTurbulence fractalNoise at 0.72 base frequency
// with 4 octaves produces a fine-grained, organic, film-like texture.
// stitchTiles="stitch" ensures seamless tiling at the edges.
const GRAIN_SVG =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='128' height='128'%3E" +
  "%3Cfilter id='g'%3E" +
  "%3CfeTurbulence type='fractalNoise' baseFrequency='0.72' numOctaves='4' stitchTiles='stitch'/%3E" +
  "%3CfeColorMatrix type='saturate' values='0'/%3E" +
  "%3C/filter%3E" +
  "%3Crect width='128' height='128' filter='url(%23g)' opacity='0.08'/%3E" +
  "%3C/svg%3E";

export function GrainLayer() {
  return (
    <>
      <style>{GRAIN_KEYFRAMES}</style>
      <div
        aria-hidden="true"
        className="rd-grain-layer"
        style={{
          position: 'fixed',
          inset: 0,
          zIndex: 1,
          pointerEvents: 'none',
          userSelect: 'none',
          backgroundImage: `url("${GRAIN_SVG}")`,
          backgroundRepeat: 'repeat',
          backgroundSize: '128px 128px',
          opacity: 0.04,
          mixBlendMode: 'overlay',
          animation: 'rd-grain-shift 0.25s steps(1) infinite',
          willChange: 'background-position',
        }}
      />
    </>
  );
}
