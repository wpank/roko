'use client';

import React, { useEffect } from 'react';
import Link from 'next/link';
import { clsx } from 'clsx';
import {
  RadialBarChart,
  RadialBar,
  ResponsiveContainer,
} from 'recharts';
import { Badge } from '@/components/atoms/Badge';
import { Sparkline } from '@/components/atoms/Sparkline';
import { StatusLED } from '@/components/atoms/StatusLED';
import { useIntelligenceStore } from '@/stores/intelligence';
import { useDashboardStore } from '@/stores/dashboard';
import { useKnowledgeTierCounts } from '@/api/hooks';
import type { KnowledgeTier, LearningStage, DreamPhase } from '@/api/types';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatRelativeTime(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  if (ms < 60_000)     return `${Math.floor(ms / 1000)}s ago`;
  if (ms < 3_600_000)  return `${Math.floor(ms / 60_000)}m ago`;
  if (ms < 86_400_000) return `${Math.floor(ms / 3_600_000)}h ago`;
  return `${Math.floor(ms / 86_400_000)}d ago`;
}

const TIER_ORDER: KnowledgeTier[] = ['transient', 'working', 'consolidated', 'persistent'];

const TIER_COLORS: Record<KnowledgeTier, string> = {
  transient:    'var(--warning)',
  working:      'var(--dream)',
  consolidated: 'var(--rose-dim)',
  persistent:   'var(--rose-glow)',
};

const TIER_BADGE_VARIANTS: Record<KnowledgeTier, 'warning' | 'dream' | 'default' | 'success'> = {
  transient:    'warning',
  working:      'dream',
  consolidated: 'default',
  persistent:   'success',
};

const STAGE_LABELS: Record<LearningStage, string> = {
  static:     'Static',
  confidence: 'Confidence',
  ucb:        'UCB',
};

const STAGE_DESCRIPTIONS: Record<LearningStage, string> = {
  static:     'Fixed weights — no online updates',
  confidence: 'Confidence-interval routing active',
  ucb:        'Upper confidence bound exploration',
};

const DREAM_PHASES: DreamPhase[] = ['hypnagogia', 'imagination', 'consolidation'];

const DREAM_PHASE_LABELS: Record<DreamPhase | 'idle', string> = {
  hypnagogia:    'Hypnagogia',
  imagination:   'Imagination',
  consolidation: 'Consolidation',
  idle:          'Idle',
};

// ---------------------------------------------------------------------------
// Mock insight feed (live data would come from useKnowledgeQuery)
// ---------------------------------------------------------------------------

interface InsightEntry {
  id: string;
  kind: 'heuristic' | 'warning' | 'causal_link' | 'insight' | 'strategy_fragment';
  topic: string;
  tier: KnowledgeTier;
  confirmations: number;
  createdAt: string;
}

const KIND_ICONS: Record<InsightEntry['kind'], string> = {
  heuristic:         'H',
  warning:           '!',
  causal_link:       '→',
  insight:           '◎',
  strategy_fragment: '⊞',
};

const KIND_COLORS: Record<InsightEntry['kind'], string> = {
  heuristic:         'var(--accent-cyan)',
  warning:           'var(--warning)',
  causal_link:       'var(--dream)',
  insight:           'var(--rose-glow)',
  strategy_fragment: 'var(--sage)',
};

const PLACEHOLDER_INSIGHTS: InsightEntry[] = [
  { id: 'p1', kind: 'insight',           topic: 'Gate pass rate improves when context < 60%',         tier: 'working',      confirmations: 3,  createdAt: new Date(Date.now() -   120_000).toISOString() },
  { id: 'p2', kind: 'heuristic',         topic: 'Anthropic provider latency spikes at 14:00–15:00',    tier: 'consolidated', confirmations: 7,  createdAt: new Date(Date.now() -   480_000).toISOString() },
  { id: 'p3', kind: 'causal_link',       topic: 'HDC fingerprint diversity correlates with c-factor',  tier: 'persistent',   confirmations: 12, createdAt: new Date(Date.now() -   900_000).toISOString() },
  { id: 'p4', kind: 'strategy_fragment', topic: 'Replan on gate failure reduces total task cost ~18%', tier: 'working',      confirmations: 5,  createdAt: new Date(Date.now() - 1_800_000).toISOString() },
  { id: 'p5', kind: 'warning',           topic: 'Budget over-run risk when wave depth > 4',            tier: 'transient',    confirmations: 1,  createdAt: new Date(Date.now() - 3_600_000).toISOString() },
  { id: 'p6', kind: 'insight',           topic: 'Context bidder tension resolves in favour of task',   tier: 'working',      confirmations: 4,  createdAt: new Date(Date.now() - 7_200_000).toISOString() },
  { id: 'p7', kind: 'heuristic',         topic: 'UCB exploration raises cost by ~8% vs static',        tier: 'consolidated', confirmations: 9,  createdAt: new Date(Date.now() - 10_800_000).toISOString() },
  { id: 'p8', kind: 'causal_link',       topic: 'Dream consolidation raises c-factor 0.03–0.07',       tier: 'persistent',   confirmations: 15, createdAt: new Date(Date.now() - 21_600_000).toISOString() },
];

const PLACEHOLDER_CFACTOR_HISTORY = [0.51, 0.53, 0.54, 0.57, 0.60, 0.62, 0.65, 0.67, 0.68, 0.71, 0.72, 0.72];

// ---------------------------------------------------------------------------
// C-Factor Card
// ---------------------------------------------------------------------------

function CFactorCard() {
  const { cfactor } = useIntelligenceStore();
  const value = cfactor?.value ?? 0;
  const trend = cfactor?.trend ?? 'flat';
  const delta = cfactor?.delta ?? 0;

  const trendArrow = trend === 'up' ? '↑' : trend === 'down' ? '↓' : '→';
  const trendColor =
    trend === 'up'   ? 'var(--sage)' :
    trend === 'down' ? 'var(--accent-error)' :
                       'var(--text-faint)';

  const radialData = [{ value: Math.max(1, Math.round(value * 100)), fill: 'var(--rose-glow)' }];
  const fillPct = Math.round(value * 100);

  return (
    <div
      className="flex flex-col gap-3 p-4 bg-[var(--bg-secondary)] border border-[var(--border-default)] relative overflow-hidden"
      style={{ boxShadow: 'var(--shadow-rose)' }}
    >
      {/* Label row */}
      <div className="flex items-center justify-between">
        <span className="font-mono text-xs text-text-faint tracking-widest uppercase">
          C-Factor
        </span>
        <span className="font-mono text-xs num tabular-nums" style={{ color: trendColor }}>
          {trendArrow}&nbsp;{delta >= 0 ? '+' : ''}{(delta * 100).toFixed(1)}
        </span>
      </div>

      {/* Radial arc + big number */}
      <div className="flex items-center gap-4">
        <div className="relative w-20 h-20 shrink-0">
          <ResponsiveContainer width="100%" height="100%">
            <RadialBarChart
              cx="50%"
              cy="50%"
              innerRadius="70%"
              outerRadius="100%"
              startAngle={225}
              endAngle={-45}
              data={radialData}
              barSize={6}
            >
              <RadialBar
                background={{ fill: 'var(--bg-highlight)' }}
                dataKey="value"
                cornerRadius={0}
              />
            </RadialBarChart>
          </ResponsiveContainer>
          <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
            <span
              className="font-mono font-medium num tabular-nums"
              style={{ fontSize: 'var(--text-2xl)', color: 'var(--rose-glow)', lineHeight: 1 }}
            >
              {value.toFixed(2)}
            </span>
          </div>
        </div>
        <div className="flex flex-col gap-1">
          <span className="font-mono text-xs text-text-faint">{fillPct}th pct</span>
          <span className="font-mono text-xs text-text-ghost">composite</span>
        </div>
      </div>

      {/* Sparkline */}
      <Sparkline
        data={PLACEHOLDER_CFACTOR_HISTORY}
        height={28}
        color="var(--rose-glow)"
        fill
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Learning Stage Card
// ---------------------------------------------------------------------------

function LearningStageCard() {
  const { learningStage } = useIntelligenceStore();
  const learning = useDashboardStore((s) => s.learning);
  const stages: LearningStage[] = ['static', 'confidence', 'ucb'];
  const activeIndex = stages.indexOf(learningStage);

  return (
    <div className="flex flex-col gap-3 p-4 bg-[var(--bg-secondary)] border border-[var(--border-default)]">
      <span className="font-mono text-xs text-text-faint tracking-widest uppercase">
        Learning Stage
      </span>

      {/* 3-step indicator */}
      <div className="flex items-center">
        {stages.map((stage, i) => {
          const isActive = i === activeIndex;
          const isDone   = i < activeIndex;
          return (
            <React.Fragment key={stage}>
              <div
                className="w-5 h-5 flex items-center justify-center border font-mono text-xs font-medium shrink-0"
                style={{
                  borderColor: isActive ? 'var(--dream-bright)' : isDone ? 'var(--rose-dim)' : 'var(--text-ghost)',
                  color:       isActive ? 'var(--dream-bright)' : isDone ? 'var(--rose-dim)' : 'var(--text-ghost)',
                  boxShadow:   isActive ? '0 0 8px var(--dream)' : undefined,
                }}
                aria-current={isActive ? 'step' : undefined}
              >
                {isDone ? '✓' : i + 1}
              </div>
              {i < stages.length - 1 && (
                <div
                  className="flex-1 h-px mx-1"
                  style={{ backgroundColor: i < activeIndex ? 'var(--rose-dim)' : 'var(--text-ghost)' }}
                />
              )}
            </React.Fragment>
          );
        })}
      </div>

      {/* Active stage label */}
      <div className="flex flex-col gap-0.5">
        <span className="font-mono text-sm font-medium" style={{ color: 'var(--dream-bright)' }}>
          {STAGE_LABELS[learningStage]}
        </span>
        <span className="font-mono text-xs text-text-ghost">
          {STAGE_DESCRIPTIONS[learningStage]}
        </span>
      </div>

      {/* Observation count */}
      <div className="flex items-baseline gap-1.5 mt-auto pt-1">
        <span
          className="font-mono font-medium num tabular-nums"
          style={{ fontSize: 'var(--text-xl)', color: 'var(--text-strong)', lineHeight: 1 }}
        >
          {(learning.observationCount ?? 0).toLocaleString()}
        </span>
        <span className="font-mono text-xs text-text-ghost">observations</span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Knowledge Store Summary Card
// ---------------------------------------------------------------------------

function KnowledgeStoreCard() {
  const { knowledgeTierCounts, handleSSEEvent } = useIntelligenceStore();
  const { data: restTierCounts } = useKnowledgeTierCounts();

  // Seed the intelligence store from REST data on first load so the counts
  // are not all zeros before SSE events arrive.
  useEffect(() => {
    if (!restTierCounts) return;
    const sseTotal = Object.values(knowledgeTierCounts).reduce((s, v) => s + v, 0);
    if (sseTotal === 0) {
      // Synthesise a snapshot-like event to warm up the store.
      handleSSEEvent({
        type: 'snapshot',
        timestamp: new Date().toISOString(),
        snapshot: {
          plans: {},
          tasks: {},
          agents: {},
          recentGates: [],
          recentEpisodes: [],
          recentErrors: [],
          inboxItems: [],
          vitals: { activeAgents: 0, totalAgents: 0, gatePassRate: 0, costToday: 0, cfactor: 0, healthyProviders: 0, totalProviders: 0, affectWord: 'idle' },
          affect: null,
          providers: {},
          learning: {
            cfactor: 0,
            cfactorDelta: 0,
            cfactorTrend: 'flat',
            learningStage: 'static',
            observationCount: 0,
            knowledgeTierCounts: restTierCounts,
            activeExperiments: 0,
          },
        },
        cursor: '',
      });
    }
  // Only run when restTierCounts first arrives — knowledgeTierCounts intentionally omitted
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [restTierCounts]);

  // Prefer SSE counts if they have been populated; fall back to REST counts.
  const sseTotal = Object.values(knowledgeTierCounts).reduce((s, v) => s + v, 0);
  const activeCounts = sseTotal > 0 ? knowledgeTierCounts : (restTierCounts ?? knowledgeTierCounts);

  const total = TIER_ORDER.reduce((sum, tier) => sum + (activeCounts[tier] ?? 0), 0);

  return (
    <div className="flex flex-col gap-3 p-4 bg-[var(--bg-secondary)] border border-[var(--border-default)]">
      <div className="flex items-center justify-between">
        <span className="font-mono text-xs text-text-faint tracking-widest uppercase">
          Knowledge Store
        </span>
        <span className="font-mono text-xs text-text-ghost num tabular-nums">
          {total.toLocaleString()}
        </span>
      </div>

      {/* Stacked horizontal bar */}
      <div className="flex w-full h-3 overflow-hidden bg-bg-highlight" style={{ gap: '1px' }}>
        {TIER_ORDER.map((tier) => {
          const count = activeCounts[tier] ?? 0;
          const pct   = total > 0 ? (count / total) * 100 : 0;
          return (
            <div
              key={tier}
              title={`${tier}: ${count}`}
              style={{
                width:           `${pct}%`,
                backgroundColor: TIER_COLORS[tier],
                transition:      'width 300ms ease-out',
                minWidth:        count > 0 ? 2 : 0,
              }}
            />
          );
        })}
      </div>

      {/* Tier breakdown grid */}
      <div className="grid grid-cols-2 gap-y-1.5 gap-x-4">
        {TIER_ORDER.map((tier) => {
          const count = activeCounts[tier] ?? 0;
          return (
            <div key={tier} className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-1.5 min-w-0">
                <span
                  className="shrink-0 w-2 h-2"
                  style={{ backgroundColor: TIER_COLORS[tier] }}
                />
                <span className="font-mono text-xs text-text-ghost capitalize truncate">
                  {tier}
                </span>
              </div>
              <span
                className="font-mono text-xs num tabular-nums shrink-0"
                style={{ color: count > 0 ? 'var(--text-muted)' : 'var(--text-ghost)' }}
              >
                {count.toLocaleString()}
              </span>
            </div>
          );
        })}
      </div>

      <Link
        href="/intelligence/knowledge"
        className="font-mono text-xs text-rose hover:text-rose-bright transition-colors duration-[80ms] mt-auto"
      >
        Browse →
      </Link>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Dream State Card
// ---------------------------------------------------------------------------

function DreamStateCard() {
  const { dreamPhase, activeDreamCycleId } = useIntelligenceStore();
  const isRunning = dreamPhase !== 'idle';

  return (
    <div className="flex flex-col gap-3 p-4 bg-[var(--bg-secondary)] border border-[var(--border-default)]">
      <div className="flex items-center justify-between">
        <span className="font-mono text-xs text-text-faint tracking-widest uppercase">
          Dream State
        </span>
        <StatusLED status={isRunning ? 'active' : 'idle'} pulse={isRunning} size="sm" />
      </div>

      {/* Phase list */}
      <div className="flex flex-col gap-2">
        {DREAM_PHASES.map((phase) => {
          const isActive      = phase === dreamPhase;
          const phaseIndex    = DREAM_PHASES.indexOf(phase);
          const currentIndex  = dreamPhase === 'idle' ? -1 : DREAM_PHASES.indexOf(dreamPhase);
          const isDone        = currentIndex > phaseIndex;

          return (
            <div key={phase} className="flex items-center gap-2">
              <span
                className="shrink-0 w-1.5 h-1.5"
                style={{
                  backgroundColor:
                    isActive ? 'var(--dream-bright)' :
                    isDone   ? 'var(--rose-dim)'      :
                               'var(--text-ghost)',
                  boxShadow: isActive ? '0 0 6px var(--dream)' : undefined,
                }}
              />
              <span
                className="font-mono text-xs"
                style={{
                  color:
                    isActive ? 'var(--dream-bright)' :
                    isDone   ? 'var(--text-muted)'   :
                               'var(--text-ghost)',
                  fontWeight: isActive ? 500 : 400,
                }}
              >
                {DREAM_PHASE_LABELS[phase]}
              </span>
            </div>
          );
        })}
      </div>

      {/* Status / cycle id */}
      <div className="mt-auto pt-1">
        {isRunning ? (
          <div className="flex items-center gap-2">
            <span className="font-mono text-xs font-medium" style={{ color: 'var(--dream-bright)' }}>
              Running
            </span>
            {activeDreamCycleId && (
              <span className="font-mono text-xs text-text-ghost truncate">
                {activeDreamCycleId.slice(0, 8)}
              </span>
            )}
          </div>
        ) : (
          <span className="font-mono text-xs text-text-ghost">No active cycle</span>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Insight Row
// ---------------------------------------------------------------------------

function InsightRow({ entry, index }: { entry: InsightEntry; index: number }) {
  return (
    <Link
      href={`/intelligence/knowledge/${entry.id}`}
      className={clsx(
        'flex items-center gap-3 px-4 py-2.5',
        'border-b border-b-[var(--text-ghost)]',
        'hover:bg-[var(--bg-highlight)]',
        'transition-[background-color] duration-[80ms] ease-[var(--ease-out)]',
        'group',
      )}
    >
      {/* Row index */}
      <span className="shrink-0 font-mono text-xs text-text-ghost num tabular-nums w-4 text-right">
        {index + 1}
      </span>

      {/* Kind icon box */}
      <span
        className="shrink-0 w-5 h-5 flex items-center justify-center border font-mono text-xs font-medium select-none"
        style={{ borderColor: KIND_COLORS[entry.kind], color: KIND_COLORS[entry.kind] }}
        title={entry.kind.replace(/_/g, ' ')}
        aria-label={entry.kind.replace(/_/g, ' ')}
      >
        {KIND_ICONS[entry.kind]}
      </span>

      {/* Topic */}
      <span className="flex-1 font-mono text-xs text-text-strong truncate group-hover:text-bone-bright transition-colors duration-[80ms]">
        {entry.topic}
      </span>

      {/* Tier badge */}
      <Badge variant={TIER_BADGE_VARIANTS[entry.tier]} className="shrink-0">
        {entry.tier}
      </Badge>

      {/* Confirmations */}
      <span className="shrink-0 font-mono text-xs text-text-ghost num tabular-nums w-8 text-right">
        ×{entry.confirmations}
      </span>

      {/* Timestamp */}
      <span className="shrink-0 font-mono text-xs text-text-ghost num tabular-nums w-16 text-right">
        {formatRelativeTime(entry.createdAt)}
      </span>
    </Link>
  );
}

// ---------------------------------------------------------------------------
// Recent Insights Feed
// ---------------------------------------------------------------------------

function RecentInsightsFeed() {
  return (
    <section className="flex flex-col bg-[var(--bg-secondary)] border border-[var(--border-default)]">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2.5 border-b border-b-[var(--text-ghost)]">
        <span className="font-mono text-xs text-text-faint tracking-widest uppercase">
          Recent Insights
        </span>
        <Link
          href="/intelligence/knowledge"
          className="font-mono text-xs text-rose hover:text-rose-bright transition-colors duration-[80ms]"
        >
          All entries →
        </Link>
      </div>

      {/* Column headers */}
      <div className="flex items-center gap-3 px-4 py-1.5 border-b border-b-[var(--text-ghost)] bg-[var(--bg-raised)]">
        <span className="font-mono text-xs text-text-ghost w-4 text-right">#</span>
        <span className="font-mono text-xs text-text-ghost w-5">kind</span>
        <span className="font-mono text-xs text-text-ghost flex-1">topic</span>
        <span className="font-mono text-xs text-text-ghost w-24 text-right">tier</span>
        <span className="font-mono text-xs text-text-ghost w-8 text-right">conf</span>
        <span className="font-mono text-xs text-text-ghost w-16 text-right">when</span>
      </div>

      {PLACEHOLDER_INSIGHTS.map((entry, i) => (
        <InsightRow key={entry.id} entry={entry} index={i} />
      ))}
    </section>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function IntelligenceOverviewPage() {
  return (
    <div className="flex flex-col gap-6 p-6">
      {/* Top row: 4 summary cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <CFactorCard />
        <LearningStageCard />
        <KnowledgeStoreCard />
        <DreamStateCard />
      </div>

      {/* Recent insights feed */}
      <RecentInsightsFeed />
    </div>
  );
}
