'use client';

import React, { useState, useMemo, useCallback, useRef, useEffect } from 'react';
import Link from 'next/link';
import {
  useReactTable,
  getCoreRowModel,
  createColumnHelper,
  flexRender,
} from '@tanstack/react-table';
import { useQuery } from '@tanstack/react-query';
import { clsx } from 'clsx';
import { useDashboardStore } from '@/stores/dashboard';
import { api } from '@/api/client';
import type { ProviderState, CircuitState, ProviderStatus } from '@/api/types';
import { StatusLED } from '@/components/atoms/StatusLED';
import { Button } from '@/components/atoms';
import DrawerOverlay from '@/components/layout/DrawerOverlay';
import { WaveformTrace } from '@/components/molecules/WaveformTrace';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Per-provider metric history for waveform traces (cost, latency, error). */
interface ProviderMetricHistory {
  cost: number[];
  latency: number[];
  errorRate: number[];
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_HISTORY_POINTS = 60;

// Pattern status derived from aggregate health.
type PatternStatus = 'ＮＯＭＩＮＡＬ' | 'ＣＡＵＴＩＯＮ' | 'ＡＬＥＲＴ' | 'ＳＴＡＮＤＢＹ';

const PATTERN_STATUS_COLOR: Record<PatternStatus, string> = {
  'ＮＯＭＩＮＡＬ': 'var(--sage)',
  'ＣＡＵＴＩＯＮ': 'var(--warning)',
  'ＡＬＥＲＴ':    'var(--accent-error)',
  'ＳＴＡＮＤＢＹ': 'var(--text-ghost)',
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function derivePatternStatus(providers: ProviderState[]): PatternStatus {
  if (providers.length === 0) return 'ＳＴＡＮＤＢＹ';
  if (providers.some((p) => p.status === 'unhealthy')) return 'ＡＬＥＲＴ';
  if (providers.some((p) => p.status === 'degraded')) return 'ＣＡＵＴＩＯＮ';
  return 'ＮＯＭＩＮＡＬ';
}

function providerStatusToLED(
  status: ProviderStatus,
): 'success' | 'warning' | 'error' | 'offline' {
  switch (status) {
    case 'healthy':      return 'success';
    case 'degraded':     return 'warning';
    case 'unhealthy':    return 'error';
    case 'unconfigured': return 'offline';
  }
}

function circuitStateLabel(state: CircuitState): { label: string; color: string } {
  switch (state) {
    case 'closed':    return { label: 'CLOSED', color: 'var(--sage)' };
    case 'open':      return { label: 'OPEN',   color: 'var(--accent-error)' };
    case 'half_open': return { label: 'HALF',   color: 'var(--warning)' };
  }
}

function formatPassPct(rate: number): { text: string; color: string } {
  const pct = ((rate ?? 0) * 100).toFixed(1);
  if (rate >= 0.9)  return { text: `${pct}%`, color: 'var(--sage)' };
  if (rate >= 0.7)  return { text: `${pct}%`, color: 'var(--warning)' };
  return { text: `${pct}%`, color: 'var(--accent-error)' };
}

function formatLatency(ms: number): string {
  const v = ms ?? 0;
  if (v < 1000) return `${Math.round(v)}ms`;
  return `${(v / 1000).toFixed(2)}s`;
}

function formatCost(usd: number): string {
  const v = usd ?? 0;
  if (v < 0.01) return '$0.00';
  if (v < 1)    return `$${v.toFixed(3)}`;
  return `$${v.toFixed(2)}`;
}

/** Circuit breaker visual: closed = ●──○, open = ○──●. */
function circuitDiagram(state: CircuitState): string {
  switch (state) {
    case 'closed':    return '●──○';
    case 'open':      return '○──●';
    case 'half_open': return '◐──○';
  }
}

// ---------------------------------------------------------------------------
// Raw API response shapes (what roko-serve actually returns)
// ---------------------------------------------------------------------------

interface RawProviderHealth {
  state: string;
  consecutive_failures: number;
  total_attempts: number;
  total_successes: number;
}

interface RawProvider {
  id: string;
  kind: string;
  has_api_key: boolean;
  model_count: number;
  base_url?: string;
  health?: RawProviderHealth;
}

interface RawProvidersResponse {
  providers: RawProvider[];
}

// ---------------------------------------------------------------------------
// Mapper: raw API provider → ProviderState
// ---------------------------------------------------------------------------

function mapRawProvider(raw: RawProvider): ProviderState {
  const health = raw.health;
  const totalAttempts = health?.total_attempts ?? 0;
  const totalSuccesses = health?.total_successes ?? 0;
  const consecutiveFailures = health?.consecutive_failures ?? 0;

  // Derive status from health state field, or from whether an API key is present.
  let status: ProviderState['status'];
  if (!health) {
    status = raw.has_api_key ? 'healthy' : 'unconfigured';
  } else {
    switch (health.state) {
      case 'healthy':   status = 'healthy';   break;
      case 'degraded':  status = 'degraded';  break;
      case 'unhealthy': status = 'unhealthy'; break;
      default:          status = 'healthy';
    }
  }

  // Derive circuit state from consecutive failures.
  let circuitState: ProviderState['circuitState'];
  if (consecutiveFailures >= 5) {
    circuitState = 'open';
  } else if (consecutiveFailures >= 2) {
    circuitState = 'half_open';
  } else {
    circuitState = 'closed';
  }

  const passRate = totalAttempts > 0 ? totalSuccesses / totalAttempts : 1;

  return {
    id: raw.id,
    name: raw.id,
    status,
    modelCount: raw.model_count,
    requestCount: totalAttempts,
    passRate,
    // Latency and cost are not available from the REST snapshot; they come via
    // SSE events later. Default to 0 so the page renders immediately.
    avgLatencyMs: 0,
    costUsd: 0,
    circuitState,
  };
}

// ---------------------------------------------------------------------------
// useProviders — React Query hook for initial hydration
// ---------------------------------------------------------------------------

function useProviders() {
  return useQuery<RawProvidersResponse>({
    queryKey: ['providers'],
    queryFn: () => api.get<RawProvidersResponse>('/api/providers'),
    staleTime: 30_000,
  });
}

// ---------------------------------------------------------------------------
// useAggregateHistory — rolling metric ring buffer across all providers
// ---------------------------------------------------------------------------

function useAggregateHistory(providers: ProviderState[]): ProviderMetricHistory {
  const historyRef = useRef<ProviderMetricHistory>({ cost: [], latency: [], errorRate: [] });

  useEffect(() => {
    if (providers.length === 0) return;

    const totalCost    = providers.reduce((s, p) => s + p.costUsd, 0);
    const avgLatency   = providers.reduce((s, p) => s + p.avgLatencyMs, 0) / providers.length;
    const avgErrorRate = providers.reduce((s, p) => s + (1 - p.passRate), 0) / providers.length;

    const push = (arr: number[], val: number): number[] => {
      const next = [...arr, val];
      return next.length > MAX_HISTORY_POINTS
        ? next.slice(next.length - MAX_HISTORY_POINTS)
        : next;
    };

    historyRef.current = {
      cost:      push(historyRef.current.cost, totalCost),
      latency:   push(historyRef.current.latency, avgLatency),
      errorRate: push(historyRef.current.errorRate, avgErrorRate),
    };
  }, [providers]);

  return historyRef.current;
}

// ---------------------------------------------------------------------------
// useProviderHistory — per-provider ring buffer for drawer traces
// ---------------------------------------------------------------------------

function useProviderHistory(
  providerId: string | null,
  provider: ProviderState | null,
): ProviderMetricHistory {
  const historyRef = useRef<ProviderMetricHistory>({ cost: [], latency: [], errorRate: [] });

  useEffect(() => {
    if (!provider || !providerId) return;

    const push = (arr: number[], val: number): number[] => {
      const next = [...arr, val];
      return next.length > MAX_HISTORY_POINTS
        ? next.slice(next.length - MAX_HISTORY_POINTS)
        : next;
    };

    historyRef.current = {
      cost:      push(historyRef.current.cost, provider.costUsd),
      latency:   push(historyRef.current.latency, provider.avgLatencyMs),
      errorRate: push(historyRef.current.errorRate, 1 - provider.passRate),
    };
  }, [provider, providerId]);

  return historyRef.current;
}

// ---------------------------------------------------------------------------
// Column definition for the detail table
// ---------------------------------------------------------------------------

const colHelper = createColumnHelper<ProviderState>();

const COLUMNS = [
  colHelper.accessor('name', {
    header: 'PROVIDER',
    cell: (info) => (
      <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-strong)] tracking-[var(--tracking-wide)] leading-none">
        {info.getValue()}
      </span>
    ),
  }),
  colHelper.accessor('modelCount', {
    header: 'MODELS',
    cell: (info) => (
      <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] tabular-nums leading-none">
        {info.getValue()}
      </span>
    ),
  }),
  colHelper.accessor('requestCount', {
    header: 'REQS',
    cell: (info) => (
      <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] tabular-nums leading-none">
        {(info.getValue() ?? 0).toLocaleString()}
      </span>
    ),
  }),
  colHelper.accessor('passRate', {
    header: 'PASS%',
    cell: (info) => {
      const { text, color } = formatPassPct(info.getValue());
      return (
        <span
          className="font-[var(--font-mono)] text-[var(--text-xs)] tabular-nums leading-none"
          style={{ color }}
        >
          {text}
        </span>
      );
    },
  }),
  colHelper.accessor('avgLatencyMs', {
    header: 'LATENCY',
    cell: (info) => (
      <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] tabular-nums leading-none">
        {formatLatency(info.getValue())}
      </span>
    ),
  }),
  colHelper.accessor('costUsd', {
    header: 'COST',
    cell: (info) => (
      <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] tabular-nums leading-none">
        {formatCost(info.getValue())}
      </span>
    ),
  }),
  colHelper.accessor('circuitState', {
    header: 'CIRCUIT',
    cell: (info) => {
      const { label, color } = circuitStateLabel(info.getValue());
      return (
        <span
          className="font-[var(--font-mono)] text-[var(--text-xs)] tracking-[var(--tracking-wider)] leading-none"
          style={{ color }}
        >
          {label}
        </span>
      );
    },
  }),
];

// ---------------------------------------------------------------------------
// UnitCard — small provider card in the unit array
// ---------------------------------------------------------------------------

interface UnitCardProps {
  provider: ProviderState;
  selected: boolean;
  onClick: () => void;
}

function UnitCard({ provider, selected, onClick }: UnitCardProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={selected}
      className={clsx(
        'relative flex flex-col justify-between',
        'min-w-[120px] h-20 p-2',
        'bg-[var(--void)]',
        'border',
        selected
          ? 'border-[var(--rose-dim)]'
          : 'border-[var(--text-ghost)] hover:border-[var(--rose-dim)]',
        'cursor-pointer select-none text-left',
        'transition-[border-color] duration-[80ms] ease-[var(--ease-out)]',
        'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
      )}
    >
      {/* Top row: LED + name */}
      <div className="flex items-center gap-1.5 min-w-0">
        <StatusLED
          status={providerStatusToLED(provider.status)}
          pulse={provider.status === 'healthy'}
          size="sm"
        />
        <span
          className="truncate font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-strong)] leading-none tracking-[var(--tracking-wide)]"
          title={provider.name}
        >
          {provider.name}
        </span>
      </div>

      {/* Bottom row: model count + circuit state */}
      <div className="flex items-end justify-between gap-1">
        <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-faint)] leading-none tabular-nums">
          {provider.modelCount}&nbsp;{provider.modelCount === 1 ? 'model' : 'models'}
        </span>
        <span
          className="font-[var(--font-mono)] text-[10px] leading-none tracking-[var(--tracking-wider)]"
          style={{ color: circuitStateLabel(provider.circuitState).color }}
        >
          {circuitStateLabel(provider.circuitState).label}
        </span>
      </div>
    </button>
  );
}

// ---------------------------------------------------------------------------
// RateLimitBar
// ---------------------------------------------------------------------------

interface RateLimitBarProps {
  label: string;
  used: number;
  limit: number;
  unit: string;
}

function RateLimitBar({ label, used, limit, unit }: RateLimitBarProps) {
  const pct = limit > 0 ? Math.min(100, (used / limit) * 100) : 0;
  const barColor =
    pct >= 90 ? 'var(--accent-error)' : pct >= 70 ? 'var(--warning)' : 'var(--rose-dim)';

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center justify-between">
        <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-faint)] leading-none tracking-[var(--tracking-wider)]">
          {label}
        </span>
        <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-muted)] leading-none tabular-nums">
          {used.toLocaleString()} / {limit.toLocaleString()} {unit}
        </span>
      </div>
      <div className="h-[2px] w-full bg-[var(--bg-highlight)] overflow-hidden">
        <div
          className="h-full transition-[width] duration-150 ease-out"
          style={{ width: `${pct}%`, backgroundColor: barColor }}
        />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// ProviderDetailDrawer
// ---------------------------------------------------------------------------

interface ProviderDetailDrawerProps {
  provider: ProviderState | null;
  onClose: () => void;
}

function ProviderDetailDrawer({ provider, onClose }: ProviderDetailDrawerProps) {
  const history = useProviderHistory(provider?.id ?? null, provider);
  const [testLoading, setTestLoading] = useState(false);

  const handleTestConnectivity = useCallback(async () => {
    if (!provider) return;
    setTestLoading(true);
    try {
      await api.post(`/api/v1/providers/${provider.id}/test`);
    } catch {
      // Silently ignore — result surfaces through SSE.
    } finally {
      setTestLoading(false);
    }
  }, [provider]);

  if (!provider) return null;

  const circuit = circuitStateLabel(provider.circuitState);
  const diagram = circuitDiagram(provider.circuitState);

  // Heuristic rate-limit estimates from request count.
  const rpmUsed  = (provider.requestCount ?? 0) % 10_000;
  const rpmLimit = 10_000;
  const tpmUsed  = Math.round(
    (provider.requestCount ?? 0) * 1.4 * ((provider.avgLatencyMs ?? 0) / 1000) * 1_000,
  );
  const tpmLimit = 2_000_000;

  return (
    <DrawerOverlay
      open={provider !== null}
      onClose={onClose}
      title={provider.name}
      width={520}
    >
      <div
        className="flex flex-col gap-0 h-full"
        style={{ background: 'var(--void)', fontFamily: 'var(--font-mono)' }}
      >

        {/* ---- Circuit breaker visualization ---- */}
        <section className="px-4 py-3 border-b border-b-[var(--text-ghost)]">
          <span
            className="block font-[var(--font-mono)] text-[10px] tracking-[var(--tracking-widest)] leading-none mb-2"
            style={{ color: 'var(--text-faint)' }}
          >
            ＣＩＲＣＵＩＴ　ＢＲＥＡＫＥＲ
          </span>
          <div className="flex items-center gap-3">
            <span
              className="font-[var(--font-mono)] text-base leading-none select-none"
              style={{ color: circuit.color, letterSpacing: '0.25em' }}
            >
              {diagram}
            </span>
            <span
              className="font-[var(--font-mono)] text-[var(--text-xs)] tracking-[var(--tracking-wider)] leading-none"
              style={{ color: circuit.color }}
            >
              {circuit.label}
            </span>
          </div>
        </section>

        {/* ---- Waveform traces ---- */}
        <section className="px-4 py-3 border-b border-b-[var(--text-ghost)] flex flex-col gap-1.5">
          <span
            className="block font-[var(--font-mono)] text-[10px] tracking-[var(--tracking-widest)] leading-none mb-1"
            style={{ color: 'var(--text-faint)' }}
          >
            ＴＲＡＣＥ　ＣＨＡＮＮＥＬＳ
          </span>
          <WaveformTrace label="COST"    data={history.cost}      color="var(--rose)"  height={36} />
          <WaveformTrace label="LATENCY" data={history.latency}   color="var(--dream)" height={36} />
          <WaveformTrace label="ERROR"   data={history.errorRate} color="var(--ember)" height={36} />
        </section>

        {/* ---- Rate limit bars ---- */}
        <section className="px-4 py-3 border-b border-b-[var(--text-ghost)] flex flex-col gap-3">
          <span
            className="block font-[var(--font-mono)] text-[10px] tracking-[var(--tracking-widest)] leading-none"
            style={{ color: 'var(--text-faint)' }}
          >
            ＲＡＴＥ　ＬＩＭＩＴＳ
          </span>
          <RateLimitBar label="RPM" used={rpmUsed}  limit={rpmLimit} unit="req" />
          <RateLimitBar label="TPM" used={tpmUsed}  limit={tpmLimit} unit="tok" />
        </section>

        {/* ---- Metrics grid ---- */}
        <section className="px-4 py-3 border-b border-b-[var(--text-ghost)]">
          <span
            className="block font-[var(--font-mono)] text-[10px] tracking-[var(--tracking-widest)] leading-none mb-2"
            style={{ color: 'var(--text-faint)' }}
          >
            ＳＴＡＴＩＳＴＩＣＳ
          </span>
          <div className="grid grid-cols-2 gap-x-6 gap-y-2">
            {([
              { label: 'REQUESTS',  value: (provider.requestCount ?? 0).toLocaleString() },
              { label: 'MODELS',    value: String(provider.modelCount) },
              { label: 'PASS RATE', value: formatPassPct(provider.passRate).text },
              { label: 'LATENCY',   value: formatLatency(provider.avgLatencyMs) },
              { label: 'COST',      value: formatCost(provider.costUsd) },
              { label: 'STATUS',    value: (provider.status ?? 'unknown').toUpperCase() },
            ] as const).map(({ label, value }) => (
              <div key={label} className="flex flex-col gap-0.5">
                <span
                  className="font-[var(--font-mono)] text-[10px] tracking-[var(--tracking-wider)] leading-none"
                  style={{ color: 'var(--text-faint)' }}
                >
                  {label}
                </span>
                <span
                  className="font-[var(--font-mono)] text-[var(--text-xs)] tabular-nums leading-none"
                  style={{ color: 'var(--text-muted)' }}
                >
                  {value}
                </span>
              </div>
            ))}
          </div>
        </section>

        {/* ---- Actions ---- */}
        <section className="px-4 py-3 flex items-center gap-3">
          <Button
            variant="secondary"
            size="sm"
            loading={testLoading}
            onClick={handleTestConnectivity}
          >
            Test Connectivity
          </Button>
          <Link
            href={`/settings?provider=${encodeURIComponent(provider.id)}`}
            className="font-[var(--font-mono)] text-[var(--text-xs)] leading-none"
            style={{ color: 'var(--rose-dim)' }}
          >
            Configure →
          </Link>
        </section>
      </div>
    </DrawerOverlay>
  );
}

// ---------------------------------------------------------------------------
// ProvidersMonitorPage
// ---------------------------------------------------------------------------

export default function ProvidersMonitorPage() {
  // --- Store ---
  const providersMap = useDashboardStore((s) => s.providers);
  const providers    = useMemo(() => Object.values(providersMap), [providersMap]);
  const applyEvent   = useDashboardStore((s) => s.applyEvent);

  // --- Initial hydration via React Query ---
  const { data: hydrationData } = useProviders();

  const hydratedRef = useRef(false);
  useEffect(() => {
    if (!hydrationData || hydratedRef.current) return;
    hydratedRef.current = true;
    const ts = new Date().toISOString();
    for (const raw of (hydrationData.providers ?? [])) {
      applyEvent({
        type: 'provider_health_updated',
        provider: mapRawProvider(raw),
        timestamp: ts,
      });
    }
  }, [hydrationData, applyEvent]);

  // --- Aggregate history for trace section ---
  const aggregateHistory = useAggregateHistory(providers);

  // --- Selection state ---
  const [selectedId, setSelectedId] = useState<string | null>(null);

  // --- Drawer state ---
  const [drawerProvider, setDrawerProvider] = useState<ProviderState | null>(null);

  const handleUnitCardClick = useCallback((provider: ProviderState) => {
    setSelectedId((prev) => (prev === provider.id ? null : provider.id));
  }, []);

  const handleRowClick = useCallback((provider: ProviderState) => {
    setDrawerProvider(provider);
  }, []);

  const handleDrawerClose = useCallback(() => {
    setDrawerProvider(null);
  }, []);

  // --- Derived status ---
  const patternStatus = useMemo(() => derivePatternStatus(providers), [providers]);

  // --- Table ---
  const table = useReactTable({
    data: providers,
    columns: COLUMNS,
    getCoreRowModel: getCoreRowModel(),
    getRowId: (row) => row.id,
  });

  // --- Footer strip (first 12) ---
  const footerProviders = useMemo(() => providers.slice(0, 12), [providers]);

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  return (
    <div
      className="flex flex-col min-h-full bg-[var(--void)]"
      style={{ fontFamily: 'var(--font-mono)' }}
    >

      {/* ================================================================== */}
      {/* NERV HEADER                                                          */}
      {/* ================================================================== */}
      <header className="px-4 pt-4 pb-3 shrink-0">
        {/*
          Double border pattern: 2px outer + 4px transparent gap + 1px inner.
          The gap is the padding between the two border boxes.
        */}
        <div style={{ border: '2px solid var(--rose-dim)', padding: '4px' }}>
          <div
            className="relative"
            style={{ border: '1px solid var(--rose-dim)', padding: '10px 14px 8px' }}
          >
            {/* NERV corner label cutout */}
            <span
              className="absolute -top-[9px] left-3 px-1 font-[var(--font-mono)] text-[10px] tracking-[var(--tracking-widest)] leading-none select-none"
              style={{ background: 'var(--void)', color: 'var(--rose-dim)' }}
              aria-hidden
            >
              NERV
            </span>

            {/* Title + pattern status */}
            <div className="flex items-baseline justify-between gap-4 flex-wrap">
              <h1
                className="font-[var(--font-mono)] leading-none tracking-[var(--tracking-widest)] select-none"
                style={{ fontSize: 'var(--text-lg)', color: 'var(--text-strong)' }}
              >
                ＰＲＯＶＩＤＥＲ　ＭＯＮＩＴＯＲ
              </h1>

              <div className="flex items-center gap-2 shrink-0">
                <span
                  className="font-[var(--font-mono)] text-[10px] tracking-[var(--tracking-widest)] leading-none"
                  style={{ color: 'var(--text-ghost)' }}
                  aria-hidden
                >
                  PATTERN:
                </span>
                <span
                  className="font-[var(--font-mono)] text-[var(--text-xs)] tracking-[var(--tracking-widest)] leading-none font-medium"
                  style={{ color: PATTERN_STATUS_COLOR[patternStatus] }}
                  aria-label={`System pattern status: ${patternStatus}`}
                >
                  {patternStatus}
                </span>
              </div>
            </div>

            {/* Sub-label: counts */}
            <div className="flex items-center gap-4 mt-1.5 flex-wrap">
              <span
                className="font-[var(--font-mono)] text-[10px] leading-none tabular-nums"
                style={{ color: 'var(--text-ghost)' }}
              >
                UNIT COUNT: {providers.length}
              </span>
              <span
                className="font-[var(--font-mono)] text-[10px] leading-none tabular-nums"
                style={{ color: 'var(--text-ghost)' }}
              >
                HEALTHY: {providers.filter((p) => p.status === 'healthy').length}
              </span>
              <span
                className="font-[var(--font-mono)] text-[10px] leading-none tabular-nums"
                style={{ color: 'var(--text-ghost)' }}
              >
                CIRCUITS OPEN: {providers.filter((p) => p.circuitState === 'open').length}
              </span>
            </div>
          </div>
        </div>
      </header>

      {/* ================================================================== */}
      {/* ＵＮＩＴ　ＡＲＲＡＹ                                              */}
      {/* ================================================================== */}
      <section className="px-4 pb-3 shrink-0">
        <SectionLabel>ＵＮＩＴ　ＡＲＲＡＹ</SectionLabel>

        {providers.length === 0 ? (
          <div
            className="flex items-center justify-center h-20 border border-[var(--text-ghost)]"
            style={{ background: 'var(--void)' }}
          >
            <span
              className="font-[var(--font-mono)] text-[10px] tracking-[var(--tracking-widest)] leading-none"
              style={{ color: 'var(--text-ghost)' }}
            >
              NO UNITS REGISTERED
            </span>
          </div>
        ) : (
          <div
            className="grid gap-2"
            style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(120px, 1fr))' }}
          >
            {providers.slice(0, 8).map((p) => (
              <UnitCard
                key={p.id}
                provider={p}
                selected={selectedId === p.id}
                onClick={() => handleUnitCardClick(p)}
              />
            ))}
          </div>
        )}
      </section>

      {/* ================================================================== */}
      {/* ＳＩＧＮＡＬ　ＴＲＡＣＥ                                          */}
      {/* ================================================================== */}
      <section className="px-4 pb-3 shrink-0">
        <SectionLabel>ＳＩＧＮＡＬ　ＴＲＡＣＥ</SectionLabel>

        <div
          className="border border-[var(--text-ghost)] px-3 py-2 flex flex-col gap-1.5"
          style={{ background: 'var(--void)' }}
        >
          <WaveformTrace label="COST"    data={aggregateHistory.cost}      color="var(--rose)"  height={32} />
          <WaveformTrace label="LATENCY" data={aggregateHistory.latency}   color="var(--dream)" height={32} />
          <WaveformTrace label="ERROR"   data={aggregateHistory.errorRate} color="var(--ember)" height={32} />
        </div>
      </section>

      {/* ================================================================== */}
      {/* ＰＲＯＶＩＤＥＲ　ＤＡＴＡ                                        */}
      {/* ================================================================== */}
      <section className="px-4 pb-3 flex-1">
        <SectionLabel>ＰＲＯＶＩＤＥＲ　ＤＡＴＡ</SectionLabel>

        <div
          className="overflow-x-auto border border-[var(--text-ghost)]"
          style={{ background: 'var(--void)' }}
        >
          <table className="w-full min-w-[600px] border-collapse">
            <thead>
              {table.getHeaderGroups().map((hg) => (
                <tr key={hg.id} className="border-b border-b-[var(--text-ghost)]">
                  {hg.headers.map((header) => (
                    <th key={header.id} className="px-3 py-1.5 text-left whitespace-nowrap">
                      <span
                        className="font-[var(--font-mono)] text-[10px] tracking-[var(--tracking-widest)] leading-none"
                        style={{ color: 'var(--text-faint)' }}
                      >
                        {flexRender(header.column.columnDef.header, header.getContext())}
                      </span>
                    </th>
                  ))}
                </tr>
              ))}
            </thead>

            <tbody>
              {table.getRowModel().rows.length === 0 ? (
                <tr>
                  <td
                    colSpan={COLUMNS.length}
                    className="px-3 py-5 text-center"
                  >
                    <span
                      className="font-[var(--font-mono)] text-[10px] tracking-[var(--tracking-widest)] leading-none"
                      style={{ color: 'var(--text-ghost)' }}
                    >
                      NO PROVIDER DATA
                    </span>
                  </td>
                </tr>
              ) : (
                table.getRowModel().rows.map((row) => (
                  <tr
                    key={row.id}
                    onClick={() => handleRowClick(row.original)}
                    className={clsx(
                      'border-b border-b-[var(--text-ghost)] cursor-pointer',
                      'hover:bg-[var(--bg-highlight)]',
                      'transition-[background-color] duration-[80ms] ease-[var(--ease-out)]',
                      drawerProvider?.id === row.original.id && 'bg-[var(--bg-highlight)]',
                    )}
                  >
                    {row.getVisibleCells().map((cell) => (
                      <td key={cell.id} className="px-3 py-1.5 whitespace-nowrap">
                        {flexRender(cell.column.columnDef.cell, cell.getContext())}
                      </td>
                    ))}
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </section>

      {/* ================================================================== */}
      {/* FOOTER CREDITS STRIP                                                 */}
      {/* ================================================================== */}
      <footer
        className="px-4 py-2 shrink-0 border-t border-t-[var(--text-ghost)] flex items-center gap-4 flex-wrap overflow-hidden"
        style={{ background: 'var(--void)' }}
      >
        {footerProviders.length === 0 ? (
          <span
            className="font-[var(--font-mono)] leading-none"
            style={{ fontSize: '10px', color: 'var(--text-ghost)' }}
          >
            ——
          </span>
        ) : (
          footerProviders.map((p) => (
            <div key={p.id} className="flex items-center gap-1 shrink-0">
              <StatusLED status={providerStatusToLED(p.status)} size="sm" />
              <span
                className="font-[var(--font-mono)] leading-none truncate max-w-[80px]"
                style={{ fontSize: '10px', color: 'var(--text-ghost)' }}
                title={p.name}
              >
                {p.name}
              </span>
            </div>
          ))
        )}
      </footer>

      {/* ================================================================== */}
      {/* PROVIDER DETAIL DRAWER                                               */}
      {/* ================================================================== */}
      <ProviderDetailDrawer provider={drawerProvider} onClose={handleDrawerClose} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// SectionLabel — shared section divider
// ---------------------------------------------------------------------------

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-2 mb-2">
      <span
        className="shrink-0 font-[var(--font-mono)] text-[10px] tracking-[var(--tracking-widest)] leading-none"
        style={{ color: 'var(--text-ghost)' }}
      >
        {children}
      </span>
      <div
        className="flex-1 h-px"
        style={{ background: 'var(--text-ghost)', opacity: 0.3 }}
      />
    </div>
  );
}
