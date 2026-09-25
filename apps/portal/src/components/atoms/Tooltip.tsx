'use client';

import React from 'react';
import { clsx } from 'clsx';

export interface TooltipProps {
  content: React.ReactNode;
  children: React.ReactNode;
  side?: 'top' | 'bottom' | 'left' | 'right';
  className?: string;
}

/**
 * CSS-only tooltip using Tailwind's `group` / `group-hover` pattern.
 *
 * The tooltip panel is always rendered but hidden (opacity-0,
 * pointer-events-none). Hovering or focusing the wrapper span reveals it.
 *
 * z-index 600 matches --z-tooltip from the ROSEDUST scale.
 */
export function Tooltip({ content, children, side = 'top', className }: TooltipProps) {
  return (
    <span className={clsx('relative inline-flex group', className)}>
      {children}

      {/* Tooltip panel */}
      <span
        role="tooltip"
        className={clsx(
          // Base layout
          'absolute pointer-events-none z-[600]',
          'whitespace-nowrap',
          // Appearance
          'bg-bg-raised border border-text-ghost',
          'text-text-muted font-mono text-xs leading-tight',
          'px-2 py-1',
          // Hidden by default; visible on group hover/focus
          'opacity-0 group-hover:opacity-100 group-focus-within:opacity-100',
          'transition-opacity duration-[80ms]',
          // Positioning
          side === 'top'    && 'bottom-full left-1/2 -translate-x-1/2 mb-2',
          side === 'bottom' && 'top-full left-1/2 -translate-x-1/2 mt-2',
          side === 'left'   && 'right-full top-1/2 -translate-y-1/2 mr-2',
          side === 'right'  && 'left-full top-1/2 -translate-y-1/2 ml-2',
        )}
      >
        {content}

        {/* Arrow — a rotated square clipped by the panel edge */}
        <span
          aria-hidden="true"
          className={clsx(
            'absolute w-1.5 h-1.5',
            'bg-bg-raised border-text-ghost',
            'rotate-45',
            // top arrow: hangs below the panel pointing down
            side === 'top' && [
              'border-b border-r',
              'top-full left-1/2 -translate-x-1/2 -translate-y-1/2',
            ],
            // bottom arrow: sits above the panel pointing up
            side === 'bottom' && [
              'border-t border-l',
              'bottom-full left-1/2 -translate-x-1/2 translate-y-1/2',
            ],
            // left arrow: sits to the right of the panel pointing right
            side === 'left' && [
              'border-t border-r',
              'left-full top-1/2 -translate-y-1/2 -translate-x-1/2',
            ],
            // right arrow: sits to the left of the panel pointing left
            side === 'right' && [
              'border-b border-l',
              'right-full top-1/2 -translate-y-1/2 translate-x-1/2',
            ],
          )}
        />
      </span>
    </span>
  );
}
