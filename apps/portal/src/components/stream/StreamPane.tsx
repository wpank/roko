'use client';

import { useEffect, useMemo, useRef, useState } from 'react';
import { usePlanTasks, useWorkspace } from '@/api/queries';
import { useDashboardStore } from '@/stores/dashboard';
import { buildTaskRows, focusTaskId } from '@/lib/taskRows';
import { taskKey } from '@/lib/runState';
import { describeEmpty } from '@/lib/emptyState';
import { Transcript } from './Transcript';
import { Checks } from './Checks';

// ── StreamPane ─────────────────────────────────────────────────────────────────

/**
 * StreamPane — live agent output for the focused task.
 *
 * Props:
 *   planId          — currently selected plan (null = no plan selected)
 *   selectedTaskId  — explicitly selected task (null = auto-focus)
 *   open            — whether the body is visible
 *   onToggle        — called when the collapse control is clicked
 *
 * The pane resolves the focused task via focusTaskId (priority: explicit
 * selection → first active → first failed → last finished) and shows either the
 * Transcript or the Checks view.
 *
 * Auto-switches to Checks when the focused task first gains a failed step; the
 * operator's manual tab pick re-pins the view until the focused task changes.
 *
 * Bar always visible:  <task-id> · transcript │ checks  [▼/▶]
 *   — inactive view carries a count badge (checks = failed steps;
 *     transcript = entries added since last viewed).
 *   — active view never shows its own badge.
 *
 * Body (open only): Transcript or Checks, or describeEmpty when nothing ran.
 */
export function StreamPane({
  planId,
  selectedTaskId,
  open,
  onToggle,
}: {
  planId: string | null;
  selectedTaskId: string | null;
  open: boolean;
  onToggle(): void;
}) {
  // ── Remote / store data ────────────────────────────────────────────────────
  const { data: tasksData } = usePlanTasks(planId ?? undefined);
  const { data: wsData } = useWorkspace();
  const run = useDashboardStore((s) => s.run);
  const connection = useDashboardStore((s) => s.connection);

  // ── Task rows + focused task ────────────────────────────────────────────────
  const rows = useMemo(() => {
    const tasks = tasksData?.tasks;
    if (!tasks?.length || planId === null) return [];
    return buildTaskRows(tasks, run, planId, Date.now()).rows;
  }, [tasksData, run, planId]);

  const focusedId = focusTaskId(rows, selectedTaskId);
  const key = planId !== null && focusedId !== null ? taskKey(planId, focusedId) : null;

  const liveTask = key !== null ? (run.tasks[key] ?? null) : null;
  const transcript = key !== null ? run.transcripts[key] : undefined;
  const checks = liveTask?.checks ?? [];

  // ── Working indicator ──────────────────────────────────────────────────────
  // sinceMs = agent's spawnedAtMs (when available) else task's startedAtMs.
  const agent =
    liveTask?.agentId != null ? (run.agents[liveTask.agentId] ?? null) : null;
  const working =
    liveTask?.status === 'active'
      ? {
          sinceMs:
            agent?.spawnedAtMs ??
            liveTask.startedAtMs ??
            Date.now(),
        }
      : null;

  // ── View state ─────────────────────────────────────────────────────────────
  const [view, setView] = useState<'transcript' | 'checks'>('transcript');
  // True once the operator has manually chosen a view; blocks auto-switch.
  const [operatorPicked, setOperatorPicked] = useState(false);

  // Reset both state flags when the focused task changes.
  const prevFocusedIdRef = useRef<string | null>(null);
  useEffect(() => {
    if (focusedId !== prevFocusedIdRef.current) {
      prevFocusedIdRef.current = focusedId;
      setView('transcript');
      setOperatorPicked(false);
    }
  }, [focusedId]);

  // Auto-switch to checks the first time a failed step appears.
  const failedCount = checks.filter((c) => c.status === 'failed').length;
  const prevFailedRef = useRef(0);
  useEffect(() => {
    if (!operatorPicked && failedCount > prevFailedRef.current && failedCount > 0) {
      setView('checks');
    }
    prevFailedRef.current = failedCount;
  }, [failedCount, operatorPicked]);

  // ── Badge counts ───────────────────────────────────────────────────────────
  // Transcript badge: entries added since transcript view was last active.
  const transcriptEntryCount = transcript?.entries.length ?? 0;
  const lastSeenTranscriptRef = useRef(0);

  // Update last-seen count while transcript view is active.
  useEffect(() => {
    if (view === 'transcript') {
      lastSeenTranscriptRef.current = transcriptEntryCount;
    }
  }, [view, transcriptEntryCount]);

  // Reset last-seen when focused task changes.
  const prevFocusedForBadgeRef = useRef<string | null>(null);
  useEffect(() => {
    if (focusedId !== prevFocusedForBadgeRef.current) {
      prevFocusedForBadgeRef.current = focusedId;
      lastSeenTranscriptRef.current = 0;
    }
  }, [focusedId]);

  // Badges are only non-zero for the inactive view.
  const transcriptBadge =
    view !== 'transcript'
      ? Math.max(0, transcriptEntryCount - lastSeenTranscriptRef.current)
      : 0;
  const checksBadge = view !== 'checks' ? failedCount : 0;

  // ── Operator view pick ─────────────────────────────────────────────────────
  function pickView(v: 'transcript' | 'checks') {
    setView(v);
    setOperatorPicked(true);
  }

  // ── Empty state inputs ─────────────────────────────────────────────────────
  // When focusedId is null nothing ran; show describeEmpty instead of dead tabs.
  const livePlan = planId !== null ? (run.plans[planId] ?? null) : null;
  const planCount =
    run.planSet?.planIds.length ?? (planId !== null ? 1 : 0);
  const tasksActive =
    planId !== null
      ? Object.values(run.tasks).filter(
          (t) => t.planId === planId && t.status === 'active',
        ).length
      : 0;

  const emptyPlan = livePlan
    ? {
        id: planId!,
        phase: livePlan.phase,
        tasksTotal: livePlan.tasksTotal,
        tasksDone: livePlan.tasksDone,
        tasksActive,
        tasksAccepted: livePlan.tasksAccepted,
      }
    : planId !== null
      ? {
          id: planId,
          phase: 'never_run' as const,
          tasksTotal: rows.length,
          tasksDone: 0,
          tasksActive: 0,
          tasksAccepted: 0,
        }
      : undefined;

  const emptySentence = describeEmpty({
    workspace: wsData?.name ?? 'workspace',
    connection,
    planCount,
    plan: emptyPlan,
  });

  // ── Render ─────────────────────────────────────────────────────────────────
  return (
    <div
      data-region="stream"
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        fontSize: 'var(--text-xs)',
        color: 'var(--text-muted)',
      }}
    >
      {/* ── Bar — always visible ─────────────────────────────────────────── */}
      <div
        className="stream-bar"
        style={{
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          gap: '0.375rem',
          padding: '0.25rem 0.75rem',
          borderBottom: open ? '1px solid var(--blur-border)' : 'none',
        }}
      >
        {/* Task ID label */}
        {focusedId !== null && (
          <>
            <span
              className="stream-bar-task"
              style={{ fontFamily: 'monospace', color: 'var(--text-faint)' }}
            >
              {focusedId}
            </span>
            <span aria-hidden="true" style={{ opacity: 0.4, userSelect: 'none' }}>
              ·
            </span>
          </>
        )}

        {/* Transcript tab */}
        <button
          type="button"
          className="stream-tab"
          data-active={view === 'transcript' ? true : undefined}
          onClick={() => pickView('transcript')}
          style={{
            background: 'transparent',
            border: 'none',
            cursor: 'pointer',
            padding: '0 0.125rem',
            fontFamily: 'inherit',
            fontSize: 'inherit',
            color: view === 'transcript' ? 'var(--text-strong)' : 'var(--text-muted)',
          }}
        >
          transcript
          {transcriptBadge > 0 && (
            <span
              className="stream-badge"
              aria-label={`${transcriptBadge} new`}
              style={{
                marginLeft: '0.25em',
                fontSize: '0.75em',
                background: 'var(--state-active)',
                color: '#fff',
                borderRadius: '3px',
                padding: '0 3px',
                lineHeight: '1.4',
              }}
            >
              {transcriptBadge}
            </span>
          )}
        </button>

        <span aria-hidden="true" style={{ opacity: 0.4, userSelect: 'none' }}>
          │
        </span>

        {/* Checks tab */}
        <button
          type="button"
          className="stream-tab"
          data-active={view === 'checks' ? true : undefined}
          onClick={() => pickView('checks')}
          style={{
            background: 'transparent',
            border: 'none',
            cursor: 'pointer',
            padding: '0 0.125rem',
            fontFamily: 'inherit',
            fontSize: 'inherit',
            color: view === 'checks' ? 'var(--text-strong)' : 'var(--text-muted)',
          }}
        >
          checks
          {checksBadge > 0 && (
            <span
              className="stream-badge"
              aria-label={`${checksBadge} failed`}
              style={{
                marginLeft: '0.25em',
                fontSize: '0.75em',
                background: 'var(--state-failed)',
                color: '#fff',
                borderRadius: '3px',
                padding: '0 3px',
                lineHeight: '1.4',
              }}
            >
              {checksBadge}
            </span>
          )}
        </button>

        {/* Flex spacer */}
        <span style={{ flex: 1 }} aria-hidden="true" />

        {/* Collapse / expand toggle */}
        <button
          type="button"
          data-action="toggle-stream"
          onClick={onToggle}
          aria-label={open ? 'Collapse stream pane' : 'Expand stream pane'}
          style={{
            background: 'transparent',
            border: 'none',
            cursor: 'pointer',
            padding: '0 0.125rem',
            fontFamily: 'inherit',
            fontSize: 'inherit',
            color: 'var(--text-muted)',
          }}
        >
          {open ? '▼' : '▶'}
        </button>
      </div>

      {/* ── Body — visible only when open ───────────────────────────────── */}
      {open && (
        <div
          className="stream-body"
          style={{
            flex: 1,
            overflow: 'auto',
            minHeight: 0,
            padding: '0.5rem 0.75rem',
            color: 'var(--text-faint)',
          }}
        >
          {focusedId === null ? (
            /* Nothing selected and nothing ran: system-state sentence */
            <div className="stream-empty" style={{ opacity: 0.6 }}>
              {emptySentence}
            </div>
          ) : view === 'transcript' ? (
            <Transcript transcript={transcript} working={working} />
          ) : (
            <Checks checks={checks} />
          )}
        </div>
      )}
    </div>
  );
}
