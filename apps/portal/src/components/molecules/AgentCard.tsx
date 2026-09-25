'use client';

import React from 'react';
import { clsx } from 'clsx';
import type { AgentState, AgentStatus } from '@/api/types';
import { StatusLED, Badge } from '@/components/atoms';
import type { StatusLEDProps } from '@/components/atoms';

export interface AgentCardProps {
  agent: AgentState;
  selected?: boolean;
  onClick?: () => void;
  density?: 'compact' | 'standard' | 'wide';
}

// ---------------------------------------------------------------------------
// Mappings
// ---------------------------------------------------------------------------

const AGENT_STATUS_LED: Record<AgentStatus, StatusLEDProps['status']> = {
  idle:      'idle',
  active:    'active',
  completed: 'success',
  failed:    'error',
  stopped:   'offline',
};

type BadgeVariant = 'default' | 'success' | 'warning' | 'error' | 'info' | 'dream';

const AGENT_STATUS_BADGE: Record<AgentStatus, { label: string; variant: BadgeVariant }> = {
  idle:      { label: 'idle',    variant: 'default' },
  active:    { label: 'active',  variant: 'info' },
  completed: { label: 'done',    variant: 'success' },
  failed:    { label: 'failed',  variant: 'error' },
  stopped:   { label: 'stopped', variant: 'default' },
};

// ---------------------------------------------------------------------------
// Role → left border accent
// ---------------------------------------------------------------------------

function roleBorderColor(role: string): string {
  const r = role.toLowerCase();
  if (r.includes('planner') || r.includes('architect')) return 'var(--dream)';
  if (r.includes('reviewer') || r.includes('critic'))   return 'var(--warning)';
  if (r.includes('tester') || r.includes('gate'))       return 'var(--ember)';
  if (r.includes('researcher'))                         return 'var(--accent-cyan)';
  if (r.includes('writer') || r.includes('author'))     return 'var(--bone)';
  return 'var(--rose-dim)';
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000)     return `${(n / 1_000).toFixed(0)}k`;
  return String(n);
}

function formatCost(usd: number): string {
  if (usd < 0.001) return '$0.00';
  if (usd < 0.01)  return `$${usd.toFixed(4)}`;
  if (usd < 1)     return `$${usd.toFixed(3)}`;
  return `$${usd.toFixed(2)}`;
}

function contextColor(pct: number): string {
  if (pct >= 85) return 'var(--accent-error)';
  if (pct >= 60) return 'var(--warning)';
  return 'var(--text-faint)';
}

function formatAge(startedAt: string | null): string {
  if (!startedAt) return '—';
  const ms = Date.now() - new Date(startedAt).getTime();
  if (ms < 60_000)    return `${Math.floor(ms / 1000)}s`;
  if (ms < 3_600_000) return `${Math.floor(ms / 60_000)}m`;
  return `${Math.floor(ms / 3_600_000)}h`;
}

function abbreviateModel(model: string): string {
  return model.split('-').slice(0, 3).join('-');
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AgentCard({ agent, selected = false, onClick, density = 'standard' }: AgentCardProps) {
  const borderColor = selected ? 'var(--rose)' : roleBorderColor(agent.role);
  const isCompact   = density === 'compact';
  const isWide      = density === 'wide';

  const ledStatus  = AGENT_STATUS_LED[agent.status]  ?? 'idle';
  const badgeCfg   = AGENT_STATUS_BADGE[agent.status] ?? AGENT_STATUS_BADGE.idle;
  const isActive   = agent.status === 'active';

  return (
    <div
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
      onClick={onClick}
      onKeyDown={onClick ? (e) => { if (e.key === 'Enter' || e.key === ' ') onClick(); } : undefined}
      style={{ borderLeftColor: borderColor }}
      className={clsx(
        'relative flex flex-col gap-1 border-l-2 px-3',
        isCompact ? 'py-1.5' : 'py-2',
        selected ? 'bg-[var(--bg-highlight)]' : 'bg-transparent',
        onClick && 'cursor-pointer hover:bg-[var(--bg-highlight)]',
        'border-b border-b-[var(--text-ghost)]',
        'transition-[background-color,border-left-color] duration-[80ms] ease-[var(--ease-out)]',
      )}
    >
      {/* Primary row: LED + name + status badge (+ task snippet if compact) */}
      <div className="flex items-center gap-2 min-w-0">
        <StatusLED status={ledStatus} pulse={isActive} size="sm" />

        <span className="flex-1 truncate font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)] leading-none">
          {agent.name}
        </span>

        {/* Compact: task snippet before badge */}
        {isCompact && agent.currentTask && (
          <span className="shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)] leading-none max-w-[120px] truncate">
            {agent.currentTask}
          </span>
        )}

        <Badge variant={badgeCfg.variant}>{badgeCfg.label}</Badge>
      </div>

      {/* Secondary row: tokens (standard) — plus model / cost / ctx / age (wide) */}
      {!isCompact && (
        <div className="flex items-center gap-3 pl-4 flex-wrap">
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] num tabular-nums leading-none">
            {formatTokens(agent.tokensIn + agent.tokensOut)}&thinsp;tok
          </span>

          {isWide && (
            <>
              <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)] leading-none border border-[var(--text-ghost)] px-1">
                {abbreviateModel(agent.model)}
              </span>

              <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] num tabular-nums leading-none">
                {formatCost(agent.costUsd)}
              </span>

              <span
                className="font-[var(--font-mono)] text-[var(--text-xs)] num tabular-nums leading-none"
                style={{ color: contextColor(agent.contextPct) }}
              >
                ctx&thinsp;{agent.contextPct.toFixed(0)}%
              </span>

              <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)] num tabular-nums leading-none ml-auto">
                {formatAge(agent.startedAt)}
              </span>
            </>
          )}
        </div>
      )}
    </div>
  );
}
