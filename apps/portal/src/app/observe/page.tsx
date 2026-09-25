'use client';

import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { clsx } from 'clsx';
import { Pill, Spinner } from '@/components/atoms';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type LogLevel = 'debug' | 'info' | 'warn' | 'error';
type LogSource = 'agent' | 'gate' | 'system';

interface LogEntry {
  id: string;
  ts: string;
  level: LogLevel;
  source: LogSource;
  message: string;
}

// ---------------------------------------------------------------------------
// Raw episode shape from /api/episodes
// ---------------------------------------------------------------------------

interface RawEpisode {
  id: string;
  episode_id?: string;
  agent_id?: string;
  plan_id?: string;
  task_id?: string;
  model?: string;
  provider?: string;
  status?: string;
  success?: boolean;
  source?: string;
  failure_reason?: string | null;
  started_at?: string;
  completed_at?: string;
  timestamp_ms?: number;
  tokens_used?: number;
  turns?: number;
  usage?: {
    cost_usd?: number;
    input_tokens?: number;
    output_tokens?: number;
    wall_ms?: number;
  };
  gate_verdicts?: Array<{ rung?: number; passed?: boolean; label?: string }>;
  kind?: string;
  role?: string;
}

// ---------------------------------------------------------------------------
// Map a raw episode to a log entry
// ---------------------------------------------------------------------------

let _seq = 0;

function episodeToLogEntry(ep: RawEpisode): LogEntry {
  _seq += 1;

  const ts = ep.started_at ?? ep.completed_at ?? (ep.timestamp_ms ? new Date(ep.timestamp_ms).toISOString() : new Date().toISOString());

  // Determine level from episode status / failure
  let level: LogLevel = 'info';
  const status = (ep.status ?? '').toLowerCase();
  if (ep.failure_reason || status === 'failed' || status === 'error') {
    level = 'error';
  } else if (status === 'warn' || status === 'warning') {
    level = 'warn';
  } else if (status === 'passed' || status === 'completed' || ep.success === true) {
    level = 'info';
  }

  // Determine source
  let source: LogSource = 'agent';
  const src = (ep.source ?? '').toLowerCase();
  const kind = (ep.kind ?? '').toLowerCase();
  if (src.includes('gate') || kind.includes('gate')) {
    source = 'gate';
  } else if (src.includes('system') || kind.includes('system') || kind.includes('trigger')) {
    source = 'system';
  }

  // Build a human-readable message
  const planPart = ep.plan_id ? `plan=${ep.plan_id}` : null;
  const taskPart = ep.task_id ? `task=${ep.task_id}` : null;
  const modelPart = ep.model ? `model=${ep.model}` : null;
  const costPart = ep.usage?.cost_usd != null && (ep.usage.cost_usd ?? 0) > 0
    ? `cost=$${(ep.usage.cost_usd ?? 0).toFixed(4)}`
    : null;
  const tokenPart = ep.usage?.input_tokens != null
    ? `tokens=${ep.usage.input_tokens}in/${ep.usage.output_tokens ?? 0}out`
    : ep.tokens_used != null
    ? `tokens=${ep.tokens_used}`
    : null;

  let messageParts: string[];
  if (level === 'error' && ep.failure_reason) {
    messageParts = [
      'Episode failed:',
      ep.failure_reason,
      planPart,
      taskPart,
    ].filter(Boolean) as string[];
  } else {
    messageParts = [
      status ? `Episode ${status}` : 'Episode',
      planPart,
      taskPart,
      modelPart,
      tokenPart,
      costPart,
    ].filter(Boolean) as string[];
  }

  return {
    id: `ep-${ep.id ?? ep.episode_id ?? _seq}`,
    ts,
    level,
    source,
    message: messageParts.join(' — '),
  };
}

// ---------------------------------------------------------------------------
// Style maps
// ---------------------------------------------------------------------------

const LEVEL_TEXT_COLOR: Record<LogLevel, string> = {
  debug: 'var(--text-ghost)',
  info:  'var(--text-muted)',
  warn:  'var(--warning)',
  error: 'var(--accent-error)',
};

const LEVEL_BADGE: Record<LogLevel, string> = {
  debug: 'text-[var(--text-ghost)]   border-[var(--text-ghost)]',
  info:  'text-[var(--text-faint)]   border-[var(--text-faint)]',
  warn:  'text-[var(--warning)]      border-[var(--warning)]',
  error: 'text-[var(--accent-error)] border-[var(--accent-error)]',
};

const SOURCE_BADGE: Record<LogSource, string> = {
  agent:  'text-[var(--accent-cyan)] border-[var(--accent-cyan)]',
  gate:   'text-[var(--ember)]       border-[var(--ember)]',
  system: 'text-[var(--dream)]       border-[var(--dream)]',
};

const LEVEL_LABEL: Record<LogLevel, string> = {
  debug: 'DEBG',
  info:  'INFO',
  warn:  'WARN',
  error: 'ERR ',
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const MAX_VISIBLE = 500;

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
// LogRow
// ---------------------------------------------------------------------------

function LogRow({ entry }: { entry: LogEntry }) {
  return (
    <div
      className={clsx(
        'flex items-start gap-2 px-4 py-[3px] min-w-0',
        'hover:bg-[var(--bg-highlight)]',
        'transition-[background-color] duration-[80ms]',
      )}
    >
      {/* Timestamp */}
      <span
        className="shrink-0 font-mono text-xs tabular-nums leading-snug"
        style={{ color: 'var(--text-faint)', width: 96, minWidth: 96 }}
      >
        {formatTs(entry.ts)}
      </span>

      {/* Level badge */}
      <span
        className={clsx(
          'shrink-0 inline-flex items-center justify-center',
          'font-mono font-medium tracking-widest',
          'text-[10px] leading-none px-1 py-0.5 border',
          'w-10',
          LEVEL_BADGE[entry.level],
        )}
      >
        {LEVEL_LABEL[entry.level]}
      </span>

      {/* Source badge */}
      <span
        className={clsx(
          'shrink-0 inline-flex items-center justify-center',
          'font-mono font-medium uppercase tracking-widest',
          'text-[10px] leading-none px-1 py-0.5 border',
          'w-[46px]',
          SOURCE_BADGE[entry.source],
        )}
      >
        {entry.source}
      </span>

      {/* Message */}
      <span
        className="flex-1 font-mono text-xs leading-snug break-all min-w-0"
        style={{ color: LEVEL_TEXT_COLOR[entry.level] }}
      >
        {entry.message}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Filter bar constants
// ---------------------------------------------------------------------------

const LEVEL_OPTS: { label: string; value: LogLevel | 'all' }[] = [
  { label: 'All',   value: 'all'   },
  { label: 'Debug', value: 'debug' },
  { label: 'Info',  value: 'info'  },
  { label: 'Warn',  value: 'warn'  },
  { label: 'Error', value: 'error' },
];

const SOURCE_OPTS: { label: string; value: LogSource | 'all' }[] = [
  { label: 'All',    value: 'all'    },
  { label: 'Agent',  value: 'agent'  },
  { label: 'Gate',   value: 'gate'   },
  { label: 'System', value: 'system' },
];

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function ObserveLogPage() {
  const [entries,      setEntries]      = useState<LogEntry[]>([]);
  const [loading,      setLoading]      = useState(true);
  const [levelFilter,  setLevelFilter]  = useState<LogLevel | 'all'>('all');
  const [sourceFilter, setSourceFilter] = useState<LogSource | 'all'>('all');
  const [search,       setSearch]       = useState('');
  const [tailing,      setTailing]      = useState(true);

  const scrollRef  = useRef<HTMLDivElement>(null);
  const bottomRef  = useRef<HTMLDivElement>(null);
  const skipScroll = useRef(false);

  // Fetch real episodes on mount
  useEffect(() => {
    let cancelled = false;
    setLoading(true);

    fetch('/api/episodes')
      .then((r) => r.json())
      .then((raw: unknown) => {
        if (cancelled) return;
        const rawEps: RawEpisode[] = Array.isArray(raw)
          ? (raw as RawEpisode[])
          : Array.isArray((raw as Record<string, unknown>)?.episodes)
          ? ((raw as Record<string, unknown>).episodes as RawEpisode[])
          : [];

        // Sort oldest-first for chronological log display
        const sorted = [...rawEps].sort((a, b) => {
          const ta = a.timestamp_ms ?? (a.started_at ? new Date(a.started_at).getTime() : 0);
          const tb = b.timestamp_ms ?? (b.started_at ? new Date(b.started_at).getTime() : 0);
          return ta - tb;
        });

        const mapped = sorted.map(episodeToLogEntry);
        const capped = mapped.length > MAX_VISIBLE
          ? mapped.slice(mapped.length - MAX_VISIBLE)
          : mapped;

        setEntries(capped);
        setLoading(false);
      })
      .catch(() => {
        if (!cancelled) setLoading(false);
      });

    return () => { cancelled = true; };
  }, []);

  // Scroll to bottom when tailing and new entries arrive
  useEffect(() => {
    if (tailing && !skipScroll.current && !loading) {
      bottomRef.current?.scrollIntoView({ block: 'end' });
    }
    skipScroll.current = false;
  }, [entries, tailing, loading]);

  // Detect manual scroll and pause tailing
  const onScroll = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 48;
    if (!atBottom) {
      skipScroll.current = true;
      setTailing(false);
    } else {
      setTailing(true);
    }
  }, []);

  // Derive visible entries from filters
  const visible = useMemo(() => {
    const q = search.toLowerCase();
    return entries.filter((e) => {
      if (levelFilter  !== 'all' && e.level  !== levelFilter)  return false;
      if (sourceFilter !== 'all' && e.source !== sourceFilter) return false;
      if (q && !e.message.toLowerCase().includes(q))           return false;
      return true;
    });
  }, [entries, levelFilter, sourceFilter, search]);

  const errorCount = useMemo(() => entries.filter((e) => e.level === 'error').length, [entries]);
  const warnCount  = useMemo(() => entries.filter((e) => e.level === 'warn').length,  [entries]);

  return (
    <div className="flex flex-col h-full min-h-0">

      {/* ---------------------------------------------------------------- */}
      {/* Filter bar                                                        */}
      {/* ---------------------------------------------------------------- */}
      <div
        className={clsx(
          'flex items-center gap-3 px-4 py-2 shrink-0 flex-wrap',
          'bg-[var(--bg-secondary)]',
          'border-b border-b-[var(--text-ghost)]',
        )}
      >
        {/* Level pills */}
        <div className="flex items-center gap-1" role="group" aria-label="Level filter">
          {LEVEL_OPTS.map((f) => (
            <Pill
              key={f.value}
              active={levelFilter === f.value}
              onClick={() => setLevelFilter(f.value)}
            >
              {f.label}
            </Pill>
          ))}
        </div>

        <span className="w-px h-4 bg-[var(--text-ghost)] shrink-0" aria-hidden />

        {/* Source pills */}
        <div className="flex items-center gap-1" role="group" aria-label="Source filter">
          {SOURCE_OPTS.map((f) => (
            <Pill
              key={f.value}
              active={sourceFilter === f.value}
              onClick={() => setSourceFilter(f.value)}
            >
              {f.label}
            </Pill>
          ))}
        </div>

        <span className="w-px h-4 bg-[var(--text-ghost)] shrink-0" aria-hidden />

        {/* Text search */}
        <input
          type="search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="filter messages…"
          aria-label="Filter log messages"
          className={clsx(
            'flex-1 min-w-[120px] max-w-xs',
            'font-mono text-xs',
            'bg-transparent text-[var(--text-strong)]',
            'border border-[var(--text-ghost)]',
            'px-2 py-1',
            'placeholder:text-[var(--text-ghost)]',
            'focus:border-[var(--rose)] focus:outline-none',
            'transition-[border-color] duration-[80ms]',
          )}
        />

        {/* Counts */}
        <div className="flex items-center gap-3 ml-auto shrink-0">
          {errorCount > 0 && (
            <span className="font-mono text-xs tabular-nums" style={{ color: 'var(--accent-error)' }}>
              {errorCount}&nbsp;err
            </span>
          )}
          {warnCount > 0 && (
            <span className="font-mono text-xs tabular-nums" style={{ color: 'var(--warning)' }}>
              {warnCount}&nbsp;warn
            </span>
          )}
          <span className="font-mono text-xs tabular-nums text-[var(--text-ghost)]">
            {visible.length}/{entries.length}
          </span>
        </div>

        {/* Tail toggle */}
        <button
          type="button"
          onClick={() => {
            setTailing(true);
            bottomRef.current?.scrollIntoView({ block: 'end' });
          }}
          title={tailing ? 'Tailing — click to pause' : 'Paused — click to resume tail'}
          className={clsx(
            'flex items-center gap-1.5 shrink-0',
            'font-mono text-xs px-2 py-1 border',
            'transition-[color,border-color] duration-[80ms]',
            tailing
              ? 'text-[var(--sage)]    border-[var(--sage)]'
              : 'text-[var(--warning)] border-[var(--warning)]',
          )}
        >
          <span
            className="w-1.5 h-1.5 shrink-0"
            style={{
              backgroundColor: tailing ? 'var(--sage)' : 'var(--warning)',
              animation: tailing ? 'rd-led-pulse 2s ease-in-out infinite' : undefined,
            }}
            aria-hidden
          />
          TAIL
        </button>
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* Log viewport                                                      */}
      {/* ---------------------------------------------------------------- */}
      <div
        ref={scrollRef}
        onScroll={onScroll}
        className="flex-1 overflow-y-auto overflow-x-hidden min-h-0 py-1"
        aria-label="Log output"
        aria-live={tailing ? 'polite' : 'off'}
        aria-atomic={false}
      >
        {loading ? (
          <div className="flex items-center justify-center gap-2 h-32">
            <Spinner size="sm" />
            <span className="font-mono text-xs text-[var(--text-ghost)]">
              Loading episodes…
            </span>
          </div>
        ) : visible.length === 0 ? (
          <div className="flex flex-col items-center justify-center gap-1.5 h-32 px-4 text-center">
            <span className="font-mono text-xs text-[var(--text-ghost)]">
              {entries.length === 0
                ? 'No episodes recorded yet.'
                : 'No log entries match the current filters.'}
            </span>
            {entries.length === 0 && (
              <span className="font-mono text-[10px] text-[var(--text-faint)]">
                Episodes appear here after agent tasks complete.
              </span>
            )}
          </div>
        ) : (
          visible.map((entry) => <LogRow key={entry.id} entry={entry} />)
        )}

        {/* Scroll anchor */}
        <div ref={bottomRef} aria-hidden />
      </div>
    </div>
  );
}
