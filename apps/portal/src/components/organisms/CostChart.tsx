'use client';

import React from 'react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts';
import { clsx } from 'clsx';

export interface CostChartProps {
  data: { name: string; cost: number }[];
  variant?: 'bar' | 'stacked';
}

// ---------------------------------------------------------------------------
// Custom tooltip
// ---------------------------------------------------------------------------

interface CustomTooltipProps {
  active?: boolean;
  payload?: Array<{ value: number; name: string }>;
  label?: string;
}

function CustomTooltip({ active, payload, label }: CustomTooltipProps) {
  if (!active || !payload?.length) return null;
  return (
    <div
      className={clsx(
        'bg-[var(--bg-raised)] border border-[var(--text-ghost)]',
        'px-3 py-2 font-mono text-xs',
      )}
    >
      <div className="text-[var(--text-muted)] mb-1">{label}</div>
      {payload.map((entry, i) => (
        <div key={i} className="text-[var(--rose)] num tabular-nums">
          ${entry.value.toFixed(4)}
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Y-axis tick formatter
// ---------------------------------------------------------------------------

function formatCostTick(value: number): string {
  if (value === 0) return '$0';
  if (value < 0.01) return `$${value.toFixed(4)}`;
  if (value < 1) return `$${value.toFixed(3)}`;
  return `$${value.toFixed(2)}`;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function CostChart({ data, variant = 'bar' }: CostChartProps) {
  if (data.length === 0) {
    return (
      <div className="flex items-center justify-center h-24 text-[var(--text-ghost)] font-mono text-xs">
        no cost data
      </div>
    );
  }

  // For the 'stacked' variant we treat each entry as its own bar (the prop
  // controls visual framing, not recharts stacking which requires multi-series
  // data). The difference is layout orientation only.
  const isHorizontal = true; // always horizontal bars per spec

  const barHeight = 20;
  const estimatedHeight = Math.max(80, data.length * (barHeight + 12) + 40);

  return (
    <div style={{ width: '100%', height: estimatedHeight }}>
      <ResponsiveContainer width="100%" height="100%">
        <BarChart
          data={data}
          layout="vertical"
          margin={{ top: 4, right: 16, bottom: 4, left: 80 }}
        >
          {/* Background grid in text-ghost color */}
          <CartesianGrid
            horizontal={false}
            vertical={true}
            stroke="var(--text-ghost)"
            strokeOpacity={0.25}
            strokeDasharray="2 4"
          />

          {/* Y-axis: category labels */}
          <YAxis
            type="category"
            dataKey="name"
            width={76}
            tick={{
              fontFamily: 'var(--font-mono)',
              fontSize: 10,
              fill: 'var(--text-muted)',
            }}
            tickLine={false}
            axisLine={{ stroke: 'var(--text-ghost)', strokeOpacity: 0.3 }}
          />

          {/* X-axis: cost values */}
          <XAxis
            type="number"
            tickFormatter={formatCostTick}
            tick={{
              fontFamily: 'var(--font-mono)',
              fontSize: 9,
              fill: 'var(--text-ghost)',
            }}
            tickLine={false}
            axisLine={{ stroke: 'var(--text-ghost)', strokeOpacity: 0.3 }}
          />

          <RechartsTooltip
            content={<CustomTooltip />}
            cursor={{ fill: 'var(--bg-highlight)', fillOpacity: 0.5 }}
          />

          <Bar dataKey="cost" radius={0} maxBarSize={barHeight} isAnimationActive={false}>
            {data.map((entry, index) => (
              <Cell
                key={`cell-${index}`}
                fill="var(--rose)"
                fillOpacity={0.7 + (index % 3) * 0.1}
              />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}
