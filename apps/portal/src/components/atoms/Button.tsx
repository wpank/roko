'use client';

import React from 'react';
import { clsx } from 'clsx';
import { Spinner } from './Spinner';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
}

const variantClasses: Record<NonNullable<ButtonProps['variant']>, string> = {
  primary: [
    'bg-rose text-bone-bright border-rose',
    'hover:bg-rose-bright hover:border-rose-bright',
  ].join(' '),

  secondary: [
    'bg-bg-highlight text-text-muted border-text-ghost',
    'hover:border-border-hover hover:text-text-strong',
  ].join(' '),

  ghost: [
    'bg-transparent text-text-muted border-transparent',
    'hover:bg-bg-highlight hover:border-border-default hover:text-text-strong',
  ].join(' '),

  danger: [
    'bg-accent-error text-bone-bright border-accent-error',
    'hover:opacity-90',
  ].join(' '),
};

const sizeClasses: Record<NonNullable<ButtonProps['size']>, string> = {
  sm: 'px-2 py-1 text-xs gap-1',
  md: 'px-3 py-2 text-sm gap-2',
  lg: 'px-4 py-2 text-base gap-2',
};

const spinnerSize: Record<NonNullable<ButtonProps['size']>, 'sm' | 'md'> = {
  sm: 'sm',
  md: 'sm',
  lg: 'md',
};

export function Button({
  variant = 'secondary',
  size = 'md',
  loading = false,
  disabled,
  children,
  className,
  ...rest
}: ButtonProps) {
  const isDisabled = disabled || loading;

  return (
    <button
      {...rest}
      disabled={isDisabled}
      aria-busy={loading || undefined}
      className={clsx(
        'inline-flex items-center justify-center',
        'font-mono font-medium border select-none',
        'transition-[background-color,color,border-color,opacity]',
        'duration-[80ms]',
        variantClasses[variant],
        sizeClasses[size],
        isDisabled && 'opacity-40 cursor-not-allowed pointer-events-none',
        className,
      )}
    >
      {loading && <Spinner size={spinnerSize[size]} />}
      {children}
    </button>
  );
}
