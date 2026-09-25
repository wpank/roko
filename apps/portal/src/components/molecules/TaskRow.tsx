'use client';

import React from 'react';
import { clsx } from 'clsx';
import type { TaskState, TaskStatus } from '@/api/types';

export interface TaskRowProps {
  task: TaskState;
  onClick?: () => void;
  /** Compact: single line. Full: two lines with dep count. */
  compact?: boolean;
}

// ---------------------------------------------------------------------------
// Status icon + label config
// ---------------------------------------------------------------------------

interface StatusConfig {
  icon: string;
  label: string;
  iconColor: string;
  labelColor: string;
}

const STATUS_CONFIG: Record<TaskStatus, StatusConfig> = {
  pending:     { icon: '○', label: 'pending',     iconColor: 'text-[var(--text-ghost)]',   labelColor: 'text-[var(--text-faint)]' },
  dispatching: { icon: '○', label: 'dispatching', iconColor: 'text-[var(--warning)]',      labelColor: 'text-[var(--warning)]' },
  running:     { icon: '●', label: 'running',     iconColor: 'text-[var(--rose)]',         labelColor: 'text-[var(--rose)]' },
  gating:      { icon: '◎', label: 'gating',      iconColor: 'text-[var(--dream)]',        labelColor: 'text-[var(--dream)]' },
  completed:   { icon: '✓', label: 'done',        iconColor: 'text-[var(--sage)]',         labelColor: 'text-[var(--sage)]' },
  failed:      { icon: '✗', label: 'failed',      iconColor: 'text-[var(--accent-error)]', labelColor: 'text-[var(--accent-error)]' },
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatElapsed(startedAt: string | null, completedAt: string | null): string {
  if (!startedAt) return '—';
  const end = completedAt ? new Date(completedAt) : new Date();
  const ms = end.getTime() - new Date(startedAt).getTime();
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  const m = Math.floor(ms / 60_000);
  const s = Math.floor((ms % 60_000) / 1000);
  return `${m}m${s.toString().padStart(2, '0')}s`;
}

function abbreviateModel(model: string | null): string {
  if (!model) return '';
  // Shorten common model identifiers for compact display
  return model
    .replace('claude-', 'cl-')
    .replace('gpt-', 'gpt-')
    .replace('gemini-', 'gem-')
    .split('-')
    .slice(0, 3)
    .join('-');
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function TaskRow({ task, onClick, compact = false }: TaskRowProps) {
  const cfg = STATUS_CONFIG[task.status] ?? STATUS_CONFIG.pending;
  const elapsed = formatElapsed(task.startedAt, task.completedAt);
  const modelLabel = abbreviateModel(task.model);

  if (compact) {
    return (
      <div
        role={onClick ? 'button' : undefined}
        tabIndex={onClick ? 0 : undefined}
        onClick={onClick}
        onKeyDown={onClick ? (e) => { if (e.key === 'Enter' || e.key === ' ') onClick(); } : undefined}
        className={clsx(
          'flex items-center gap-2 px-3 py-1',
          onClick && 'cursor-pointer hover:bg-[var(--bg-highlight)]',
          'border-b border-b-[var(--text-ghost)]',
          'transition-[background-color] duration-[80ms] ease-[var(--ease-out)]',
        )}
      >
        {/* Status icon */}
        <span
          className={clsx(
            'shrink-0 w-3 text-center font-[var(--font-mono)] text-[var(--text-xs)] leading-none select-none',
            cfg.iconColor,
            task.status === 'running' && 'animate-pulse',
          )}
          aria-label={cfg.label}
        >
          {cfg.icon}
        </span>

        {/* Task name */}
        <span className="flex-1 truncate font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-strong)] leading-none">
          {task.name}
        </span>

        {/* Model badge */}
        {modelLabel && (
          <span className="shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)] leading-none">
            {modelLabel}
          </span>
        )}

        {/* Elapsed */}
        <span className="shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] num tabular-nums leading-none w-14 text-right">
          {elapsed}
        </span>
      </div>
    );
  }

  // Full mode: two lines
  return (
    <div
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
      onClick={onClick}
      onKeyDown={onClick ? (e) => { if (e.key === 'Enter' || e.key === ' ') onClick(); } : undefined}
      className={clsx(
        'flex flex-col gap-1 px-3 py-2',
        onClick && 'cursor-pointer hover:bg-[var(--bg-highlight)]',
        'border-b border-b-[var(--text-ghost)]',
        'transition-[background-color] duration-[80ms] ease-[var(--ease-out)]',
      )}
    >
      {/* Line 1: status icon + name + elapsed */}
      <div className="flex items-center gap-2 min-w-0">
        <span
          className={clsx(
            'shrink-0 w-3 text-center font-[var(--font-mono)] text-[var(--text-sm)] leading-none select-none',
            cfg.iconColor,
            task.status === 'running' && 'animate-pulse',
          )}
          aria-label={cfg.label}
        >
          {cfg.icon}
        </span>

        <span className="flex-1 truncate font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)] leading-none tracking-[var(--tracking-normal)]">
          {task.name}
        </span>

        <span className="shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] num tabular-nums leading-none">
          {elapsed}
        </span>
      </div>

      {/* Line 2: model badge + status label + dep count */}
      <div className="flex items-center gap-3 pl-5">
        {modelLabel && (
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)] leading-none border border-[var(--text-ghost)] px-1">
            {modelLabel}
          </span>
        )}

        <span
          className={clsx(
            'font-[var(--font-mono)] text-[var(--text-xs)] leading-none tracking-[var(--tracking-wider)]',
            cfg.labelColor,
          )}
        >
          {cfg.label}
        </span>

        {task.dependsOn.length > 0 && (
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] leading-none">
            deps:{task.dependsOn.length}
          </span>
        )}

        {task.wave > 0 && (
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)] leading-none">
            w{task.wave}
          </span>
        )}
      </div>
    </div>
  );
}
