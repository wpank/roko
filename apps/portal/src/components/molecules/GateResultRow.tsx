'use client';

import React, { useState } from 'react';
import { clsx } from 'clsx';
import type { GateResult } from '@/api/types';

export interface GateResultRowProps {
  gate: GateResult;
  expandable?: boolean;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatDuration(ms: number): string {
  if (ms < 1000)     return `${ms}ms`;
  if (ms < 60_000)   return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60_000).toFixed(1)}m`;
}

function formatTimestamp(iso: string): string {
  try {
    const d = new Date(iso);
    const hh = d.getHours().toString().padStart(2, '0');
    const mm = d.getMinutes().toString().padStart(2, '0');
    const ss = d.getSeconds().toString().padStart(2, '0');
    return `${hh}:${mm}:${ss}`;
  } catch {
    return iso;
  }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function GateResultRow({ gate, expandable = false }: GateResultRowProps) {
  const [expanded, setExpanded] = useState(false);

  const canExpand = expandable && !!gate.output;

  return (
    <div
      className={clsx(
        'flex flex-col',
        'border-b border-b-[var(--text-ghost)]',
      )}
    >
      {/* Summary row */}
      <div
        role={canExpand ? 'button' : undefined}
        tabIndex={canExpand ? 0 : undefined}
        onClick={canExpand ? () => setExpanded((x) => !x) : undefined}
        onKeyDown={canExpand ? (e) => { if (e.key === 'Enter' || e.key === ' ') setExpanded((x) => !x); } : undefined}
        className={clsx(
          'flex items-center gap-2 px-3 py-2',
          canExpand && 'cursor-pointer hover:bg-[var(--bg-highlight)]',
          'transition-[background-color] duration-[80ms] ease-[var(--ease-out)]',
        )}
      >
        {/* Pass/fail icon */}
        <span
          className={clsx(
            'shrink-0 w-3 text-center font-[var(--font-mono)] text-[var(--text-sm)] leading-none select-none',
            gate.passed ? 'text-[var(--sage)]' : 'text-[var(--accent-error)]',
          )}
          aria-label={gate.passed ? 'pass' : 'fail'}
        >
          {gate.passed ? '✓' : '✗'}
        </span>

        {/* Gate name */}
        <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-strong)] leading-none tracking-[var(--tracking-wide)]">
          {gate.gateName}
        </span>

        {/* Rung number */}
        <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)] leading-none border border-[var(--text-ghost)] px-1">
          r{gate.rung}
        </span>

        {/* Summary */}
        <span className="flex-1 truncate font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] leading-none">
          {gate.summary}
        </span>

        {/* Duration */}
        <span className="shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] num tabular-nums leading-none">
          {formatDuration(gate.durationMs)}
        </span>

        {/* Timestamp */}
        <span className="shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)] num tabular-nums leading-none w-16 text-right">
          {formatTimestamp(gate.timestamp)}
        </span>

        {/* Expand chevron */}
        {canExpand && (
          <span
            className={clsx(
              'shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] leading-none select-none',
              'transition-transform duration-[80ms]',
              expanded && 'rotate-90',
            )}
            aria-hidden
          >
            ›
          </span>
        )}
      </div>

      {/* Expanded output */}
      {expanded && gate.output && (
        <div
          className={clsx(
            'px-3 pb-3 pt-1',
            'bg-[var(--bg-secondary)]',
            'border-t border-t-[var(--text-ghost)]',
          )}
        >
          <pre
            className={clsx(
              'font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)]',
              'whitespace-pre-wrap break-words leading-relaxed',
              'max-h-48 overflow-y-auto',
            )}
          >
            {gate.output}
          </pre>
        </div>
      )}
    </div>
  );
}
