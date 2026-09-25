'use client';

import React from 'react';
import { clsx } from 'clsx';
import {
  Briefcase,
  Bot,
  Shield,
  Brain,
  AlertTriangle,
} from 'lucide-react';
import { useDashboardStore, type ActivityItem } from '@/stores/dashboard';

export interface ActivityFeedProps {
  maxItems?: number;
}

// ---------------------------------------------------------------------------
// Icon mapping
// ---------------------------------------------------------------------------

type IconComponent = React.ComponentType<{ className?: string; size?: number }>;

const KIND_ICONS: Record<ActivityItem['kind'], IconComponent> = {
  plan_started:            Briefcase,
  plan_completed:          Briefcase,
  plan_failed:             Briefcase,
  task_completed:          Briefcase,
  task_failed:             Briefcase,
  gate_result:             Shield,
  agent_spawned:           Bot,
  agent_completed:         Bot,
  cost_updated:            Briefcase,
  budget_warning:          AlertTriangle,
  affect_updated:          Brain,
  cfactor_updated:         Brain,
  provider_health_updated: Shield,
  knowledge_tier_changed:  Brain,
  inbox_item_added:        AlertTriangle,
  error_occurred:          AlertTriangle,
};

const KIND_ICON_COLOR: Record<ActivityItem['kind'], string> = {
  plan_started:            'text-[var(--text-muted)]',
  plan_completed:          'text-[var(--sage)]',
  plan_failed:             'text-[var(--accent-error)]',
  task_completed:          'text-[var(--sage)]',
  task_failed:             'text-[var(--accent-error)]',
  gate_result:             'text-[var(--dream)]',
  agent_spawned:           'text-[var(--rose)]',
  agent_completed:         'text-[var(--text-muted)]',
  cost_updated:            'text-[var(--text-ghost)]',
  budget_warning:          'text-[var(--warning)]',
  affect_updated:          'text-[var(--dream)]',
  cfactor_updated:         'text-[var(--dream)]',
  provider_health_updated: 'text-[var(--text-muted)]',
  knowledge_tier_changed:  'text-[var(--dream)]',
  inbox_item_added:        'text-[var(--warning)]',
  error_occurred:          'text-[var(--accent-error)]',
};

// ---------------------------------------------------------------------------
// Relative time formatter
// ---------------------------------------------------------------------------

function relativeTime(iso: string): string {
  const diffMs = Date.now() - new Date(iso).getTime();
  if (diffMs < 1_000) return 'just now';
  if (diffMs < 60_000) return `${Math.floor(diffMs / 1_000)}s ago`;
  if (diffMs < 3_600_000) return `${Math.floor(diffMs / 60_000)}m ago`;
  if (diffMs < 86_400_000) return `${Math.floor(diffMs / 3_600_000)}h ago`;
  return `${Math.floor(diffMs / 86_400_000)}d ago`;
}

// ---------------------------------------------------------------------------
// Single feed item
// ---------------------------------------------------------------------------

interface FeedItemProps {
  item: ActivityItem;
}

function FeedItemRow({ item }: FeedItemProps) {
  const Icon = KIND_ICONS[item.kind] ?? Bot;
  const iconColor = KIND_ICON_COLOR[item.kind] ?? 'text-[var(--text-ghost)]';

  return (
    <div className="flex items-start gap-2.5 px-3 py-2 border-b border-[var(--text-ghost)] border-opacity-30 last:border-0">
      <span
        className={clsx('shrink-0 mt-0.5', iconColor)}
        aria-hidden="true"
      >
        <Icon size={12} />
      </span>

      <div className="flex-1 min-w-0">
        <p className="font-mono text-[11px] text-[var(--text-muted)] leading-snug truncate">
          {item.summary}
        </p>
      </div>

      <span
        className="shrink-0 font-mono text-[9px] text-[var(--text-ghost)] num tabular-nums whitespace-nowrap"
        title={item.timestamp}
      >
        {relativeTime(item.timestamp)}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function ActivityFeed({ maxItems = 20 }: ActivityFeedProps) {
  const allItems = useDashboardStore((s) => s.activityFeed);
  const items = allItems.slice(0, maxItems);

  return (
    <div className="flex flex-col border border-[var(--text-ghost)] bg-[var(--bg-raised)]">
      {/* Header */}
      <div className="px-3 py-1.5 border-b border-[var(--text-ghost)] shrink-0">
        <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
          activity
        </span>
      </div>

      {/* Feed list */}
      <div className="overflow-y-auto">
        {items.length === 0 ? (
          <div className="flex items-center justify-center h-16 text-[var(--text-ghost)] font-mono text-xs">
            no recent activity
          </div>
        ) : (
          items.map((item) => (
            <FeedItemRow key={item.id} item={item} />
          ))
        )}
      </div>
    </div>
  );
}
