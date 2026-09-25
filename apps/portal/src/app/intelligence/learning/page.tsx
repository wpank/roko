'use client';

import React, { useState, useMemo, useCallback } from 'react';
import { clsx } from 'clsx';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from 'recharts';
import { Sparkline } from '@/components/atoms/Sparkline';
import { Spinner } from '@/components/atoms/Spinner';
import { Pill } from '@/components/atoms/Pill';
import {
  useCascadeRouter,
  useEfficiency,
  usePlaybooks,
} from '@/api/hooks';
import { useDashboardStore } from '@/stores/dashboard';
import type { PlaybookEntry } from '@/api/types';

// ---------------------------------------------------------------------------
// Section collapse state — persisted to localStorage
// ---------------------------------------------------------------------------

const STORAGE_KEY = 'roko:intelligence:learning:collapsed';

function readCollapsed(): Record<string, boolean> {
  if (typeof window === 'undefined') return {};
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}');
  } catch {
    return {};
  }
}

function writeCollapsed(state: Record<string, boolean>): void {
  if (typeof window === 'undefined') return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

function useCollapseState(sectionId: string, defaultOpen: boolean) {
  const [collapsed, setCollapsed] = useState<boolean>(() => {
    const stored = readCollapsed();
    return sectionId in stored ? stored[sectionId] : !defaultOpen;
  });

  const toggle = useCallback(() => {
    setCollapsed((prev) => {
      const next = !prev;
      const stored = readCollapsed();
      stored[sectionId] = next;
      writeCollapsed(stored);
      return next;
    });
  }, [sectionId]);

  return { collapsed, toggle };
}

// ---------------------------------------------------------------------------
// Section wrapper
// ---------------------------------------------------------------------------

function Section({
  id,
  title,
  defaultOpen = true,
  children,
}: {
  id: string;
  title: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}) {
  const { collapsed, toggle } = useCollapseState(id, defaultOpen);

  return (
    <div className="flex flex-col bg-[var(--bg-secondary)] border border-[var(--border-default)]">
      {/* Section header */}
      <button
        type="button"
        onClick={toggle}
        className={clsx(
          'flex items-center justify-between w-full px-4 py-3',
          'border-b',
          collapsed ? 'border-transparent' : 'border-b-[var(--text-ghost)]',
          'hover:bg-[var(--bg-highlight)]',
          'transition-[background-color,border-color] duration-[80ms]',
          'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose)]',
        )}
        aria-expanded={!collapsed}
      >
        <span className="font-mono text-xs font-medium text-text-strong tracking-wider uppercase">
          {title}
        </span>
        <span
          className="font-mono text-xs text-text-ghost transition-transform duration-[150ms]"
          style={{ transform: collapsed ? 'rotate(-90deg)' : 'rotate(0deg)' }}
        >
          ▾
        </span>
      </button>

      {/* Collapsible content */}
      {!collapsed && <div>{children}</div>}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Placeholder c-factor component history
// ---------------------------------------------------------------------------

interface CfactorComponent {
  name:   string;
  value:  number;
  weight: number;
}

const CFACTOR_COMPONENTS: CfactorComponent[] = [
  { name: 'Gate pass rate',        value: 0.81, weight: 0.30 },
  { name: 'Efficiency score',      value: 0.74, weight: 0.25 },
  { name: 'Knowledge tier ratio',  value: 0.68, weight: 0.20 },
  { name: 'Routing entropy',       value: 0.65, weight: 0.15 },
  { name: 'Episode diversity',     value: 0.58, weight: 0.10 },
];

const CFACTOR_HISTORY_POINTS = [
  { t: '09:00', gatePass: 0.78, efficiency: 0.70, knowledge: 0.62, routing: 0.60, diversity: 0.52 },
  { t: '10:00', gatePass: 0.80, efficiency: 0.71, knowledge: 0.63, routing: 0.61, diversity: 0.54 },
  { t: '11:00', gatePass: 0.79, efficiency: 0.72, knowledge: 0.65, routing: 0.62, diversity: 0.55 },
  { t: '12:00', gatePass: 0.82, efficiency: 0.73, knowledge: 0.66, routing: 0.63, diversity: 0.56 },
  { t: '13:00', gatePass: 0.83, efficiency: 0.73, knowledge: 0.67, routing: 0.63, diversity: 0.57 },
  { t: '14:00', gatePass: 0.80, efficiency: 0.74, knowledge: 0.67, routing: 0.64, diversity: 0.57 },
  { t: '15:00', gatePass: 0.81, efficiency: 0.74, knowledge: 0.68, routing: 0.65, diversity: 0.58 },
];

const AREA_COLORS: Record<string, string> = {
  gatePass:   'var(--sage)',
  efficiency: 'var(--rose)',
  knowledge:  'var(--dream)',
  routing:    'var(--accent-cyan)',
  diversity:  'var(--warning)',
};

// ---------------------------------------------------------------------------
// C-Factor Detail Section
// ---------------------------------------------------------------------------

function CFactorDetailSection() {
  const learning = useDashboardStore((s) => s.learning);

  return (
    <Section id="cfactor" title="C-Factor Detail">
      <div className="p-4 flex flex-col gap-4">
        {/* Summary row */}
        <div className="flex items-baseline gap-3">
          <span
            className="font-mono font-medium num tabular-nums"
            style={{ fontSize: 'var(--text-3xl)', color: 'var(--rose-glow)', lineHeight: 1 }}
          >
            {(learning.cfactor ?? 0).toFixed(3)}
          </span>
          <span
            className="font-mono text-sm"
            style={{
              color:
                learning.cfactorTrend === 'up'   ? 'var(--sage)'         :
                learning.cfactorTrend === 'down' ? 'var(--accent-error)' :
                                                    'var(--text-faint)',
            }}
          >
            {learning.cfactorTrend === 'up' ? '↑' : learning.cfactorTrend === 'down' ? '↓' : '→'}
            &nbsp;
            {(learning.cfactorDelta ?? 0) >= 0 ? '+' : ''}
            {((learning.cfactorDelta ?? 0) * 100).toFixed(2)}
          </span>
        </div>

        {/* Component breakdown table */}
        <div className="flex flex-col border border-[var(--border-default)]">
          {/* Header */}
          <div className="flex items-center gap-4 px-3 py-1.5 bg-[var(--bg-raised)] border-b border-b-[var(--text-ghost)]">
            <span className="flex-1 font-mono text-xs text-text-ghost">component</span>
            <span className="w-12 font-mono text-xs text-text-ghost text-right">value</span>
            <span className="w-12 font-mono text-xs text-text-ghost text-right">weight</span>
            <span className="w-32 font-mono text-xs text-text-ghost">contribution</span>
          </div>

          {CFACTOR_COMPONENTS.map((c) => {
            const contribution = c.value * c.weight;
            const barPct       = (c.value * 100);

            return (
              <div
                key={c.name}
                className="flex items-center gap-4 px-3 py-2 border-b border-b-[var(--text-ghost)] last:border-b-0"
              >
                <span className="flex-1 font-mono text-xs text-text-muted truncate">{c.name}</span>
                <span
                  className="w-12 font-mono text-xs num tabular-nums text-right"
                  style={{ color: 'var(--text-strong)' }}
                >
                  {c.value.toFixed(2)}
                </span>
                <span className="w-12 font-mono text-xs num tabular-nums text-right text-text-faint">
                  {(c.weight * 100).toFixed(0)}%
                </span>
                <div className="w-32 flex items-center gap-2">
                  <div className="flex-1 h-1 bg-bg-highlight overflow-hidden">
                    <div
                      style={{
                        width:           `${barPct}%`,
                        height:          '100%',
                        backgroundColor: barPct >= 75 ? 'var(--sage)' : barPct >= 50 ? 'var(--rose)' : 'var(--warning)',
                        transition:      'width 200ms ease-out',
                      }}
                    />
                  </div>
                  <span className="font-mono text-xs num tabular-nums text-text-ghost shrink-0 w-10 text-right">
                    {contribution.toFixed(3)}
                  </span>
                </div>
              </div>
            );
          })}
        </div>

        {/* Stacked area chart */}
        <div className="h-40 w-full">
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={CFACTOR_HISTORY_POINTS} margin={{ top: 4, right: 0, left: -24, bottom: 0 }}>
              <CartesianGrid stroke="var(--text-ghost)" strokeDasharray="2 4" strokeOpacity={0.4} />
              <XAxis
                dataKey="t"
                tick={{ fontFamily: 'var(--font-mono)', fontSize: 10, fill: 'var(--text-faint)' }}
                axisLine={false}
                tickLine={false}
              />
              <YAxis
                domain={[0.4, 1.0]}
                tick={{ fontFamily: 'var(--font-mono)', fontSize: 10, fill: 'var(--text-faint)' }}
                axisLine={false}
                tickLine={false}
              />
              <Tooltip
                contentStyle={{
                  background:  'var(--bg-glass)',
                  border:      '1px solid var(--border-default)',
                  fontFamily:  'var(--font-mono)',
                  fontSize:    10,
                  color:       'var(--text-strong)',
                  borderRadius: 0,
                }}
                itemStyle={{ color: 'var(--text-muted)' }}
                labelStyle={{ color: 'var(--text-strong)' }}
              />
              {Object.entries(AREA_COLORS).map(([key, color]) => (
                <Area
                  key={key}
                  type="monotone"
                  dataKey={key}
                  stroke={color}
                  strokeWidth={1.5}
                  fill={color}
                  fillOpacity={0.08}
                  dot={false}
                  activeDot={{ r: 3 }}
                />
              ))}
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </div>
    </Section>
  );
}

// ---------------------------------------------------------------------------
// Cascade Router Section
// ---------------------------------------------------------------------------

const STAGE_LABELS: Record<string, string> = {
  static:     'Static',
  confidence: 'Confidence',
  ucb:        'UCB',
};

const STAGE_COLORS: Record<string, string> = {
  static:     'var(--text-faint)',
  confidence: 'var(--dream-bright)',
  ucb:        'var(--rose-glow)',
};

function CascadeRouterSection() {
  const { data, isLoading, isError } = useCascadeRouter();

  if (isLoading) {
    return (
      <Section id="cascade" title="Cascade Router">
        <div className="flex items-center justify-center py-10">
          <Spinner size="md" />
        </div>
      </Section>
    );
  }

  if (isError || !data) {
    return (
      <Section id="cascade" title="Cascade Router">
        <div className="px-4 py-6 font-mono text-xs text-accent-error">
          Failed to load cascade router data.
        </div>
      </Section>
    );
  }

  const models = data.models ?? [];
  const topModel = models.length > 0
    ? models.reduce(
        (best, m) => ((m.weight ?? 0) > (best.weight ?? 0) ? m : best),
        models[0],
      )
    : { model: '—', provider: '—', weight: 0, successRate: 0, avgCostUsd: 0 };

  const PLACEHOLDER_SPARKLINES: Record<string, number[]> = {};
  models.forEach((m, i) => {
    PLACEHOLDER_SPARKLINES[m.model] = [0.6, 0.65, 0.67, 0.66, 0.70, 0.71 + i * 0.01, 0.72 + i * 0.01, 0.73 + i * 0.01];
  });

  return (
    <Section id="cascade" title="Cascade Router">
      <div className="p-4 flex flex-col gap-4">
        {/* Stage indicator */}
        <div className="flex items-center gap-4">
          <div
            className="px-2 py-1 border font-mono text-xs font-medium"
            style={{
              borderColor: STAGE_COLORS[data.stage] ?? 'var(--border-default)',
              color:        STAGE_COLORS[data.stage] ?? 'var(--text-muted)',
            }}
          >
            {STAGE_LABELS[data.stage] ?? data.stage}
          </div>
          <span className="font-mono text-xs text-text-ghost">
            {(data.observationCount ?? 0).toLocaleString()} observations
          </span>
          <span className="font-mono text-xs text-text-ghost ml-auto">
            top: <span className="text-text-muted">{(topModel.model ?? '—').split('/').pop()}</span>
          </span>
        </div>

        {/* Model stats table */}
        <div className="flex flex-col border border-[var(--border-default)]">
          {/* Header */}
          <div className="flex items-center gap-4 px-3 py-1.5 bg-[var(--bg-raised)] border-b border-b-[var(--text-ghost)]">
            <span className="flex-1 font-mono text-xs text-text-ghost">model</span>
            <span className="w-16 font-mono text-xs text-text-ghost text-right">select %</span>
            <span className="w-16 font-mono text-xs text-text-ghost text-right">success</span>
            <span className="w-20 font-mono text-xs text-text-ghost text-right">avg cost</span>
            <span className="w-20 font-mono text-xs text-text-ghost">trend</span>
          </div>

          {models.map((m) => {
            const weightPct  = ((m.weight ?? 0) * 100).toFixed(1);
            const successPct = ((m.successRate ?? 0) * 100).toFixed(1);

            return (
              <div
                key={m.model}
                className="flex items-center gap-4 px-3 py-2 border-b border-b-[var(--text-ghost)] last:border-b-0"
              >
                {/* Model name */}
                <div className="flex-1 min-w-0 flex flex-col gap-0.5">
                  <span className="font-mono text-xs text-text-strong truncate">
                    {(m.model ?? '').split('/').pop() || m.model || '—'}
                  </span>
                  <span className="font-mono text-xs text-text-ghost">{m.provider}</span>
                </div>

                {/* Weight % */}
                <div className="w-16 flex flex-col items-end gap-0.5">
                  <span className="font-mono text-xs num tabular-nums text-text-muted">
                    {weightPct}%
                  </span>
                  <div className="w-12 h-0.5 bg-bg-highlight overflow-hidden">
                    <div
                      style={{
                        width:           `${m.weight * 100}%`,
                        height:          '100%',
                        backgroundColor: 'var(--rose)',
                      }}
                    />
                  </div>
                </div>

                {/* Success rate */}
                <span
                  className="w-16 font-mono text-xs num tabular-nums text-right"
                  style={{
                    color:
                      m.successRate >= 0.8  ? 'var(--sage)'    :
                      m.successRate >= 0.6  ? 'var(--warning)' :
                                               'var(--accent-error)',
                  }}
                >
                  {successPct}%
                </span>

                {/* Avg cost */}
                <span className="w-20 font-mono text-xs num tabular-nums text-right text-text-faint">
                  ${(m.avgCostUsd ?? 0).toFixed(4)}
                </span>

                {/* Sparkline */}
                <div className="w-20">
                  <Sparkline
                    data={PLACEHOLDER_SPARKLINES[m.model] ?? [0.7, 0.7]}
                    height={18}
                    color="var(--rose-dim)"
                  />
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </Section>
  );
}

// ---------------------------------------------------------------------------
// Efficiency Section
// ---------------------------------------------------------------------------

type PeriodOption = 'day' | 'week' | 'month' | 'all';

const PERIOD_VALUES: Record<PeriodOption, string | undefined> = {
  day:   '24h',
  week:  '7d',
  month: '30d',
  all:   undefined,
};

function EfficiencySection() {
  const [period, setPeriod] = useState<PeriodOption>('week');
  const { data, isLoading, isError } = useEfficiency(PERIOD_VALUES[period]);

  return (
    <Section id="efficiency" title="Efficiency">
      <div className="p-4 flex flex-col gap-4">
        {/* Period selector */}
        <div className="flex items-center gap-2">
          <span className="font-mono text-xs text-text-ghost">Period:</span>
          {(['day', 'week', 'month', 'all'] as PeriodOption[]).map((p) => (
            <Pill key={p} active={period === p} onClick={() => setPeriod(p)}>
              {p}
            </Pill>
          ))}
        </div>

        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <Spinner size="md" />
          </div>
        ) : isError || !data ? (
          <span className="font-mono text-xs text-accent-error">Failed to load efficiency data.</span>
        ) : (
          <>
            {/* Summary row */}
            <div className="grid grid-cols-3 gap-4">
              {[
                { label: 'Events',    value: (data.summary?.totalEvents ?? 0).toLocaleString(),                 color: 'var(--text-strong)' },
                { label: 'Pass rate', value: `${((data.summary?.avgGatePassRate ?? 0) * 100).toFixed(1)}%`,     color: (data.summary?.avgGatePassRate ?? 0) >= 0.8 ? 'var(--sage)' : 'var(--warning)' },
                { label: 'Total cost',value: `$${(data.summary?.totalCostUsd ?? 0).toFixed(2)}`,                color: 'var(--ember)' },
              ].map(({ label, value, color }) => (
                <div key={label} className="flex flex-col gap-0.5 p-2 border border-[var(--border-default)]">
                  <span className="font-mono text-xs text-text-ghost">{label}</span>
                  <span className="font-mono text-sm num tabular-nums font-medium" style={{ color }}>
                    {value}
                  </span>
                </div>
              ))}
            </div>

            {/* Events table */}
            <div className="flex flex-col border border-[var(--border-default)]">
              <div className="flex items-center gap-4 px-3 py-1.5 bg-[var(--bg-raised)] border-b border-b-[var(--text-ghost)]">
                <span className="flex-1 font-mono text-xs text-text-ghost">task</span>
                <span className="w-28 font-mono text-xs text-text-ghost truncate">plan</span>
                <span className="w-16 font-mono text-xs text-text-ghost text-right">pass rate</span>
                <span className="w-20 font-mono text-xs text-text-ghost text-right">cost</span>
                <span className="w-16 font-mono text-xs text-text-ghost text-right">when</span>
              </div>

              {(data.events ?? []).slice(0, 50).map((ev, i) => (
                <div
                  key={`${ev.agentId}-${i}`}
                  className="flex items-center gap-4 px-3 py-1.5 border-b border-b-[var(--text-ghost)] last:border-b-0"
                >
                  <span className="flex-1 font-mono text-xs text-text-muted truncate">{ev.agentId}</span>
                  <span className="w-28 font-mono text-xs text-text-ghost truncate">
                    {ev.model || '—'}
                  </span>
                  <span
                    className="w-16 font-mono text-xs num tabular-nums text-right"
                    style={{
                      color:
                        (ev.gatePassRate ?? 0) >= 0.8 ? 'var(--sage)'    :
                        (ev.gatePassRate ?? 0) >= 0.5 ? 'var(--warning)' :
                                                  'var(--accent-error)',
                    }}
                  >
                    {((ev.gatePassRate ?? 0) * 100).toFixed(0)}%
                  </span>
                  <span className="w-20 font-mono text-xs num tabular-nums text-right text-text-faint">
                    ${(ev.costUsd ?? 0).toFixed(4)}
                  </span>
                  <span className="w-16 font-mono text-xs num tabular-nums text-right text-text-ghost">
                    {formatRelativeTime(ev.timestamp)}
                  </span>
                </div>
              ))}

              {(data.events ?? []).length === 0 && (
                <div className="px-3 py-6 font-mono text-xs text-text-ghost text-center">
                  No efficiency events for this period.
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </Section>
  );
}

function formatRelativeTime(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  if (ms < 60_000)     return `${Math.floor(ms / 1000)}s ago`;
  if (ms < 3_600_000)  return `${Math.floor(ms / 60_000)}m ago`;
  if (ms < 86_400_000) return `${Math.floor(ms / 3_600_000)}h ago`;
  return `${Math.floor(ms / 86_400_000)}d ago`;
}

// ---------------------------------------------------------------------------
// Playbooks Section
// ---------------------------------------------------------------------------

function PlaybooksSection() {
  const [searchText, setSearchText] = useState('');
  const { data, isLoading, isError } = usePlaybooks();

  const filtered = useMemo(() => {
    if (!data) return [];
    const q = searchText.toLowerCase().trim();
    if (!q) return data;
    return data.filter(
      (p) =>
        p.whenCondition.toLowerCase().includes(q) ||
        p.thenAction.toLowerCase().includes(q),
    );
  }, [data, searchText]);

  return (
    <Section id="playbooks" title="Playbooks" defaultOpen={false}>
      <div className="p-4 flex flex-col gap-4">
        {/* Search */}
        <input
          type="text"
          value={searchText}
          onChange={(e) => setSearchText(e.target.value)}
          placeholder="Search when/then rules…"
          className={clsx(
            'w-full max-w-sm font-mono text-xs text-text-strong',
            'bg-bg-secondary border border-[var(--border-default)]',
            'px-3 py-2',
            'focus:border-[var(--border-active)] focus:bg-bg-highlight',
            'placeholder:text-text-ghost',
            'transition-[border-color,background-color] duration-[80ms]',
          )}
          aria-label="Search playbook rules"
        />

        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <Spinner size="md" />
          </div>
        ) : isError || !data ? (
          <span className="font-mono text-xs text-accent-error">Failed to load playbooks.</span>
        ) : (
          <div className="flex flex-col border border-[var(--border-default)]">
            {/* Header */}
            <div className="flex items-center gap-4 px-3 py-1.5 bg-[var(--bg-raised)] border-b border-b-[var(--text-ghost)]">
              <span className="flex-1 font-mono text-xs text-text-ghost">when condition</span>
              <span className="flex-1 font-mono text-xs text-text-ghost">then action</span>
              <span className="w-12 font-mono text-xs text-text-ghost text-right">hits</span>
              <span className="w-16 font-mono text-xs text-text-ghost text-right">last</span>
            </div>

            {filtered.length === 0 ? (
              <div className="px-3 py-6 font-mono text-xs text-text-ghost text-center">
                {searchText ? 'No matching rules.' : 'No playbook rules yet.'}
              </div>
            ) : (
              filtered.map((p: PlaybookEntry) => (
                <div
                  key={p.id}
                  className="flex items-start gap-4 px-3 py-2.5 border-b border-b-[var(--text-ghost)] last:border-b-0"
                >
                  <span className="flex-1 font-mono text-xs text-text-muted leading-relaxed">
                    {p.whenCondition}
                  </span>
                  <span className="flex-1 font-mono text-xs text-text-ghost leading-relaxed">
                    {p.thenAction}
                  </span>
                  <span
                    className="w-12 font-mono text-xs num tabular-nums text-right shrink-0"
                    style={{ color: p.hitCount > 10 ? 'var(--rose)' : p.hitCount > 3 ? 'var(--text-muted)' : 'var(--text-ghost)' }}
                  >
                    {p.hitCount}
                  </span>
                  <span className="w-16 font-mono text-xs text-text-ghost text-right shrink-0 num tabular-nums">
                    {p.lastAppliedAt ? formatRelativeTime(p.lastAppliedAt) : '—'}
                  </span>
                </div>
              ))
            )}
          </div>
        )}
      </div>
    </Section>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function LearningDashboardPage() {
  return (
    <div className="flex flex-col gap-4 p-6">
      <CFactorDetailSection />
      <CascadeRouterSection />
      <EfficiencySection />
      <PlaybooksSection />
    </div>
  );
}
