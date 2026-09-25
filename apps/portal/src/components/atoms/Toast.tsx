'use client';

import React, { useEffect, useState, useCallback } from 'react';
import { clsx } from 'clsx';

// ---------------------------------------------------------------------------
// Toast store (simple module-level state)
// ---------------------------------------------------------------------------

type ToastVariant = 'success' | 'error' | 'info';

interface ToastItem {
  id: string;
  message: string;
  variant: ToastVariant;
  action?: { label: string; href: string };
}

let listeners: Array<() => void> = [];
let toasts: ToastItem[] = [];

function notify() {
  listeners.forEach((l) => l());
}

export function showToast(
  message: string,
  variant: ToastVariant = 'info',
  action?: { label: string; href: string },
) {
  const id = `toast-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
  toasts = [...toasts, { id, message, variant, action }];
  notify();
  // Auto-dismiss after 4 seconds
  setTimeout(() => {
    toasts = toasts.filter((t) => t.id !== id);
    notify();
  }, 4000);
}

function useToasts(): ToastItem[] {
  const [, forceUpdate] = useState(0);
  useEffect(() => {
    const listener = () => forceUpdate((n) => n + 1);
    listeners.push(listener);
    return () => {
      listeners = listeners.filter((l) => l !== listener);
    };
  }, []);
  return toasts;
}

// ---------------------------------------------------------------------------
// Toast container component — mount once in root layout
// ---------------------------------------------------------------------------

export function ToastContainer() {
  const items = useToasts();

  const dismiss = useCallback((id: string) => {
    toasts = toasts.filter((t) => t.id !== id);
    notify();
  }, []);

  if (items.length === 0) return null;

  return (
    <div className="fixed bottom-12 right-4 z-[9999] flex flex-col gap-2 pointer-events-none">
      {items.map((toast) => {
        const borderColor =
          toast.variant === 'success'
            ? 'var(--sage)'
            : toast.variant === 'error'
              ? 'var(--accent-error)'
              : 'var(--accent-cyan)';

        return (
          <div
            key={toast.id}
            className={clsx(
              'pointer-events-auto',
              'flex items-center gap-3 px-4 py-3',
              'bg-[var(--bg-raised)] border-l-2',
              'shadow-[var(--shadow-lg)]',
              'font-mono text-xs text-[var(--text-strong)]',
              'animate-[slideInRight_200ms_ease-out]',
              'min-w-[280px] max-w-[420px]',
            )}
            style={{ borderLeftColor: borderColor }}
            role="alert"
          >
            <span className="flex-1">{toast.message}</span>
            {toast.action && (
              <a
                href={toast.action.href}
                className="shrink-0 text-[var(--rose)] hover:text-[var(--rose-bright)] underline"
              >
                {toast.action.label}
              </a>
            )}
            <button
              type="button"
              onClick={() => dismiss(toast.id)}
              className="shrink-0 text-[var(--text-ghost)] hover:text-[var(--text-muted)] ml-1"
              aria-label="Dismiss"
            >
              ×
            </button>
          </div>
        );
      })}
      <style jsx>{`
        @keyframes slideInRight {
          from {
            opacity: 0;
            transform: translateX(100%);
          }
          to {
            opacity: 1;
            transform: translateX(0);
          }
        }
      `}</style>
    </div>
  );
}
