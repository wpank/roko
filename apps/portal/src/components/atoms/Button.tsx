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
    'bg-button-primary-bg text-button-primary-fg border-button-primary-bg',
    'enabled:hover:bg-button-primary-hover enabled:hover:border-button-primary-hover',
  ].join(' '),

  secondary: [
    'bg-bg-highlight text-text-muted border-text-ghost',
    'enabled:hover:border-border-hover enabled:hover:text-text-strong',
  ].join(' '),

  ghost: [
    'bg-transparent text-text-muted border-transparent',
    'enabled:hover:bg-bg-highlight enabled:hover:border-border-default enabled:hover:text-text-strong',
  ].join(' '),

  danger: [
    'bg-danger-fill text-danger-fill-fg border-danger-fill',
    'enabled:hover:border-danger-fill-fg enabled:hover:text-danger-fill-fg',
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
        // Disabled keeps pointer events: the hover shows the title saying why.
        isDisabled && 'opacity-40 cursor-not-allowed',
        className,
      )}
    >
      {loading && <Spinner size={spinnerSize[size]} />}
      {children}
    </button>
  );
}
