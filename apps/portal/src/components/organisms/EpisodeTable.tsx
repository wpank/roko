'use client';

import React, { useMemo } from 'react';
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  flexRender,
  createColumnHelper,
  type SortingState,
} from '@tanstack/react-table';
import { clsx } from 'clsx';
import type { EpisodeEntry, GatePassResult } from '@/api/types';
import { Tooltip } from '@/components/atoms';

export interface EpisodeTableProps {
  episodes: EpisodeEntry[];
  onSelect?: (episode: EpisodeEntry) => void;
}

// ---------------------------------------------------------------------------
// Formatters
// ---------------------------------------------------------------------------

function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });
}

function formatCost(usd: number): string {
  if (usd < 0.001) return '<$0.001';
  if (usd < 0.01) return `$${usd.toFixed(4)}`;
  return `$${usd.toFixed(3)}`;
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

function abbreviateModel(model: string): string {
  return model
    .replace('claude-', 'cl-')
    .replace('gemini-', 'gem-')
    .replace('gpt-', 'gpt-')
    .split('-')
    .slice(0, 3)
    .join('-');
}

function truncateId(id: string, len = 12): string {
  return id.length > len ? id.slice(0, len) + '…' : id;
}

// ---------------------------------------------------------------------------
// Gate badge
// ---------------------------------------------------------------------------

interface GateBadgeProps {
  result: GatePassResult;
}

function GateBadge({ result }: GateBadgeProps) {
  if (result === 'pass') {
    return (
      <span className="font-mono text-[10px] text-[var(--sage)]">✓</span>
    );
  }
  if (result === 'fail') {
    return (
      <span className="font-mono text-[10px] text-[var(--accent-error)]">✗</span>
    );
  }
  return (
    <span className="font-mono text-[10px] text-[var(--text-ghost)]">—</span>
  );
}

// ---------------------------------------------------------------------------
// Sort indicator
// ---------------------------------------------------------------------------

function SortIndicator({ dir }: { dir: 'asc' | 'desc' | false }) {
  if (!dir) {
    return (
      <span className="text-[var(--text-ghost)] text-[8px] ml-1 opacity-40">
        ⇅
      </span>
    );
  }
  return (
    <span className="text-[var(--rose)] text-[8px] ml-1">
      {dir === 'asc' ? '↑' : '↓'}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Column definitions
// ---------------------------------------------------------------------------

const columnHelper = createColumnHelper<EpisodeEntry>();

const COLUMNS = [
  columnHelper.accessor('timestamp', {
    id: 'time',
    header: 'Time',
    cell: (info) => (
      <span className="num tabular-nums text-[var(--text-faint)]">
        {formatTimestamp(info.getValue() as string)}
      </span>
    ),
    sortingFn: 'datetime',
  }),
  columnHelper.accessor('agentId', {
    id: 'agent',
    header: 'Agent',
    cell: (info) => (
      <span className="text-[var(--text-muted)]">
        {truncateId(info.getValue() as string)}
      </span>
    ),
  }),
  columnHelper.accessor('taskId', {
    id: 'task',
    header: 'Task',
    cell: (info) => (
      <span className="text-[var(--text-muted)]">
        {truncateId(info.getValue() as string)}
      </span>
    ),
  }),
  columnHelper.accessor('model', {
    id: 'model',
    header: 'Model',
    cell: (info) => (
      <span className="text-[var(--text-ghost)] border border-[var(--text-ghost)] px-1">
        {abbreviateModel(info.getValue() as string)}
      </span>
    ),
  }),
  columnHelper.accessor('tokensIn', {
    id: 'tokensIn',
    header: 'In',
    cell: (info) => (
      <span className="num tabular-nums text-[var(--text-faint)]">
        {formatTokens(info.getValue() as number)}
      </span>
    ),
  }),
  columnHelper.accessor('tokensOut', {
    id: 'tokensOut',
    header: 'Out',
    cell: (info) => (
      <span className="num tabular-nums text-[var(--text-faint)]">
        {formatTokens(info.getValue() as number)}
      </span>
    ),
  }),
  columnHelper.accessor('costUsd', {
    id: 'cost',
    header: 'Cost',
    cell: (info) => (
      <span className="num tabular-nums text-[var(--text-muted)]">
        {formatCost(info.getValue() as number)}
      </span>
    ),
  }),
  columnHelper.accessor('gateResult', {
    id: 'gate',
    header: 'Gate',
    cell: (info) => <GateBadge result={info.getValue() as GatePassResult} />,
  }),
  columnHelper.accessor('hdcFingerprint', {
    id: 'hdc',
    header: 'HDC',
    enableSorting: false,
    cell: (info) => {
      const fp = info.getValue() as string | null;
      if (!fp) {
        return (
          <span className="text-[var(--text-ghost)]">—</span>
        );
      }
      return (
        <Tooltip content={fp} side="top">
          <span className="num tabular-nums text-[var(--text-ghost)] font-mono cursor-default">
            {fp.slice(0, 8)}
          </span>
        </Tooltip>
      );
    },
  }),
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function EpisodeTable({ episodes, onSelect }: EpisodeTableProps) {
  const [sorting, setSorting] = React.useState<SortingState>([
    { id: 'time', desc: true },
  ]);

  const table = useReactTable({
    data: episodes,
    columns: COLUMNS,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  if (episodes.length === 0) {
    return (
      <div className="flex items-center justify-center h-20 text-[var(--text-ghost)] font-mono text-xs">
        no episodes recorded
      </div>
    );
  }

  return (
    <div className="w-full overflow-x-auto">
      <table className="w-full border-collapse min-w-max">
        <thead>
          {table.getHeaderGroups().map((hg) => (
            <tr key={hg.id} className="border-b border-[var(--text-ghost)]">
              {hg.headers.map((header) => {
                const canSort = header.column.getCanSort();
                const sortDir = header.column.getIsSorted();
                return (
                  <th
                    key={header.id}
                    onClick={canSort ? header.column.getToggleSortingHandler() : undefined}
                    className={clsx(
                      'px-3 py-2 text-left',
                      'font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest',
                      'whitespace-nowrap',
                      canSort && 'cursor-pointer select-none hover:text-[var(--text-muted)]',
                    )}
                  >
                    {header.isPlaceholder ? null : (
                      <>
                        {flexRender(
                          header.column.columnDef.header,
                          header.getContext(),
                        )}
                        {canSort && (
                          <SortIndicator dir={sortDir || false} />
                        )}
                      </>
                    )}
                  </th>
                );
              })}
            </tr>
          ))}
        </thead>

        <tbody>
          {table.getRowModel().rows.map((row) => (
            <tr
              key={row.id}
              onClick={onSelect ? () => onSelect(row.original) : undefined}
              className={clsx(
                'border-b border-[var(--text-ghost)] border-opacity-40',
                'font-mono text-[11px]',
                onSelect && 'cursor-pointer hover:bg-[var(--bg-highlight)]',
                'transition-colors duration-[80ms]',
              )}
            >
              {row.getVisibleCells().map((cell) => (
                <td key={cell.id} className="px-3 py-1.5 whitespace-nowrap">
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
