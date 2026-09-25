'use client';

import React from 'react';
import { clsx } from 'clsx';

export interface ProgressBarProps {
  value: number;       // 0–100
  variant?: 'default' | 'cost';
  height?: number;     // px, default 4
  showLabel?: boolean;
  className?: string;
}

/** Resolve fill colour for the cost variant based on thresholds. */
function costFillColor(value: number): string {
  if (value >= 80) return 'var(--accent-error)';
  if (value >= 50) return 'var(--warning)';
  return 'var(--sage)';
}

export function ProgressBar({
  value,
  variant = 'default',
  height = 4,
  showLabel = false,
  className,
}: ProgressBarProps) {
  const clamped = Math.min(100, Math.max(0, value));

  const fillColor =
    variant === 'cost'
      ? costFillColor(clamped)
      : 'var(--rose)';

  return (
    <div className={clsx('flex items-center gap-2 w-full', className)}>
      {/* Track */}
      <div
        className="flex-1 bg-bg-highlight overflow-hidden"
        style={{ height }}
        role="progressbar"
        aria-valuenow={clamped}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        {/* Fill */}
        <div
          style={{
            width: `${clamped}%`,
            height: '100%',
            backgroundColor: fillColor,
            transition: 'width 200ms ease-out, background-color 200ms ease-out',
          }}
        />
      </div>

      {/* Optional label */}
      {showLabel && (
        <span className="font-mono text-xs tabular-nums shrink-0 w-9 text-right text-text-faint">
          {clamped.toFixed(0)}%
        </span>
      )}
    </div>
  );
}
