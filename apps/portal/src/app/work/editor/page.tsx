'use client';

import React, { useState, useCallback, useId, useEffect } from 'react';
import { clsx } from 'clsx';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/api/client';
import { queryKeys, usePlans } from '@/api/hooks';
import { Button } from '@/components/atoms/Button';
import { Spinner } from '@/components/atoms/Spinner';
import { showToast } from '@/components/atoms/Toast';
import { useDashboardStore } from '@/stores/dashboard';
import type { TaskStatus } from '@/api/types';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

const ROLES = [
  'implementer',
  'researcher',
  'reviewer',
  'scribe',
  'auditor',
] as const;

type Role = typeof ROLES[number];

interface EditorTask {
  /** Client-side stable id for React keys */
  _uid: string;
  id: string;
  title: string;
  role: Role;
  depends: string[];
  files: string;
}

/** Shape returned by POST /api/plans/generate — always async (202 Accepted). */
interface GenerateResponse {
  /** Background operation id. */
  id: string;
  /** Resolved plan_id that was created/updated. */
  plan_id: string;
}

/** Shape returned by GET /api/plans/:id/tasks */
interface PlanTasksResponse {
  plan_id: string;
  task_count: number;
  tasks: Array<{
    id: string;
    /** Server-side canonical field name is "description". */
    description: string;
    depends_on: string[];
    files: string[];
    completed: boolean;
    status: string;
  }>;
}

/** Shape returned by PUT /api/plans/:id/tasks */
interface SaveResponse {
  plan_id: string;
  updated: boolean;
}

/** Shape returned by POST /api/plans/:id/execute (202 Accepted) */
interface ExecuteResponse {
  /** Background run/operation id. */
  id: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let _uid = 0;

function nextUid(): string {
  _uid += 1;
  return `t-${_uid}`;
}

function makeEmptyTask(index: number): EditorTask {
  const n = String(index + 1).padStart(2, '0');
  return {
    _uid: nextUid(),
    id: `T${n}`,
    title: '',
    role: 'implementer',
    depends: [],
    files: '',
  };
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 48)
    || 'my-plan';
}

/**
 * Generate a tasks.toml preview matching the canonical CLI format.
 *
 * The canonical format uses `[[task]]` (singular) with a `[meta]` header, and
 * `title` / `role` / `depends_on` / `files` fields per task.
 */
function generateToml(planId: string, tasks: EditorTask[]): string {
  const lines: string[] = [];
  lines.push('[meta]');
  lines.push(`plan = "${planId}"`);
  lines.push(`total = ${tasks.length}`);
  lines.push('done = 0');
  lines.push('');

  for (const task of tasks) {
    lines.push(`[[task]]`);
    lines.push(`id = "${task.id}"`);
    lines.push(`title = ${JSON.stringify(task.title || '(untitled)')}`);
    lines.push(`role = "${task.role}"`);
    if (task.depends.length > 0) {
      lines.push(
        `depends_on = [${task.depends.map((d) => `"${d}"`).join(', ')}]`,
      );
    }
    if (task.files.trim()) {
      const fileList = task.files
        .split(',')
        .map((f) => f.trim())
        .filter(Boolean)
        .map((f) => `"${f}"`)
        .join(', ');
      if (fileList) lines.push(`files = [${fileList}]`);
    }
    lines.push('');
  }

  return lines.join('\n');
}

/** Map a raw server task (uses `description`) into an editor task. */
function serverTaskToEditorTask(
  // Accept any shape — the server returns flat TaskState objects or wrapped objects
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  raw: any,
  index?: number,
): EditorTask {
  const t = raw as Record<string, unknown>;
  const idx = index ?? 0;
  const rawRole = t.role as string | undefined;
  const role: Role = ROLES.includes(rawRole as Role)
    ? (rawRole as Role)
    : 'implementer';
  const rawDeps = t.depends_on ?? t.dependsOn ?? [];
  const rawFiles = t.files ?? [];
  return {
    _uid: nextUid(),
    id: (t.id as string) || `T${String(idx + 1).padStart(2, '0')}`,
    title: (t.description ?? t.title ?? t.name ?? '') as string,
    role,
    depends: Array.isArray(rawDeps) ? rawDeps : [],
    files: Array.isArray(rawFiles) ? rawFiles.join(', ') : String(rawFiles),
  };
}

// ---------------------------------------------------------------------------
// DependsSelect — multi-select from other task IDs
// ---------------------------------------------------------------------------

interface DependsSelectProps {
  taskUid: string;
  value: string[];
  allTasks: EditorTask[];
  onChange: (ids: string[]) => void;
}

function DependsSelect({ taskUid, value, allTasks, onChange }: DependsSelectProps) {
  // Only offer tasks other than the current one as choices
  const options = allTasks.filter((t) => t._uid !== taskUid && t.id.trim() !== '');

  const toggle = useCallback(
    (id: string) => {
      if (value.includes(id)) {
        onChange(value.filter((v) => v !== id));
      } else {
        onChange([...value, id]);
      }
    },
    [value, onChange],
  );

  if (options.length === 0) {
    return (
      <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] px-2">
        —
      </span>
    );
  }

  return (
    <div className="flex flex-wrap gap-1 px-1">
      {options.map((t) => {
        const active = value.includes(t.id);
        return (
          <button
            key={t._uid}
            type="button"
            onClick={() => toggle(t.id)}
            className={clsx(
              'font-[var(--font-mono)] text-[10px] leading-none',
              'px-1.5 py-0.5 border',
              'transition-[background-color,color,border-color] duration-[80ms]',
              'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
              active
                ? 'bg-[var(--rose-dim)] border-[var(--rose)] text-[var(--text-strong)]'
                : 'bg-transparent border-[var(--text-ghost)] text-[var(--text-ghost)] hover:border-[var(--border-hover)] hover:text-[var(--text-muted)]',
            )}
          >
            {t.id}
          </button>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// TaskStatusDot — colored indicator for live execution status
// ---------------------------------------------------------------------------

interface TaskStatusDotProps {
  status: TaskStatus | 'ready';
}

function TaskStatusDot({ status }: TaskStatusDotProps) {
  if (status === 'running' || status === 'dispatching' || status === 'gating') {
    return (
      <span
        aria-label={`Status: ${status}`}
        className={clsx(
          'shrink-0 w-3 text-center font-[var(--font-mono)] text-[10px] pt-[7px]',
          'text-[var(--warning)]',
          'animate-pulse',
        )}
      >
        ●
      </span>
    );
  }
  if (status === 'completed') {
    return (
      <span
        aria-label="Status: completed"
        className="shrink-0 w-3 text-center font-[var(--font-mono)] text-[10px] pt-[7px] text-[var(--sage)]"
      >
        ✓
      </span>
    );
  }
  if (status === 'failed') {
    return (
      <span
        aria-label="Status: failed"
        className="shrink-0 w-3 text-center font-[var(--font-mono)] text-[10px] pt-[7px] text-[var(--accent-error)]"
      >
        ✗
      </span>
    );
  }
  // 'ready' / 'pending' / unknown
  return (
    <span
      aria-label="Status: ready"
      className="shrink-0 w-3 text-center text-[var(--text-ghost)] font-[var(--font-mono)] text-[10px] pt-[7px]"
    >
      ○
    </span>
  );
}

// ---------------------------------------------------------------------------
// TaskEditorRow
// ---------------------------------------------------------------------------

interface TaskEditorRowProps {
  task: EditorTask;
  index: number;
  allTasks: EditorTask[];
  onChange: (uid: string, patch: Partial<EditorTask>) => void;
  onDelete: (uid: string) => void;
  /** Live execution status from the SSE store. `undefined` when not executing. */
  liveStatus?: TaskStatus | 'ready';
}

function TaskEditorRow({ task, index, allTasks, onChange, onDelete, liveStatus }: TaskEditorRowProps) {
  const labelId = useId();

  const effectiveStatus: TaskStatus | 'ready' = liveStatus ?? 'ready';
  const isActive = effectiveStatus === 'running' || effectiveStatus === 'dispatching' || effectiveStatus === 'gating';

  return (
    <div
      className={clsx(
        'flex items-start gap-2 px-3 py-2',
        'border-b border-b-[var(--text-ghost)]',
        isActive
          ? 'bg-[var(--bg-highlight)]'
          : 'hover:bg-[var(--bg-highlight)]',
        'transition-[background-color] duration-[80ms]',
      )}
    >
      {/* Status dot */}
      <TaskStatusDot status={effectiveStatus} />

      {/* Task ID */}
      <input
        type="text"
        value={task.id}
        onChange={(e) => onChange(task._uid, { id: e.target.value.toUpperCase().replace(/\s/g, '') })}
        aria-label={`Task ${index + 1} ID`}
        placeholder={`T${String(index + 1).padStart(2, '0')}`}
        className={clsx(
          'shrink-0 w-12 bg-transparent border border-[var(--text-ghost)]',
          'font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-strong)]',
          'px-1.5 py-1 leading-none',
          'placeholder:text-[var(--text-ghost)]',
          'outline-none focus:border-[var(--rose-dim)]',
          'transition-[border-color] duration-[80ms]',
        )}
      />

      {/* Title */}
      <input
        id={labelId}
        type="text"
        value={task.title}
        onChange={(e) => onChange(task._uid, { title: e.target.value })}
        placeholder="Task title…"
        className={clsx(
          'flex-1 min-w-0 bg-transparent border border-[var(--text-ghost)]',
          'font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-strong)]',
          'px-2 py-1 leading-none',
          'placeholder:text-[var(--text-ghost)]',
          'outline-none focus:border-[var(--rose-dim)]',
          'transition-[border-color] duration-[80ms]',
        )}
      />

      {/* Role select */}
      <select
        value={task.role}
        onChange={(e) => onChange(task._uid, { role: e.target.value as Role })}
        aria-label={`Task ${index + 1} role`}
        className={clsx(
          'shrink-0 w-28 bg-[var(--bg-secondary)] border border-[var(--text-ghost)]',
          'font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)]',
          'px-1.5 py-1 leading-none',
          'outline-none focus:border-[var(--rose-dim)]',
          'transition-[border-color] duration-[80ms]',
          'cursor-pointer',
        )}
      >
        {ROLES.map((r) => (
          <option key={r} value={r}>
            {r}
          </option>
        ))}
      </select>

      {/* Depends */}
      <div
        className={clsx(
          'shrink-0 flex items-center min-w-[80px]',
          'border border-[var(--text-ghost)] py-1',
        )}
        aria-label={`Task ${index + 1} dependencies`}
      >
        <DependsSelect
          taskUid={task._uid}
          value={task.depends}
          allTasks={allTasks}
          onChange={(ids) => onChange(task._uid, { depends: ids })}
        />
      </div>

      {/* Files */}
      <input
        type="text"
        value={task.files}
        onChange={(e) => onChange(task._uid, { files: e.target.value })}
        placeholder="src/foo.rs, …"
        aria-label={`Task ${index + 1} files`}
        className={clsx(
          'shrink-0 w-40 bg-transparent border border-[var(--text-ghost)]',
          'font-[var(--font-mono)] text-[10px] text-[var(--text-faint)]',
          'px-1.5 py-1 leading-none',
          'placeholder:text-[var(--text-ghost)]',
          'outline-none focus:border-[var(--rose-dim)]',
          'transition-[border-color] duration-[80ms]',
        )}
      />

      {/* Delete */}
      <button
        type="button"
        onClick={() => onDelete(task._uid)}
        aria-label={`Remove task ${task.id || index + 1}`}
        className={clsx(
          'shrink-0 w-6 h-6 flex items-center justify-center mt-0.5',
          'font-[var(--font-mono)] text-[var(--text-ghost)]',
          'hover:text-[var(--accent-error)]',
          'transition-colors duration-[80ms]',
          'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
        )}
      >
        ×
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Section wrapper
// ---------------------------------------------------------------------------

function Section({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div
      className={clsx(
        'border border-[var(--text-ghost)]',
        'bg-[var(--bg-raised)]',
      )}
    >
      <div
        className={clsx(
          'px-3 py-1.5',
          'border-b border-b-[var(--text-ghost)]',
          'bg-[var(--bg-secondary)]',
        )}
      >
        <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tracking-widest uppercase">
          {label}
        </span>
      </div>
      {children}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function WorkEditorPage() {
  const qc = useQueryClient();

  // ----- State -----

  const [prompt, setPrompt] = useState('');
  const [planId, setPlanId] = useState('my-plan');
  const [tasks, setTasks] = useState<EditorTask[]>([makeEmptyTask(0)]);
  /** Plan id selected from the existing-plan dropdown (empty = new plan). */
  const [selectedExistingPlan, setSelectedExistingPlan] = useState('');

  /**
   * True once Execute is clicked, stays true until the plan transitions to a
   * terminal state (completed/failed) or the user loads a different plan.
   * This is separate from `executeMut.isPending` because the 202 Accepted
   * response settles instantly while actual execution continues asynchronously.
   */
  const [isRunning, setIsRunning] = useState(false);
  /** Whether the Agent Output panel is expanded. */
  const [agentOutputOpen, setAgentOutputOpen] = useState(true);

  // ----- SSE store subscriptions -----

  const storeTasks = useDashboardStore((s) => s.tasks);
  const storePlans = useDashboardStore((s) => s.plans);
  const storeAgentOutput = useDashboardStore((s) => s.agentOutput);
  const storeAgents = useDashboardStore((s) => s.agents);

  // Watch the live plan state so we can auto-clear isRunning on terminal status
  const livePlan = storePlans[planId] ?? null;
  useEffect(() => {
    if (!isRunning) return;
    if (livePlan && (livePlan.status === 'completed' || livePlan.status === 'failed')) {
      setIsRunning(false);
    }
  }, [isRunning, livePlan]);

  // ----- Existing plans list -----

  const { data: plansList, isLoading: plansListLoading } = usePlans();

  // ----- Load tasks for existing plan -----

  const {
    data: existingTasksData,
    isFetching: existingTasksFetching,
    isError: existingTasksError,
  } = useQuery<PlanTasksResponse>({
    queryKey: queryKeys.planTasks(selectedExistingPlan),
    queryFn: () => api.get<PlanTasksResponse>(`/api/plans/${selectedExistingPlan}/tasks`),
    enabled: Boolean(selectedExistingPlan),
    staleTime: 30_000,
  });

  // Populate editor when an existing plan's tasks are fetched.
  // The server returns a flat TaskState[] array (not { tasks: [...] }).
  useEffect(() => {
    if (!existingTasksData) return;
    if (existingTasksError) return;

    const planIdFromServer = selectedExistingPlan || planId;
    setPlanId(planIdFromServer);

    // Handle both shapes: flat array or { tasks: [...] } wrapper
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const rawData = existingTasksData as any;
    const taskArray: unknown[] = Array.isArray(rawData)
      ? rawData
      : Array.isArray(rawData?.tasks)
        ? rawData.tasks
        : [];

    if (taskArray.length > 0) {
      setTasks(taskArray.map((t) => serverTaskToEditorTask(t as Record<string, unknown>)));
      showToast(
        `Loaded ${taskArray.length} task${taskArray.length !== 1 ? 's' : ''} from "${planIdFromServer}"`,
        'success',
      );
    } else {
      setTasks([makeEmptyTask(0)]);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [existingTasksData]);

  // Show error toast when fetching existing plan tasks fails
  useEffect(() => {
    if (existingTasksError && selectedExistingPlan) {
      showToast(`Failed to load tasks for "${selectedExistingPlan}"`, 'error');
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [existingTasksError]);

  // ----- Derived -----

  const toml = generateToml(planId, tasks);
  const isLoading = existingTasksFetching;

  /**
   * Map each editor task to its live status from the SSE store.
   * Tasks are keyed as `planId/taskId` in the store.
   */
  const liveTaskStatuses: Record<string, TaskStatus | 'ready'> = {};
  if (isRunning) {
    for (const t of tasks) {
      const key = `${planId}/${t.id}`;
      const stored = storeTasks[key];
      liveTaskStatuses[t.id] = stored ? stored.status : 'ready';
    }
  }

  /** Counts of completed/failed tasks from the live store. */
  const completedCount = isRunning
    ? Object.values(liveTaskStatuses).filter((s) => s === 'completed').length
    : 0;
  const failedCount = isRunning
    ? Object.values(liveTaskStatuses).filter((s) => s === 'failed').length
    : 0;

  /**
   * The currently active agent for this plan: the first agent whose
   * `currentTask` points to one of our task IDs.
   */
  const activeAgentId: string | null = isRunning
    ? (Object.entries(storeAgents).find(([, agent]) => {
        if (agent.status !== 'active') return false;
        if (!agent.currentTask) return false;
        return tasks.some((t) => t.id === agent.currentTask);
      })?.[0] ?? null)
    : null;

  /** Agent output lines for the active agent, or empty. */
  const agentOutputLines: string[] =
    activeAgentId ? (storeAgentOutput[activeAgentId] ?? []) : [];

  // ----- Handlers -----

  const handlePromptChange = useCallback((v: string) => {
    setPrompt(v);
    // Auto-update planId from the first non-empty words of the prompt when not
    // editing an existing plan
    if (v.trim() && !selectedExistingPlan) {
      setPlanId(slugify(v.split(/\s+/).slice(0, 4).join(' ')));
    }
  }, [selectedExistingPlan]);

  const handleExistingPlanSelect = useCallback((id: string) => {
    setSelectedExistingPlan(id);
    setIsRunning(false);
    if (!id) {
      // Cleared — reset to a blank new plan
      setPlanId('my-plan');
      setTasks([makeEmptyTask(0)]);
      setPrompt('');
    }
  }, []);

  const handleTaskChange = useCallback(
    (uid: string, patch: Partial<EditorTask>) => {
      setTasks((prev) =>
        prev.map((t) => (t._uid === uid ? { ...t, ...patch } : t)),
      );
    },
    [],
  );

  const handleDeleteTask = useCallback((uid: string) => {
    setTasks((prev) => {
      const next = prev.filter((t) => t._uid !== uid);
      return next.length === 0 ? [makeEmptyTask(0)] : next;
    });
  }, []);

  const handleAddTask = useCallback(() => {
    setTasks((prev) => [...prev, makeEmptyTask(prev.length)]);
  }, []);

  // ----- Generate mutation -----
  //
  // POST /api/plans/generate always returns 202 Accepted with { id, plan_id }.
  // Generation runs in the background; tasks are written to disk by the server.
  // After the operation completes we auto-load the tasks via GET /api/plans/:id/tasks.

  const generateMut = useMutation<GenerateResponse, Error, { prompt: string; plan_id: string }>({
    mutationFn: (body) =>
      api.post<GenerateResponse>('/api/plans/generate', body),

    onSuccess: (data) => {
      const newPlanId = data.plan_id || planId;
      if (data.plan_id) {
        setPlanId(data.plan_id);
      }

      // Invalidate the plan list so the selector reflects the new plan
      void qc.invalidateQueries({ queryKey: queryKeys.plans() });

      showToast(
        `Generation started for "${newPlanId}" — loading tasks when ready…`,
        'info',
      );

      // Poll for tasks: try up to 8 times at 2.5 s intervals (20 s total)
      let attempts = 0;
      const maxAttempts = 8;
      const poll = () => {
        attempts += 1;
        api
          .get<PlanTasksResponse>(`/api/plans/${newPlanId}/tasks`)
          .then((res) => {
            if (res.tasks && res.tasks.length > 0) {
              setTasks(res.tasks.map(serverTaskToEditorTask));
              void qc.invalidateQueries({ queryKey: queryKeys.planTasks(newPlanId) });
              showToast(
                `Generated ${res.tasks.length} task${res.tasks.length !== 1 ? 's' : ''} for "${newPlanId}"`,
                'success',
              );
            } else if (attempts < maxAttempts) {
              setTimeout(poll, 2500);
            } else {
              showToast(
                `Generation complete — open "${newPlanId}" from the plan selector to review tasks`,
                'info',
              );
            }
          })
          .catch(() => {
            if (attempts < maxAttempts) {
              setTimeout(poll, 2500);
            }
          });
      };
      setTimeout(poll, 2500);
    },

    onError: (err) => {
      showToast(`Generate failed: ${err.message}`, 'error');
    },
  });

  // ----- Save mutation -----
  //
  // PUT /api/plans/:id/tasks — body is JSON; server converts to TOML internally.
  // The server reads tasks with field name "description", not "title", so we
  // map title → description on the way out.

  const saveMut = useMutation<SaveResponse, Error, { planId: string; tasks: EditorTask[] }>({
    mutationFn: ({ planId: pid, tasks: ts }) => {
      const body = {
        tasks: ts.map((t) => ({
          id: t.id,
          description: t.title,
          role: t.role,
          depends_on: t.depends,
          files: t.files
            .split(',')
            .map((f) => f.trim())
            .filter(Boolean),
        })),
      };
      return api.put<SaveResponse>(`/api/plans/${pid}/tasks`, body);
    },

    onSuccess: (_, { planId: pid }) => {
      void qc.invalidateQueries({ queryKey: queryKeys.plans() });
      void qc.invalidateQueries({ queryKey: queryKeys.plan(pid) });
      void qc.invalidateQueries({ queryKey: queryKeys.planTasks(pid) });
      showToast('Plan saved', 'success');
    },

    onError: (err) => {
      showToast(`Save failed: ${err.message}`, 'error');
    },
  });

  // ----- Execute mutation -----
  //
  // POST /api/plans/:id/execute returns 202 Accepted with { id: run_id }.

  const executeMut = useMutation<ExecuteResponse, Error, string>({
    mutationFn: (pid) =>
      api.post<ExecuteResponse>(`/api/plans/${pid}/execute`),

    onSuccess: (data) => {
      void qc.invalidateQueries({ queryKey: queryKeys.plans() });
      setIsRunning(true);
      setAgentOutputOpen(true);
      showToast(
        `Plan executing${data.id ? ` — run ${data.id.slice(0, 8)}` : ''}`,
        'success',
      );
    },

    onError: (err) => {
      showToast(`Execute failed: ${err.message}`, 'error');
    },
  });

  // ----- Render -----

  const isGenerating = generateMut.isPending;
  const isSaving     = saveMut.isPending;
  const isExecuting  = executeMut.isPending || isRunning;
  const isTasksBusy  = isLoading || isGenerating;

  return (
    <div className="flex flex-col h-full min-h-0 overflow-y-auto overflow-x-hidden">
      {/* Toolbar */}
      <div
        className={clsx(
          'flex items-center justify-between gap-4',
          'px-4 py-2 shrink-0',
          'border-b border-b-[var(--text-ghost)]',
          'bg-[var(--bg-raised)]',
        )}
      >
        <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] tracking-widest uppercase">
          Plan Editor
        </span>

        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            loading={isGenerating}
            disabled={!prompt.trim()}
            onClick={() =>
              generateMut.mutate({ prompt, plan_id: planId })
            }
          >
            Generate
          </Button>

          <Button
            variant="secondary"
            size="sm"
            loading={isSaving}
            disabled={!planId.trim()}
            onClick={() => saveMut.mutate({ planId, tasks })}
          >
            Save
          </Button>

          <Button
            variant="primary"
            size="sm"
            loading={isExecuting}
            disabled={!planId.trim() || tasks.every((t) => !t.title.trim()) || isExecuting}
            onClick={() => executeMut.mutate(planId)}
          >
            {isRunning ? 'Running…' : 'Execute'}
          </Button>
        </div>
      </div>

      {/* Execution progress bar */}
      {isRunning && (
        <div
          className={clsx(
            'shrink-0 px-4 py-2',
            'border-b border-b-[var(--text-ghost)]',
            'bg-[var(--bg-secondary)]',
            'flex items-center gap-3',
          )}
          aria-live="polite"
          aria-label="Execution progress"
        >
          <span className="font-[var(--font-mono)] text-[10px] text-[var(--warning)] tracking-widest uppercase animate-pulse">
            Executing
          </span>

          {/* Text counter */}
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)]">
            {completedCount}/{tasks.length} tasks complete
            {failedCount > 0 && (
              <span className="ml-2 text-[var(--accent-error)]">
                · {failedCount} failed
              </span>
            )}
          </span>

          {/* Bar */}
          <div className="flex-1 h-1 bg-[var(--bg-raised)] border border-[var(--text-ghost)] overflow-hidden">
            <div
              className="h-full bg-[var(--sage)] transition-[width] duration-500"
              style={{ width: tasks.length > 0 ? `${Math.round((completedCount / tasks.length) * 100)}%` : '0%' }}
            />
          </div>

          {/* Percentage */}
          <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tabular-nums">
            {tasks.length > 0 ? Math.round((completedCount / tasks.length) * 100) : 0}%
          </span>
        </div>
      )}

      {/* Body */}
      <div className="flex-1 p-4 flex flex-col gap-4">

        {/* Existing plan selector */}
        <Section label="Load Existing Plan">
          <div className="flex items-center gap-3 p-3">
            {plansListLoading ? (
              <div className="flex items-center gap-2">
                <Spinner size="sm" />
                <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]">
                  Loading plans…
                </span>
              </div>
            ) : (
              <>
                <select
                  value={selectedExistingPlan}
                  onChange={(e) => handleExistingPlanSelect(e.target.value)}
                  aria-label="Select an existing plan to load"
                  className={clsx(
                    'flex-1 min-w-0 bg-[var(--bg-secondary)] border border-[var(--text-ghost)]',
                    'font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)]',
                    'px-2 py-1 leading-none',
                    'outline-none focus:border-[var(--rose-dim)]',
                    'transition-[border-color] duration-[80ms]',
                    'cursor-pointer',
                  )}
                >
                  <option value="">— new plan —</option>
                  {(plansList ?? []).map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.id}
                      {p.title && p.title !== p.id ? ` — ${p.title}` : ''}
                      {` (${p.completed_task_count ?? 0}/${p.task_count ?? 0})`}
                    </option>
                  ))}
                </select>

                {existingTasksFetching && (
                  <Spinner size="sm" />
                )}
              </>
            )}
          </div>
        </Section>

        {/* Plan ID row */}
        <div className="flex items-center gap-3">
          <label
            htmlFor="plan-id"
            className="shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] tracking-wide"
          >
            Plan ID
          </label>
          <input
            id="plan-id"
            type="text"
            value={planId}
            onChange={(e) => setPlanId(slugify(e.target.value) || e.target.value)}
            placeholder="my-plan"
            className={clsx(
              'w-72 bg-[var(--bg-secondary)] border border-[var(--text-ghost)]',
              'font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-strong)]',
              'px-2 py-1 leading-none',
              'placeholder:text-[var(--text-ghost)]',
              'outline-none focus:border-[var(--rose-dim)]',
              'transition-[border-color] duration-[80ms]',
            )}
          />
        </div>

        {/* Prompt */}
        <Section label="Prompt">
          <div className="p-3">
            <textarea
              value={prompt}
              onChange={(e) => handlePromptChange(e.target.value)}
              placeholder="Describe what to build…"
              rows={4}
              aria-label="Plan generation prompt"
              className={clsx(
                'w-full bg-[var(--bg-secondary)] border border-[var(--text-ghost)]',
                'font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-strong)]',
                'px-2 py-2 leading-relaxed resize-y',
                'placeholder:text-[var(--text-ghost)]',
                'outline-none focus:border-[var(--rose-dim)]',
                'transition-[border-color] duration-[80ms]',
              )}
            />
          </div>
        </Section>

        {/* Tasks */}
        <Section label="Tasks">
          {/* Column header */}
          <div
            className={clsx(
              'flex items-center gap-2 px-3 py-1.5',
              'border-b border-b-[var(--text-ghost)]',
              'bg-[var(--bg-secondary)]',
            )}
          >
            {/* dot */}
            <span className="shrink-0 w-3" />
            <span className="shrink-0 w-12 font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tracking-widest uppercase">
              ID
            </span>
            <span className="flex-1 font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tracking-widest uppercase">
              Title
            </span>
            <span className="shrink-0 w-28 font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tracking-widest uppercase">
              Role
            </span>
            <span className="shrink-0 w-auto min-w-[80px] font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tracking-widest uppercase">
              Depends
            </span>
            <span className="shrink-0 w-40 font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tracking-widest uppercase">
              Files
            </span>
            <span className="shrink-0 w-6" />
          </div>

          {/* Rows */}
          {isTasksBusy ? (
            <div className="flex items-center justify-center gap-2 px-3 py-6">
              <Spinner size="sm" />
              <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]">
                {isGenerating ? 'Generating tasks…' : 'Loading tasks…'}
              </span>
            </div>
          ) : (
            <>
              {tasks.map((task, i) => (
                <TaskEditorRow
                  key={task._uid}
                  task={task}
                  index={i}
                  allTasks={tasks}
                  onChange={handleTaskChange}
                  onDelete={handleDeleteTask}
                  liveStatus={isRunning ? (liveTaskStatuses[task.id] ?? 'ready') : undefined}
                />
              ))}

              <div className="px-3 py-2">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleAddTask}
                >
                  + Add Task
                </Button>
              </div>
            </>
          )}
        </Section>

        {/* Agent Output — only shown while executing */}
        {isRunning && (
          <div
            className={clsx(
              'border border-[var(--text-ghost)]',
              'bg-[var(--bg-raised)]',
            )}
          >
            {/* Header / toggle */}
            <button
              type="button"
              onClick={() => setAgentOutputOpen((v) => !v)}
              className={clsx(
                'w-full flex items-center justify-between gap-2',
                'px-3 py-1.5',
                'border-b border-b-[var(--text-ghost)]',
                'bg-[var(--bg-secondary)]',
                'cursor-pointer hover:bg-[var(--bg-highlight)]',
                'transition-[background-color] duration-[80ms]',
                'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-[var(--rose-glow)]',
              )}
              aria-expanded={agentOutputOpen}
            >
              <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] tracking-widest uppercase">
                Agent Output
                {activeAgentId && (
                  <span className="ml-2 text-[var(--warning)] normal-case tracking-normal">
                    {storeAgents[activeAgentId]?.name ?? activeAgentId}
                  </span>
                )}
              </span>
              <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)]">
                {agentOutputOpen ? '▲' : '▼'}
              </span>
            </button>

            {agentOutputOpen && (
              <div className="relative">
                {agentOutputLines.length === 0 ? (
                  <p className="px-3 py-4 font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] italic">
                    {activeAgentId
                      ? 'Waiting for output…'
                      : 'No active agent yet — output will appear here once a task starts.'}
                  </p>
                ) : (
                  <pre
                    aria-label="Agent output"
                    aria-live="polite"
                    className={clsx(
                      'px-3 py-3 max-h-64 overflow-y-auto',
                      'font-[var(--font-mono)] text-[10px] text-[var(--text-faint)] leading-relaxed',
                      'whitespace-pre-wrap break-all',
                    )}
                  >
                    {agentOutputLines.join('\n')}
                  </pre>
                )}
              </div>
            )}
          </div>
        )}

        {/* TOML Preview */}
        <Section label="TOML Preview">
          <div className="relative">
            <pre
              aria-label="Generated TOML"
              className={clsx(
                'px-3 py-3',
                'font-[var(--font-mono)] text-[10px] text-[var(--text-faint)] leading-relaxed',
                'whitespace-pre overflow-x-auto',
                'min-h-[6rem]',
                // Subtle fade at bottom
                'after:absolute after:inset-x-0 after:bottom-0 after:h-6',
                'after:bg-gradient-to-t after:from-[var(--bg-raised)] after:to-transparent',
                'after:pointer-events-none',
              )}
            >
              {toml}
            </pre>

            {/* Copy button */}
            <button
              type="button"
              onClick={() => {
                void navigator.clipboard.writeText(toml);
                showToast('TOML copied to clipboard', 'info');
              }}
              aria-label="Copy TOML to clipboard"
              className={clsx(
                'absolute top-2 right-2',
                'font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)]',
                'border border-[var(--text-ghost)] px-1.5 py-0.5',
                'hover:text-[var(--text-muted)] hover:border-[var(--border-hover)]',
                'transition-[color,border-color] duration-[80ms]',
                'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
              )}
            >
              copy
            </button>
          </div>
        </Section>

        {/* Spacer at bottom so the last section doesn't butt up against the edge */}
        <div className="h-4 shrink-0" />
      </div>
    </div>
  );
}
