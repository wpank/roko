'use client';

import React, { useState, useMemo } from 'react';
import { clsx } from 'clsx';
import { useDashboardStore } from '@/stores/dashboard';
import { Badge } from '@/components/atoms/Badge';
import { Button } from '@/components/atoms/Button';
import { ProgressBar } from '@/components/atoms/ProgressBar';
import { TaskRow } from '@/components/molecules/TaskRow';
import type { PlanState, TaskState, TaskStatus } from '@/api/types';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACTIVE_STATUSES = new Set<PlanState['status']>(['running', 'gating', 'paused']);

const TASK_COLUMNS: { key: TaskStatus; label: string }[] = [
  { key: 'pending',     label: 'Pending' },
  { key: 'dispatching', label: 'Dispatching' },
  { key: 'running',     label: 'Running' },
  { key: 'gating',      label: 'Gating' },
  { key: 'completed',   label: 'Done / Failed' },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function progressPct(plan: PlanState): number {
  if (plan.progress.total === 0) return 0;
  return Math.round((plan.progress.completed / plan.progress.total) * 100);
}

function formatElapsed(startedAt: string | null): string {
  if (!startedAt) return '—';
  const ms = Date.now() - new Date(startedAt).getTime();
  if (ms < 60_000) return `${Math.floor(ms / 1000)}s`;
  if (ms < 3_600_000) {
    const m = Math.floor(ms / 60_000);
    const s = Math.floor((ms % 60_000) / 1000);
    return `${m}m${s.toString().padStart(2, '0')}s`;
  }
  const h = Math.floor(ms / 3_600_000);
  const m = Math.floor((ms % 3_600_000) / 60_000);
  return `${h}h${m.toString().padStart(2, '0')}m`;
}

function taskElapsed(task: TaskState): string {
  return formatElapsed(task.startedAt);
}

// Gate rung LEDs: show up to 7 rungs from the task's gate results.
function GateRungs({ task }: { task: TaskState }) {
  const rungs = Array.from({ length: 7 }, (_, i) => {
    const result = task.gateResults.find((r) => r.rung === i);
    if (!result) {
      return { state: 'empty' as const, rung: i };
    }
    return { state: result.passed ? 'pass' : ('fail' as const), rung: i };
  });

  return (
    <div className="flex items-center gap-[3px]" aria-label="Gate rungs">
      {rungs.map(({ state, rung }) => (
        <span
          key={rung}
          title={`Rung ${rung}: ${state}`}
          className={clsx(
            'inline-block w-[6px] h-[6px] shrink-0',
            state === 'pass'  && 'bg-[var(--sage)]',
            state === 'fail'  && 'bg-[var(--accent-error)]',
            state === 'empty' && 'bg-[var(--text-ghost)]',
          )}
        />
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Timeline mode
// ---------------------------------------------------------------------------

function PlanStatusBadge({ status }: { status: PlanState['status'] }) {
  const variant = (
    status === 'running'   ? 'warning' :
    status === 'gating'    ? 'dream' :
    status === 'completed' ? 'success' :
    status === 'failed'    ? 'error' :
    status === 'paused'    ? 'default' :
    'default'
  ) as 'warning' | 'dream' | 'success' | 'error' | 'default';

  return (
    <Badge variant={variant}>
      {(status ?? 'unknown').toUpperCase()}
    </Badge>
  );
}

function TimelinePlanCard({
  plan,
  tasks,
}: {
  plan: PlanState;
  tasks: TaskState[];
}) {
  const [expanded, setExpanded] = useState(false);
  const pct = progressPct(plan);
  const agentCount = tasks.filter(
    (t) => t.status === 'running' || t.status === 'dispatching'
  ).length;
  const failedCount = plan.progress.failed;

  return (
    <div
      className={clsx(
        'border border-[var(--text-ghost)] bg-[var(--bg-raised)]',
        'transition-[border-color] duration-[80ms] ease-[var(--ease-out)]',
        'hover:border-[var(--border-hover)]',
      )}
    >
      {/* Card header */}
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        aria-expanded={expanded}
        className={clsx(
          'w-full flex flex-col gap-2 px-4 py-3 text-left',
          'cursor-pointer',
          'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
        )}
      >
        {/* Row 1: expand chevron + plan name + status badge */}
        <div className="flex items-center gap-2 min-w-0">
          <span
            className={clsx(
              'shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]',
              'transition-transform duration-[80ms]',
              expanded ? 'rotate-90' : 'rotate-0',
            )}
            aria-hidden="true"
          >
            ▶
          </span>

          <span className="flex-1 truncate font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)] tracking-wide leading-none">
            {plan.name}
          </span>

          <PlanStatusBadge status={plan.status} />
        </div>

        {/* Row 2: progress bar */}
        <div className="pl-4">
          <ProgressBar
            value={pct}
            variant={failedCount > 0 ? 'cost' : 'default'}
            height={3}
            showLabel
          />
        </div>

        {/* Row 3: meta row */}
        <div className="pl-4 flex items-center gap-4">
          {/* Tasks */}
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] tabular-nums text-[var(--text-faint)]">
            <span className="text-[var(--text-muted)]">{plan.progress.completed}</span>
            <span className="text-[var(--text-ghost)]">/{plan.progress.total}</span>
          </span>

          {/* Active agents */}
          {agentCount > 0 && (
            <span className="font-[var(--font-mono)] text-[var(--text-xs)] tabular-nums text-[var(--text-faint)]">
              <span className="text-[var(--rose)]">{agentCount}</span>
              {' '}agent{agentCount !== 1 ? 's' : ''}
            </span>
          )}

          {/* Failures */}
          {failedCount > 0 && (
            <span className="font-[var(--font-mono)] text-[var(--text-xs)] tabular-nums text-[var(--accent-error)]">
              {failedCount}✗
            </span>
          )}

          {/* Elapsed */}
          <span className="ml-auto font-[var(--font-mono)] text-[var(--text-xs)] tabular-nums text-[var(--text-faint)]">
            {formatElapsed(plan.startedAt)}
          </span>
        </div>
      </button>

      {/* Expandable task list */}
      {expanded && (
        <div
          className={clsx(
            'border-t border-t-[var(--text-ghost)]',
            'bg-[var(--bg-secondary)]',
          )}
        >
          {tasks.length === 0 ? (
            <div className="px-4 py-3 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]">
              No tasks yet.
            </div>
          ) : (
            tasks.map((task) => (
              <TaskRow key={task.id} task={task} compact />
            ))
          )}
        </div>
      )}
    </div>
  );
}

function TimelineView({ plans, tasksByPlan }: {
  plans: PlanState[];
  tasksByPlan: Map<string, TaskState[]>;
}) {
  if (plans.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-2">
        <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-ghost)]">
          No active plans
        </span>
        <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]">
          Run a plan to see it here
        </span>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2 p-4">
      {plans.map((plan) => (
        <TimelinePlanCard
          key={plan.id}
          plan={plan}
          tasks={tasksByPlan.get(plan.id) ?? []}
        />
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Board mode (Kanban)
// ---------------------------------------------------------------------------

// Map 'completed' and 'failed' into the same column for the board.
function boardColumn(status: TaskStatus): TaskStatus {
  if (status === 'failed') return 'completed';
  return status;
}

function TaskBoardCard({ task }: { task: TaskState }) {
  const isRunning = task.status === 'running';
  const isFailed  = task.status === 'failed';
  const isGating  = task.status === 'gating';
  const isDone    = task.status === 'completed';

  return (
    <div
      className={clsx(
        'flex flex-col gap-1.5 px-2.5 py-2',
        'border',
        'transition-[border-color,background-color] duration-[80ms] ease-[var(--ease-out)]',
        isFailed  && 'border-[var(--accent-error)] bg-[var(--bg-secondary)]',
        isRunning && 'border-[var(--rose-dim)] bg-[var(--bg-secondary)]',
        isGating  && 'border-[var(--dream)] bg-[var(--bg-secondary)]',
        isDone    && 'border-[var(--sage)] bg-[var(--bg-secondary)] opacity-60',
        !isFailed && !isRunning && !isGating && !isDone &&
          'border-[var(--text-ghost)] bg-[var(--bg-secondary)]',
      )}
    >
      {/* Row 1: task id + wave badge */}
      <div className="flex items-center justify-between gap-1 min-w-0">
        <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] leading-none truncate">
          {task.id.slice(0, 8)}
        </span>
        {task.wave > 0 && (
          <span className="shrink-0 font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] leading-none border border-[var(--text-ghost)] px-1">
            W{task.wave}
          </span>
        )}
      </div>

      {/* Row 2: task title (2-line truncated) */}
      <p
        className={clsx(
          'font-[var(--font-mono)] text-[var(--text-xs)] leading-tight',
          isFailed  ? 'text-[var(--accent-error)]' :
          isRunning ? 'text-[var(--text-strong)]'  :
          isGating  ? 'text-[var(--dream-bright)]' :
          isDone    ? 'text-[var(--sage)]'          :
                      'text-[var(--text-muted)]',
          'overflow-hidden',
          '[display:-webkit-box] [-webkit-box-orient:vertical] [-webkit-line-clamp:2]',
        )}
        title={task.name}
      >
        {task.name}
      </p>

      {/* Row 3: agent role pill + elapsed + gate rungs */}
      <div className="flex items-center gap-1.5 min-w-0">
        {task.agentName && (
          <span className="shrink-0 font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] border border-[var(--text-ghost)] px-1 leading-none truncate max-w-[72px]">
            {task.agentName}
          </span>
        )}

        <span className="ml-auto shrink-0 font-[var(--font-mono)] text-[10px] tabular-nums text-[var(--text-faint)] leading-none">
          {taskElapsed(task)}
        </span>

        {task.gateResults.length > 0 && (
          <GateRungs task={task} />
        )}
      </div>
    </div>
  );
}

function BoardColumn({
  col,
  tasks,
}: {
  col: { key: TaskStatus; label: string };
  tasks: TaskState[];
}) {
  return (
    <div className="flex flex-col min-w-0 w-0 flex-1">
      {/* Column header */}
      <div
        className={clsx(
          'flex items-center justify-between gap-2',
          'px-3 py-2 shrink-0',
          'border-b border-b-[var(--text-ghost)]',
          'bg-[var(--bg-raised)]',
        )}
      >
        <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] tracking-widest uppercase">
          {col.label}
        </span>
        <span className="font-[var(--font-mono)] text-[var(--text-xs)] tabular-nums text-[var(--text-faint)]">
          {tasks.length}
        </span>
      </div>

      {/* Cards */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden p-2 flex flex-col gap-1.5">
        {tasks.map((task) => (
          <TaskBoardCard key={`${task.planId}/${task.id}`} task={task} />
        ))}
        {tasks.length === 0 && (
          <div className="flex items-center justify-center h-16">
            <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)]">
              empty
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

function BoardView({ tasks }: { tasks: TaskState[] }) {
  // Group tasks by their effective column
  const byColumn = useMemo(() => {
    const map = new Map<TaskStatus, TaskState[]>(
      TASK_COLUMNS.map((c) => [c.key, []])
    );
    for (const task of tasks) {
      const col = boardColumn(task.status);
      const bucket = map.get(col);
      if (bucket) bucket.push(task);
    }
    return map;
  }, [tasks]);

  return (
    <div className="flex h-full min-h-0 gap-px bg-[var(--text-ghost)]">
      {TASK_COLUMNS.map((col) => (
        <BoardColumn
          key={col.key}
          col={col}
          tasks={byColumn.get(col.key) ?? []}
        />
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

type ViewMode = 'timeline' | 'board';

export default function WorkActivePage() {
  const [viewMode, setViewMode] = useState<ViewMode>('timeline');

  const allPlans = useDashboardStore((s) => s.plans);
  const allTasks = useDashboardStore((s) => s.tasks);

  // Filter to active plans only
  const activePlans = useMemo(
    () =>
      Object.values(allPlans)
        .filter((p) => ACTIVE_STATUSES.has(p.status))
        .sort((a, b) => {
          // Running first, then gating, then paused
          const order = { running: 0, gating: 1, paused: 2 } as const;
          const oa = order[a.status as keyof typeof order] ?? 3;
          const ob = order[b.status as keyof typeof order] ?? 3;
          return oa - ob;
        }),
    [allPlans]
  );

  // All tasks belonging to active plans
  const activeTasks = useMemo(
    () =>
      Object.values(allTasks).filter((t) =>
        ACTIVE_STATUSES.has(allPlans[t.planId]?.status ?? 'pending')
      ),
    [allTasks, allPlans]
  );

  // Map planId → tasks[] for the Timeline view
  const tasksByPlan = useMemo(() => {
    const map = new Map<string, TaskState[]>();
    for (const task of activeTasks) {
      const bucket = map.get(task.planId) ?? [];
      bucket.push(task);
      map.set(task.planId, bucket);
    }
    return map;
  }, [activeTasks]);

  return (
    <div className="flex flex-col h-full min-h-0">
      {/* Toolbar */}
      <div
        className={clsx(
          'flex items-center justify-between',
          'px-4 py-2 shrink-0',
          'border-b border-b-[var(--text-ghost)]',
          'bg-[var(--bg-raised)]',
        )}
      >
        <div className="flex items-center gap-3">
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] tracking-widest uppercase">
            Active
          </span>
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] tabular-nums text-[var(--text-faint)]">
            {activePlans.length} plan{activePlans.length !== 1 ? 's' : ''}
          </span>
        </div>

        {/* View toggle */}
        <div className="flex items-center gap-1">
          <Button
            variant={viewMode === 'timeline' ? 'secondary' : 'ghost'}
            size="sm"
            onClick={() => setViewMode('timeline')}
            aria-pressed={viewMode === 'timeline'}
          >
            Timeline
          </Button>
          <Button
            variant={viewMode === 'board' ? 'secondary' : 'ghost'}
            size="sm"
            onClick={() => setViewMode('board')}
            aria-pressed={viewMode === 'board'}
          >
            Board
          </Button>
        </div>
      </div>

      {/* Content area */}
      <div className="flex-1 min-h-0 overflow-hidden">
        {viewMode === 'timeline' ? (
          <div className="h-full overflow-y-auto overflow-x-hidden">
            <TimelineView plans={activePlans} tasksByPlan={tasksByPlan} />
          </div>
        ) : (
          <div className="h-full overflow-hidden flex">
            <BoardView tasks={activeTasks} />
          </div>
        )}
      </div>
    </div>
  );
}
