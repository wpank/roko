'use client';

import React from 'react';
import { clsx } from 'clsx';

export interface SpinnerProps {
  size?: 'sm' | 'md' | 'lg';
}

const sizeMap: Record<NonNullable<SpinnerProps['size']>, number> = {
  sm: 12,
  md: 16,
  lg: 24,
};

const strokeMap: Record<NonNullable<SpinnerProps['size']>, number> = {
  sm: 1.5,
  md: 2,
  lg: 2.5,
};

export function Spinner({ size = 'md' }: SpinnerProps) {
  const px = sizeMap[size];
  const stroke = strokeMap[size];
  const r = (px - stroke * 2) / 2;
  const cx = px / 2;
  const circumference = 2 * Math.PI * r;

  return (
    <svg
      width={px}
      height={px}
      viewBox={`0 0 ${px} ${px}`}
      fill="none"
      aria-label="Loading"
      role="status"
      className={clsx(
        'shrink-0 text-rose',
        // CSS animation via inline style — no Tailwind needed
      )}
      style={{ animation: 'rd-spin 1s linear infinite' }}
    >
      <style>{`
        @keyframes rd-spin {
          from { transform: rotate(0deg); }
          to   { transform: rotate(360deg); }
        }
      `}</style>
      {/* Track */}
      <circle
        cx={cx}
        cy={cx}
        r={r}
        stroke="currentColor"
        strokeWidth={stroke}
        opacity={0.2}
      />
      {/* Arc */}
      <circle
        cx={cx}
        cy={cx}
        r={r}
        stroke="currentColor"
        strokeWidth={stroke}
        strokeLinecap="square"
        strokeDasharray={circumference}
        strokeDashoffset={circumference * 0.75}
        transform={`rotate(-90 ${cx} ${cx})`}
      />
    </svg>
  );
}
