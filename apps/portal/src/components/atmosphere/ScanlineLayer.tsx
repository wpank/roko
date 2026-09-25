'use client';

// ---------------------------------------------------------------------------
// ScanlineLayer — repeating horizontal CRT scanline overlay
//
// A pure CSS repeating-linear-gradient produces alternating transparent and
// very slightly dark 2-pixel bands, mimicking the physical scanlines of a
// cathode-ray tube display. The bands must be exactly 4px in period (2px
// transparent + 2px tinted) to read naturally at typical screen densities.
//
// Opacity is kept at 0.5 on the element itself, but the gradient's rgba
// values are already very subtle (0.03 alpha), making the total effect
// imperceptible in isolation. It adds depth when composited with the grain
// and vignette layers.
//
// No animation — CRT scanlines are static. This layer has zero runtime cost
// after initial paint.
// ---------------------------------------------------------------------------

export function ScanlineLayer() {
  return (
    <div
      aria-hidden="true"
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 1,
        pointerEvents: 'none',
        userSelect: 'none',
        backgroundImage:
          'repeating-linear-gradient(' +
          'to bottom,' +
          'transparent 0px,' +
          'transparent 2px,' +
          'rgba(0, 0, 0, 0.03) 2px,' +
          'rgba(0, 0, 0, 0.03) 4px' +
          ')',
        backgroundRepeat: 'repeat',
        backgroundSize: 'auto 4px',
        opacity: 0.5,
        mixBlendMode: 'multiply',
      }}
    />
  );
}
