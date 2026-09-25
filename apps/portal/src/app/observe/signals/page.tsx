'use client';

import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { clsx } from 'clsx';
import { Spinner } from '@/components/atoms/Spinner';
import { Pill } from '@/components/atoms';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type SignalKind = 'engram' | 'episode' | 'gate' | 'system' | 'trigger';

interface SignalEntry {
  id: string;
  kind: SignalKind;
  topic: string;
  source: string;
  timestamp: string;
  payload?: string;
}

// ---------------------------------------------------------------------------
// Raw API shapes
// ---------------------------------------------------------------------------

interface RawSignal {
  id?: string;
  kind?: string;
  topic?: string;
  source?: string;
  timestamp?: string;
  created_at?: string;
  payload?: unknown;
}

// ---------------------------------------------------------------------------
// Mapper
// ---------------------------------------------------------------------------

let _seq = 0;

function mapRawSignal(raw: RawSignal): SignalEntry {
  _seq += 1;
  const kind = ((raw.kind ?? 'engram').toLowerCase()) as SignalKind;
  return {
    id: raw.id ?? `sig-${_seq}`,
    kind,
    topic: raw.topic ?? '(unknown)',
    source: raw.source ?? '—',
    timestamp: raw.timestamp ?? raw.created_at ?? new Date().toISOString(),
    payload: raw.payload != null
      ? (typeof raw.payload === 'string' ? raw.payload : JSON.stringify(raw.payload))
      : undefined,
  };
}

// ---------------------------------------------------------------------------
// Style maps
// ---------------------------------------------------------------------------

const KIND_COLOR: Record<SignalKind, string> = {
  engram:  'var(--rose-dim)',
  episode: 'var(--dream)',
  gate:    'var(--warning)',
  system:  'var(--accent-cyan)',
  trigger: 'var(--sage)',
};

const KIND_LABEL: Record<SignalKind, string> = {
  engram:  'ENG',
  episode: 'EPS',
  gate:    'GAT',
  system:  'SYS',
  trigger: 'TRG',
};

const KIND_OPTS: { label: string; value: SignalKind | 'all' }[] = [
  { label: 'All',     value: 'all'     },
  { label: 'Engram',  value: 'engram'  },
  { label: 'Episode', value: 'episode' },
  { label: 'Gate',    value: 'gate'    },
  { label: 'System',  value: 'system'  },
  { label: 'Trigger', value: 'trigger' },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTs(iso: string): string {
  try {
    const d  = new Date(iso);
    const hh = d.getHours().toString().padStart(2, '0');
    const mm = d.getMinutes().toString().padStart(2, '0');
    const ss = d.getSeconds().toString().padStart(2, '0');
    const ms = d.getMilliseconds().toString().padStart(3, '0');
    return `${hh}:${mm}:${ss}.${ms}`;
  } catch {
    return iso;
  }
}

// ---------------------------------------------------------------------------
// SignalRow
// ---------------------------------------------------------------------------

function SignalRow({ signal }: { signal: SignalEntry }) {
  const [expanded, setExpanded] = useState(false);
  const color = KIND_COLOR[signal.kind];

  return (
    <div
      className={clsx(
        'border-b border-b-[var(--text-ghost)]',
        expanded && 'bg-[var(--bg-highlight)]',
      )}
    >
      <button
        type="button"
        className={clsx(
          'w-full flex items-start gap-2 px-4 py-[3px] text-left',
          'hover:bg-[var(--bg-highlight)]',
          'transition-[background-color] duration-[80ms]',
          'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-[var(--rose)]',
        )}
        onClick={() => signal.payload && setExpanded((x) => !x)}
        aria-expanded={signal.payload ? expanded : undefined}
      >
        {/* Timestamp */}
        <span className="shrink-0 font-mono text-[10px] tabular-nums text-[var(--text-faint)] leading-snug pt-[1px]" style={{ width: 96, minWidth: 96 }}>
          {formatTs(signal.timestamp)}
        </span>

        {/* Kind badge */}
        <span
          className="shrink-0 inline-flex items-center justify-center font-mono text-[10px] font-medium leading-none px-1 py-0.5 border w-8"
          style={{ borderColor: color, color }}
        >
          {KIND_LABEL[signal.kind]}
        </span>

        {/* Topic */}
        <span className="flex-1 font-mono text-xs text-[var(--text-strong)] truncate leading-snug">
          {signal.topic}
        </span>

        {/* Source */}
        <span className="shrink-0 font-mono text-[10px] text-[var(--text-ghost)] truncate leading-snug max-w-[120px]">
          {signal.source}
        </span>

        {/* Expand chevron */}
        {signal.payload && (
          <span
            className={clsx(
              'shrink-0 font-mono text-[10px] text-[var(--text-faint)] leading-none select-none pt-0.5',
              'transition-transform duration-[80ms]',
              expanded && 'rotate-90',
            )}
            aria-hidden
          >
            ›
          </span>
        )}
      </button>

      {expanded && signal.payload && (
        <div className="px-4 pb-3 pt-1 bg-[var(--bg-secondary)] border-t border-t-[var(--text-ghost)]">
          <pre className="font-mono text-[10px] text-[var(--text-muted)] whitespace-pre-wrap break-words leading-relaxed">
            {signal.payload}
          </pre>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function ObserveSignalsPage() {
  const [entries,    setEntries]    = useState<SignalEntry[]>([]);
  const [loading,    setLoading]    = useState(true);
  const [error,      setError]      = useState<string | null>(null);
  const [kindFilter, setKindFilter] = useState<SignalKind | 'all'>('all');
  const [search,     setSearch]     = useState('');

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    fetch('/api/signals')
      .then((r) => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return r.json();
      })
      .then((raw: unknown) => {
        if (cancelled) return;
        const arr: RawSignal[] = Array.isArray(raw)
          ? (raw as RawSignal[])
          : Array.isArray((raw as Record<string, unknown>)?.signals)
          ? ((raw as Record<string, unknown>).signals as RawSignal[])
          : [];
        const sorted = [...arr].sort((a, b) => {
          const ta = a.timestamp ?? a.created_at ?? '';
          const tb = b.timestamp ?? b.created_at ?? '';
          return ta < tb ? 1 : ta > tb ? -1 : 0;
        });
        setEntries(sorted.slice(0, 1000).map(mapRawSignal));
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const msg = err instanceof Error ? err.message : 'Failed to load signals';
        setError(msg);
        setLoading(false);
      });

    return () => { cancelled = true; };
  }, []);

  const visible = useMemo(() => {
    const q = search.toLowerCase();
    return entries.filter((e) => {
      if (kindFilter !== 'all' && e.kind !== kindFilter) return false;
      if (q && !(e.topic ?? '').toLowerCase().includes(q) && !(e.source ?? '').toLowerCase().includes(q)) return false;
      return true;
    });
  }, [entries, kindFilter, search]);

  const onClearSearch = useCallback(() => setSearch(''), []);

  return (
    <div className="flex flex-col h-full min-h-0">

      {/* Filter bar */}
      <div className="flex items-center gap-3 px-4 py-2 shrink-0 flex-wrap bg-[var(--bg-secondary)] border-b border-b-[var(--text-ghost)]">
        <div className="flex items-center gap-1" role="group" aria-label="Kind filter">
          {KIND_OPTS.map((f) => (
            <Pill key={f.value} active={kindFilter === f.value} onClick={() => setKindFilter(f.value)}>
              {f.label}
            </Pill>
          ))}
        </div>

        <span className="w-px h-4 bg-[var(--text-ghost)] shrink-0" aria-hidden />

        <input
          type="search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="filter topic / source…"
          aria-label="Filter signals"
          className={clsx(
            'flex-1 min-w-[120px] max-w-xs font-mono text-xs',
            'bg-transparent text-[var(--text-strong)]',
            'border border-[var(--text-ghost)] px-2 py-1',
            'placeholder:text-[var(--text-ghost)]',
            'focus:border-[var(--rose)] focus:outline-none',
            'transition-[border-color] duration-[80ms]',
          )}
        />

        <span className="font-mono text-xs tabular-nums text-[var(--text-ghost)] ml-auto shrink-0">
          {visible.length}/{entries.length}
        </span>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden min-h-0 py-1">
        {loading ? (
          <div className="flex items-center justify-center h-32 gap-2">
            <Spinner size="sm" />
            <span className="font-mono text-xs text-[var(--text-ghost)]">Loading signals…</span>
          </div>
        ) : error ? (
          <div className="flex flex-col items-center justify-center h-32 gap-2">
            <span className="font-mono text-xs text-[var(--accent-error)]">{error}</span>
            <span className="font-mono text-[10px] text-[var(--text-faint)]">
              Is roko serve running on :6677?
            </span>
          </div>
        ) : visible.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-32 gap-1.5">
            <span className="font-mono text-xs text-[var(--text-ghost)]">
              {entries.length === 0
                ? 'No signals recorded yet. Signals appear here after agents run.'
                : 'No signals match the current filters.'}
            </span>
            {search && (
              <button
                type="button"
                onClick={onClearSearch}
                className="font-mono text-[10px] text-[var(--rose)] hover:text-[var(--rose-bright)] transition-colors duration-[80ms]"
              >
                Clear search
              </button>
            )}
          </div>
        ) : (
          visible.map((sig) => <SignalRow key={sig.id} signal={sig} />)
        )}
      </div>
    </div>
  );
}
