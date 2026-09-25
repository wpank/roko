'use client';

import React from 'react';
import { clsx } from 'clsx';

export interface BadgeProps {
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info' | 'dream';
  children: React.ReactNode;
  className?: string;
}

const variantClasses: Record<NonNullable<BadgeProps['variant']>, string> = {
  default:  'text-text-faint   border-text-ghost',
  success:  'text-sage         border-sage',
  warning:  'text-warning      border-warning',
  error:    'text-accent-error border-accent-error',
  info:     'text-accent-cyan  border-accent-cyan',
  dream:    'text-dream-bright border-dream',
};

export function Badge({ variant = 'default', children, className }: BadgeProps) {
  return (
    <span
      className={clsx(
        'inline-flex items-center',
        'font-mono font-medium uppercase tracking-widest',
        'text-[10px] leading-none',
        'px-1.5 py-0.5',
        'border',
        variantClasses[variant],
        className,
      )}
    >
      {children}
    </span>
  );
}
