'use client';

// ---------------------------------------------------------------------------
// Atmosphere — combined ROSEDUST visual effect wrapper
//
// Drop this once into your root layout and all enabled effect layers will
// be rendered on top of the entire viewport. All layers use position: fixed
// and z-index: 1 so they float above the body background but below any
// application content, which should live at z-index >= 2.
//
// Usage:
//   <Atmosphere />                    // all layers enabled
//   <Atmosphere scanlines={false} />  // disable scanlines
//   <Atmosphere grain={false} vignette={false} /> // only scanlines
//
// The component is intentionally zero-dependency: it renders only the layers
// requested, each of which is CSS-only with no runtime overhead once painted.
// ---------------------------------------------------------------------------

import { GrainLayer } from './GrainLayer';
import { ScanlineLayer } from './ScanlineLayer';
import { VignetteLayer } from './VignetteLayer';

export interface AtmosphereProps {
  /** Render the animated film-grain overlay. Default: true. */
  grain?: boolean;
  /** Render the static CRT scanline overlay. Default: true. */
  scanlines?: boolean;
  /** Render the radial edge-darkening vignette. Default: true. */
  vignette?: boolean;
}

export function Atmosphere({
  grain = true,
  scanlines = true,
  vignette = true,
}: AtmosphereProps) {
  return (
    <>
      {grain && <GrainLayer />}
      {scanlines && <ScanlineLayer />}
      {vignette && <VignetteLayer />}
    </>
  );
}
