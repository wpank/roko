'use client';

import React, { useState, useCallback, useRef, useEffect } from 'react';
import { useRouter } from 'next/navigation';
import { clsx } from 'clsx';
import {
  Play,
  FileText,
  Bot,
  CheckSquare,
  XSquare,
  AlertTriangle,
  DollarSign,
  Zap,
  Cpu,
  Activity,
  Heart,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';

import { useDashboardStore } from '@/stores/dashboard';
import { useDismissInbox, useRunPrompt, useHealth, useAgents, usePlans } from '@/api/hooks';
import { Button } from '@/components/atoms';
import { showToast } from '@/components/atoms/Toast';
import { InboxItem, PlanCard, VitalCell } from '@/components/molecules';
import type { ActivityItem } from '@/stores/dashboard';
import type { PlanStatus, AlertSeverity } from '@/api/types';

// ---------------------------------------------------------------------------
// Severity ordering for inbox sort
// ---------------------------------------------------------------------------

const SEVERITY_ORDER: Record<AlertSeverity, number> = {
  error:   0,
  warning: 1,
  success: 2,
  info:    3,
};

// ---------------------------------------------------------------------------
// Relative time helper
// ---------------------------------------------------------------------------

function relativeTime(iso: string): string {
  try {
    const diff = Date.now() - new Date(iso).getTime();
    if (diff < 60_000) return `${Math.floor(diff / 1_000)}s ago`;
    if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
    if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
    return `${Math.floor(diff / 86_400_000)}d ago`;
  } catch {
    return '?';
  }
}

// ---------------------------------------------------------------------------
// Activity feed icon helper
// ---------------------------------------------------------------------------

function ActivityIcon({ kind }: { kind: ActivityItem['kind'] }) {
  const cls = 'shrink-0';
  const sz = 12;
  const sw = 1.5;
  switch (kind) {
    case 'plan_started':
    case 'plan_completed':
    case 'plan_failed':
      return <Activity size={sz} strokeWidth={sw} className={clsx(cls, 'text-text-muted')} aria-hidden />;
    case 'task_completed':
      return <CheckSquare size={sz} strokeWidth={sw} className={clsx(cls, 'text-sage')} aria-hidden />;
    case 'task_failed':
    case 'gate_result':
      return <XSquare size={sz} strokeWidth={sw} className={clsx(cls, 'text-accent-error')} aria-hidden />;
    case 'agent_spawned':
    case 'agent_completed':
      return <Bot size={sz} strokeWidth={sw} className={clsx(cls, 'text-text-muted')} aria-hidden />;
    case 'budget_warning':
      return <AlertTriangle size={sz} strokeWidth={sw} className={clsx(cls, 'text-warning')} aria-hidden />;
    case 'cost_updated':
      return <DollarSign size={sz} strokeWidth={sw} className={clsx(cls, 'text-ember')} aria-hidden />;
    case 'affect_updated':
      return <Heart size={sz} strokeWidth={sw} className={clsx(cls, 'text-rose')} aria-hidden />;
    case 'cfactor_updated':
      return <Zap size={sz} strokeWidth={sw} className={clsx(cls, 'text-dream')} aria-hidden />;
    case 'provider_health_updated':
      return <Cpu size={sz} strokeWidth={sw} className={clsx(cls, 'text-accent-cyan')} aria-hidden />;
    case 'knowledge_tier_changed':
      return <Zap size={sz} strokeWidth={sw} className={clsx(cls, 'text-dream-bright')} aria-hidden />;
    case 'inbox_item_added':
      return <AlertTriangle size={sz} strokeWidth={sw} className={clsx(cls, 'text-warning')} aria-hidden />;
    case 'error_occurred':
      return <AlertTriangle size={sz} strokeWidth={sw} className={clsx(cls, 'text-accent-error')} aria-hidden />;
    default:
      return <Activity size={sz} strokeWidth={sw} className={clsx(cls, 'text-text-ghost')} aria-hidden />;
  }
}

// ---------------------------------------------------------------------------
// Run Prompt Modal
// ---------------------------------------------------------------------------

interface RunPromptModalProps {
  open: boolean;
  onClose: () => void;
}

function RunPromptModal({ open, onClose }: RunPromptModalProps) {
  const [prompt, setPrompt] = useState('');
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const { mutate: runPrompt, isPending, isSuccess, reset } = useRunPrompt();

  // Auto-focus textarea when modal opens
  useEffect(() => {
    if (open) {
      textareaRef.current?.focus();
      reset();
    } else {
      setPrompt('');
    }
  }, [open, reset]);

  // Close on Escape
  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose();
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [open, onClose]);

  const handleSubmit = useCallback(() => {
    const trimmed = prompt.trim();
    if (!trimmed) return;
    runPrompt(trimmed, {
      onSuccess: () => {
        setPrompt('');
        onClose();
        showToast('Prompt dispatched — check Active Work for progress', 'success');
      },
      onError: (err: Error) => {
        showToast(`Prompt failed: ${err.message}`, 'error');
      },
    });
  }, [prompt, runPrompt, onClose]);

  if (!open) return null;

  return (
    // Backdrop
    <div
      className={clsx(
        'fixed inset-0 flex items-start justify-center pt-[15vh]',
        'bg-black/60',
        'z-[var(--z-modal)]',
      )}
      onClick={onClose}
    >
      {/* Panel */}
      <div
        className={clsx(
          'w-full max-w-2xl mx-4',
          'bg-[var(--bg-raised)]',
          'border border-[var(--text-ghost)]',
          'shadow-[var(--shadow-lg)]',
          'flex flex-col',
        )}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label="Run Prompt"
      >
        {/* Header */}
        <div className="flex items-center justify-between h-10 px-3 border-b border-b-[var(--text-ghost)]">
          <span className="font-mono text-sm text-text-strong leading-none tracking-wide">
            Run Prompt
          </span>
          <span className="font-mono text-xs text-text-faint leading-none">
            POST /api/run
          </span>
        </div>

        {/* Body */}
        <div className="p-3 flex flex-col gap-3">
          <textarea
            ref={textareaRef}
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                handleSubmit();
              }
            }}
            placeholder="Enter a prompt to run through the graph template pipeline..."
            rows={4}
            className={clsx(
              'w-full font-mono text-sm text-text-strong',
              'bg-[var(--bg-secondary)] border border-[var(--text-ghost)]',
              'px-2.5 py-2 resize-none leading-relaxed',
              'placeholder:text-text-ghost',
              'focus:border-[var(--rose-dim)] focus:outline-none',
              'transition-[border-color] duration-[80ms]',
            )}
          />

          <div className="flex items-center justify-between gap-2">
            <span className="font-mono text-xs text-text-ghost leading-none">
              ⌘↵ to submit
            </span>
            <div className="flex items-center gap-2">
              <Button variant="ghost" size="sm" onClick={onClose}>
                Cancel
              </Button>
              <Button
                variant="primary"
                size="sm"
                onClick={handleSubmit}
                loading={isPending}
                disabled={!prompt.trim()}
              >
                {isSuccess ? 'Dispatched' : 'Run'}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Develop Modal
// ---------------------------------------------------------------------------

interface DevelopModalProps {
  open: boolean;
  onClose: () => void;
}

function DevelopModal({ open, onClose }: DevelopModalProps) {
  const [text, setText] = useState('');
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const { mutate: runPromptMutation, isPending } = useRunPrompt();

  useEffect(() => {
    if (open) {
      textareaRef.current?.focus();
    } else {
      setText('');
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose();
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [open, onClose]);

  const handleSubmit = useCallback(() => {
    const trimmed = text.trim();
    if (!trimmed) return;
    runPromptMutation(trimmed, {
      onSuccess: (data: unknown) => {
        const id = (data as { id?: string })?.id ?? '';
        setText('');
        onClose();
        showToast(
          `Task dispatched${id ? `: ${id.slice(0, 12)}` : ''}`,
          'success',
          { label: 'View agents →', href: '/agents' },
        );
      },
      onError: (err: Error) => {
        showToast(`Failed: ${err.message}`, 'error');
      },
    });
  }, [text, runPromptMutation, onClose]);

  if (!open) return null;

  return (
    <div
      className={clsx(
        'fixed inset-0 flex items-start justify-center pt-[15vh]',
        'bg-black/60',
        'z-[var(--z-modal)]',
      )}
      onClick={onClose}
    >
      <div
        className={clsx(
          'w-full max-w-2xl mx-4',
          'bg-[var(--bg-raised)]',
          'border border-[var(--text-ghost)]',
          'shadow-[var(--shadow-lg)]',
          'flex flex-col',
        )}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label="Develop"
      >
        <div className="flex items-center justify-between h-10 px-3 border-b border-b-[var(--text-ghost)]">
          <span className="font-mono text-sm text-[var(--text-strong)] leading-none tracking-wide">
            Develop
          </span>
          <span className="font-mono text-xs text-[var(--text-faint)] leading-none">
            Dispatches agent to build it
          </span>
        </div>

        <div className="p-3 flex flex-col gap-3">
          <textarea
            ref={textareaRef}
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                handleSubmit();
              }
            }}
            placeholder="Describe what to build, e.g. 'a rust app that prints hello world'..."
            rows={4}
            className={clsx(
              'w-full font-mono text-sm text-[var(--text-strong)]',
              'bg-[var(--bg-secondary)] border border-[var(--text-ghost)]',
              'px-2.5 py-2 resize-none leading-relaxed',
              'placeholder:text-[var(--text-ghost)]',
              'focus:border-[var(--rose-dim)] focus:outline-none',
              'transition-[border-color] duration-[80ms]',
            )}
          />

          <div className="flex items-center justify-between gap-2">
            <span className="font-mono text-xs text-[var(--text-ghost)] leading-none">
              ⌘↵ to dispatch
            </span>
            <div className="flex items-center gap-2">
              <Button variant="ghost" size="sm" onClick={onClose}>
                Cancel
              </Button>
              <Button
                variant="primary"
                size="sm"
                onClick={handleSubmit}
                loading={isPending}
                disabled={!text.trim()}
              >
                {isPending ? 'Dispatching...' : 'Develop'}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Expanded plan task list (inline, below card)
// ---------------------------------------------------------------------------

interface PlanTaskListProps {
  planId: string;
}

function PlanTaskList({ planId }: PlanTaskListProps) {
  // Use the raw tasks record and derive the filtered list in render
  // to avoid creating a new array reference on every store update
  // (which causes infinite re-renders with Zustand's shallow equality).
  const allTasks = useDashboardStore((s) => s.tasks);
  const tasks = React.useMemo(
    () => Object.values(allTasks).filter((t) => t.planId === planId),
    [allTasks, planId],
  );

  if (tasks.length === 0) {
    return (
      <div className="px-4 py-2 font-mono text-xs text-text-faint border-b border-b-[var(--text-ghost)]">
        No tasks loaded yet
      </div>
    );
  }

  return (
    <div className="border-b border-b-[var(--text-ghost)]">
      {tasks.map((task) => {
        const statusColor: Record<string, string> = {
          pending:     'text-text-ghost',
          dispatching: 'text-warning',
          running:     'text-warning',
          gating:      'text-dream',
          completed:   'text-sage',
          failed:      'text-accent-error',
        };
        return (
          <div
            key={task.id}
            className="flex items-center gap-3 px-6 py-1.5 border-b border-b-[var(--text-ghost)] last:border-b-0"
          >
            <span
              className={clsx(
                'font-mono text-xs leading-none tracking-wider uppercase',
                statusColor[task.status] ?? 'text-text-ghost',
              )}
            >
              {(task.status ?? 'unk').slice(0, 3)}
            </span>
            <span className="flex-1 truncate font-mono text-xs text-text-muted leading-snug">
              {task.name}
            </span>
            {task.agentName && (
              <span className="shrink-0 font-mono text-xs text-text-ghost leading-none">
                {task.agentName}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Section header
// ---------------------------------------------------------------------------

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <h2 className="font-mono text-xs text-text-faint leading-none tracking-widest uppercase mb-0">
      {children}
    </h2>
  );
}

// ---------------------------------------------------------------------------
// Needs Attention (Inbox) section
// ---------------------------------------------------------------------------

const INBOX_MAX_VISIBLE = 5;

function NeedsAttentionSection() {
  const rawItems = useDashboardStore((s) => s.inboxItems);
  const { mutate: dismiss } = useDismissInbox();
  const router = useRouter();

  // Show only undismissed items, sorted by severity
  const activeItems = rawItems
    .filter((i) => !i.dismissedAt)
    .sort((a, b) => (SEVERITY_ORDER[a.severity] ?? 99) - (SEVERITY_ORDER[b.severity] ?? 99));

  const visible = activeItems.slice(0, INBOX_MAX_VISIBLE);
  const overflow = activeItems.length - INBOX_MAX_VISIBLE;

  return (
    <section
      className={clsx(
        'bg-[var(--bg-raised)] border border-[var(--text-ghost)]',
        'flex flex-col',
      )}
      aria-label="Needs attention"
    >
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-b-[var(--text-ghost)]">
        <SectionLabel>Needs Attention</SectionLabel>
        {activeItems.length > 0 && (
          <span className="font-mono text-xs text-text-faint tabular-nums num leading-none">
            {activeItems.length}
          </span>
        )}
      </div>

      {/* Items */}
      {visible.length === 0 ? (
        <div className="px-3 py-3 font-mono text-xs text-text-faint leading-snug">
          All clear — no items need attention
        </div>
      ) : (
        <div>
          {visible.map((item) => (
            <InboxItem
              key={item.id}
              item={item}
              onDismiss={dismiss}
              onAction={(url) => router.push(url)}
            />
          ))}

          {overflow > 0 && (
            <button
              type="button"
              onClick={() => router.push('/work')}
              className={clsx(
                'w-full flex items-center justify-center gap-1',
                'px-3 py-2',
                'font-mono text-xs text-text-muted',
                'hover:bg-[var(--bg-highlight)] hover:text-text-strong',
                'transition-[background-color,color] duration-[80ms]',
                'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose)]',
              )}
            >
              <span>{overflow} more</span>
              <ChevronRight size={11} strokeWidth={1.5} aria-hidden />
            </button>
          )}
        </div>
      )}
    </section>
  );
}

// ---------------------------------------------------------------------------
// Active Work section
// ---------------------------------------------------------------------------

const ACTIVE_STATUSES: PlanStatus[] = ['running', 'gating'];
// Statuses that should show in the "Active Work" panel (all non-terminal, non-archived)
const VISIBLE_STATUSES: PlanStatus[] = ['running', 'gating', 'pending', 'paused'];

function ActiveWorkSection() {
  const storePlans = useDashboardStore((s) => s.plans);
  const [expandedPlanId, setExpandedPlanId] = useState<string | null>(null);
  const router = useRouter();

  // Also fetch from the REST plans endpoint so recently completed/pending plans
  // that arrived before the SSE stream connected are visible.
  const { data: restPlans } = usePlans();

  // Merge SSE-store plans with REST plans.  SSE data is authoritative for live
  // state; REST fills in plans the SSE stream hasn't seen yet.
  const mergedPlans = React.useMemo(() => {
    const merged: Record<string, import('@/api/types').PlanState> = { ...storePlans };
    if (restPlans) {
      for (const rp of restPlans) {
        if (!merged[rp.id]) {
          // REST response uses a lighter shape; coerce it into PlanState
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          const raw = rp as any;
          merged[rp.id] = {
            id: rp.id,
            name: raw.title ?? raw.name ?? rp.id,
            status: raw.completed ? 'completed' : raw.status ?? 'pending',
            progress: {
              total: raw.task_count ?? raw.total_tasks ?? 0,
              completed: raw.completed_task_count ?? raw.completed_tasks ?? 0,
              failed: raw.failed_tasks ?? 0,
            },
            tasks: [],
            costUsd: raw.cost_usd ?? 0,
            budgetUsd: raw.budget_usd ?? null,
            startedAt: raw.started_at ?? raw.startedAt ?? null,
            completedAt: raw.completed_at ?? raw.completedAt ?? null,
          };
        }
      }
    }
    return merged;
  }, [storePlans, restPlans]);

  const activePlans = Object.values(mergedPlans).filter((p) =>
    ACTIVE_STATUSES.includes(p.status),
  );

  // Show non-terminal non-active plans in a secondary list below active ones.
  // Cap at 5 to avoid overwhelming the panel — the full list is on /work/plans.
  const MAX_OTHER_PLANS = 5;
  const allOtherVisiblePlans = Object.values(mergedPlans).filter((p) =>
    VISIBLE_STATUSES.includes(p.status) && !ACTIVE_STATUSES.includes(p.status),
  );
  const otherVisiblePlans = allOtherVisiblePlans.slice(0, MAX_OTHER_PLANS);
  const otherVisibleOverflow = allOtherVisiblePlans.length - MAX_OTHER_PLANS;

  // If nothing at all is running/visible, show recent plans from REST
  // (completed within the last 7 days) to give the user context.
  const recentRestPlans = React.useMemo(() => {
    if (activePlans.length > 0 || otherVisiblePlans.length > 0) return [];
    if (!restPlans) return [];
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return (restPlans as any[]).slice(0, 5).map((rp: any) => ({
      id: rp.id,
      name: rp.title ?? rp.name ?? rp.id,
      status: rp.completed ? 'completed' : rp.status ?? 'pending',
      progress: {
        total: rp.task_count ?? 0,
        completed: rp.completed_task_count ?? 0,
        failed: rp.failed_tasks ?? 0,
      },
      tasks: [],
      costUsd: rp.cost_usd ?? 0,
      budgetUsd: rp.budget_usd ?? null,
      startedAt: rp.started_at ?? null,
      completedAt: rp.completed_at ?? null,
    } as import('@/api/types').PlanState));
  }, [activePlans.length, otherVisiblePlans.length, restPlans]);

  const displayPlans = [...activePlans, ...otherVisiblePlans, ...recentRestPlans];

  const toggle = useCallback((id: string) => {
    setExpandedPlanId((prev) => (prev === id ? null : id));
  }, []);

  return (
    <section
      className={clsx(
        'bg-[var(--bg-raised)] border border-[var(--text-ghost)]',
        'flex flex-col',
      )}
      aria-label="Active work"
    >
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-b-[var(--text-ghost)]">
        <SectionLabel>Active Work</SectionLabel>
        {activePlans.length > 0 ? (
          <span className="font-mono text-xs text-text-faint tabular-nums num leading-none">
            {activePlans.length} running
          </span>
        ) : allOtherVisiblePlans.length > 0 ? (
          <span className="font-mono text-xs text-text-ghost tabular-nums num leading-none">
            {allOtherVisiblePlans.length} plans
          </span>
        ) : null}
      </div>

      {/* Plans */}
      {displayPlans.length === 0 ? (
        <div className="px-3 py-3 font-mono text-xs text-text-faint leading-snug">
          No active work — start a plan or run a prompt
        </div>
      ) : (
        <div>
          {displayPlans.map((plan) => {
            const isExpanded = expandedPlanId === plan.id;
            return (
              <div key={plan.id}>
                {/* Card header row with expand toggle */}
                <div className="relative group">
                  <PlanCard
                    plan={plan}
                    selected={isExpanded}
                    onClick={() => toggle(plan.id)}
                  />
                  {/* Expand chevron — overlaid on the right edge */}
                  <span
                    className={clsx(
                      'absolute right-3 top-1/2 -translate-y-1/2',
                      'text-text-ghost group-hover:text-text-faint',
                      'transition-[color,transform] duration-[80ms]',
                      isExpanded && 'rotate-180',
                    )}
                    aria-hidden
                  >
                    <ChevronDown size={12} strokeWidth={1.5} />
                  </span>
                </div>

                {/* Inline task list */}
                {isExpanded && <PlanTaskList planId={plan.id} />}
              </div>
            );
          })}

          {/* Overflow link */}
          {otherVisibleOverflow > 0 && (
            <button
              type="button"
              onClick={() => router.push('/work/plans')}
              className={clsx(
                'w-full flex items-center justify-center gap-1',
                'px-3 py-2',
                'font-mono text-xs text-text-muted',
                'hover:bg-[var(--bg-highlight)] hover:text-text-strong',
                'transition-[background-color,color] duration-[80ms]',
                'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose)]',
              )}
            >
              <span>{otherVisibleOverflow} more</span>
              <ChevronRight size={11} strokeWidth={1.5} aria-hidden />
            </button>
          )}
        </div>
      )}
    </section>
  );
}

// ---------------------------------------------------------------------------
// Vitals Mosaic section
// ---------------------------------------------------------------------------

function VitalsMosaicSection() {
  const vitals = useDashboardStore((s) => s.vitals);
  const cfactorTrend = useDashboardStore((s) => s.learning.cfactorTrend);
  const seedVitalsFromHealth = useDashboardStore((s) => s.seedVitalsFromHealth);

  // Fetch health on load and whenever stale so vitals show real data even
  // before SSE events arrive that would update individual fields.
  const { data: health } = useHealth();

  // Fetch the agent list to get an accurate total-agents count.
  const { data: agentList } = useAgents();

  // Sync health data into the vitals store whenever the query result changes.
  React.useEffect(() => {
    if (!health) return;
    const snap = health.statehub?.snapshot;
    const totalAgents = (Array.isArray(agentList) ? agentList.length : 0) || health.active_agents || 0;
    // Agents that have status 'active' or 'idle' (registered = considered active in portal)
    const activeAgents = Array.isArray(agentList)
      ? agentList.filter(
          (a) => {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            const status = (a as any).status ?? '';
            return status === 'active' || status === 'registered' || status === 'idle';
          },
        ).length
      : health.active_agents || 0;
    seedVitalsFromHealth({
      activeAgents,
      totalAgents,
      costToday: snap?.cost_usd_total ?? 0,
      healthyProviders: health?.providers?.healthy ?? 0,
      totalProviders: health?.providers?.total ?? 0,
    });
  }, [health, agentList, seedVitalsFromHealth]);

  const gatePassPct = vitals.gatePassRate > 0
    ? `${Math.round(vitals.gatePassRate * 100)}%`
    : '—';

  // Only show a real cost once the value is non-zero; otherwise show dash
  // so we don't display "$0.00" before the health data arrives.
  const costDisplay = vitals.costToday > 0
    ? `$${vitals.costToday.toFixed(2)}`
    : '—';

  // C-factor is meaningless at exactly 0.000 before any learning data loads.
  const cfactorDisplay = vitals.cfactor !== 0
    ? vitals.cfactor.toFixed(3)
    : '—';

  const cells: Array<{
    label: string;
    value: string;
    trend?: 'up' | 'down' | 'flat';
  }> = [
    {
      label: 'Agents',
      value: vitals.totalAgents > 0
        ? `${vitals.activeAgents}/${vitals.totalAgents}`
        : vitals.activeAgents > 0
          ? vitals.activeAgents.toString()
          : '—',
    },
    {
      label: 'Gates',
      value: gatePassPct,
    },
    {
      label: 'Cost',
      value: costDisplay,
    },
    {
      label: 'C-factor',
      value: cfactorDisplay,
      trend: vitals.cfactor !== 0 ? cfactorTrend : undefined,
    },
    {
      label: 'Providers',
      value: vitals.totalProviders > 0
        ? `${vitals.healthyProviders}/${vitals.totalProviders}`
        : '—',
    },
    {
      label: 'Affect',
      value: vitals.affectWord,
    },
  ];

  return (
    <section
      className={clsx(
        'bg-[var(--bg-raised)] border border-[var(--text-ghost)]',
        'flex flex-col',
      )}
      aria-label="System vitals"
    >
      {/* Header */}
      <div className="px-3 py-2 border-b border-b-[var(--text-ghost)]">
        <SectionLabel>Vitals</SectionLabel>
      </div>

      {/* 2×3 grid */}
      <div className="grid grid-cols-2 grid-rows-3">
        {cells.map((cell, i) => (
          <div
            key={cell.label}
            className={clsx(
              // Right border on left column cells, except avoid double border
              i % 2 === 0 && 'border-r border-r-[var(--text-ghost)]',
              // Bottom border on all but last row
              i < 4 && 'border-b border-b-[var(--text-ghost)]',
            )}
          >
            <VitalCell
              label={cell.label}
              value={cell.value}
              trend={cell.trend}
              flash
            />
          </div>
        ))}
      </div>
    </section>
  );
}

// ---------------------------------------------------------------------------
// Recent Activity feed
// ---------------------------------------------------------------------------

function RecentActivitySection() {
  const feed = useDashboardStore((s) => s.activityFeed);
  const connectionStatus = useDashboardStore((s) => s.connectionStatus);

  return (
    <section
      className={clsx(
        'bg-[var(--bg-raised)] border border-[var(--text-ghost)]',
        'flex flex-col',
      )}
      aria-label="Recent activity"
    >
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-b-[var(--text-ghost)]">
        <SectionLabel>Recent Activity</SectionLabel>
        {feed.length > 0 && (
          <span className="font-mono text-xs text-text-faint tabular-nums num leading-none">
            {feed.length}
          </span>
        )}
      </div>

      {/* Feed list */}
      <div
        className="overflow-y-auto"
        style={{ maxHeight: 300 }}
      >
        {feed.length === 0 ? (
          <div className="px-3 py-3 font-mono text-xs text-text-faint leading-snug">
            {connectionStatus === 'connected'
              ? 'Connected — waiting for events…'
              : connectionStatus === 'connecting'
                ? 'Connecting to event stream…'
                : 'Disconnected — reconnecting…'}
          </div>
        ) : (
          feed.map((item) => (
            <div
              key={item.id}
              className={clsx(
                'flex items-start gap-2.5 px-3 py-1.5',
                'border-b border-b-[var(--text-ghost)] last:border-b-0',
              )}
            >
              {/* Icon */}
              <span className="mt-[2px]">
                <ActivityIcon kind={item.kind} />
              </span>

              {/* Summary */}
              <span className="flex-1 min-w-0 font-mono text-xs text-text-muted leading-snug truncate">
                {item.summary}
              </span>

              {/* Timestamp */}
              <span className="shrink-0 font-mono text-xs text-text-ghost tabular-nums num leading-snug whitespace-nowrap">
                {relativeTime(item.timestamp)}
              </span>
            </div>
          ))
        )}
      </div>
    </section>
  );
}

// ---------------------------------------------------------------------------
// Quick Actions
// ---------------------------------------------------------------------------

interface QuickActionsProps {
  onRunPrompt: () => void;
  onDevelop: () => void;
}

function QuickActions({ onRunPrompt, onDevelop }: QuickActionsProps) {
  const router = useRouter();

  return (
    <section
      className="bg-[var(--bg-raised)] border border-[var(--text-ghost)] p-3"
      aria-label="Quick actions"
    >
      <div className="flex flex-wrap items-center gap-2">
        <Button
          variant="primary"
          size="sm"
          onClick={onRunPrompt}
        >
          <Play size={11} strokeWidth={2} aria-hidden />
          Run Prompt
        </Button>

        <Button
          variant="secondary"
          size="sm"
          onClick={onDevelop}
        >
          <FileText size={11} strokeWidth={1.5} aria-hidden />
          Develop
        </Button>

        <Button
          variant="ghost"
          size="sm"
          onClick={() => router.push('/work/plans')}
        >
          <Activity size={11} strokeWidth={1.5} aria-hidden />
          Plans
        </Button>

        <Button
          variant="ghost"
          size="sm"
          onClick={() => router.push('/agents')}
        >
          <Bot size={11} strokeWidth={1.5} aria-hidden />
          Agents
        </Button>
      </div>
    </section>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function OverviewPage() {
  const [runPromptOpen, setRunPromptOpen] = useState(false);
  const [developOpen, setDevelopOpen] = useState(false);

  const openRunPrompt = useCallback(() => setRunPromptOpen(true), []);
  const closeRunPrompt = useCallback(() => setRunPromptOpen(false), []);
  const openDevelop = useCallback(() => setDevelopOpen(true), []);
  const closeDevelop = useCallback(() => setDevelopOpen(false), []);

  // Listen for the command-palette event that opens the Run Prompt modal
  useEffect(() => {
    function onOpenRunPrompt() {
      setRunPromptOpen(true);
    }
    window.addEventListener('roko:open-run-prompt', onOpenRunPrompt);
    return () => window.removeEventListener('roko:open-run-prompt', onOpenRunPrompt);
  }, []);

  return (
    <>
      {/* ---- Page content ---- */}
      <div className="flex flex-col gap-4 p-4 min-h-full">
        {/* Needs Attention — full width at top */}
        <NeedsAttentionSection />

        {/* Two-column grid */}
        <div className="grid grid-cols-1 lg:grid-cols-[3fr_2fr] gap-4">
          {/* Left: Active Work (60%) */}
          <ActiveWorkSection />

          {/* Right: Vitals Mosaic (40%) */}
          <VitalsMosaicSection />
        </div>

        {/* Recent Activity — full width */}
        <RecentActivitySection />

        {/* Quick Actions — full width at bottom */}
        <QuickActions
          onRunPrompt={openRunPrompt}
          onDevelop={openDevelop}
        />
      </div>

      {/* ---- Modals ---- */}
      <RunPromptModal open={runPromptOpen} onClose={closeRunPrompt} />
      <DevelopModal open={developOpen} onClose={closeDevelop} />
    </>
  );
}
