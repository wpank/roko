'use client';

import React from 'react';
import { clsx } from 'clsx';
import type { PlanState, PlanStatus } from '@/api/types';
import { ProgressBar } from '@/components/atoms';

export interface PlanCardProps {
  plan: PlanState;
  onClick?: () => void;
  selected?: boolean;
}

// ---------------------------------------------------------------------------
// Status badge config
// ---------------------------------------------------------------------------

interface BadgeConfig {
  label: string;
  color: string;
}

const STATUS_BADGE: Record<PlanStatus, BadgeConfig> = {
  pending:   { label: 'PENDING',   color: 'text-[var(--text-muted)]' },
  running:   { label: 'RUNNING',   color: 'text-[var(--warning)]' },
  gating:    { label: 'GATING',    color: 'text-[var(--dream)]' },
  completed: { label: 'COMPLETED', color: 'text-[var(--sage)]' },
  failed:    { label: 'FAILED',    color: 'text-[var(--accent-error)]' },
  paused:    { label: 'PAUSED',    color: 'text-[var(--text-muted)]' },
  cancelled: { label: 'CANCELLED', color: 'text-[var(--text-faint)]' },
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatCost(usd: number): string {
  if (usd < 0.01) return '$0.00';
  if (usd < 1) return `$${usd.toFixed(3)}`;
  return `$${usd.toFixed(2)}`;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function PlanCard({ plan, onClick, selected = false }: PlanCardProps) {
  const badge = STATUS_BADGE[plan.status] ?? STATUS_BADGE.pending;
  const { total, completed, failed } = plan.progress;
  const agentCount = plan.tasks.length; // proxy; real agent count would come from tasks

  return (
    <div
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
      onClick={onClick}
      onKeyDown={onClick ? (e) => { if (e.key === 'Enter' || e.key === ' ') onClick(); } : undefined}
      className={clsx(
        // Base layout
        'relative flex flex-col gap-2 px-3 py-2',
        // Left border accent when selected
        selected
          ? 'border-l-2 border-l-[var(--rose)] bg-[var(--bg-highlight)]'
          : 'border-l-2 border-l-transparent',
        // Hover state
        onClick && 'cursor-pointer hover:bg-[var(--bg-highlight)]',
        // Bottom separator
        'border-b border-b-[var(--text-ghost)]',
        // Transition
        'transition-[background-color,border-color] duration-[80ms] ease-[var(--ease-out)]',
      )}
    >
      {/* Row 1: name + status badge */}
      <div className="flex items-center justify-between gap-2 min-w-0">
        <span
          className="truncate font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)] leading-none tracking-[var(--tracking-wide)]"
        >
          {plan.name}
        </span>
        <span
          className={clsx(
            'shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] leading-none tracking-[var(--tracking-wider)]',
            badge.color,
          )}
        >
          {badge.label}
        </span>
      </div>

      {/* Progress bar */}
      <ProgressBar
        value={total > 0 ? Math.round((completed / total) * 100) : 0}
        variant={plan.status === 'failed' ? 'cost' : 'default'}
        height={2}
      />

      {/* Row 3: task counts + cost + agents */}
      <div className="flex items-center gap-4">
        {/* Done / total */}
        <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] num tabular-nums">
          <span className="text-[var(--text-muted)]">{completed}</span>
          <span className="text-[var(--text-ghost)]">/{total}</span>
          {failed > 0 && (
            <span className="ml-1 text-[var(--accent-error)]">({failed}✗)</span>
          )}
        </span>

        {/* Cost */}
        <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] num tabular-nums">
          {formatCost(plan.costUsd)}
        </span>

        {/* Agent count */}
        {agentCount > 0 && (
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] num tabular-nums ml-auto">
            {agentCount} agent{agentCount !== 1 ? 's' : ''}
          </span>
        )}
      </div>
    </div>
  );
}
