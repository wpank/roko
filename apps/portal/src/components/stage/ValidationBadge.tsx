'use client';

/**
 * ValidationBadge
 *
 * Shows the validation state of a plan as a compact inline badge.
 *
 * States:
 *  - Loading                → `·`
 *  - 404 / 405              → `validation unavailable` (server lacks the route;
 *                              never blocks Run)
 *  - errors > 0             → `N errors`  (error token — plan cannot run)
 *  - warnings > 0, no errors→ `N warnings` (muted — a normal editing state)
 *  - clean                  → `valid ✓`  (done token)
 *
 * Clicking the badge opens a panel listing diagnostics as
 * `rule_id · task · message`; clicking a row with a task_id calls
 * `onSelectTask`.
 */

import React, { useRef, useEffect, useState } from 'react';
import { clsx } from 'clsx';
import { useValidation } from '@/api/queries';
import { isMissingRoute } from '@/lib/apiErrors';
import type { WireDiagnostic } from '@/api/contracts';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface ValidationBadgeProps {
  planId: string;
  onSelectTask(id: string): void;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function ValidationBadge({ planId, onSelectTask }: ValidationBadgeProps) {
  const { data, isLoading, error } = useValidation(planId);
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Close panel when clicking outside the container.
  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, [open]);

  // -------------------------------------------------------------------------
  // Loading state
  // -------------------------------------------------------------------------

  if (isLoading) {
    return (
      <span className="text-text-faint font-mono text-xs select-none" aria-label="validating">
        validating…
      </span>
    );
  }

  // -------------------------------------------------------------------------
  // Missing-route state (404 / 405)
  // -------------------------------------------------------------------------

  if (isMissingRoute(error)) {
    return (
      <span
        className="text-text-faint font-mono text-xs select-none cursor-default"
        title="The connected roko-serve instance does not expose the validation endpoint. Run is unaffected."
      >
        validation unavailable
      </span>
    );
  }

  // -------------------------------------------------------------------------
  // Determine badge label + variant from validation result
  // -------------------------------------------------------------------------

  const errorCount = data?.errors?.length ?? 0;
  const warnCount = data?.warnings?.length ?? 0;
  const diagnostics: WireDiagnostic[] = data?.diagnostics ?? [];

  // Badge display properties
  let label: string;
  let colorClasses: string;

  if (errorCount > 0) {
    label = `${errorCount} error${errorCount === 1 ? '' : 's'}`;
    // error token — uses accent-error palette
    colorClasses = 'text-accent-error border-accent-error hover:bg-accent-error/10';
  } else if (warnCount > 0) {
    label = `${warnCount} warning${warnCount === 1 ? '' : 's'}`;
    // muted — a normal, non-alarming editing state
    colorClasses = 'text-text-faint border-text-ghost hover:bg-bg-highlight';
  } else {
    label = 'valid ✓';
    // done token — uses sage/success palette
    colorClasses = 'text-sage border-sage hover:bg-sage/10';
  }

  // -------------------------------------------------------------------------
  // Render badge + optional diagnostics panel
  // -------------------------------------------------------------------------

  return (
    <div ref={containerRef} className="relative inline-flex">
      {/* Badge trigger */}
      <button
        type="button"
        onClick={() => {
          // Only open panel when there is something to show
          if (diagnostics.length > 0) setOpen((v) => !v);
        }}
        aria-expanded={open}
        aria-haspopup={diagnostics.length > 0 ? 'listbox' : undefined}
        className={clsx(
          'inline-flex items-center',
          'font-mono font-medium uppercase tracking-widest',
          'text-[10px] leading-none',
          'px-1.5 py-0.5',
          'border',
          'transition-colors duration-[80ms]',
          colorClasses,
          diagnostics.length > 0 ? 'cursor-pointer' : 'cursor-default',
        )}
      >
        {label}
      </button>

      {/* Diagnostics panel */}
      {open && diagnostics.length > 0 && (
        <div
          role="listbox"
          aria-label="validation diagnostics"
          className={clsx(
            'absolute top-full left-0 mt-1 z-[600]',
            'min-w-[22rem] max-w-[36rem]',
            'bg-bg-raised border border-text-ghost',
            'shadow-lg',
            'py-1',
            'overflow-y-auto max-h-72',
          )}
        >
          {diagnostics.map((d, idx) => (
            <DiagnosticRow
              key={idx}
              diagnostic={d}
              onSelectTask={(id) => {
                onSelectTask(id);
                setOpen(false);
              }}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// DiagnosticRow — one line in the panel
// ---------------------------------------------------------------------------

interface DiagnosticRowProps {
  diagnostic: WireDiagnostic;
  onSelectTask(id: string): void;
}

function DiagnosticRow({ diagnostic, onSelectTask }: DiagnosticRowProps) {
  const { rule_id, task_id, message, severity } = diagnostic;
  const isClickable = Boolean(task_id);

  return (
    <div
      role="option"
      aria-selected={false}
      onClick={() => {
        if (task_id) onSelectTask(task_id);
      }}
      className={clsx(
        'flex items-baseline gap-1',
        'px-3 py-1.5',
        'font-mono text-xs leading-snug',
        'border-b border-text-ghost/30 last:border-b-0',
        isClickable
          ? 'cursor-pointer hover:bg-bg-highlight'
          : 'cursor-default',
        severity === 'error'
          ? 'text-accent-error'
          : 'text-text-muted',
      )}
    >
      {/* rule_id */}
      <span className="shrink-0 text-text-faint">{rule_id}</span>
      <span className="shrink-0 text-text-ghost">·</span>

      {/* task id (if present) */}
      {task_id ? (
        <>
          <span className="shrink-0 text-text-muted truncate max-w-[10rem]">{task_id}</span>
          <span className="shrink-0 text-text-ghost">·</span>
        </>
      ) : (
        <>
          <span className="shrink-0 text-text-ghost">—</span>
          <span className="shrink-0 text-text-ghost">·</span>
        </>
      )}

      {/* message */}
      <span className="flex-1 whitespace-pre-wrap break-words">{message}</span>
    </div>
  );
}
