'use client';

import React, {
  useEffect,
  useRef,
  useState,
  useCallback,
  useMemo,
} from 'react';
import { clsx } from 'clsx';
import { useDashboardStore } from '@/stores/dashboard';

export interface AgentOutputStreamProps {
  agentId: string;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_VISIBLE = 500;

// ---------------------------------------------------------------------------
// Tool call folding
// ---------------------------------------------------------------------------

interface FoldedLine {
  /** Original line index in the `lines` array. */
  index: number;
  raw: string;
  /** If true, this line starts a collapsible tool block. */
  isToolHeader: boolean;
  /** True while the block is collapsed. */
  collapsed: boolean;
  /** Summary extracted from the header (used when collapsed). */
  summary: string;
}

const TOOL_HEADER_RE = /Tool:\s*(\w+)\s*[—–-]\s*(.*?)(?:\s*\[(\d+)ms\])?$/i;

function buildFoldedLines(lines: string[]): FoldedLine[] {
  return lines.map((raw, index) => {
    const match = TOOL_HEADER_RE.exec(raw);
    if (match) {
      const tool = match[1] ?? '';
      const cmd = match[2] ?? '';
      const ms = match[3] ? `[${match[3]}ms]` : '';
      return {
        index,
        raw,
        isToolHeader: true,
        collapsed: true,
        summary: `${tool} — ${cmd} ${ms}`.trim(),
      };
    }
    return { index, raw, isToolHeader: false, collapsed: false, summary: '' };
  });
}

// ---------------------------------------------------------------------------
// Search highlighting
// ---------------------------------------------------------------------------

function highlightMatches(text: string, query: string): React.ReactNode {
  if (!query) return text;
  const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const parts = text.split(new RegExp(`(${escaped})`, 'gi'));
  return parts.map((part, i) =>
    i % 2 === 1 ? (
      <mark key={i} style={{ background: 'var(--warning)', color: 'var(--bg-base)' }}>
        {part}
      </mark>
    ) : (
      part
    ),
  );
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AgentOutputStream({ agentId }: AgentOutputStreamProps) {
  const lines = useDashboardStore(
    (s) => s.agentOutput[agentId] ?? [],
  );

  const containerRef = useRef<HTMLDivElement>(null);
  const [isTailing, setIsTailing] = useState(true);
  const [searchMode, setSearchMode] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const searchInputRef = useRef<HTMLInputElement>(null);

  // Folding state: map from original line index → collapsed boolean
  const [collapsedMap, setCollapsedMap] = useState<Map<number, boolean>>(
    new Map(),
  );

  // Rebuild folded lines when content changes
  const foldedLines: FoldedLine[] = useMemo(() => {
    const built = buildFoldedLines(lines.slice(-MAX_VISIBLE));
    // Apply persisted collapsed state
    return built.map((fl) => {
      if (!fl.isToolHeader) return fl;
      const persisted = collapsedMap.get(fl.index);
      return { ...fl, collapsed: persisted ?? true };
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lines]);

  // Auto-scroll to bottom when tailing
  useEffect(() => {
    if (!isTailing) return;
    const el = containerRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [lines, isTailing]);

  // Detect user scroll to stop tailing
  const handleScroll = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 8;
    setIsTailing(atBottom);
  }, []);

  // Keyboard handler: '/' to enter search, 'Escape' to exit
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === '/' && !searchMode) {
        e.preventDefault();
        setSearchMode(true);
        setTimeout(() => searchInputRef.current?.focus(), 0);
      }
      if (e.key === 'Escape' && searchMode) {
        setSearchMode(false);
        setSearchQuery('');
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [searchMode]);

  const toggleCollapse = useCallback((lineIndex: number) => {
    setCollapsedMap((prev) => {
      const next = new Map(prev);
      next.set(lineIndex, !prev.get(lineIndex));
      return next;
    });
  }, []);

  const scrollToBottom = useCallback(() => {
    const el = containerRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
      setIsTailing(true);
    }
  }, []);

  // Filter visible lines: hide lines immediately after a collapsed tool header
  const visibleLines = useMemo(() => {
    const result: FoldedLine[] = [];
    let skipUntilIndex = -1;

    for (let i = 0; i < foldedLines.length; i++) {
      const fl = foldedLines[i];
      if (i < skipUntilIndex) continue;
      if (fl.isToolHeader) {
        const currentCollapsed =
          collapsedMap.has(fl.index) ? collapsedMap.get(fl.index)! : true;
        result.push({ ...fl, collapsed: currentCollapsed });
        if (currentCollapsed) {
          // Skip subsequent lines until the next tool header or 30 lines max
          skipUntilIndex = i + 30;
        }
      } else {
        result.push(fl);
      }
    }

    // Apply search filter
    if (searchQuery) {
      return result.filter((fl) =>
        fl.raw.toLowerCase().includes(searchQuery.toLowerCase()),
      );
    }

    return result;
  }, [foldedLines, collapsedMap, searchQuery]);

  return (
    <div className="flex flex-col h-full min-h-0 bg-[var(--bg-base)] border border-[var(--text-ghost)]">
      {/* Toolbar */}
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-[var(--text-ghost)] shrink-0">
        <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
          agent output
        </span>

        <div className="flex items-center gap-3">
          {searchMode ? (
            <input
              ref={searchInputRef}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="search…"
              className={clsx(
                'w-40 px-2 py-0.5',
                'font-mono text-[11px] text-[var(--text-muted)]',
                'bg-[var(--bg-raised)] border border-[var(--text-ghost)]',
                'outline-none focus:border-[var(--rose)]',
              )}
            />
          ) : (
            <span className="font-mono text-[10px] text-[var(--text-faint)]">
              / to search
            </span>
          )}

          <button
            onClick={scrollToBottom}
            className={clsx(
              'font-mono text-[10px] tracking-widest uppercase',
              'px-2 py-0.5 border',
              isTailing
                ? 'text-[var(--warning)] border-[var(--warning)]'
                : 'text-[var(--text-ghost)] border-[var(--text-ghost)] hover:border-[var(--text-muted)]',
              'transition-colors duration-[80ms]',
            )}
          >
            {isTailing ? 'TAIL' : 'SCROLL'}
          </button>
        </div>
      </div>

      {/* Output area */}
      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto p-3 min-h-0"
      >
        {lines.length === 0 ? (
          <span className="font-mono text-[11px] text-[var(--text-ghost)]">
            waiting for output…
          </span>
        ) : (
          <pre className="m-0 p-0 font-mono text-[11px] leading-[1.6] text-[var(--text-muted)] whitespace-pre-wrap break-all">
            {visibleLines.map((fl) => {
              if (fl.isToolHeader) {
                const isCollapsed =
                  collapsedMap.has(fl.index)
                    ? collapsedMap.get(fl.index)!
                    : true;
                return (
                  <div
                    key={fl.index}
                    role="button"
                    tabIndex={0}
                    onClick={() => toggleCollapse(fl.index)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        toggleCollapse(fl.index);
                      }
                    }}
                    className="flex items-start gap-1 cursor-pointer hover:bg-[var(--bg-highlight)] py-0.5 px-1 -mx-1"
                  >
                    <span className="text-[var(--dream)] select-none shrink-0">
                      {isCollapsed ? '▶' : '▼'}
                    </span>
                    <span className="text-[var(--dream)]">
                      {isCollapsed
                        ? highlightMatches(`Tool: ${fl.summary}`, searchQuery)
                        : highlightMatches(fl.raw, searchQuery)}
                    </span>
                  </div>
                );
              }

              return (
                <div key={fl.index} className="py-0">
                  {highlightMatches(fl.raw, searchQuery)}
                  {'\n'}
                </div>
              );
            })}
          </pre>
        )}
      </div>

      {/* Footer: line count */}
      <div className="flex items-center justify-between px-3 py-1 border-t border-[var(--text-ghost)] shrink-0">
        <span className="font-mono text-[9px] text-[var(--text-faint)]">
          {lines.length} line{lines.length !== 1 ? 's' : ''}
          {lines.length >= MAX_VISIBLE && ' (capped)'}
        </span>
        {searchQuery && (
          <span className="font-mono text-[9px] text-[var(--warning)]">
            {visibleLines.length} match{visibleLines.length !== 1 ? 'es' : ''}
          </span>
        )}
      </div>
    </div>
  );
}
