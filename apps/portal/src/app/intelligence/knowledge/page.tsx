'use client';

import React, { useState, useMemo, useCallback } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { clsx } from 'clsx';
import { Badge } from '@/components/atoms/Badge';
import { Button } from '@/components/atoms/Button';
import { Pill } from '@/components/atoms/Pill';
import { Spinner } from '@/components/atoms/Spinner';
import { useKnowledgeQuery, useKnowledgeStats, useKnowledgeGC } from '@/api/hooks';
import type { KnowledgeTier, KnowledgeEntry } from '@/api/types';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TIER_ORDER: KnowledgeTier[] = ['transient', 'working', 'consolidated', 'persistent'];

const TIER_COLORS: Record<KnowledgeTier, string> = {
  transient:    'var(--warning)',
  working:      'var(--dream)',
  consolidated: 'var(--rose-dim)',
  persistent:   'var(--rose-glow)',
};

const TIER_BADGE_VARIANTS: Record<KnowledgeTier, 'warning' | 'dream' | 'default' | 'success'> = {
  transient:    'warning',
  working:      'dream',
  consolidated: 'default',
  persistent:   'success',
};

type KindFilter = 'all' | 'heuristic' | 'warning' | 'causal_link' | 'insight' | 'strategy_fragment';
type TierFilter = 'all' | KnowledgeTier;
type SortKey    = 'tier' | 'confirmations' | 'createdAt';
type SortDir    = 'asc' | 'desc';
type SearchMode = 'semantic' | 'keyword';

const KIND_LABELS: Record<Exclude<KindFilter, 'all'>, string> = {
  heuristic:         'Heuristic',
  warning:           'Warning',
  causal_link:       'Causal Link',
  insight:           'Insight',
  strategy_fragment: 'Strategy',
};

const KIND_ICONS: Record<Exclude<KindFilter, 'all'>, string> = {
  heuristic:         'H',
  warning:           '!',
  causal_link:       '→',
  insight:           '◎',
  strategy_fragment: '⊞',
};

const KIND_ICON_COLORS: Record<Exclude<KindFilter, 'all'>, string> = {
  heuristic:         'var(--accent-cyan)',
  warning:           'var(--warning)',
  causal_link:       'var(--dream)',
  insight:           'var(--rose-glow)',
  strategy_fragment: 'var(--sage)',
};

const TIER_SORT_ORDER: Record<KnowledgeTier, number> = {
  transient:    0,
  working:      1,
  consolidated: 2,
  persistent:   3,
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

/** Derive kind from KnowledgeEntry.domain (placeholder heuristic). */
function deriveKind(entry: KnowledgeEntry): Exclude<KindFilter, 'all'> {
  const domain = (entry.domain ?? '').toLowerCase();
  if (domain.includes('warn') || domain.includes('risk'))   return 'warning';
  if (domain.includes('cause') || domain.includes('link'))  return 'causal_link';
  if (domain.includes('strategy') || domain.includes('plan')) return 'strategy_fragment';
  if (domain.includes('insight') || domain.includes('obs'))   return 'insight';
  return 'heuristic';
}

// ---------------------------------------------------------------------------
// Tier Distribution Bar
// ---------------------------------------------------------------------------

function TierDistributionBar({ byTier }: { byTier: Record<string, number> }) {
  const total = TIER_ORDER.reduce((s, t) => s + (byTier[t] ?? 0), 0);
  if (total === 0) return null;

  return (
    <div className="flex flex-col gap-1.5">
      {/* Stacked bar */}
      <div className="flex w-full h-2 bg-bg-highlight overflow-hidden" style={{ gap: '1px' }}>
        {TIER_ORDER.map((tier) => {
          const count = byTier[tier] ?? 0;
          const pct   = (count / total) * 100;
          return (
            <div
              key={tier}
              title={`${tier}: ${count}`}
              style={{
                width:           `${pct}%`,
                backgroundColor: TIER_COLORS[tier],
                transition:      'width 300ms ease-out',
                minWidth:        count > 0 ? 2 : 0,
              }}
            />
          );
        })}
      </div>

      {/* Legend */}
      <div className="flex items-center gap-4 flex-wrap">
        {TIER_ORDER.map((tier) => {
          const count = byTier[tier] ?? 0;
          const pct   = total > 0 ? ((count / total) * 100).toFixed(0) : '0';
          return (
            <div key={tier} className="flex items-center gap-1.5">
              <span className="w-2 h-2 shrink-0" style={{ backgroundColor: TIER_COLORS[tier] }} />
              <span className="font-mono text-xs text-text-ghost capitalize">{tier}</span>
              <span className="font-mono text-xs text-text-faint num tabular-nums">
                {count.toLocaleString()} ({pct}%)
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Sort header cell
// ---------------------------------------------------------------------------

function SortableHeader({
  label,
  sortKey,
  currentKey,
  currentDir,
  onSort,
  className,
}: {
  label: string;
  sortKey: SortKey;
  currentKey: SortKey;
  currentDir: SortDir;
  onSort: (key: SortKey) => void;
  className?: string;
}) {
  const isActive = sortKey === currentKey;

  return (
    <button
      type="button"
      onClick={() => onSort(sortKey)}
      className={clsx(
        'font-mono text-xs text-left',
        'transition-colors duration-[80ms]',
        isActive ? 'text-text-strong' : 'text-text-ghost hover:text-text-faint',
        className,
      )}
    >
      {label}
      {isActive && (
        <span className="ml-1 text-rose-dim">
          {currentDir === 'asc' ? '↑' : '↓'}
        </span>
      )}
    </button>
  );
}

// ---------------------------------------------------------------------------
// Knowledge Entry Row
// ---------------------------------------------------------------------------

function KnowledgeRow({ entry }: { entry: KnowledgeEntry }) {
  const router = useRouter();
  const kind   = deriveKind(entry);

  return (
    <div
      role="row"
      tabIndex={0}
      onClick={() => router.push(`/intelligence/knowledge/${entry.id}`)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          router.push(`/intelligence/knowledge/${entry.id}`);
        }
      }}
      className={clsx(
        'flex items-center gap-4 px-4 py-2.5',
        'border-b border-b-[var(--text-ghost)]',
        'cursor-pointer hover:bg-[var(--bg-highlight)]',
        'transition-[background-color] duration-[80ms] ease-[var(--ease-out)]',
        'group outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose)]',
      )}
      aria-label={entry.content}
    >
      {/* Kind icon */}
      <span
        className="shrink-0 w-5 h-5 flex items-center justify-center border font-mono text-xs font-medium select-none"
        style={{
          borderColor: KIND_ICON_COLORS[kind],
          color:       KIND_ICON_COLORS[kind],
        }}
        title={kind.replace(/_/g, ' ')}
      >
        {KIND_ICONS[kind]}
      </span>

      {/* Topic / content snippet */}
      <span className="flex-1 min-w-0 font-mono text-xs text-text-strong truncate group-hover:text-bone-bright transition-colors duration-[80ms]">
        {entry.content}
      </span>

      {/* Tier badge */}
      <div className="shrink-0 w-24 flex justify-end">
        <Badge variant={TIER_BADGE_VARIANTS[entry.tier]}>
          {entry.tier}
        </Badge>
      </div>

      {/* Confirmations */}
      <span className="shrink-0 w-10 font-mono text-xs text-text-faint num tabular-nums text-right">
        ×{entry.confirmations}
      </span>

      {/* Source / domain */}
      <span className="shrink-0 w-28 font-mono text-xs text-text-ghost truncate text-right hidden md:block">
        {entry.domain || '—'}
      </span>

      {/* Acquired */}
      <span className="shrink-0 w-16 font-mono text-xs text-text-ghost num tabular-nums text-right">
        {formatRelativeTime(entry.createdAt)}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Empty / error states
// ---------------------------------------------------------------------------

function EmptyState({ query }: { query: string }) {
  return (
    <div className="flex flex-col items-center gap-3 py-16 px-4">
      <span className="font-mono text-2xl text-text-ghost">◎</span>
      <span className="font-mono text-sm text-text-muted">
        {query.trim() ? `No results for "${query}"` : 'No entries match the current filters.'}
      </span>
      <span className="font-mono text-xs text-text-ghost">
        Try a broader search or remove filters.
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function KnowledgeBrowserPage() {
  const [searchText,    setSearchText]    = useState('');
  const [searchMode,    setSearchMode]    = useState<SearchMode>('semantic');
  const [tierFilter,    setTierFilter]    = useState<TierFilter>('all');
  const [kindFilter,    setKindFilter]    = useState<KindFilter>('all');
  const [sortKey,       setSortKey]       = useState<SortKey>('createdAt');
  const [sortDir,       setSortDir]       = useState<SortDir>('desc');

  // Debounced query — only fire the React Query call once the user stops typing.
  const [debouncedQuery, setDebouncedQuery] = useState('');
  const debounceRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleSearchChange = useCallback((value: string) => {
    setSearchText(value);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => setDebouncedQuery(value), 300);
  }, []);

  const knowledgeResult = useKnowledgeQuery(debouncedQuery, {
    tier:  tierFilter !== 'all' ? tierFilter : undefined,
    limit: 100,
  });

  const statsResult = useKnowledgeStats();
  const gcMutation  = useKnowledgeGC();

  // Client-side kind filter + sort (the API only filters by tier and semantic query)
  const displayEntries = useMemo(() => {
    const raw = knowledgeResult.data?.results ?? [];

    const filtered = kindFilter === 'all'
      ? raw
      : raw.filter((e) => deriveKind(e) === kindFilter);

    return [...filtered].sort((a, b) => {
      let cmp = 0;
      if (sortKey === 'tier') {
        cmp = TIER_SORT_ORDER[a.tier] - TIER_SORT_ORDER[b.tier];
      } else if (sortKey === 'confirmations') {
        cmp = a.confirmations - b.confirmations;
      } else {
        cmp = new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime();
      }
      return sortDir === 'asc' ? cmp : -cmp;
    });
  }, [knowledgeResult.data, kindFilter, sortKey, sortDir]);

  const handleSort = useCallback((key: SortKey) => {
    setSortKey((prev) => {
      if (prev === key) {
        setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
        return key;
      }
      setSortDir('desc');
      return key;
    });
  }, []);

  const totalCount = knowledgeResult.data?.total ?? 0;
  const byTier     = statsResult.data?.byTier ?? {};

  return (
    <div className="flex flex-col h-full min-h-0">

      {/* ---- Toolbar ---- */}
      <div className="flex flex-col gap-3 px-4 py-3 border-b border-b-[var(--text-ghost)] bg-[var(--bg-raised)] shrink-0">

        {/* Search row */}
        <div className="flex items-center gap-3">
          <div className="relative flex-1 max-w-xl">
            <input
              type="text"
              value={searchText}
              onChange={(e) => handleSearchChange(e.target.value)}
              placeholder={searchMode === 'semantic' ? 'Semantic search…' : 'Keyword search…'}
              className={clsx(
                'w-full font-mono text-xs text-text-strong',
                'bg-bg-secondary border border-[var(--border-default)]',
                'px-3 py-2 pr-8',
                'focus:border-[var(--border-active)] focus:bg-bg-highlight',
                'placeholder:text-text-ghost',
                'transition-[border-color,background-color] duration-[80ms]',
              )}
              aria-label="Search knowledge entries"
            />
            {(knowledgeResult.isFetching) && (
              <span className="absolute right-2 top-1/2 -translate-y-1/2">
                <Spinner size="sm" />
              </span>
            )}
          </div>

          {/* Semantic / keyword toggle */}
          <div className="flex items-center border border-[var(--border-default)] shrink-0">
            {(['semantic', 'keyword'] as SearchMode[]).map((mode) => (
              <button
                key={mode}
                type="button"
                onClick={() => setSearchMode(mode)}
                aria-pressed={searchMode === mode}
                className={clsx(
                  'px-2 py-1.5 font-mono text-xs select-none',
                  'transition-[background-color,color] duration-[80ms]',
                  searchMode === mode
                    ? 'bg-bg-highlight text-text-strong'
                    : 'bg-transparent text-text-ghost hover:text-text-faint',
                )}
              >
                {mode}
              </button>
            ))}
          </div>

          {/* GC button */}
          <Button
            variant="ghost"
            size="sm"
            loading={gcMutation.isPending}
            onClick={() => gcMutation.mutate()}
            title="Run knowledge garbage collection"
          >
            GC
          </Button>

          {/* Custody link */}
          <Link
            href="/intelligence/knowledge/custody"
            className="font-mono text-xs text-text-ghost hover:text-text-faint transition-colors duration-[80ms] shrink-0"
          >
            Custody →
          </Link>
        </div>

        {/* Filter pills row */}
        <div className="flex items-center gap-2 flex-wrap">
          {/* Tier pills */}
          <span className="font-mono text-xs text-text-ghost">tier:</span>
          {(['all', ...TIER_ORDER] as TierFilter[]).map((t) => (
            <Pill
              key={t}
              active={tierFilter === t}
              onClick={() => setTierFilter(t)}
            >
              {t === 'all' ? 'All' : t}
            </Pill>
          ))}

          <span className="font-mono text-xs text-text-ghost ml-2">kind:</span>
          {(['all', 'heuristic', 'warning', 'causal_link', 'insight', 'strategy_fragment'] as KindFilter[]).map((k) => (
            <Pill
              key={k}
              active={kindFilter === k}
              onClick={() => setKindFilter(k)}
            >
              {k === 'all' ? 'All' : KIND_LABELS[k]}
            </Pill>
          ))}
        </div>
      </div>

      {/* ---- Table area (scrollable) ---- */}
      <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden">

        {/* Column headers */}
        <div
          role="row"
          className="flex items-center gap-4 px-4 py-1.5 border-b border-b-[var(--text-ghost)] bg-[var(--bg-raised)] sticky top-0 z-10"
        >
          <span className="w-5 shrink-0" aria-hidden="true" />

          <span className="flex-1 font-mono text-xs text-text-ghost">topic</span>

          <div className="shrink-0 w-24 flex justify-end">
            <SortableHeader
              label="tier"
              sortKey="tier"
              currentKey={sortKey}
              currentDir={sortDir}
              onSort={handleSort}
            />
          </div>

          <SortableHeader
            label="conf"
            sortKey="confirmations"
            currentKey={sortKey}
            currentDir={sortDir}
            onSort={handleSort}
            className="shrink-0 w-10 text-right"
          />

          <span className="shrink-0 w-28 font-mono text-xs text-text-ghost text-right hidden md:block">
            source
          </span>

          <SortableHeader
            label="acquired"
            sortKey="createdAt"
            currentKey={sortKey}
            currentDir={sortDir}
            onSort={handleSort}
            className="shrink-0 w-16 text-right"
          />
        </div>

        {/* Row area */}
        {knowledgeResult.isLoading ? (
          <div className="flex items-center justify-center py-16">
            <Spinner size="md" />
          </div>
        ) : knowledgeResult.isError ? (
          <div className="flex items-center justify-center py-16">
            <span className="font-mono text-xs text-accent-error">
              Failed to load knowledge entries.
            </span>
          </div>
        ) : displayEntries.length === 0 ? (
          <EmptyState query={searchText} />
        ) : (
          <div role="table" aria-label="Knowledge entries">
            {displayEntries.map((entry) => (
              <KnowledgeRow key={entry.id} entry={entry} />
            ))}
          </div>
        )}
      </div>

      {/* ---- Footer: tier distribution bar + count ---- */}
      <div className="shrink-0 border-t border-t-[var(--text-ghost)] px-4 py-3 bg-[var(--bg-raised)] flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <span className="font-mono text-xs text-text-ghost">
            {displayEntries.length.toLocaleString()} shown
            {totalCount > 0 && ` of ${totalCount.toLocaleString()} total`}
          </span>
          {statsResult.isLoading && (
            <span className="font-mono text-xs text-text-ghost">Loading stats…</span>
          )}
        </div>
        {Object.keys(byTier).length > 0 && <TierDistributionBar byTier={byTier} />}
      </div>
    </div>
  );
}
