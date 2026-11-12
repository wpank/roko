import React from 'react';
import { cn } from '@/lib/cn';

/**
 * A key: a button with travel. `you` makes it the orange key, the next thing to do; a view has at most one.
 * `kbd` is its shortcut, shown on the cap.
 */
export function Key({
  you = false,
  size,
  kbd,
  className,
  children,
  ...rest
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  you?: boolean;
  size?: 's' | 'l';
  kbd?: string;
}) {
  return (
    <button
      type="button"
      {...rest}
      className={cn('rk-key', className)}
      data-you={you ? '' : undefined}
      data-size={size}
    >
      {children}
      {kbd && <kbd aria-hidden="true">{kbd}</kbd>}
    </button>
  );
}
