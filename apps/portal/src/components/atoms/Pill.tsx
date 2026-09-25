'use client';

import React from 'react';
import { clsx } from 'clsx';

export interface PillProps {
  active?: boolean;
  onClick?: () => void;
  children: React.ReactNode;
  className?: string;
}

export function Pill({ active = false, onClick, children, className }: PillProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={clsx(
        'inline-flex items-center',
        'font-mono text-xs font-medium',
        'px-2 py-1 border',
        'select-none cursor-pointer',
        'transition-[background-color,border-color,color]',
        'duration-[80ms]',
        active
          ? [
              'bg-bg-highlight border-rose text-text-strong',
            ]
          : [
              'bg-transparent border-text-ghost text-text-ghost',
              'hover:border-border-hover hover:text-text-muted',
            ],
        className,
      )}
    >
      {children}
    </button>
  );
}
