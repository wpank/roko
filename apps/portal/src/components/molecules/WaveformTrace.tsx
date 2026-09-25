'use client';

import React, { useMemo } from 'react';
import { clsx } from 'clsx';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface WaveformTraceProps {
  /** Label shown to the left of the trace (ALL CAPS expected from caller). */
  label: string;
  /** Array of data points, newest at the end. Max ~60 points rendered. */
  data: number[];
  /** Stroke / fill colour CSS value, e.g. var(--rose). */
  color: string;
  /** Height of the SVG canvas in px. Defaults to 32. */
  height?: number;
  /** Override the y-domain maximum; auto-derived from data if omitted. */
  yMax?: number;
  className?: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Build a polyline points string and a closed area path from normalised data. */
function buildPaths(
  points: number[],
  width: number,
  height: number,
): { linePath: string; areaPath: string } {
  if (points.length === 0) {
    return {
      linePath: `M 0 ${height} L ${width} ${height}`,
      areaPath: `M 0 ${height} L ${width} ${height} Z`,
    };
  }

  const n = points.length;
  const step = n > 1 ? width / (n - 1) : width;
  const pad = 2; // vertical padding in px

  const coords = points.map((v, i) => {
    const x = i * step;
    const y = pad + (1 - v) * (height - pad * 2);
    return [x, y] as const;
  });

  const linePath = coords
    .map(([x, y], i) => `${i === 0 ? 'M' : 'L'} ${x.toFixed(1)} ${y.toFixed(1)}`)
    .join(' ');

  const areaPath = [
    linePath,
    `L ${(coords[coords.length - 1][0]).toFixed(1)} ${height}`,
    `L ${coords[0][0].toFixed(1)} ${height}`,
    'Z',
  ].join(' ');

  return { linePath, areaPath };
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function WaveformTrace({
  label,
  data,
  color,
  height = 32,
  yMax,
  className,
}: WaveformTraceProps) {
  // Keep the last 60 data points for rendering.
  const trimmed = useMemo(() => data.slice(-60), [data]);

  // Normalise to [0, 1].
  const normalised = useMemo(() => {
    const max = yMax ?? Math.max(...trimmed, 1e-9);
    return trimmed.map((v) => (max > 0 ? Math.min(1, v / max) : 0));
  }, [trimmed, yMax]);

  // SVG paths are generated without a known width — we rely on preserveAspectRatio
  // and a viewBox with a fixed logical width (300 units) so the SVG scales.
  const logicalWidth = 300;
  const { linePath, areaPath } = useMemo(
    () => buildPaths(normalised, logicalWidth, height),
    [normalised, height],
  );

  // Unique gradient id to avoid collisions when multiple traces render on the same page.
  const gradId = useMemo(
    () => `wf-grad-${label.replace(/[^a-z0-9]/gi, '-').toLowerCase()}`,
    [label],
  );

  return (
    <div className={clsx('flex items-center gap-2 min-w-0', className)}>
      {/* Label */}
      <span
        className="shrink-0 w-14 font-[var(--font-mono)] text-[10px] leading-none tracking-[var(--tracking-wider)] text-[var(--text-faint)] select-none text-right"
        aria-hidden
      >
        {label}
      </span>

      {/* SVG trace — fills remaining width */}
      <div className="flex-1 min-w-0 overflow-hidden" style={{ height }}>
        <svg
          width="100%"
          height={height}
          viewBox={`0 0 ${logicalWidth} ${height}`}
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          <defs>
            <linearGradient id={gradId} x1="0" x2="0" y1="0" y2="1">
              <stop offset="0%" stopColor={color} stopOpacity="0.20" />
              <stop offset="100%" stopColor={color} stopOpacity="0.02" />
            </linearGradient>
          </defs>

          {/* Area fill */}
          <path d={areaPath} fill={`url(#${gradId})`} />

          {/* Line stroke */}
          <path
            d={linePath}
            fill="none"
            stroke={color}
            strokeWidth="1.2"
            strokeLinejoin="round"
            strokeLinecap="round"
          />
        </svg>
      </div>
    </div>
  );
}
