'use client';

import React, { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { clsx } from 'clsx';
import { api } from '@/api/client';
import { Spinner } from '@/components/atoms/Spinner';
import { Sparkline } from '@/components/atoms';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ProviderCostEntry {
  id: string;
  name: string;
  costToday: number;
  costWeek: number;
  costTotal: number;
  requestCount: number;
  avgCostPerReq: number;
}

// ---------------------------------------------------------------------------
// Raw API shapes
// ---------------------------------------------------------------------------

interface RawProvider {
  id: string;
  kind: string;
  health?: {
    total_attempts?: number;
    total_cost_usd?: number;
  };
  cost_today_usd?: number;
  cost_week_usd?: number;
  cost_total_usd?: number;
}

interface RawProvidersResponse {
  providers: RawProvider[];
}

// ---------------------------------------------------------------------------
// Mapper
// ---------------------------------------------------------------------------

function mapProvider(raw: RawProvider): ProviderCostEntry {
  const reqs = raw.health?.total_attempts ?? 0;
  const total = raw.cost_total_usd ?? raw.health?.total_cost_usd ?? 0;
  return {
    id: raw.id,
    name: raw.id,
    costToday: raw.cost_today_usd ?? 0,
    costWeek: raw.cost_week_usd ?? 0,
    costTotal: total,
    requestCount: reqs,
    avgCostPerReq: reqs > 0 ? total / reqs : 0,
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function fmt(usd: number): string {
  const v = usd ?? 0;
  if (v < 0.001) return '$0.00';
  if (v < 0.01)  return `$${v.toFixed(4)}`;
  if (v < 1)     return `$${v.toFixed(3)}`;
  return `$${v.toFixed(2)}`;
}

const PLACEHOLDER_COST_HISTORY = [0.02, 0.04, 0.03, 0.06, 0.05, 0.08, 0.07, 0.10, 0.09, 0.12, 0.11, 0.14];

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function ProvidersCostPage() {
  const { data, isLoading, isError } = useQuery<RawProvidersResponse>({
    queryKey: ['providers', 'cost'],
    queryFn: () => api.get<RawProvidersResponse>('/api/providers'),
    staleTime: 30_000,
  });

  const entries = useMemo<ProviderCostEntry[]>(() => {
    const raw = data?.providers ?? [];
    return raw.map(mapProvider).sort((a, b) => b.costTotal - a.costTotal);
  }, [data]);

  const grandTotal = useMemo(
    () => entries.reduce((sum, e) => sum + e.costTotal, 0),
    [entries],
  );

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64 gap-2">
        <Spinner size="sm" />
        <span className="font-mono text-xs text-[var(--text-ghost)]">Loading cost data…</span>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-2">
        <span className="font-mono text-xs text-[var(--accent-error)]">
          Failed to load provider cost data. Is roko serve running?
        </span>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6 p-6">

      {/* Summary header */}
      <div className="flex items-end gap-8 pb-4 border-b border-b-[var(--text-ghost)]">
        <div className="flex flex-col gap-1">
          <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
            Grand Total
          </span>
          <span className="font-mono text-2xl tabular-nums num text-[var(--rose-glow)]">
            {fmt(grandTotal)}
          </span>
        </div>

        <div className="flex-1">
          <Sparkline
            data={PLACEHOLDER_COST_HISTORY}
            height={40}
            color="var(--rose)"
            fill
          />
        </div>
      </div>

      {/* Provider cost table */}
      {entries.length === 0 ? (
        <div className="flex flex-col items-center justify-center h-40 gap-2 text-center">
          <span className="font-mono text-xs text-[var(--text-ghost)]">
            No cost data available yet.
          </span>
          <span className="font-mono text-[10px] text-[var(--text-faint)]">
            Costs accumulate as agents make API requests.
          </span>
        </div>
      ) : (
        <div className="border border-[var(--text-ghost)] overflow-x-auto">
          <table className="w-full min-w-[500px] border-collapse">
            <thead>
              <tr className="border-b border-b-[var(--text-ghost)]">
                {['PROVIDER', 'TODAY', 'WEEK', 'TOTAL', 'REQS', 'AVG/REQ'].map((col) => (
                  <th key={col} className="px-4 py-2 text-left">
                    <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
                      {col}
                    </span>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {entries.map((entry, i) => (
                <tr
                  key={entry.id}
                  className={clsx(
                    'border-b border-b-[var(--text-ghost)] last:border-b-0',
                    'hover:bg-[var(--bg-highlight)] transition-[background-color] duration-[80ms]',
                    i === 0 && grandTotal > 0 && 'bg-[rgba(var(--rose-raw),0.04)]',
                  )}
                >
                  <td className="px-4 py-2">
                    <span className="font-mono text-xs text-[var(--text-strong)]">{entry.name}</span>
                  </td>
                  <td className="px-4 py-2">
                    <span className="font-mono text-xs tabular-nums text-[var(--text-muted)]">
                      {fmt(entry.costToday)}
                    </span>
                  </td>
                  <td className="px-4 py-2">
                    <span className="font-mono text-xs tabular-nums text-[var(--text-muted)]">
                      {fmt(entry.costWeek)}
                    </span>
                  </td>
                  <td className="px-4 py-2">
                    <span className="font-mono text-xs tabular-nums font-medium" style={{ color: 'var(--rose-dim)' }}>
                      {fmt(entry.costTotal)}
                    </span>
                  </td>
                  <td className="px-4 py-2">
                    <span className="font-mono text-xs tabular-nums text-[var(--text-muted)]">
                      {entry.requestCount.toLocaleString()}
                    </span>
                  </td>
                  <td className="px-4 py-2">
                    <span className="font-mono text-xs tabular-nums text-[var(--text-faint)]">
                      {fmt(entry.avgCostPerReq)}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
