'use client';

import React, { useEffect, useRef, useState } from 'react';
import { clsx } from 'clsx';

export interface VitalCellProps {
  label: string;
  value: string | number;
  trend?: 'up' | 'down' | 'flat';
  /** Flash the cell background briefly on value change. */
  flash?: boolean;
}

// ---------------------------------------------------------------------------
// Trend arrow
// ---------------------------------------------------------------------------

function TrendArrow({ trend }: { trend: 'up' | 'down' | 'flat' }) {
  if (trend === 'flat') {
    return (
      <span
        className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)] leading-none select-none"
        aria-label="stable"
      >
        →
      </span>
    );
  }
  return (
    <span
      className={clsx(
        'font-[var(--font-mono)] text-[var(--text-xs)] leading-none select-none',
        trend === 'up'   ? 'text-[var(--sage)]'         : 'text-[var(--accent-error)]',
      )}
      aria-label={trend === 'up' ? 'trending up' : 'trending down'}
    >
      {trend === 'up' ? '↑' : '↓'}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function VitalCell({ label, value, trend, flash = false }: VitalCellProps) {
  const [flashing, setFlashing] = useState(false);
  const prevValueRef = useRef<string | number>(value);

  useEffect(() => {
    if (!flash) return;
    if (value !== prevValueRef.current) {
      prevValueRef.current = value;
      setFlashing(true);
      const timer = setTimeout(() => setFlashing(false), 200);
      return () => clearTimeout(timer);
    }
  }, [value, flash]);

  return (
    <div
      className={clsx(
        'flex flex-col gap-1 px-3 py-2',
        'border border-[var(--text-ghost)]',
        'transition-[background-color] duration-[80ms]',
        flashing ? 'bg-[var(--bg-highlight)]' : 'bg-transparent',
      )}
    >
      {/* Label */}
      <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] leading-none tracking-[var(--tracking-wider)] uppercase">
        {label}
      </span>

      {/* Value + trend */}
      <div className="flex items-baseline gap-1.5">
        <span className="font-[var(--font-mono)] text-[var(--text-2xl)] text-[var(--text-strong)] leading-none num tabular-nums font-medium">
          {value}
        </span>
        {trend && <TrendArrow trend={trend} />}
      </div>
    </div>
  );
}
