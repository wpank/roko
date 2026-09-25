'use client';

import React, { useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { clsx } from 'clsx';
import { api } from '@/api/client';
import { Badge } from '@/components/atoms';
import { Spinner } from '@/components/atoms/Spinner';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type BacklogPriority = 'critical' | 'high' | 'medium' | 'low';
type BacklogStatus   = 'open' | 'in_progress' | 'deferred' | 'closed';

interface BacklogEntry {
  id: string;
  title: string;
  description?: string;
  priority: BacklogPriority;
  status: BacklogStatus;
  tags: string[];
  createdAt: string;
}

// ---------------------------------------------------------------------------
// Raw API shapes
// ---------------------------------------------------------------------------

interface RawBacklogItem {
  id?: string;
  title?: string;
  description?: string;
  priority?: string;
  status?: string;
  tags?: string[];
  created_at?: string;
}

interface RawBacklogResponse {
  items?: RawBacklogItem[];
  backlog?: RawBacklogItem[];
}

// ---------------------------------------------------------------------------
// Mapper
// ---------------------------------------------------------------------------

function mapRawItem(raw: RawBacklogItem): BacklogEntry {
  return {
    id: raw.id ?? crypto.randomUUID(),
    title: raw.title ?? '(untitled)',
    description: raw.description,
    priority: (raw.priority ?? 'medium').toLowerCase() as BacklogPriority,
    status: (raw.status ?? 'open').toLowerCase() as BacklogStatus,
    tags: raw.tags ?? [],
    createdAt: raw.created_at ?? new Date().toISOString(),
  };
}

// ---------------------------------------------------------------------------
// Style maps
// ---------------------------------------------------------------------------

const PRIORITY_BADGE_VARIANT: Record<BacklogPriority, 'error' | 'warning' | 'info' | 'default'> = {
  critical: 'error',
  high:     'warning',
  medium:   'info',
  low:      'default',
};

const STATUS_COLOR: Record<BacklogStatus, string> = {
  open:        'var(--text-muted)',
  in_progress: 'var(--warning)',
  deferred:    'var(--text-ghost)',
  closed:      'var(--sage)',
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatRelativeTime(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  if (ms < 60_000)     return `${Math.floor(ms / 1000)}s ago`;
  if (ms < 3_600_000)  return `${Math.floor(ms / 60_000)}m ago`;
  if (ms < 86_400_000) return `${Math.floor(ms / 3_600_000)}h ago`;
  return `${Math.floor(ms / 86_400_000)}d ago`;
}

// ---------------------------------------------------------------------------
// BacklogRow
// ---------------------------------------------------------------------------

function BacklogRow({ item }: { item: BacklogEntry }) {
  return (
    <div
      className={clsx(
        'flex items-start gap-3 px-4 py-2.5',
        'border-b border-b-[var(--text-ghost)]',
        'hover:bg-[var(--bg-highlight)]',
        'transition-[background-color] duration-[80ms]',
      )}
    >
      <div className="shrink-0 pt-px">
        <Badge variant={PRIORITY_BADGE_VARIANT[item.priority]}>
          {item.priority}
        </Badge>
      </div>

      <div className="flex-1 min-w-0 flex flex-col gap-0.5">
        <span className="font-mono text-xs text-[var(--text-strong)] truncate">
          {item.title}
        </span>
        {item.description && (
          <span className="font-mono text-[10px] text-[var(--text-ghost)] truncate">
            {item.description}
          </span>
        )}
        {(item.tags?.length ?? 0) > 0 && (
          <div className="flex flex-wrap gap-1 mt-0.5">
            {(item.tags ?? []).map((tag) => (
              <span
                key={tag}
                className="font-mono text-[10px] text-[var(--text-faint)] border border-[var(--text-ghost)] px-1 leading-none py-[1px]"
              >
                {tag}
              </span>
            ))}
          </div>
        )}
      </div>

      <span
        className="shrink-0 font-mono text-[10px] tabular-nums"
        style={{ color: STATUS_COLOR[item.status] }}
      >
        {item.status.replace('_', ' ')}
      </span>

      <span className="shrink-0 font-mono text-[10px] text-[var(--text-ghost)] tabular-nums">
        {formatRelativeTime(item.createdAt)}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const PRIORITY_ORDER: BacklogPriority[] = ['critical', 'high', 'medium', 'low'];

const FILTER_OPTS: { label: string; value: BacklogStatus | 'all' }[] = [
  { label: 'Open',        value: 'open'        },
  { label: 'In Progress', value: 'in_progress' },
  { label: 'Deferred',    value: 'deferred'    },
  { label: 'All',         value: 'all'         },
];

export default function WorkBacklogPage() {
  const [statusFilter, setStatusFilter] = useState<BacklogStatus | 'all'>('open');

  const { data, isLoading, isError } = useQuery<RawBacklogResponse>({
    queryKey: ['backlog'],
    queryFn: () => api.get<RawBacklogResponse>('/api/backlog'),
    staleTime: 60_000,
  });

  const items = useMemo<BacklogEntry[]>(() => {
    const raw = data?.items ?? data?.backlog ?? [];
    return raw.map(mapRawItem);
  }, [data]);

  const visible = useMemo(() => {
    const base = statusFilter === 'all' ? items : items.filter((i) => i.status === statusFilter);
    return [...base].sort((a, b) => {
      const po = PRIORITY_ORDER.indexOf(a.priority) - PRIORITY_ORDER.indexOf(b.priority);
      if (po !== 0) return po;
      return new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime();
    });
  }, [items, statusFilter]);

  return (
    <div className="flex flex-col h-full min-h-0">

      {/* Filter bar */}
      <div className="flex items-center gap-2 px-4 py-2 shrink-0 bg-[var(--bg-secondary)] border-b border-b-[var(--text-ghost)]">
        {FILTER_OPTS.map((opt) => (
          <button
            key={opt.value}
            type="button"
            onClick={() => setStatusFilter(opt.value)}
            className={clsx(
              'font-mono text-[10px] px-2 py-0.5 border leading-none',
              'transition-[color,border-color] duration-[80ms]',
              statusFilter === opt.value
                ? 'text-[var(--text-strong)] border-[var(--rose-dim)]'
                : 'text-[var(--text-ghost)] border-[var(--text-ghost)] hover:text-[var(--text-muted)] hover:border-[var(--border-hover)]',
            )}
          >
            {opt.label}
          </button>
        ))}

        <span className="ml-auto font-mono text-[10px] text-[var(--text-ghost)] tabular-nums">
          {visible.length}/{items.length}
        </span>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden min-h-0">
        {isLoading ? (
          <div className="flex items-center justify-center h-40 gap-2">
            <Spinner size="sm" />
            <span className="font-mono text-xs text-[var(--text-ghost)]">Loading backlog…</span>
          </div>
        ) : isError ? (
          <div className="flex flex-col items-center justify-center h-40 gap-2">
            <span className="font-mono text-xs text-[var(--accent-error)]">
              Failed to load backlog. Is roko serve running?
            </span>
          </div>
        ) : visible.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-40 gap-2 px-4 text-center">
            <span className="font-mono text-xs text-[var(--text-ghost)]">
              {items.length === 0
                ? 'Backlog is empty. Import items with: roko backlog import'
                : 'No items match the current filter.'}
            </span>
          </div>
        ) : (
          visible.map((item) => <BacklogRow key={item.id} item={item} />)
        )}
      </div>
    </div>
  );
}
