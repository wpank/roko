'use client';

import React from 'react';
import { clsx } from 'clsx';
import { ProgressBar } from '@/components/atoms';

export interface MetricBarProps {
  label: string;
  value: number;
  max: number;
  unit?: string;
  variant?: 'default' | 'cost';
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatValue(v: number, unit?: string): string {
  const formatted =
    v >= 1_000_000 ? `${(v / 1_000_000).toFixed(1)}M` :
    v >= 1_000     ? `${(v / 1_000).toFixed(0)}k` :
    v % 1 === 0    ? String(v) :
    v.toFixed(2);
  return unit ? `${formatted}${unit}` : formatted;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function MetricBar({ label, value, max, unit, variant = 'default' }: MetricBarProps) {
  const pct = max > 0 ? Math.round(Math.min(100, (value / max) * 100)) : 0;

  return (
    <div className="flex flex-col gap-1 w-full">
      {/* Header row: label on left, value/max on right */}
      <div className="flex items-baseline justify-between gap-2">
        <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] leading-none truncate">
          {label}
        </span>

        <span className="shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] num tabular-nums leading-none">
          <span className="text-[var(--text-strong)]">{formatValue(value, unit)}</span>
          <span className="text-[var(--text-ghost)]">/{formatValue(max, unit)}</span>
        </span>
      </div>

      {/* Bar */}
      <ProgressBar value={pct} variant={variant} height={3} />
    </div>
  );
}
