'use client';

import React from 'react';
import { clsx } from 'clsx';

export interface StatusLEDProps {
  status: 'active' | 'idle' | 'success' | 'warning' | 'error' | 'offline';
  pulse?: boolean;
  size?: 'sm' | 'md';
}

// Map each status to a CSS colour variable name (used inline for the glow)
const statusColor: Record<StatusLEDProps['status'], string> = {
  active:  'var(--rose-glow)',
  idle:    'var(--text-muted)',
  success: 'var(--sage)',
  warning: 'var(--warning)',
  error:   'var(--accent-error)',
  offline: 'var(--text-ghost)',
};

const sizeClasses: Record<NonNullable<StatusLEDProps['size']>, string> = {
  sm: 'w-2 h-2',   // 8px
  md: 'w-3 h-3',   // 12px
};

export function StatusLED({ status, pulse = false, size = 'sm' }: StatusLEDProps) {
  const color = statusColor[status];

  return (
    <>
      {pulse && (
        <style>{`
          @keyframes rd-led-pulse {
            0%, 100% {
              transform: scale(1);
              box-shadow: 0 0 0 0 ${color};
            }
            50% {
              transform: scale(1.15);
              box-shadow: 0 0 6px 3px ${color};
            }
          }
        `}</style>
      )}
      <span
        role="status"
        aria-label={status}
        className={clsx(
          'inline-block shrink-0',
          sizeClasses[size],
        )}
        style={{
          backgroundColor: color,
          animation: pulse ? 'rd-led-pulse 2s ease-in-out infinite' : undefined,
          boxShadow: status === 'active' && !pulse
            ? `0 0 4px 1px ${color}`
            : undefined,
        }}
      />
    </>
  );
}
