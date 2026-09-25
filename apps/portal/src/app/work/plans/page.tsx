'use client';

import React, { useState, useMemo, useCallback, useEffect } from 'react';
import { clsx } from 'clsx';
import { useRouter } from 'next/navigation';
import { useDashboardStore } from '@/stores/dashboard';
import {
  usePlans,
  usePlanTasks,
  useStartPlan,
  usePausePlan,
  useResumePlan,
  useCancelPlan,
  useRetryPlan,
} from '@/api/hooks';
import { Badge } from '@/components/atoms/Badge';
import { Button } from '@/components/atoms/Button';
import { ProgressBar } from '@/components/atoms/ProgressBar';
import { Spinner } from '@/components/atoms/Spinner';
import { TaskRow } from '@/components/molecules/TaskRow';
import { showToast } from '@/components/atoms/Toast';
import type {
  PlanState,
  TaskState,
  PlanStatus,
  GateResult,
  TaskStatus,
} from '@/api/types';

// ---------------------------------------------------------------------------
// Safe mapping layer — the ONLY place raw API data is touched
// ---------------------------------------------------------------------------

/**
 * Raw shape returned by `GET /api/plans`. The server uses snake_case and a
 * minimal set of fields; everything else must be defaulted.
 */
interface RawPlanListItem {
  id?: unknown;
  title?: unknown;
  task_count?: unknown;
  completed?: unknown;
  completed_task_count?: unknown;
  status?: unknown;
}

/** Raw shape for a single task inside the tasks response. */
interface RawTask {
  id?: unknown;
  status?: unknown;
  completed?: unknown;
  depends_on?: unknown;
  dependsOn?: unknown;
  description?: unknown;
  files?: unknown;
  wave?: unknown;
  agent_name?: unknown;
  agentName?: unknown;
  model?: unknown;
  cost_usd?: unknown;
  costUsd?: unknown;
  started_at?: unknown;
  startedAt?: unknown;
  completed_at?: unknown;
  completedAt?: unknown;
  gate_results?: unknown;
  gateResults?: unknown;
  tier?: unknown;
  name?: unknown;
  title?: unknown;
  plan_id?: unknown;
  planId?: unknown;
}

/** Raw shape returned by `GET /api/plans/:id/tasks`. May be an array or wrapped. */
interface RawTasksResponse {
  tasks?: unknown;
  plan_id?: unknown;
  task_count?: unknown;
}

// --- Mappers ---

const VALID_PLAN_STATUSES = new Set<string>([
  'pending', 'running', 'gating', 'completed', 'failed', 'paused', 'cancelled',
]);

const VALID_TASK_STATUSES = new Set<string>([
  'pending', 'dispatching', 'running', 'gating', 'completed', 'failed',
]);

function toStr(v: unknown, fallback: string): string {
  return typeof v === 'string' && v.length > 0 ? v : fallback;
}

function toNum(v: unknown, fallback: number): number {
  return typeof v === 'number' && Number.isFinite(v) ? v : fallback;
}

function toBool(v: unknown): boolean {
  return v === true;
}

function toStrArray(v: unknown): string[] {
  return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [];
}

function toPlanStatus(v: unknown, completed: boolean): PlanStatus {
  if (typeof v === 'string' && VALID_PLAN_STATUSES.has(v)) return v as PlanStatus;
  return completed ? 'completed' : 'pending';
}

function toTaskStatus(v: unknown, completed: boolean): TaskStatus {
  if (typeof v === 'string' && VALID_TASK_STATUSES.has(v)) return v as TaskStatus;
  return completed ? 'completed' : 'pending';
}

function toGateResults(v: unknown): GateResult[] {
  if (!Array.isArray(v)) return [];
  return v
    .filter((g): g is Record<string, unknown> => g != null && typeof g === 'object')
    .map((g) => ({
      id: toStr(g.id, `gate-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`),
      taskId: toStr(g.task_id ?? g.taskId, ''),
      planId: toStr(g.plan_id ?? g.planId, ''),
      gateName: toStr(g.gate_name ?? g.gateName ?? g.gate, 'gate'),
      rung: Math.min(6, Math.max(0, toNum(g.rung, 0))) as GateResult['rung'],
      passed: toBool(g.passed),
      summary: toStr(g.summary, ''),
      output: toStr(g.output ?? g.output_text, ''),
      durationMs: toNum(g.duration_ms ?? g.durationMs, 0),
      timestamp: toStr(g.timestamp, new Date().toISOString()),
    }));
}

function mapRawPlan(raw: unknown): PlanState | null {
  if (raw == null || typeof raw !== 'object') return null;
  const r = raw as RawPlanListItem;
  const id = toStr(r.id, '');
  if (!id) return null;

  const completed = toBool(r.completed);
  const total = toNum(r.task_count, 0);
  const completedCount = toNum(r.completed_task_count, 0);

  return {
    id,
    name: toStr(r.title, id),
    status: toPlanStatus(r.status, completed),
    progress: {
      total,
      completed: completedCount,
      failed: 0,
    },
    tasks: [],
    costUsd: 0,
    budgetUsd: null,
    startedAt: null,
    completedAt: completed ? new Date(0).toISOString() : null,
  };
}

function mapRawTask(raw: unknown, fallbackPlanId: string): TaskState | null {
  if (raw == null || typeof raw !== 'object') return null;
  const r = raw as RawTask;
  const id = toStr(r.id, '');
  if (!id) return null;

  const completed = toBool(r.completed);

  return {
    id,
    planId: toStr(r.plan_id ?? r.planId, fallbackPlanId),
    name: toStr(r.name ?? r.title ?? r.id, id),
    status: toTaskStatus(r.status, completed),
    wave: toNum(r.wave, 0),
    agentName: typeof (r.agent_name ?? r.agentName) === 'string'
      ? (r.agent_name ?? r.agentName) as string
      : null,
    model: typeof r.model === 'string' ? r.model : null,
    costUsd: toNum(r.cost_usd ?? r.costUsd, 0),
    startedAt: typeof (r.started_at ?? r.startedAt) === 'string'
      ? (r.started_at ?? r.startedAt) as string
      : null,
    completedAt: typeof (r.completed_at ?? r.completedAt) === 'string'
      ? (r.completed_at ?? r.completedAt) as string
      : null,
    dependsOn: toStrArray(r.depends_on ?? r.dependsOn),
    gateResults: toGateResults(r.gate_results ?? r.gateResults),
    description: typeof r.description === 'string' ? r.description : undefined,
    files: Array.isArray(r.files) ? toStrArray(r.files) : undefined,
    tier: typeof r.tier === 'string' ? r.tier : undefined,
  };
}

/**
 * Normalise the tasks endpoint response. The server wraps the array inside
 * `{ plan_id, task_count, tasks: [...] }` but we handle both shapes.
 */
function extractTasks(data: unknown, planId: string): TaskState[] {
  // Direct array
  if (Array.isArray(data)) {
    return data.map((t) => mapRawTask(t, planId)).filter((t): t is TaskState => t !== null);
  }
  // Wrapped object
  if (data != null && typeof data === 'object') {
    const wrapped = data as RawTasksResponse;
    if (Array.isArray(wrapped.tasks)) {
      return (wrapped.tasks as unknown[])
        .map((t) => mapRawTask(t, planId))
        .filter((t): t is TaskState => t !== null);
    }
  }
  return [];
}

// ---------------------------------------------------------------------------
// Display helpers (pure functions, no side effects)
// ---------------------------------------------------------------------------

function progressPct(plan: PlanState): number {
  const total = plan?.progress?.total ?? 0;
  if (total === 0) return 0;
  return Math.round(((plan?.progress?.completed ?? 0) / total) * 100);
}

function budgetPct(plan: PlanState): number {
  const budget = plan?.budgetUsd ?? 0;
  if (budget === 0) return 0;
  return Math.min(100, Math.round(((plan?.costUsd ?? 0) / budget) * 100));
}

function formatCost(usd: number): string {
  const v = Number.isFinite(usd) ? usd : 0;
  if (v < 0.001) return '$0.000';
  if (v < 1) return `$${v.toFixed(3)}`;
  return `$${v.toFixed(2)}`;
}

function miniBar(pct: number, width = 12): string {
  const clamped = Math.min(100, Math.max(0, Number.isFinite(pct) ? pct : 0));
  const filled = Math.round((clamped / 100) * width);
  return '\u2588'.repeat(filled) + '\u2500'.repeat(width - filled);
}

function abbreviateModel(model: string | null): string {
  if (!model) return '\u2014';
  return model
    .replace('claude-', 'cl-')
    .replace('gemini-', 'gem-')
    .split('-')
    .slice(0, 3)
    .join('-');
}

// ---------------------------------------------------------------------------
// Sorting
// ---------------------------------------------------------------------------

type SortField = 'status' | 'name' | 'cost';

const STATUS_ORDER: Record<string, number> = {
  running: 0,
  dispatching: 1,
  gating: 2,
  pending: 3,
  completed: 4,
  failed: 5,
};

function sortTasks(tasks: TaskState[], field: SortField): TaskState[] {
  const copy = [...tasks];
  switch (field) {
    case 'status':
      copy.sort((a, b) => (STATUS_ORDER[a.status] ?? 99) - (STATUS_ORDER[b.status] ?? 99));
      break;
    case 'name':
      copy.sort((a, b) => (a.name ?? '').localeCompare(b.name ?? ''));
      break;
    case 'cost':
      copy.sort((a, b) => (b.costUsd ?? 0) - (a.costUsd ?? 0));
      break;
  }
  return copy;
}

// ---------------------------------------------------------------------------
// Wave grouping
// ---------------------------------------------------------------------------

function buildWaveGroups(tasks: TaskState[]): Map<number, TaskState[]> {
  const map = new Map<number, TaskState[]>();
  for (const task of tasks) {
    const w = task?.wave ?? 0;
    const bucket = map.get(w) ?? [];
    bucket.push(task);
    map.set(w, bucket);
  }
  return map;
}

function waveProgress(tasks: TaskState[]): { done: number; total: number; failed: number } {
  const done = (tasks ?? []).filter((t) => t?.status === 'completed').length;
  const failed = (tasks ?? []).filter((t) => t?.status === 'failed').length;
  return { done, total: (tasks ?? []).length, failed };
}

// ---------------------------------------------------------------------------
// StatusBadge
// ---------------------------------------------------------------------------

const STATUS_BADGE_VARIANT: Record<string, 'warning' | 'dream' | 'success' | 'error' | 'default' | 'info'> = {
  running: 'warning',
  gating: 'dream',
  completed: 'success',
  failed: 'error',
  pending: 'default',
  paused: 'default',
  cancelled: 'default',
};

function StatusBadge({ status }: { status: PlanStatus }) {
  return (
    <Badge variant={STATUS_BADGE_VARIANT[status] ?? 'default'}>
      {(status ?? 'unknown').toUpperCase()}
    </Badge>
  );
}

// ---------------------------------------------------------------------------
// GateResultItem
// ---------------------------------------------------------------------------

function GateResultItem({ result }: { result: GateResult }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div className="border-b border-b-[var(--text-ghost)] last:border-b-0">
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        className={clsx(
          'w-full flex items-center gap-2 px-3 py-1',
          'font-[var(--font-mono)] text-[10px]',
          'hover:bg-[var(--bg-highlight)]',
          'transition-[background-color] duration-[80ms]',
          'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
        )}
      >
        <span
          className={clsx(
            'shrink-0 w-3 text-center',
            result?.passed ? 'text-[var(--sage)]' : 'text-[var(--accent-error)]',
          )}
        >
          {result?.passed ? '\u2713' : '\u2717'}
        </span>
        <span className="flex-1 text-left text-[var(--text-muted)]">
          {result?.gateName ?? 'gate'}
        </span>
        <span className="shrink-0 text-[var(--text-ghost)]">
          rung {result?.rung ?? 0}
        </span>
        {(result?.durationMs ?? 0) > 0 && (
          <span className="shrink-0 tabular-nums text-[var(--text-faint)] w-12 text-right">
            {(result.durationMs ?? 0) < 1000
              ? `${result.durationMs}ms`
              : `${((result.durationMs ?? 0) / 1000).toFixed(1)}s`}
          </span>
        )}
        <span className="shrink-0 text-[var(--text-ghost)] text-[8px] ml-1">
          {expanded ? '\u25BC' : '\u25B6'}
        </span>
      </button>
      {expanded && (
        <div className="px-4 pb-2 pt-1 bg-[var(--bg-secondary)]">
          {result?.summary && (
            <p className="font-[var(--font-mono)] text-[10px] text-[var(--text-faint)] mb-1">
              {result.summary}
            </p>
          )}
          {result?.output && (
            <pre className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] whitespace-pre-wrap break-all max-h-32 overflow-y-auto">
              {result.output}
            </pre>
          )}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// TaskDetailPanel (expanded row)
// ---------------------------------------------------------------------------

function TaskDetailPanel({ task }: { task: TaskState }) {
  const files = task?.files ?? [];
  const deps = task?.dependsOn ?? [];
  const gates = task?.gateResults ?? [];
  const hasFiles = files.length > 0;
  const hasDeps = deps.length > 0;
  const hasGates = gates.length > 0;
  const hasDesc = Boolean(task?.description);
  const hasTier = Boolean(task?.tier);

  if (!hasFiles && !hasDeps && !hasGates && !hasDesc && !hasTier) return null;

  return (
    <div className="bg-[var(--bg-secondary)] border-b border-b-[var(--text-ghost)] px-5 py-2">
      {hasDesc && (
        <p className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] mb-2 leading-relaxed">
          {task.description}
        </p>
      )}
      {(hasTier || hasDeps) && (
        <div className="flex items-center gap-4 mb-2 flex-wrap">
          {hasTier && (
            <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] border border-[var(--text-ghost)] px-1.5 py-0.5">
              {task.tier}
            </span>
          )}
          {hasDeps && (
            <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-faint)]">
              depends on:{' '}
              <span className="text-[var(--text-ghost)]">{deps.join(', ')}</span>
            </span>
          )}
        </div>
      )}
      {hasFiles && (
        <div className="mb-2">
          <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tracking-wider uppercase mr-2">
            files
          </span>
          <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-faint)]">
            {files.join('  ')}
          </span>
        </div>
      )}
      {hasGates && (
        <div className="border border-[var(--text-ghost)] mt-1">
          <div className="px-3 py-1 border-b border-b-[var(--text-ghost)] bg-[var(--bg-raised)]">
            <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tracking-wider uppercase">
              Gate Results ({gates.filter((g) => g?.passed).length}/{gates.length} passed)
            </span>
          </div>
          {gates.map((gr, idx) => (
            <GateResultItem key={gr?.id ?? idx} result={gr} />
          ))}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// WaveTree (left panel)
// ---------------------------------------------------------------------------

function WaveRow({
  waveIndex,
  tasks,
  selected,
  onClick,
}: {
  waveIndex: number;
  tasks: TaskState[];
  selected: boolean;
  onClick: () => void;
}) {
  const [open, setOpen] = useState(true);
  const { done, total, failed } = waveProgress(tasks);
  const pct = total > 0 ? Math.round((done / total) * 100) : 0;

  return (
    <div>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className={clsx(
          'w-full flex items-center gap-1.5 px-3 py-1',
          'font-[var(--font-mono)] text-[var(--text-xs)]',
          'text-[var(--text-muted)] hover:text-[var(--text-strong)]',
          'hover:bg-[var(--bg-highlight)]',
          'border-b border-b-[var(--text-ghost)]',
          'transition-[background-color,color] duration-[80ms]',
          'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
          selected && 'bg-[var(--bg-highlight)] text-[var(--text-strong)]',
        )}
        aria-expanded={open}
      >
        <span
          className={clsx(
            'text-[10px] text-[var(--text-ghost)] transition-transform duration-[80ms]',
            open ? 'rotate-90' : 'rotate-0',
          )}
          aria-hidden="true"
        >
          {'\u25B6'}
        </span>
        <span className="font-medium tracking-wide">W{waveIndex}</span>
        <span className="text-[var(--text-ghost)]">({done}/{total})</span>
        <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] ml-1 tracking-tighter">
          [{miniBar(pct, 8)}]
        </span>
        {failed > 0 && (
          <span className="ml-1 text-[var(--accent-error)]">{'\u2717'}{failed}</span>
        )}
      </button>

      {open &&
        (tasks ?? []).map((task) => (
          <button
            key={task?.id ?? Math.random()}
            type="button"
            onClick={onClick}
            className={clsx(
              'w-full flex items-center gap-2 px-5 py-1',
              'font-[var(--font-mono)] text-[10px]',
              'border-b border-b-[var(--text-ghost)]',
              'hover:bg-[var(--bg-highlight)]',
              'transition-[background-color] duration-[80ms]',
              'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
            )}
          >
            <span
              className={clsx(
                'shrink-0 w-3 text-center leading-none',
                task?.status === 'completed' && 'text-[var(--sage)]',
                task?.status === 'failed' && 'text-[var(--accent-error)]',
                task?.status === 'running' && 'text-[var(--rose)] animate-pulse',
                task?.status === 'gating' && 'text-[var(--dream)]',
                task?.status === 'dispatching' && 'text-[var(--warning)]',
                task?.status === 'pending' && 'text-[var(--text-ghost)]',
              )}
            >
              {task?.status === 'completed'
                ? '\u2713'
                : task?.status === 'failed'
                  ? '\u2717'
                  : task?.status === 'running'
                    ? '\u25CF'
                    : task?.status === 'gating'
                      ? '\u25CE'
                      : '\u25CB'}
            </span>
            <span className="flex-1 truncate text-left text-[var(--text-faint)]">
              {task?.name ?? 'untitled'}
            </span>
          </button>
        ))}
    </div>
  );
}

function WaveTree({
  plans,
  selectedPlanId,
  onSelectPlan,
  tasksByPlan,
  filter,
  onFilterChange,
  plansLoading,
  plansError,
}: {
  plans: PlanState[];
  selectedPlanId: string | null;
  onSelectPlan: (id: string) => void;
  tasksByPlan: Map<string, TaskState[]>;
  filter: string;
  onFilterChange: (v: string) => void;
  plansLoading: boolean;
  plansError: boolean;
}) {
  const safePlans = plans ?? [];

  const totalTasks = useMemo(
    () => safePlans.reduce((s, p) => s + (p?.progress?.total ?? 0), 0),
    [safePlans],
  );
  const completedTasks = useMemo(
    () => safePlans.reduce((s, p) => s + (p?.progress?.completed ?? 0), 0),
    [safePlans],
  );
  const pipelinePct = totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0;

  const filteredPlans = useMemo(() => {
    const q = (filter ?? '').trim().toLowerCase();
    if (!q) return safePlans;
    return safePlans.filter(
      (p) =>
        (p?.name ?? '').toLowerCase().includes(q) ||
        (p?.id ?? '').toLowerCase().includes(q),
    );
  }, [safePlans, filter]);

  return (
    <div className="flex flex-col h-full min-h-0 border-r border-r-[var(--text-ghost)]">
      {/* Pipeline header */}
      <div className="shrink-0 px-3 py-2 border-b border-b-[var(--text-ghost)] bg-[var(--bg-raised)]">
        <div className="flex items-center gap-2 mb-1">
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] tracking-widest">
            {'\u25C8'} Pipeline
          </span>
          <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tracking-tighter">
            [{miniBar(pipelinePct, 10)}]
          </span>
          <span className="font-[var(--font-mono)] text-[10px] tabular-nums text-[var(--text-faint)] ml-auto">
            {completedTasks}/{totalTasks}
          </span>
        </div>
        <input
          type="text"
          value={filter}
          onChange={(e) => onFilterChange(e.target.value)}
          placeholder="filter plans\u2026"
          className={clsx(
            'w-full bg-[var(--bg-secondary)] border border-[var(--text-ghost)]',
            'font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-strong)]',
            'px-2 py-1 leading-none',
            'placeholder:text-[var(--text-ghost)]',
            'outline-none focus:border-[var(--rose-dim)]',
            'transition-[border-color] duration-[80ms]',
          )}
          aria-label="Filter plans"
        />
      </div>

      {/* Plan list */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden">
        {plansLoading && safePlans.length === 0 ? (
          <div className="flex items-center gap-2 px-3 py-4">
            <Spinner size="sm" />
            <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]">
              Loading plans\u2026
            </span>
          </div>
        ) : plansError && safePlans.length === 0 ? (
          <div className="flex flex-col px-3 py-4 gap-1.5">
            <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--accent-error)]">
              Failed to load plans.
            </span>
            <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-faint)]">
              Is roko serve running on :6677?
            </span>
          </div>
        ) : filteredPlans.length === 0 ? (
          <div className="px-3 py-4 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]">
            {filter
              ? 'No matching plans'
              : 'No plans yet \u2014 run: roko plan run plans/'}
          </div>
        ) : (
          filteredPlans.map((plan) => {
            const tasks = tasksByPlan.get(plan.id) ?? [];
            const waves = buildWaveGroups(tasks);
            const pct = progressPct(plan);
            const isSelected = plan.id === selectedPlanId;

            return (
              <div key={plan.id}>
                <button
                  type="button"
                  onClick={() => onSelectPlan(plan.id)}
                  className={clsx(
                    'w-full flex items-center gap-2 px-3 py-1.5',
                    'font-[var(--font-mono)] text-[var(--text-xs)]',
                    'border-b border-b-[var(--text-ghost)]',
                    isSelected
                      ? 'border-l-2 border-l-[var(--rose)] bg-[var(--bg-highlight)] text-[var(--text-strong)]'
                      : 'border-l-2 border-l-transparent text-[var(--text-muted)] hover:bg-[var(--bg-highlight)]',
                    'transition-[background-color,color,border-color] duration-[80ms]',
                    'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
                  )}
                >
                  <span
                    className={clsx(
                      'shrink-0 w-2 h-2',
                      plan.status === 'running' && 'bg-[var(--warning)]',
                      plan.status === 'gating' && 'bg-[var(--dream)]',
                      plan.status === 'completed' && 'bg-[var(--sage)]',
                      plan.status === 'failed' && 'bg-[var(--accent-error)]',
                      (plan.status === 'paused' || plan.status === 'pending' || plan.status === 'cancelled') &&
                        'bg-[var(--text-ghost)]',
                    )}
                  />
                  <span className="flex-1 truncate text-left">{plan.name}</span>
                  <span className="shrink-0 tabular-nums text-[var(--text-ghost)] text-[10px]">
                    {pct}%
                  </span>
                </button>

                {isSelected &&
                  Array.from(waves.entries())
                    .sort(([a], [b]) => a - b)
                    .map(([waveIdx, waveTasks]) => (
                      <WaveRow
                        key={waveIdx}
                        waveIndex={waveIdx}
                        tasks={waveTasks}
                        selected={isSelected}
                        onClick={() => onSelectPlan(plan.id)}
                      />
                    ))}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// SortHeader
// ---------------------------------------------------------------------------

function SortHeader({
  field,
  label,
  current,
  onSort,
}: {
  field: SortField;
  label: string;
  current: SortField;
  onSort: (f: SortField) => void;
}) {
  const active = field === current;
  return (
    <button
      type="button"
      onClick={() => onSort(field)}
      className={clsx(
        'font-[var(--font-mono)] text-[10px] tracking-wider uppercase leading-none',
        'transition-colors duration-[80ms]',
        active ? 'text-[var(--text-strong)]' : 'text-[var(--text-ghost)] hover:text-[var(--text-muted)]',
      )}
    >
      {label}
      {active && ' \u2191'}
    </button>
  );
}

// ---------------------------------------------------------------------------
// PlanActions
// ---------------------------------------------------------------------------

function PlanActions({ plan }: { plan: PlanState }) {
  const router = useRouter();
  const startMut = useStartPlan({
    onSuccess: () => showToast('Plan execution started', 'success'),
    onError: (err: Error) => showToast(`Execute failed: ${err?.message ?? 'unknown'}`, 'error'),
  });
  const pauseMut = usePausePlan({
    onSuccess: () => showToast('Plan paused', 'info'),
    onError: (err: Error) => showToast(`Pause failed: ${err?.message ?? 'unknown'}`, 'error'),
  });
  const resumeMut = useResumePlan({
    onSuccess: () => showToast('Plan resumed', 'success'),
    onError: (err: Error) => showToast(`Resume failed: ${err?.message ?? 'unknown'}`, 'error'),
  });
  const cancelMut = useCancelPlan({
    onSuccess: () => showToast('Plan cancelled', 'info'),
    onError: (err: Error) => showToast(`Cancel failed: ${err?.message ?? 'unknown'}`, 'error'),
  });
  const retryMut = useRetryPlan({
    onSuccess: () => showToast('Plan retry queued', 'success'),
    onError: (err: Error) => showToast(`Retry failed: ${err?.message ?? 'unknown'}`, 'error'),
  });

  const status = plan?.status ?? 'pending';
  const id = plan?.id ?? '';

  return (
    <div className="flex items-center gap-2 flex-wrap">
      {status === 'pending' && (
        <Button size="sm" variant="primary" loading={startMut.isPending} onClick={() => startMut.mutate(id)}>
          Execute
        </Button>
      )}
      {status === 'running' && (
        <Button size="sm" variant="secondary" loading={pauseMut.isPending} onClick={() => pauseMut.mutate(id)}>
          Pause
        </Button>
      )}
      {status === 'paused' && (
        <>
          <Button size="sm" variant="primary" loading={resumeMut.isPending} onClick={() => resumeMut.mutate(id)}>
            Resume
          </Button>
          <Button size="sm" variant="danger" loading={cancelMut.isPending} onClick={() => cancelMut.mutate(id)}>
            Cancel
          </Button>
        </>
      )}
      {status === 'failed' && (
        <>
          <Button size="sm" variant="secondary" loading={retryMut.isPending} onClick={() => retryMut.mutate(id)}>
            Retry Failed
          </Button>
          <Button size="sm" variant="primary" loading={startMut.isPending} onClick={() => startMut.mutate(id)}>
            Execute
          </Button>
        </>
      )}
      {(status === 'running' || status === 'gating') && (
        <Button size="sm" variant="danger" loading={cancelMut.isPending} onClick={() => cancelMut.mutate(id)}>
          Cancel
        </Button>
      )}
      <Button
        size="sm"
        variant="secondary"
        onClick={() => router.push(`/work/editor?plan=${encodeURIComponent(id)}`)}
      >
        View in Editor
      </Button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PlanDetail (right panel)
// ---------------------------------------------------------------------------

function PlanDetail({
  plan,
  tasks,
  tasksLoading,
}: {
  plan: PlanState;
  tasks: TaskState[];
  tasksLoading: boolean;
}) {
  const [sortField, setSortField] = useState<SortField>('status');
  const [expandedTaskId, setExpandedTaskId] = useState<string | null>(null);

  const safeTasks = tasks ?? [];

  const sortedTasks = useMemo(() => sortTasks(safeTasks, sortField), [safeTasks, sortField]);

  const pct = progressPct(plan);
  const bPct = budgetPct(plan);

  const totalCost = useMemo(
    () => safeTasks.reduce((s, t) => s + (t?.costUsd ?? 0), 0),
    [safeTasks],
  );
  const gatePassCount = useMemo(
    () => safeTasks.reduce((s, t) => s + (t?.gateResults ?? []).filter((g) => g?.passed).length, 0),
    [safeTasks],
  );
  const gateFailCount = useMemo(
    () => safeTasks.reduce((s, t) => s + (t?.gateResults ?? []).filter((g) => !g?.passed).length, 0),
    [safeTasks],
  );

  const toggleTaskExpand = useCallback((taskId: string) => {
    setExpandedTaskId((prev) => (prev === taskId ? null : taskId));
  }, []);

  return (
    <div className="flex flex-col h-full min-h-0">
      {/* Plan header */}
      <div className="shrink-0 px-5 py-4 border-b border-b-[var(--text-ghost)] bg-[var(--bg-raised)]">
        <div className="flex items-start gap-3 mb-3">
          <h2 className="flex-1 font-[var(--font-mono)] text-[var(--text-base)] text-[var(--text-strong)] leading-tight">
            {plan?.name ?? 'Untitled Plan'}
          </h2>
          <StatusBadge status={plan?.status ?? 'pending'} />
        </div>

        <div className="mb-2">
          <ProgressBar value={pct} height={4} showLabel />
        </div>

        <div className="flex items-center gap-6 mb-3 flex-wrap">
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] tabular-nums text-[var(--text-faint)]">
            <span className="text-[var(--text-muted)]">{plan?.progress?.completed ?? 0}</span>
            <span className="text-[var(--text-ghost)]">/{plan?.progress?.total ?? 0}</span>
            {' '}tasks
          </span>

          {(plan?.progress?.failed ?? 0) > 0 && (
            <span className="font-[var(--font-mono)] text-[var(--text-xs)] tabular-nums text-[var(--accent-error)]">
              {plan.progress.failed} failed
            </span>
          )}

          <span className="font-[var(--font-mono)] text-[var(--text-xs)] tabular-nums text-[var(--text-faint)]">
            {formatCost(totalCost > 0 ? totalCost : (plan?.costUsd ?? 0))}
            {plan?.budgetUsd != null && plan.budgetUsd > 0 && (
              <span className="text-[var(--text-ghost)]"> / {formatCost(plan.budgetUsd)}</span>
            )}
          </span>

          {gatePassCount + gateFailCount > 0 && (
            <span className="font-[var(--font-mono)] text-[var(--text-xs)] tabular-nums text-[var(--text-faint)]">
              gates:{' '}
              <span className="text-[var(--sage)]">{gatePassCount}{'\u2713'}</span>
              {gateFailCount > 0 && (
                <span className="text-[var(--accent-error)] ml-1">{gateFailCount}{'\u2717'}</span>
              )}
            </span>
          )}
        </div>

        {plan?.budgetUsd != null && plan.budgetUsd > 0 && (
          <div className="mb-3">
            <div className="flex items-center justify-between mb-1">
              <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tracking-wider uppercase">
                Budget
              </span>
              <span className="font-[var(--font-mono)] text-[10px] tabular-nums text-[var(--text-faint)]">
                {bPct}%
              </span>
            </div>
            <ProgressBar value={bPct} variant="cost" height={3} />
          </div>
        )}

        <PlanActions plan={plan} />
      </div>

      {/* Tasks table header */}
      <div className="shrink-0 flex items-center gap-4 px-5 py-2 border-b border-b-[var(--text-ghost)] bg-[var(--bg-raised)]">
        <SortHeader field="status" label="Status" current={sortField} onSort={setSortField} />
        <SortHeader field="name" label="Task" current={sortField} onSort={setSortField} />
        <div className="flex-1" />
        <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tracking-wider uppercase">
          Agent
        </span>
        <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tracking-wider uppercase">
          Model
        </span>
        <SortHeader field="cost" label="Cost" current={sortField} onSort={setSortField} />
      </div>

      {/* Tasks list */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden">
        {tasksLoading ? (
          <div className="flex items-center gap-2 px-5 py-4">
            <Spinner size="sm" />
            <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]">
              Loading tasks\u2026
            </span>
          </div>
        ) : sortedTasks.length === 0 ? (
          <div className="px-5 py-4 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]">
            No tasks for this plan.
          </div>
        ) : (
          sortedTasks.map((task) => {
            const isExpanded = expandedTaskId === task.id;
            const gates = task?.gateResults ?? [];
            const deps = task?.dependsOn ?? [];
            const hasDetail = Boolean(
              task?.description ||
                (task?.files ?? []).length > 0 ||
                deps.length > 0 ||
                gates.length > 0 ||
                task?.tier,
            );
            return (
              <div key={task.id}>
                <div
                  className={clsx(
                    'flex items-center border-b border-b-[var(--text-ghost)]',
                    hasDetail && 'cursor-pointer hover:bg-[var(--bg-highlight)]',
                    isExpanded && 'bg-[var(--bg-highlight)]',
                    'transition-[background-color] duration-[80ms]',
                  )}
                  role={hasDetail ? 'button' : undefined}
                  tabIndex={hasDetail ? 0 : undefined}
                  onClick={hasDetail ? () => toggleTaskExpand(task.id) : undefined}
                  onKeyDown={
                    hasDetail
                      ? (e) => {
                          if (e.key === 'Enter' || e.key === ' ') toggleTaskExpand(task.id);
                        }
                      : undefined
                  }
                  aria-expanded={hasDetail ? isExpanded : undefined}
                >
                  <div className="flex-1 min-w-0">
                    <TaskRow task={task} compact />
                  </div>

                  <div className="shrink-0 w-24 px-2">
                    <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] truncate block">
                      {task?.agentName ?? '\u2014'}
                    </span>
                  </div>

                  <div className="shrink-0 w-24 px-2">
                    <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] truncate block">
                      {abbreviateModel(task?.model ?? null)}
                    </span>
                  </div>

                  <div className="shrink-0 w-16 px-2 text-right">
                    <span className="font-[var(--font-mono)] text-[10px] tabular-nums text-[var(--text-faint)]">
                      {formatCost(task?.costUsd ?? 0)}
                    </span>
                  </div>

                  {hasDetail && (
                    <div className="shrink-0 w-5 pr-2 text-right">
                      <span className="font-[var(--font-mono)] text-[8px] text-[var(--text-ghost)]">
                        {isExpanded ? '\u25BC' : '\u25B6'}
                      </span>
                    </div>
                  )}
                </div>

                {isExpanded && <TaskDetailPanel task={task} />}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// NoSelection
// ---------------------------------------------------------------------------

function NoSelection() {
  return (
    <div className="flex flex-col items-center justify-center h-full gap-2">
      <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-ghost)]">
        Select a plan
      </span>
      <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]">
        Click a plan in the wave tree to inspect it
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PlanDetailLoader
// ---------------------------------------------------------------------------

function PlanDetailLoader({ planId, plan }: { planId: string; plan: PlanState }) {
  // Stable reference: read the whole record, derive filtered list via useMemo
  const allTasks = useDashboardStore((s) => s.tasks);
  const storeTasks = useMemo(
    () =>
      Object.values(allTasks ?? {}).filter(
        (t) => t?.planId === planId,
      ),
    [allTasks, planId],
  );

  const {
    data: fetchedRaw,
    isLoading,
    isError,
    error,
  } = usePlanTasks(planId, {
    enabled: storeTasks.length === 0,
  });

  // Map through the safe extraction layer
  const fetchedTasks = useMemo(
    () => extractTasks(fetchedRaw, planId),
    [fetchedRaw, planId],
  );

  const tasks = storeTasks.length > 0 ? storeTasks : fetchedTasks;

  if (isError && tasks.length === 0) {
    return (
      <div className="flex flex-col h-full">
        {/* Still show the plan header even if tasks fail */}
        <div className="shrink-0 px-5 py-4 border-b border-b-[var(--text-ghost)] bg-[var(--bg-raised)]">
          <div className="flex items-start gap-3 mb-3">
            <h2 className="flex-1 font-[var(--font-mono)] text-[var(--text-base)] text-[var(--text-strong)] leading-tight">
              {plan?.name ?? 'Untitled Plan'}
            </h2>
            <StatusBadge status={plan?.status ?? 'pending'} />
          </div>
        </div>
        <div className="flex-1 flex flex-col items-center justify-center gap-2 px-5">
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--accent-error)]">
            Failed to load tasks
          </span>
          <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-faint)] text-center max-w-sm">
            {error instanceof Error ? error.message : 'Unknown error'}
          </span>
        </div>
      </div>
    );
  }

  return (
    <PlanDetail
      plan={plan}
      tasks={tasks}
      tasksLoading={isLoading && storeTasks.length === 0}
    />
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function WorkPlansPage() {
  const [selectedPlanId, setSelectedPlanId] = useState<string | null>(null);
  const [filter, setFilter] = useState('');

  // SSE-driven store state
  const storePlans = useDashboardStore((s) => s.plans);
  const storeTasks = useDashboardStore((s) => s.tasks);

  // REST hydration
  const {
    data: fetchedPlans,
    isLoading: plansLoading,
    isError: plansError,
  } = usePlans();

  // Merge: SSE store takes precedence, REST fills gaps
  const plans = useMemo(() => {
    const restMap: Record<string, PlanState> = {};
    for (const item of (fetchedPlans ?? []) as unknown[]) {
      const mapped = mapRawPlan(item);
      if (mapped) restMap[mapped.id] = mapped;
    }
    const merged: Record<string, PlanState> = { ...restMap, ...(storePlans ?? {}) };
    return Object.values(merged).sort((a, b) => {
      const ta = a?.startedAt ? new Date(a.startedAt).getTime() : 0;
      const tb = b?.startedAt ? new Date(b.startedAt).getTime() : 0;
      if (tb !== ta) return tb - ta;
      return (a?.name ?? '').localeCompare(b?.name ?? '');
    });
  }, [storePlans, fetchedPlans]);

  // Tasks grouped by plan
  const tasksByPlan = useMemo(() => {
    const map = new Map<string, TaskState[]>();
    for (const task of Object.values(storeTasks ?? {})) {
      if (!task?.planId) continue;
      const bucket = map.get(task.planId) ?? [];
      bucket.push(task);
      map.set(task.planId, bucket);
    }
    return map;
  }, [storeTasks]);

  // Selected plan
  const selectedPlan = useMemo(() => {
    if (!selectedPlanId) return null;
    return (
      (storePlans ?? {})[selectedPlanId] ??
      plans.find((p) => p?.id === selectedPlanId) ??
      null
    );
  }, [selectedPlanId, storePlans, plans]);

  // Auto-select first plan
  useEffect(() => {
    if (!selectedPlanId && plans.length > 0 && plans[0]?.id) {
      setSelectedPlanId(plans[0].id);
    }
  }, [plans, selectedPlanId]);

  return (
    <div className="flex h-full min-h-0">
      {/* Left panel: Wave Tree */}
      <div className="shrink-0 overflow-hidden flex flex-col" style={{ width: '31%' }}>
        <WaveTree
          plans={plans}
          selectedPlanId={selectedPlanId}
          onSelectPlan={setSelectedPlanId}
          tasksByPlan={tasksByPlan}
          filter={filter}
          onFilterChange={setFilter}
          plansLoading={plansLoading}
          plansError={plansError}
        />
      </div>

      {/* Right panel: Plan Detail */}
      <div className="flex-1 min-w-0 overflow-hidden flex flex-col">
        {selectedPlan ? (
          <PlanDetailLoader planId={selectedPlan.id} plan={selectedPlan} />
        ) : (
          <NoSelection />
        )}
      </div>
    </div>
  );
}
