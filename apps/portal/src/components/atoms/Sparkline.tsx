'use client';

import React from 'react';

export interface SparklineProps {
  data: number[];
  width?: number;    // omit to fill container width (responsive)
  height?: number;   // default 24
  color?: string;    // any CSS colour expression, default 'var(--rose)'
  fill?: boolean;    // area fill below the line
  className?: string;
}

interface Point {
  x: number;
  y: number;
}

/**
 * Normalise a data array into SVG coordinate pairs scaled to [w, h].
 * Returns null when there are fewer than 2 data points.
 */
function normalise(data: number[], w: number, h: number): Point[] | null {
  if (data.length < 2) return null;

  const min = Math.min(...data);
  const max = Math.max(...data);
  const range = max - min || 1; // flat data → prevent divide-by-zero

  return data.map((v, i) => ({
    x: (i / (data.length - 1)) * w,
    // 1 px inset top/bottom so the stroke is never clipped at the edge
    y: h - ((v - min) / range) * (h - 2) - 1,
  }));
}

function toPolylinePoints(pts: Point[]): string {
  return pts.map(({ x, y }) => `${x.toFixed(2)},${y.toFixed(2)}`).join(' ');
}

function toFillPoints(pts: Point[], w: number, h: number): string {
  const line = toPolylinePoints(pts);
  return `0,${h} ${line} ${w},${h}`;
}

export function Sparkline({
  data,
  width,
  height = 24,
  color = 'var(--rose)',
  fill = false,
  className,
}: SparklineProps) {
  const svgWidth = width ?? 80;
  const pts = normalise(data, svgWidth, height);

  if (!pts) return null;

  return (
    <svg
      width={width === undefined ? '100%' : svgWidth}
      height={height}
      viewBox={`0 0 ${svgWidth} ${height}`}
      preserveAspectRatio="none"
      aria-hidden="true"
      className={className}
      style={{ display: 'block', overflow: 'visible' }}
    >
      {/* Area fill */}
      {fill && (
        <polygon
          points={toFillPoints(pts, svgWidth, height)}
          fill={color}
          fillOpacity={0.12}
        />
      )}

      {/* Line */}
      <polyline
        points={toPolylinePoints(pts)}
        fill="none"
        stroke={color}
        strokeWidth={1.5}
        strokeLinejoin="round"
        strokeLinecap="round"
        vectorEffect="non-scaling-stroke"
      />
    </svg>
  );
}
