'use client';

import React, { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { clsx } from 'clsx';
import { api } from '@/api/client';
import { Badge } from '@/components/atoms';
import { Spinner } from '@/components/atoms/Spinner';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ModelEntry {
  id: string;
  provider: string;
  name: string;
  contextWindow: number;
  supportsVision: boolean;
  supportsThinking: boolean;
  isDefault: boolean;
}

// ---------------------------------------------------------------------------
// Raw API shapes
// ---------------------------------------------------------------------------

interface RawModel {
  id?: string;
  name?: string;
  context_window?: number;
  supports_vision?: boolean;
  supports_thinking?: boolean;
  is_default?: boolean;
}

interface RawProvider {
  id: string;
  models?: RawModel[];
}

interface RawProvidersResponse {
  providers: RawProvider[];
}

// ---------------------------------------------------------------------------
// Mapper
// ---------------------------------------------------------------------------

function mapModels(provider: RawProvider): ModelEntry[] {
  return (provider.models ?? []).map((m) => ({
    id: m.id ?? m.name ?? crypto.randomUUID(),
    provider: provider.id,
    name: m.name ?? m.id ?? '(unknown)',
    contextWindow: m.context_window ?? 0,
    supportsVision: m.supports_vision ?? false,
    supportsThinking: m.supports_thinking ?? false,
    isDefault: m.is_default ?? false,
  }));
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function fmtCtx(n: number): string {
  if (n === 0) return '—';
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000)     return `${(n / 1_000).toFixed(0)}k`;
  return String(n);
}

// ---------------------------------------------------------------------------
// ModelRow
// ---------------------------------------------------------------------------

function ModelRow({ model }: { model: ModelEntry }) {
  return (
    <tr
      className={clsx(
        'border-b border-b-[var(--text-ghost)] last:border-b-0',
        'hover:bg-[var(--bg-highlight)] transition-[background-color] duration-[80ms]',
      )}
    >
      <td className="px-4 py-2">
        <div className="flex items-center gap-2">
          <span className="font-mono text-xs text-[var(--text-strong)]">{model.name}</span>
          {model.isDefault && (
            <Badge variant="info">default</Badge>
          )}
        </div>
      </td>
      <td className="px-4 py-2">
        <span className="font-mono text-xs text-[var(--text-muted)]">{model.provider}</span>
      </td>
      <td className="px-4 py-2">
        <span className="font-mono text-xs tabular-nums text-[var(--text-faint)]">
          {fmtCtx(model.contextWindow)}
        </span>
      </td>
      <td className="px-4 py-2">
        <span
          className="font-mono text-xs"
          style={{ color: model.supportsVision ? 'var(--sage)' : 'var(--text-ghost)' }}
        >
          {model.supportsVision ? '✓' : '—'}
        </span>
      </td>
      <td className="px-4 py-2">
        <span
          className="font-mono text-xs"
          style={{ color: model.supportsThinking ? 'var(--dream)' : 'var(--text-ghost)' }}
        >
          {model.supportsThinking ? '✓' : '—'}
        </span>
      </td>
    </tr>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function ProvidersModelsPage() {
  const [providerFilter, setProviderFilter] = useState<string>('all');

  const { data, isLoading, isError } = useQuery<RawProvidersResponse>({
    queryKey: ['providers', 'models'],
    queryFn: () => api.get<RawProvidersResponse>('/api/providers'),
    staleTime: 60_000,
  });

  const allModels = useMemo<ModelEntry[]>(() => {
    return (data?.providers ?? []).flatMap(mapModels);
  }, [data]);

  const providers = useMemo(
    () => [...new Set(allModels.map((m) => m.provider))].sort(),
    [allModels],
  );

  const visible = useMemo(
    () => providerFilter === 'all' ? allModels : allModels.filter((m) => m.provider === providerFilter),
    [allModels, providerFilter],
  );

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64 gap-2">
        <Spinner size="sm" />
        <span className="font-mono text-xs text-[var(--text-ghost)]">Loading models…</span>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-2">
        <span className="font-mono text-xs text-[var(--accent-error)]">
          Failed to load model data. Is roko serve running?
        </span>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full min-h-0">

      {/* Filter bar */}
      <div className="flex items-center gap-2 px-4 py-2 shrink-0 bg-[var(--bg-secondary)] border-b border-b-[var(--text-ghost)] flex-wrap">
        <button
          type="button"
          onClick={() => setProviderFilter('all')}
          className={clsx(
            'font-mono text-[10px] px-2 py-0.5 border leading-none',
            'transition-[color,border-color] duration-[80ms]',
            providerFilter === 'all'
              ? 'text-[var(--text-strong)] border-[var(--rose-dim)]'
              : 'text-[var(--text-ghost)] border-[var(--text-ghost)] hover:text-[var(--text-muted)]',
          )}
        >
          All
        </button>
        {providers.map((p) => (
          <button
            key={p}
            type="button"
            onClick={() => setProviderFilter(p)}
            className={clsx(
              'font-mono text-[10px] px-2 py-0.5 border leading-none',
              'transition-[color,border-color] duration-[80ms]',
              providerFilter === p
                ? 'text-[var(--text-strong)] border-[var(--rose-dim)]'
                : 'text-[var(--text-ghost)] border-[var(--text-ghost)] hover:text-[var(--text-muted)]',
            )}
          >
            {p}
          </button>
        ))}
        <span className="ml-auto font-mono text-[10px] text-[var(--text-ghost)] tabular-nums">
          {visible.length} model{visible.length !== 1 ? 's' : ''}
        </span>
      </div>

      {/* Table */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden min-h-0">
        {visible.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 gap-2 text-center px-4">
            <span className="font-mono text-xs text-[var(--text-ghost)]">
              {allModels.length === 0
                ? 'No models registered. Configure providers to see their models.'
                : 'No models match the current filter.'}
            </span>
          </div>
        ) : (
          <div className="border-b border-b-[var(--text-ghost)]">
            <table className="w-full min-w-[480px] border-collapse">
              <thead className="sticky top-0 bg-[var(--bg-raised)]">
                <tr className="border-b border-b-[var(--text-ghost)]">
                  {['MODEL', 'PROVIDER', 'CTX', 'VISION', 'THINKING'].map((col) => (
                    <th key={col} className="px-4 py-2 text-left">
                      <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
                        {col}
                      </span>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {visible.map((model) => (
                  <ModelRow key={`${model.provider}/${model.id}`} model={model} />
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
