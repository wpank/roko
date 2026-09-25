'use client';

import React from 'react';
import { clsx } from 'clsx';
import type { InboxItem as InboxItemType, AlertSeverity, InboxItemKind } from '@/api/types';
import { Button } from '@/components/atoms';

export interface InboxItemProps {
  item: InboxItemType;
  onDismiss?: (id: string) => void;
  onAction?: (url: string) => void;
}

// ---------------------------------------------------------------------------
// Severity config
// ---------------------------------------------------------------------------

interface SeverityConfig {
  icon: string;
  color: string;
  borderColor: string;
}

const SEVERITY: Record<AlertSeverity, SeverityConfig> = {
  error:   { icon: '✗', color: 'var(--ember)',        borderColor: 'var(--ember)' },
  warning: { icon: '⚠', color: 'var(--warning)',      borderColor: 'var(--warning)' },
  success: { icon: '✓', color: 'var(--sage)',          borderColor: 'var(--sage)' },
  info:    { icon: 'ℹ', color: 'var(--dream-bright)', borderColor: 'var(--dream)' },
};

// ---------------------------------------------------------------------------
// Action label per kind
// ---------------------------------------------------------------------------

const ACTION_LABEL: Partial<Record<InboxItemKind, string>> = {
  gate_failure:      'Retry',
  plan_completed:    'View',
  approval_needed:   'Review',
  budget_warning:    'View',
  provider_degraded: 'View',
  cost_anomaly:      'View',
  safety_incident:   'View',
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTimestamp(iso: string): string {
  try {
    const d = new Date(iso);
    const now = Date.now();
    const ms = now - d.getTime();
    if (ms < 60_000)   return 'just now';
    if (ms < 3_600_000) return `${Math.floor(ms / 60_000)}m ago`;
    if (ms < 86_400_000) return `${Math.floor(ms / 3_600_000)}h ago`;
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  } catch {
    return iso;
  }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function InboxItem({ item, onDismiss, onAction }: InboxItemProps) {
  const isDismissed = !!item.dismissedAt;
  const sev         = SEVERITY[item.severity] ?? SEVERITY.info;
  const actionLabel = ACTION_LABEL[item.kind];

  return (
    <div
      style={{ borderLeftColor: sev.borderColor }}
      className={clsx(
        'flex flex-col gap-1.5 border-l-2 px-3 py-2',
        'border-b border-b-[var(--text-ghost)]',
        isDismissed && 'opacity-40',
        'transition-opacity duration-[80ms]',
      )}
    >
      {/* Row 1: icon + title + timestamp */}
      <div className="flex items-start gap-2 min-w-0">
        <span
          className="shrink-0 w-3 text-center font-[var(--font-mono)] text-[var(--text-sm)] leading-none select-none mt-px"
          style={{ color: sev.color }}
          aria-label={item.severity}
        >
          {sev.icon}
        </span>

        <span className="flex-1 font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)] leading-snug">
          {item.title}
        </span>

        <span className="shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)] num tabular-nums leading-none whitespace-nowrap">
          {formatTimestamp(item.timestamp)}
        </span>
      </div>

      {/* Row 2: description */}
      {item.description && (
        <p className="pl-5 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] leading-snug line-clamp-2">
          {item.description}
        </p>
      )}

      {/* Row 3: action buttons */}
      {!isDismissed && (
        <div className="flex items-center gap-2 pl-5 pt-0.5">
          {/* Contextual action */}
          {actionLabel && item.actionUrl && onAction && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onAction(item.actionUrl!)}
            >
              {actionLabel}
            </Button>
          )}

          {/* Dismiss */}
          {onDismiss && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onDismiss(item.id)}
            >
              Dismiss
            </Button>
          )}
        </div>
      )}
    </div>
  );
}
