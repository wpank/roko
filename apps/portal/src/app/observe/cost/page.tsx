'use client';

import React, { useMemo } from 'react';
import { clsx } from 'clsx';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts';
import { useDashboardStore } from '@/stores/dashboard';
import { ProgressBar } from '@/components/atoms';
import { useEfficiency } from '@/api/hooks';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ModelCostRow {
  model: string;
  cost: number;
  requests: number;
}

interface PlanCostRow {
  planId: string;
  name: string;
  taskCount: number;
  cost: number;
}

interface AgentCostRow {
  agentId: string;
  name: string;
  model: string;
  cost: number;
  tokensIn: number;
  tokensOut: number;
}

// ---------------------------------------------------------------------------
// Fallback mock data (used only when APIs are unavailable)
// ---------------------------------------------------------------------------

const MOCK_MODEL_COSTS: ModelCostRow[] = [];
const MOCK_PLAN_COSTS: PlanCostRow[] = [];
const MOCK_AGENT_COSTS: AgentCostRow[] = [];

// ---------------------------------------------------------------------------
// Colour ramp for the bar chart
// ---------------------------------------------------------------------------

const BAR_COLORS = [
  'var(--rose)',
  'var(--accent-cyan)',
  'var(--dream)',
  'var(--ember)',
  'var(--warning)',
  'var(--sage)',
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatCost(usd: number): string {
  const v = usd ?? 0;
  if (v < 0.001) return '$0.000';
  if (v < 0.01)  return `$${v.toFixed(4)}`;
  if (v < 1)     return `$${v.toFixed(3)}`;
  return `$${v.toFixed(2)}`;
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000)     return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

function abbreviateModel(model: string): string {
  // Keep at most 20 chars, preferring the last segment(s)
  const parts = model.split('-');
  let out = parts.join('-');
  if (out.length <= 20) return out;
  // Take the last 3 parts
  return parts.slice(-3).join('-');
}

// ---------------------------------------------------------------------------
// Custom recharts tooltip
// ---------------------------------------------------------------------------

interface ChartTooltipPayloadEntry {
  name: string;
  value: number;
  payload?: { requests?: number };
}

interface ChartTooltipProps {
  active?: boolean;
  payload?: ChartTooltipPayloadEntry[];
  label?: string;
}

function CustomTooltip({ active, payload, label }: ChartTooltipProps) {
  if (!active || !payload || payload.length === 0) return null;
  const entry = payload[0];
  return (
    <div
      className={clsx(
        'bg-[var(--bg-glass)] backdrop-blur-sm',
        'border border-[var(--text-ghost)]',
        'px-3 py-2 font-mono text-xs',
      )}
    >
      <p className="text-[var(--text-strong)] mb-1">{label}</p>
      <p className="text-[var(--rose)] tabular-nums">{formatCost(entry.value)}</p>
      {entry.payload?.requests !== undefined && (
        <p className="text-[var(--text-ghost)] tabular-nums mt-0.5">
          {entry.payload.requests} requests
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Stat card
// ---------------------------------------------------------------------------

interface StatCardProps {
  label: string;
  value: string;
  sub?: string;
}

function StatCard({ label, value, sub }: StatCardProps) {
  return (
    <div
      className={clsx(
        'flex flex-col gap-1 px-5 py-4',
        'bg-[var(--bg-secondary)]',
        'border border-[var(--text-ghost)]',
        'flex-1',
      )}
    >
      <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
        {label}
      </span>
      <span className="font-mono text-2xl tabular-nums text-[var(--text-strong)]">
        {value}
      </span>
      {sub && (
        <span className="font-mono text-xs text-[var(--text-ghost)]">{sub}</span>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Section header
// ---------------------------------------------------------------------------

function SectionHeader({ children }: { children: React.ReactNode }) {
  return (
    <div
      className={clsx(
        'flex items-center gap-2 px-4 py-2',
        'border-b border-b-[var(--text-ghost)]',
        'bg-[var(--bg-secondary)]',
      )}
    >
      <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
        {children}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function ObserveCostPage() {
  const plans    = useDashboardStore((s) => s.plans);
  const agents   = useDashboardStore((s) => s.agents);
  const vitals   = useDashboardStore((s) => s.vitals);
  const providers = useDashboardStore((s) => s.providers);

  // Fetch real efficiency data from /api/learn/efficiency
  const { data: efficiencyData } = useEfficiency();

  // Total cost: prefer live efficiency total, then statehub vitals, then 0
  const totalCostAllTime = efficiencyData?.summary?.totalCostUsd ?? 0;
  const costToday = (vitals?.costToday ?? 0) > 0 ? vitals.costToday : 0;

  // Model cost data — from providers when live, then from efficiency events,
  // then empty
  const modelCosts: ModelCostRow[] = useMemo(() => {
    const providerList = Object.values(providers);
    if (providerList.length > 0) {
      return providerList
        .filter((p) => p.costUsd > 0)
        .map((p) => ({
          model:    p.name,
          cost:     p.costUsd,
          requests: p.requestCount,
        }))
        .sort((a, b) => b.cost - a.cost);
    }
    return MOCK_MODEL_COSTS;
  }, [providers]);

  // Plan cost data — from SSE store when live, then empty
  const planCosts: PlanCostRow[] = useMemo(() => {
    const planList = Object.values(plans);
    if (planList.length > 0) {
      return planList
        .filter((p) => p.costUsd > 0)
        .map((p) => ({
          planId:    p.id,
          name:      p.name,
          taskCount: (p.tasks ?? []).length,
          cost:      p.costUsd,
        }))
        .sort((a, b) => b.cost - a.cost);
    }
    return MOCK_PLAN_COSTS;
  }, [plans]);

  // Agent cost data — from SSE store when live, then empty
  const agentCosts: AgentCostRow[] = useMemo(() => {
    const agentList = Object.values(agents);
    if (agentList.length > 0) {
      return agentList
        .filter((a) => a.costUsd > 0)
        .map((a) => ({
          agentId:   a.id,
          name:      a.name,
          model:     a.model,
          cost:      a.costUsd,
          tokensIn:  a.tokensIn,
          tokensOut: a.tokensOut,
        }))
        .sort((a, b) => b.cost - a.cost);
    }
    return MOCK_AGENT_COSTS;
  }, [agents]);

  // Chart data shape for recharts
  const chartData = modelCosts.map((r) => ({
    model:    abbreviateModel(r.model),
    cost:     r.cost,
    requests: r.requests,
  }));

  // Max cost for relative bar widths in plan/agent tables
  const maxPlanCost  = Math.max(...planCosts.map((r)  => r.cost),  0.001);
  const maxAgentCost = Math.max(...agentCosts.map((r) => r.cost), 0.001);

  return (
    <div className="flex flex-col min-h-0">

      {/* ---------------------------------------------------------------- */}
      {/* Summary stat cards                                                */}
      {/* ---------------------------------------------------------------- */}
      <div
        className={clsx(
          'flex flex-wrap gap-px px-4 py-4 shrink-0',
          'border-b border-b-[var(--text-ghost)]',
        )}
      >
        <StatCard
          label="Today"
          value={formatCost(costToday)}
          sub="since midnight (statehub)"
        />
        <StatCard
          label="All time"
          value={formatCost(totalCostAllTime)}
          sub={`${efficiencyData?.summary?.totalEvents ?? 0} tasks`}
        />
        <StatCard
          label="Pass rate"
          value={efficiencyData ? `${Math.round((efficiencyData.summary?.avgGatePassRate ?? 0) * 100)}%` : '—'}
          sub="avg gate pass rate"
        />
        <StatCard
          label="Total plans"
          value={String(planCosts.length)}
          sub={`${agentCosts.length} agents`}
        />
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* Cost by model — horizontal bar chart                              */}
      {/* ---------------------------------------------------------------- */}
      <SectionHeader>Cost by model</SectionHeader>

      <div className="px-4 py-4 border-b border-b-[var(--text-ghost)]">
        <ResponsiveContainer width="100%" height={Math.max(120, chartData.length * 32)}>
          <BarChart
            data={chartData}
            layout="vertical"
            margin={{ top: 0, right: 48, left: 0, bottom: 0 }}
          >
            <XAxis
              type="number"
              dataKey="cost"
              tickFormatter={(v: number) => formatCost(v)}
              tick={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                fill: 'var(--text-faint)',
              }}
              axisLine={{ stroke: 'var(--text-ghost)' }}
              tickLine={false}
            />
            <YAxis
              type="category"
              dataKey="model"
              width={140}
              tick={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                fill: 'var(--text-muted)',
              }}
              axisLine={false}
              tickLine={false}
            />
            <Tooltip content={<CustomTooltip />} cursor={{ fill: 'rgba(192,96,112,0.06)' }} />
            <Bar dataKey="cost" maxBarSize={18} radius={0}>
              {chartData.map((_entry, index) => (
                <Cell
                  key={`cell-${index}`}
                  fill={BAR_COLORS[index % BAR_COLORS.length]}
                />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* Cost by plan                                                      */}
      {/* ---------------------------------------------------------------- */}
      <SectionHeader>Cost by plan</SectionHeader>

      <div className="border-b border-b-[var(--text-ghost)]">
        {/* Table header */}
        <div
          className={clsx(
            'grid px-4 py-1.5',
            'bg-[var(--bg-secondary)]',
            'border-b border-b-[var(--text-ghost)]',
          )}
          style={{ gridTemplateColumns: '1fr 48px 64px 120px' }}
        >
          {['Plan', 'Tasks', 'Cost', 'Relative'].map((h) => (
            <span key={h} className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
              {h}
            </span>
          ))}
        </div>

        {planCosts.length === 0 ? (
          <div className="flex items-center justify-center h-12">
            <span className="font-mono text-xs text-[var(--text-ghost)]">No plan cost data.</span>
          </div>
        ) : (
          planCosts.map((row) => (
            <div
              key={row.planId}
              className={clsx(
                'grid items-center px-4 py-2',
                'border-b border-b-[var(--text-ghost)]',
                'hover:bg-[var(--bg-highlight)]',
                'transition-[background-color] duration-[80ms]',
              )}
              style={{ gridTemplateColumns: '1fr 48px 64px 120px' }}
            >
              <span className="font-mono text-xs text-[var(--text-strong)] truncate">
                {row.name}
              </span>
              <span className="font-mono text-xs tabular-nums text-[var(--text-faint)]">
                {row.taskCount}
              </span>
              <span className="font-mono text-xs tabular-nums text-[var(--rose)]">
                {formatCost(row.cost)}
              </span>
              <ProgressBar
                value={(row.cost / maxPlanCost) * 100}
                height={3}
              />
            </div>
          ))
        )}
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* Cost by agent                                                     */}
      {/* ---------------------------------------------------------------- */}
      <SectionHeader>Cost by agent</SectionHeader>

      <div>
        {/* Table header */}
        <div
          className={clsx(
            'grid px-4 py-1.5',
            'bg-[var(--bg-secondary)]',
            'border-b border-b-[var(--text-ghost)]',
          )}
          style={{ gridTemplateColumns: '1fr 140px 72px 72px 96px' }}
        >
          {['Agent', 'Model', 'Tokens in', 'Tokens out', 'Cost'].map((h) => (
            <span key={h} className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
              {h}
            </span>
          ))}
        </div>

        {agentCosts.length === 0 ? (
          <div className="flex items-center justify-center h-12">
            <span className="font-mono text-xs text-[var(--text-ghost)]">No agent cost data.</span>
          </div>
        ) : (
          agentCosts.map((row) => (
            <div
              key={row.agentId}
              className={clsx(
                'grid items-center px-4 py-2',
                'border-b border-b-[var(--text-ghost)]',
                'hover:bg-[var(--bg-highlight)]',
                'transition-[background-color] duration-[80ms]',
              )}
              style={{ gridTemplateColumns: '1fr 140px 72px 72px 96px' }}
            >
              <span className="font-mono text-xs text-[var(--text-strong)] truncate">
                {row.name}
              </span>
              <span className="font-mono text-xs text-[var(--text-ghost)] truncate">
                {abbreviateModel(row.model)}
              </span>
              <span className="font-mono text-xs tabular-nums text-[var(--text-faint)]">
                {formatTokens(row.tokensIn)}
              </span>
              <span className="font-mono text-xs tabular-nums text-[var(--text-faint)]">
                {formatTokens(row.tokensOut)}
              </span>
              <div className="flex items-center gap-2">
                <span className="font-mono text-xs tabular-nums text-[var(--rose)]">
                  {formatCost(row.cost)}
                </span>
                <ProgressBar
                  value={(row.cost / maxAgentCost) * 100}
                  height={2}
                  className="flex-1"
                />
              </div>
            </div>
          ))
        )}
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* Footer                                                            */}
      {/* ---------------------------------------------------------------- */}
      <div
        className={clsx(
          'px-4 py-3 mt-auto',
          'border-t border-t-[var(--text-ghost)]',
          'bg-[var(--bg-secondary)]',
        )}
      >
        <p className="font-mono text-xs text-[var(--text-ghost)] leading-relaxed">
          Cost totals from <span className="text-[var(--text-faint)]">/api/learn/efficiency</span>.
          Per-model and per-agent breakdowns update via the SSE stream when plans are running.
          USD figures are estimates; actual billing may differ by provider.
        </p>
      </div>
    </div>
  );
}
