'use client';

// ---------------------------------------------------------------------------
// VignetteLayer — radial gradient edge-darkening
//
// A large elliptical gradient darkens the viewport corners and edges,
// drawing the viewer's eye toward the centre of the screen. The effect
// mimics the natural light falloff of a physical CRT or optical lens.
//
// The gradient is intentionally asymmetric: the centre point is slightly
// above the geometric centre (45% from top) to compensate for the visual
// weight of a header bar and account for the natural downward gaze bias.
//
// No animation, no JS. The element is purely declarative.
// ---------------------------------------------------------------------------

export function VignetteLayer() {
  return (
    <div
      aria-hidden="true"
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 1,
        pointerEvents: 'none',
        userSelect: 'none',
        background:
          'radial-gradient(ellipse 110% 90% at 50% 45%,' +
          'transparent 40%,' +
          'rgba(0, 0, 0, 0.22) 70%,' +
          'rgba(0, 0, 0, 0.55) 100%)',
        mixBlendMode: 'multiply',
      }}
    />
  );
}
