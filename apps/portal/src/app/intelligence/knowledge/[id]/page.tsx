'use client';

// Route: /intelligence/knowledge/[id]
// Shows full detail of a single knowledge entry

import React, { useState, useCallback } from 'react';
import { useParams, useRouter } from 'next/navigation';
import { clsx } from 'clsx';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '@/api/client';
import { Badge } from '@/components/atoms/Badge';
import { Button } from '@/components/atoms/Button';
import { Spinner } from '@/components/atoms/Spinner';
import type { KnowledgeTier } from '@/api/types';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type KnowledgeKind =
  | 'Heuristic'
  | 'Warning'
  | 'CausalLink'
  | 'Insight'
  | 'StrategyFragment';

type ProvenanceAction =
  | 'created'
  | 'promoted'
  | 'demoted'
  | 'edited';

interface ConfirmationRecord {
  id: string;
  timestamp: string;
  episodeId: string;
  gateName: string;
  model: string;
  confidence: number;
}

interface Falsifier {
  id: string;
  condition: string;
  lastCheckAt: string | null;
  nextCheckAt: string | null;
}

interface ProvenanceRecord {
  id: string;
  action: ProvenanceAction;
  timestamp: string;
  actor: string;
  notes: string | null;
}

interface NeighborEntry {
  id: string;
  content: string;
  tier: KnowledgeTier;
  similarity: number;
}

interface KnowledgeDetail {
  id: string;
  kind: KnowledgeKind;
  content: string;
  tier: KnowledgeTier;
  domain: string;
  confidence: number;
  tags: string[];
  confirmations: number;
  hdcFingerprint: string | null;
  source: string;
  createdAt: string;
  confirmationRecords: ConfirmationRecord[];
  falsifiers: Falsifier[];
  provenance: ProvenanceRecord[];
}

// ---------------------------------------------------------------------------
// Color maps
// ---------------------------------------------------------------------------

const TIER_COLOR: Record<KnowledgeTier, string> = {
  transient:    'var(--warning)',
  working:      'var(--dream)',
  consolidated: 'var(--rose-dim)',
  persistent:   'var(--rose-glow)',
};

const TIER_BADGE_VARIANT: Record<KnowledgeTier, 'warning' | 'dream' | 'default' | 'info'> = {
  transient:    'warning',
  working:      'dream',
  consolidated: 'default',
  persistent:   'info',
};

const KIND_COLOR: Record<KnowledgeKind, string> = {
  Heuristic:        'var(--accent-cyan)',
  Warning:          'var(--accent-error)',
  CausalLink:       'var(--warning)',
  Insight:          'var(--dream-bright)',
  StrategyFragment: 'var(--sage)',
};

const PROVENANCE_ACTION_COLOR: Record<ProvenanceAction, string> = {
  created:  'var(--sage)',
  promoted: 'var(--dream-bright)',
  demoted:  'var(--warning)',
  edited:   'var(--accent-cyan)',
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTs(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleString('en-US', {
      year:   'numeric',
      month:  'short',
      day:    'numeric',
      hour:   '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    });
  } catch {
    return iso;
  }
}

function formatRelativeTime(iso: string | null): string {
  if (!iso) return '—';
  try {
    const diff = Date.now() - new Date(iso).getTime();
    if (diff < 60_000) return 'just now';
    if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
    if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
    return `${Math.floor(diff / 86_400_000)}d ago`;
  } catch {
    return iso;
  }
}

function looksLikeCode(content: string): boolean {
  return (
    content.includes('fn ') ||
    content.includes('let ') ||
    content.includes('const ') ||
    content.includes('def ') ||
    content.includes('import ') ||
    content.includes('```') ||
    /^\s{4}/.test(content)
  );
}

// ---------------------------------------------------------------------------
// Mock / fallback factory for development (replaced by real API)
// ---------------------------------------------------------------------------

function makeMockDetail(id: string): KnowledgeDetail {
  const now = new Date().toISOString();
  return {
    id,
    kind: 'Heuristic',
    content:
      'When dispatching to claude-opus-4-6 with a context window above 60%, ' +
      'enable compression of prior conversation turns to avoid degraded generation quality. ' +
      'This pattern reliably reduces token usage by ~18% with no observable quality loss ' +
      'across 94 confirmed gate-passing episodes.',
    tier: 'working',
    domain: 'dispatch',
    confidence: 0.84,
    tags: ['dispatch', 'context-management', 'claude-opus-4-6'],
    confirmations: 12,
    hdcFingerprint: 'a3f9e2c1b74d6e8f',
    source: 'gate/rung-2',
    createdAt: new Date(Date.now() - 86_400_000 * 3).toISOString(),
    confirmationRecords: [
      {
        id: 'conf-1',
        timestamp: new Date(Date.now() - 3600 * 1000).toISOString(),
        episodeId: 'ep-00a1b2c3',
        gateName:  'clippy',
        model:     'claude-opus-4-6',
        confidence: 0.91,
      },
      {
        id: 'conf-2',
        timestamp: new Date(Date.now() - 7200 * 1000).toISOString(),
        episodeId: 'ep-00d4e5f6',
        gateName:  'test',
        model:     'claude-opus-4-6',
        confidence: 0.87,
      },
      {
        id: 'conf-3',
        timestamp: new Date(Date.now() - 14400 * 1000).toISOString(),
        episodeId: 'ep-00a7b8c9',
        gateName:  'diff',
        model:     'claude-opus-4-6',
        confidence: 0.79,
      },
    ],
    falsifiers: [
      {
        id: 'fals-1',
        condition:
          'If context compression overhead latency exceeds 400ms, this heuristic should be demoted.',
        lastCheckAt:  new Date(Date.now() - 86_400_000).toISOString(),
        nextCheckAt:  new Date(Date.now() + 86_400_000 * 2).toISOString(),
      },
    ],
    provenance: [
      {
        id:        'prov-1',
        action:    'created',
        timestamp: new Date(Date.now() - 86_400_000 * 3).toISOString(),
        actor:     'system',
        notes:     'Created after 5 consecutive confirmations at gate rung 2.',
      },
      {
        id:        'prov-2',
        action:    'promoted',
        timestamp: new Date(Date.now() - 86_400_000).toISOString(),
        actor:     'system',
        notes:     'Transient → Working after confidence crossed 0.80 threshold.',
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// React Query hooks
// ---------------------------------------------------------------------------

function useKnowledgeEntry(id: string) {
  return useQuery<KnowledgeDetail>({
    queryKey: ['knowledge', 'entry', id],
    queryFn:  () => api.get<KnowledgeDetail>(`/api/knowledge/${id}`),
    staleTime: 60_000,
    enabled:   Boolean(id),
    // Use mock data when the API is unavailable during development
    placeholderData: makeMockDetail(id),
  });
}

function useKnowledgeNeighbors(id: string, enabled: boolean) {
  return useQuery<{ neighbors: NeighborEntry[] }>({
    queryKey: ['knowledge', 'neighbors', id],
    queryFn:  () => api.get<{ neighbors: NeighborEntry[] }>(`/api/knowledge/${id}/neighbors`),
    staleTime: 120_000,
    enabled:   Boolean(id) && enabled,
  });
}

function usePromoteEntry() {
  const qc = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (id: string) => api.post(`/api/knowledge/${id}/promote`),
    onSuccess: (_data, id) => {
      void qc.invalidateQueries({ queryKey: ['knowledge', 'entry', id] });
    },
  });
}

function useDeleteEntry() {
  const qc = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (id: string) => api.delete(`/api/knowledge/${id}`),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['knowledge'] });
    },
  });
}

// ---------------------------------------------------------------------------
// Section panel wrapper
// ---------------------------------------------------------------------------

function Panel({
  title,
  children,
  className,
  action,
}: {
  title: string;
  children: React.ReactNode;
  className?: string;
  action?: React.ReactNode;
}) {
  return (
    <div
      className={clsx(
        'border border-[var(--text-ghost)]',
        'bg-[var(--bg-raised)]',
        className,
      )}
    >
      <div
        className={clsx(
          'flex items-center justify-between gap-2',
          'px-4 py-2',
          'border-b border-b-[var(--text-ghost)]',
          'bg-[var(--bg-secondary)]',
        )}
      >
        <span className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)]">
          {title}
        </span>
        {action}
      </div>
      <div className="p-4">
        {children}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Metadata card
// ---------------------------------------------------------------------------

function MetadataCard({ entry }: { entry: KnowledgeDetail }) {
  const tierColor = TIER_COLOR[entry.tier];
  const kindColor = KIND_COLOR[entry.kind];

  return (
    <Panel title="Metadata">
      <div className="grid grid-cols-2 gap-x-8 gap-y-3 sm:grid-cols-3">
        {/* ID */}
        <div>
          <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-1">
            ID
          </div>
          <div className="font-mono text-xs text-[var(--text-faint)] truncate" title={entry.id}>
            {entry.id.slice(0, 16)}…
          </div>
        </div>

        {/* Kind */}
        <div>
          <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-1">
            Kind
          </div>
          <span
            className={clsx(
              'inline-flex items-center',
              'font-mono font-medium text-[10px] leading-none',
              'px-1.5 py-0.5 border',
            )}
            style={{ color: kindColor, borderColor: kindColor }}
          >
            {(entry.kind ?? 'unknown').toUpperCase()}
          </span>
        </div>

        {/* Tier */}
        <div>
          <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-1">
            Tier
          </div>
          <span
            className={clsx(
              'inline-flex items-center',
              'font-mono font-medium text-[10px] leading-none uppercase tracking-widest',
              'px-1.5 py-0.5 border',
            )}
            style={{ color: tierColor, borderColor: tierColor }}
          >
            {entry.tier}
          </span>
        </div>

        {/* Created */}
        <div>
          <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-1">
            Created
          </div>
          <div className="font-mono text-xs text-[var(--text-faint)]">
            {formatTs(entry.createdAt)}
          </div>
        </div>

        {/* Source */}
        <div>
          <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-1">
            Source
          </div>
          <div className="font-mono text-xs text-[var(--text-faint)]">
            {entry.source}
          </div>
        </div>

        {/* Confidence */}
        <div>
          <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-1">
            Confidence
          </div>
          <div className="flex items-center gap-2">
            <div
              className="h-1 bg-[var(--bg-highlight)]"
              style={{ width: 64 }}
            >
              <div
                style={{
                  width: `${(entry.confidence ?? 0) * 100}%`,
                  height: '100%',
                  backgroundColor: tierColor,
                  transition: 'width 300ms ease-out',
                }}
              />
            </div>
            <span className="font-mono text-xs tabular-nums text-[var(--text-faint)]">
              {((entry.confidence ?? 0) * 100).toFixed(0)}%
            </span>
          </div>
        </div>

        {/* Domain */}
        <div>
          <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-1">
            Domain
          </div>
          <div className="font-mono text-xs text-[var(--text-faint)]">
            {entry.domain}
          </div>
        </div>

        {/* Confirmations */}
        <div>
          <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-1">
            Confirmations
          </div>
          <div className="font-mono text-xs tabular-nums text-[var(--text-faint)]">
            {entry.confirmations}
          </div>
        </div>

        {/* Tags */}
        {(entry.tags?.length ?? 0) > 0 && (
          <div className="col-span-2 sm:col-span-3">
            <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-1">
              Tags
            </div>
            <div className="flex flex-wrap gap-1">
              {(entry.tags ?? []).map((tag) => (
                <span
                  key={tag}
                  className={clsx(
                    'font-mono text-[10px] leading-none',
                    'px-1.5 py-0.5',
                    'bg-[var(--bg-highlight)] border border-[var(--text-ghost)]',
                    'text-[var(--text-faint)]',
                  )}
                >
                  {tag}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>
    </Panel>
  );
}

// ---------------------------------------------------------------------------
// Content block
// ---------------------------------------------------------------------------

function ContentBlock({ content }: { content: string }) {
  const isCode = looksLikeCode(content);

  return (
    <Panel title="Content">
      <div
        className={clsx(
          'font-mono text-xs leading-relaxed',
          'text-[var(--text-muted)]',
          isCode && 'bg-[var(--bg-secondary)] px-4 py-3 border border-[var(--text-ghost)]',
        )}
        style={isCode ? { whiteSpace: 'pre-wrap', wordBreak: 'break-word' } : undefined}
      >
        {content}
      </div>
    </Panel>
  );
}

// ---------------------------------------------------------------------------
// Confirmation evidence list
// ---------------------------------------------------------------------------

function ConfirmationTable({ records }: { records: ConfirmationRecord[] }) {
  const router = useRouter();

  if (records.length === 0) {
    return (
      <Panel title="Confirmation Evidence">
        <EmptyState message="No confirmation records." />
      </Panel>
    );
  }

  return (
    <Panel title={`Confirmation Evidence (${records.length})`}>
      <div className="overflow-x-auto">
        <table className="w-full min-w-[560px] border-collapse">
          <thead>
            <tr className="border-b border-b-[var(--text-ghost)]">
              {(['Timestamp', 'Episode', 'Gate', 'Model', 'Confidence'] as const).map((col) => (
                <th
                  key={col}
                  className={clsx(
                    'pb-2 text-left',
                    'font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)]',
                    col === 'Confidence' && 'text-right',
                  )}
                >
                  {col}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {records.map((rec) => (
              <tr
                key={rec.id}
                className={clsx(
                  'border-b border-b-[var(--text-ghost)]',
                  'hover:bg-[var(--bg-highlight)]',
                  'transition-[background-color] duration-[80ms]',
                  'cursor-pointer',
                )}
                onClick={() => router.push(`/observe?episode=${rec.episodeId}`)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    router.push(`/observe?episode=${rec.episodeId}`);
                  }
                }}
              >
                <td className="py-2 pr-4 font-mono text-[10px] text-[var(--text-faint)] whitespace-nowrap">
                  {formatRelativeTime(rec.timestamp)}
                </td>
                <td className="py-2 pr-4">
                  <span
                    className={clsx(
                      'font-mono text-[10px]',
                      'text-[var(--accent-cyan)]',
                      'hover:underline',
                    )}
                  >
                    {rec.episodeId.slice(0, 12)}
                  </span>
                </td>
                <td className="py-2 pr-4 font-mono text-[10px] text-[var(--text-faint)]">
                  {rec.gateName}
                </td>
                <td className="py-2 pr-4 font-mono text-[10px] text-[var(--text-ghost)]">
                  {rec.model.replace('claude-', 'cl-').replace('gemini-', 'gem-')}
                </td>
                <td className="py-2 text-right">
                  <span
                    className="font-mono text-[10px] tabular-nums"
                    style={{
                      color: rec.confidence >= 0.85
                        ? 'var(--sage)'
                        : rec.confidence >= 0.65
                          ? 'var(--warning)'
                          : 'var(--accent-error)',
                    }}
                  >
                    {((rec.confidence ?? 0) * 100).toFixed(0)}%
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Panel>
  );
}

// ---------------------------------------------------------------------------
// Falsifier panel
// ---------------------------------------------------------------------------

function FalsifierPanel({ falsifiers }: { falsifiers: Falsifier[] }) {
  if (falsifiers.length === 0) {
    return (
      <Panel title="Falsifiers">
        <EmptyState message="No active falsifiers." />
      </Panel>
    );
  }

  return (
    <Panel title={`Falsifiers (${falsifiers.length})`}>
      <div className="flex flex-col gap-3">
        {falsifiers.map((f) => (
          <div
            key={f.id}
            className={clsx(
              'px-3 py-2',
              'border border-[var(--text-ghost)]',
              'bg-[var(--bg-secondary)]',
            )}
          >
            <p className="font-mono text-xs text-[var(--text-muted)] leading-relaxed mb-2">
              {f.condition}
            </p>
            <div className="flex items-center gap-6 flex-wrap">
              <div>
                <span className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mr-2">
                  Last Check
                </span>
                <span className="font-mono text-[10px] text-[var(--text-faint)]">
                  {formatRelativeTime(f.lastCheckAt)}
                </span>
              </div>
              <div>
                <span className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mr-2">
                  Next Check
                </span>
                <span className="font-mono text-[10px] text-[var(--text-faint)]">
                  {f.nextCheckAt ? formatTs(f.nextCheckAt) : '—'}
                </span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </Panel>
  );
}

// ---------------------------------------------------------------------------
// Provenance panel
// ---------------------------------------------------------------------------

function ProvenancePanel({ records }: { records: ProvenanceRecord[] }) {
  if (records.length === 0) {
    return (
      <Panel title="Provenance">
        <EmptyState message="No provenance records." />
      </Panel>
    );
  }

  return (
    <Panel title="Provenance">
      <div className="relative">
        {/* Vertical guide line */}
        <div
          className="absolute top-0 bottom-0 left-[6px] w-px bg-[var(--text-ghost)]"
          aria-hidden
        />

        <div className="flex flex-col gap-0">
          {records.map((rec, idx) => {
            const actionColor = PROVENANCE_ACTION_COLOR[rec.action];
            return (
              <div key={rec.id} className="flex gap-4 pl-7 relative">
                {/* Dot */}
                <div
                  className="absolute left-0 top-[5px] w-3 h-3 border"
                  style={{
                    borderColor: actionColor,
                    backgroundColor: idx === 0 ? actionColor : 'var(--bg-raised)',
                  }}
                  aria-hidden
                />

                <div
                  className={clsx(
                    'flex-1 pb-4',
                    idx < records.length - 1 && 'border-b border-b-[var(--bg-highlight)]',
                  )}
                >
                  {/* Action + timestamp */}
                  <div className="flex items-center gap-3 mb-1">
                    <span
                      className="font-mono text-xs font-medium uppercase tracking-wide"
                      style={{ color: actionColor }}
                    >
                      {rec.action}
                    </span>
                    <span className="font-mono text-[10px] text-[var(--text-ghost)]">
                      {formatTs(rec.timestamp)}
                    </span>
                    <span
                      className={clsx(
                        'font-mono text-[10px] px-1 border',
                        'text-[var(--text-faint)] border-[var(--text-ghost)]',
                      )}
                    >
                      {rec.actor}
                    </span>
                  </div>

                  {/* Notes */}
                  {rec.notes && (
                    <p className="font-mono text-xs text-[var(--text-faint)] leading-relaxed">
                      {rec.notes}
                    </p>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </Panel>
  );
}

// ---------------------------------------------------------------------------
// HDC fingerprint panel
// ---------------------------------------------------------------------------

function HDCPanel({
  fingerprint,
  entryId,
}: {
  fingerprint: string | null;
  entryId: string;
}) {
  const [showNeighbors, setShowNeighbors] = useState(false);
  const [copied, setCopied] = useState(false);

  const { data: neighborsData, isLoading: neighborsLoading } =
    useKnowledgeNeighbors(entryId, showNeighbors);

  const handleCopy = useCallback(() => {
    if (!fingerprint) return;
    void navigator.clipboard.writeText(fingerprint).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  }, [fingerprint]);

  return (
    <Panel
      title="HDC Fingerprint"
      action={
        fingerprint ? (
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => setShowNeighbors((v) => !v)}
              className={clsx(
                'font-mono text-[10px] px-2 py-0.5 border',
                'transition-[color,border-color] duration-[80ms]',
                showNeighbors
                  ? 'text-[var(--rose)] border-[var(--rose)]'
                  : 'text-[var(--text-ghost)] border-[var(--text-ghost)] hover:text-[var(--text-muted)] hover:border-[var(--text-muted)]',
              )}
            >
              {showNeighbors ? 'Hide neighbors' : 'View neighbors'}
            </button>
          </div>
        ) : undefined
      }
    >
      {fingerprint ? (
        <div className="flex flex-col gap-4">
          {/* Fingerprint display */}
          <div className="flex items-center gap-3">
            <code
              className={clsx(
                'font-mono text-sm tracking-widest',
                'text-[var(--rose-dim)]',
                'px-3 py-1.5',
                'bg-[var(--bg-secondary)] border border-[var(--text-ghost)]',
              )}
            >
              {fingerprint.slice(0, 8)}
              <span className="text-[var(--text-ghost)]">…</span>
            </code>
            <button
              type="button"
              onClick={handleCopy}
              className={clsx(
                'font-mono text-[10px] px-2 py-1 border',
                'transition-[color,border-color] duration-[80ms]',
                copied
                  ? 'text-[var(--sage)] border-[var(--sage)]'
                  : 'text-[var(--text-ghost)] border-[var(--text-ghost)] hover:text-[var(--text-muted)]',
              )}
            >
              {copied ? 'Copied' : 'Copy'}
            </button>
          </div>

          {/* Neighbors */}
          {showNeighbors && (
            <div className="border-t border-t-[var(--text-ghost)] pt-3">
              <div className="font-mono text-[10px] uppercase tracking-widest text-[var(--text-ghost)] mb-2">
                Semantically Similar Entries
              </div>
              {neighborsLoading ? (
                <div className="flex items-center gap-2 py-2">
                  <Spinner size="sm" />
                  <span className="font-mono text-xs text-[var(--text-ghost)]">
                    Querying neighbors…
                  </span>
                </div>
              ) : !neighborsData?.neighbors.length ? (
                <EmptyState message="No similar entries found." />
              ) : (
                <div className="flex flex-col gap-1">
                  {neighborsData.neighbors.map((n) => (
                    <div
                      key={n.id}
                      className={clsx(
                        'flex items-start gap-3 px-2 py-1.5',
                        'border border-[var(--text-ghost)]',
                        'hover:bg-[var(--bg-highlight)]',
                        'transition-[background-color] duration-[80ms]',
                      )}
                    >
                      <span
                        className="font-mono text-[10px] tabular-nums shrink-0 pt-0.5"
                        style={{
                          color: n.similarity >= 0.8
                            ? 'var(--sage)'
                            : n.similarity >= 0.6
                              ? 'var(--warning)'
                              : 'var(--text-ghost)',
                        }}
                      >
                        {((n.similarity ?? 0) * 100).toFixed(0)}%
                      </span>
                      <p className="flex-1 font-mono text-[10px] text-[var(--text-faint)] leading-snug line-clamp-2">
                        {n.content}
                      </p>
                      <span
                        className="shrink-0 font-mono text-[10px] px-1 border border-[var(--text-ghost)] text-[var(--text-ghost)]"
                        style={{ color: TIER_COLOR[n.tier], borderColor: TIER_COLOR[n.tier] }}
                      >
                        {(n.tier ?? '').slice(0, 4).toUpperCase()}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      ) : (
        <EmptyState message="No HDC fingerprint computed for this entry." />
      )}
    </Panel>
  );
}

// ---------------------------------------------------------------------------
// Empty state
// ---------------------------------------------------------------------------

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center py-6">
      <span className="font-mono text-xs text-[var(--text-ghost)]">{message}</span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Delete confirmation modal
// ---------------------------------------------------------------------------

function DeleteModal({
  onConfirm,
  onCancel,
  loading,
}: {
  onConfirm: () => void;
  onCancel: () => void;
  loading: boolean;
}) {
  return (
    <div
      className="fixed inset-0 flex items-center justify-center z-[var(--z-modal)]"
      style={{ backgroundColor: 'rgba(0,0,0,0.7)' }}
      role="dialog"
      aria-modal
      aria-label="Confirm deletion"
    >
      <div
        className={clsx(
          'w-full max-w-sm mx-4',
          'bg-[var(--bg-raised)] border border-[var(--accent-error)]',
          'p-6',
        )}
      >
        <h2 className="font-mono text-sm text-[var(--accent-error)] uppercase tracking-widest mb-3">
          Delete Entry
        </h2>
        <p className="font-mono text-xs text-[var(--text-muted)] leading-relaxed mb-6">
          This will permanently delete the knowledge entry and all associated
          confirmations, falsifiers, and provenance records. This action cannot
          be undone.
        </p>
        <div className="flex items-center gap-3 justify-end">
          <Button variant="ghost" size="sm" onClick={onCancel} disabled={loading}>
            Cancel
          </Button>
          <Button
            variant="danger"
            size="sm"
            loading={loading}
            onClick={onConfirm}
          >
            Delete
          </Button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Export helpers
// ---------------------------------------------------------------------------

function downloadJson(entry: KnowledgeDetail) {
  const blob = new Blob([JSON.stringify(entry, null, 2)], { type: 'application/json' });
  const url  = URL.createObjectURL(blob);
  const a    = document.createElement('a');
  a.href     = url;
  a.download = `knowledge-${entry.id.slice(0, 8)}.json`;
  a.click();
  URL.revokeObjectURL(url);
}

function downloadMarkdown(entry: KnowledgeDetail) {
  const lines = [
    `# ${entry.kind}: ${entry.id.slice(0, 8)}`,
    '',
    `**Tier:** ${entry.tier}  `,
    `**Domain:** ${entry.domain}  `,
    `**Confidence:** ${((entry.confidence ?? 0) * 100).toFixed(0)}%  `,
    `**Created:** ${formatTs(entry.createdAt)}  `,
    `**Source:** ${entry.source}`,
    '',
    '## Content',
    '',
    entry.content,
    '',
  ];
  if ((entry.tags ?? []).length) {
    lines.push('## Tags', '', (entry.tags ?? []).map((t) => `- ${t}`).join('\n'), '');
  }
  const blob = new Blob([lines.join('\n')], { type: 'text/markdown' });
  const url  = URL.createObjectURL(blob);
  const a    = document.createElement('a');
  a.href     = url;
  a.download = `knowledge-${entry.id.slice(0, 8)}.md`;
  a.click();
  URL.revokeObjectURL(url);
}

// ---------------------------------------------------------------------------
// Action bar
// ---------------------------------------------------------------------------

function ActionBar({
  entry,
  onDeleteRequest,
}: {
  entry: KnowledgeDetail;
  onDeleteRequest: () => void;
}) {
  const promoteMut  = usePromoteEntry();
  const canPromote  = entry.tier === 'transient' || entry.tier === 'working';

  return (
    <div
      className={clsx(
        'flex items-center gap-2 flex-wrap',
        'px-5 py-3',
        'border-b border-b-[var(--text-ghost)]',
        'bg-[var(--bg-secondary)]',
        'shrink-0',
      )}
    >
      {canPromote && (
        <Button
          size="sm"
          variant="primary"
          loading={promoteMut.isPending}
          onClick={() => promoteMut.mutate(entry.id)}
        >
          Promote
        </Button>
      )}

      {/* Export group */}
      <Button
        size="sm"
        variant="ghost"
        onClick={() => downloadJson(entry)}
      >
        Export JSON
      </Button>
      <Button
        size="sm"
        variant="ghost"
        onClick={() => downloadMarkdown(entry)}
      >
        Export MD
      </Button>

      <div className="flex-1" />

      <Button
        size="sm"
        variant="danger"
        onClick={onDeleteRequest}
      >
        Delete
      </Button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function KnowledgeDetailPage() {
  const params  = useParams<{ id: string }>();
  const router  = useRouter();
  const id      = params?.id ?? '';

  const [showDeleteModal, setShowDeleteModal] = useState(false);

  const { data: entry, isLoading, isError } = useKnowledgeEntry(id);
  const deleteMut = useDeleteEntry();

  const handleDeleteConfirm = useCallback(() => {
    deleteMut.mutate(id, {
      onSuccess: () => {
        setShowDeleteModal(false);
        router.push('/intelligence/knowledge');
      },
    });
  }, [deleteMut, id, router]);

  // ---- Loading state ----
  if (isLoading) {
    return (
      <div className="flex flex-col h-full min-h-0 items-center justify-center gap-3">
        <Spinner size="md" />
        <span className="font-mono text-xs text-[var(--text-ghost)]">
          Loading entry…
        </span>
      </div>
    );
  }

  // ---- Error state ----
  if (isError || !entry) {
    return (
      <div className="flex flex-col h-full min-h-0 items-center justify-center gap-3">
        <span className="font-mono text-sm text-[var(--accent-error)]">
          Entry not found.
        </span>
        <Button size="sm" variant="secondary" onClick={() => router.push('/intelligence/knowledge')}>
          ← Knowledge
        </Button>
      </div>
    );
  }

  return (
    <>
      {showDeleteModal && (
        <DeleteModal
          onConfirm={handleDeleteConfirm}
          onCancel={() => setShowDeleteModal(false)}
          loading={deleteMut.isPending}
        />
      )}

      <div className="flex flex-col h-full min-h-0">

        {/* ------------------------------------------------------------------ */}
        {/* Page header                                                          */}
        {/* ------------------------------------------------------------------ */}
        <div
          className={clsx(
            'shrink-0 px-5 py-3',
            'border-b border-b-[var(--text-ghost)]',
            'bg-[var(--bg-raised)]',
          )}
        >
          {/* Back link */}
          <button
            type="button"
            onClick={() => router.push('/intelligence/knowledge')}
            className={clsx(
              'flex items-center gap-1.5 mb-3',
              'font-mono text-xs text-[var(--text-ghost)]',
              'hover:text-[var(--text-muted)]',
              'transition-[color] duration-[80ms]',
            )}
          >
            <span aria-hidden>←</span>
            <span>Knowledge</span>
          </button>

          {/* Title row */}
          <div className="flex items-start gap-3 flex-wrap">
            <h1 className="flex-1 font-mono text-base text-[var(--text-strong)] min-w-0 break-all">
              {entry.id}
            </h1>
            <div className="flex items-center gap-2 shrink-0">
              <span
                className={clsx(
                  'font-mono font-medium text-[10px] uppercase tracking-widest leading-none',
                  'px-1.5 py-0.5 border',
                )}
                style={{
                  color: KIND_COLOR[entry.kind],
                  borderColor: KIND_COLOR[entry.kind],
                }}
              >
                {entry.kind}
              </span>
              <span
                className={clsx(
                  'font-mono font-medium text-[10px] uppercase tracking-widest leading-none',
                  'px-1.5 py-0.5 border',
                )}
                style={{
                  color: TIER_COLOR[entry.tier],
                  borderColor: TIER_COLOR[entry.tier],
                }}
              >
                {entry.tier}
              </span>
            </div>
          </div>
        </div>

        {/* ------------------------------------------------------------------ */}
        {/* Action bar                                                           */}
        {/* ------------------------------------------------------------------ */}
        <ActionBar
          entry={entry}
          onDeleteRequest={() => setShowDeleteModal(true)}
        />

        {/* ------------------------------------------------------------------ */}
        {/* Body — scrollable                                                    */}
        {/* ------------------------------------------------------------------ */}
        <div className="flex-1 overflow-y-auto overflow-x-hidden min-h-0 p-5">
          <div className="max-w-4xl flex flex-col gap-5">
            <MetadataCard   entry={entry} />
            <ContentBlock   content={entry.content} />
            <ConfirmationTable records={entry.confirmationRecords} />
            <FalsifierPanel falsifiers={entry.falsifiers} />
            <ProvenancePanel records={entry.provenance} />
            <HDCPanel       fingerprint={entry.hdcFingerprint} entryId={entry.id} />
          </div>
        </div>

      </div>
    </>
  );
}
