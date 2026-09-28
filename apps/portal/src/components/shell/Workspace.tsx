'use client';

import React, { useState, useRef, useEffect, useCallback } from 'react';
import { ErrorBoundary } from '@/components/ErrorBoundary';
import { Header } from '@/components/shell/Header';
import { AlertBand } from '@/components/shell/AlertBand';
import { PlanRail } from '@/components/rail/PlanRail';
import { Stage } from '@/components/stage/Stage';
import { usePrimaryAction } from '@/components/stage/PlanView';
import { StreamPane } from '@/components/stream/StreamPane';
import { RunBand } from '@/components/run/RunBand';
import {
  usePlans,
  useRunPlan,
  useRunPlans,
  useCancelPlan,
  useWorkspace,
} from '@/api/queries';
import { ApiError } from '@/api/client';
import { useDashboardStore } from '@/stores/dashboard';
import { useSelection } from '@/lib/useSelection';
import { useKeyboard } from '@/lib/useKeyboard';
import { buildPlanRows } from '@/lib/planRows';
import { resolveSelection } from '@/lib/selection';
import { pickAlert } from '@/lib/alerts';
import { describeEmpty } from '@/lib/emptyState';
import { SIGN_IN_HINT } from '@/lib/bootstrap';
import type { AlertAction } from '@/lib/alerts';

// ── Error message extraction ───────────────────────────────────────────────────

/**
 * Extract a human-readable message from any thrown value.
 * A 401 always returns the sign-in hint regardless of the body text.
 */
function extractErrorMessage(err: unknown): string {
  if (err instanceof ApiError) {
    if (err.status === 401) return SIGN_IN_HINT;
    const body = err.body;
    if (typeof body === 'string' && body.length > 0) return body;
    if (body !== null && typeof body === 'object' && 'message' in body) {
      const msg = (body as { message: unknown }).message;
      if (typeof msg === 'string' && msg.length > 0) return msg;
    }
    return err.message;
  }
  if (err instanceof Error) return err.message;
  return String(err);
}

// ── Workspace ──────────────────────────────────────────────────────────────────

/**
 * Workspace — the screen owner.
 *
 * Lays out a full-screen CSS grid:
 *   row 1: header  (full width, auto height)
 *   row 2: alert   (full width, auto height — collapses to zero when no alert)
 *   row 3: rail (`var(--rail-width)`) | main column (1fr)
 *
 * The main column stacks the stage (flex 1) above the stream panel (collapsed
 * to a single bar by default; ~40% of the column height when open).
 *
 * Every scrollable region has `overflow: auto; min-height: 0`.
 * The page itself never scrolls.
 */
export function Workspace() {
  // ── Local state ──────────────────────────────────────────────────────────────
  const [filter, setFilter] = useState('');
  const [streamOpen, setStreamOpen] = useState(false);
  const [promptOpen, setPromptOpen] = useState(false);
  const [dismissedAlertKey, setDismissedAlertKey] = useState<string | null>(null);
  const [requestError, setRequestError] = useState<string | null>(null);

  // Filter input ref — keyboard '/' handler focuses it.
  const filterInputRef = useRef<HTMLInputElement | null>(null);

  // ── Global data ──────────────────────────────────────────────────────────────
  const { data: plans, isLoading: plansLoading, error: plansError } = usePlans();
  const { data: workspace } = useWorkspace();
  const run = useDashboardStore((s) => s.run);
  const connection = useDashboardStore((s) => s.connection);
  const { plan: selectedPlanId, task: selectedTaskId, select } = useSelection();

  // ── Build rows ───────────────────────────────────────────────────────────────
  const rows = buildPlanRows(plans ?? [], run, { filter, nowMs: Date.now() });

  // ── Resolve selection ────────────────────────────────────────────────────────
  // resolveSelection clears an unknown plan once the list has loaded, and
  // auto-selects the first running plan when nothing is chosen.
  const resolved = resolveSelection(
    { plan: selectedPlanId, task: selectedTaskId },
    {
      loaded: !plansLoading && plans !== undefined,
      planIds: rows.order,
      runningPlanIds: rows.runningPlanIds,
    },
  );

  // Sync resolved selection back to the URL when it diverges.
  useEffect(() => {
    if (resolved.plan !== selectedPlanId || resolved.task !== selectedTaskId) {
      select({ plan: resolved.plan, task: resolved.task });
    }
    // `select` identity changes on every render but its effect (replaceState)
    // is idempotent — using resolved.{plan,task} + selected{PlanId,TaskId} as
    // the driver is sufficient and avoids an infinite re-registration loop.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resolved.plan, resolved.task, selectedPlanId, selectedTaskId]);

  // ── Auto-open stream when a run starts ───────────────────────────────────────
  // Track the previous running count; open the stream pane whenever it rises
  // (a plan just started executing). 'o' still toggles it manually.
  const prevRunningCountRef = useRef(rows.runningPlanIds.length);
  useEffect(() => {
    const prev = prevRunningCountRef.current;
    const curr = rows.runningPlanIds.length;
    if (curr > prev) {
      setStreamOpen(true);
    }
    prevRunningCountRef.current = curr;
  }, [rows.runningPlanIds.length]);

  // ── Plan list error → requestError ──────────────────────────────────────────
  // GET /api/plans returns an error when two plan folders share one id
  // (discovery refuses to guess which one to use).
  useEffect(() => {
    if (plansError) {
      setRequestError(extractErrorMessage(plansError));
    }
  }, [plansError]);

  // ── Mutations ─────────────────────────────────────────────────────────────────
  const runPlanMutation = useRunPlan();
  const runPlansMutation = useRunPlans();
  const cancelPlanMutation = useCancelPlan();

  // ── Primary action for the selected plan (used by `r` key binding) ───────────
  const primaryAction = usePrimaryAction(resolved.plan, {
    onError: setRequestError,
  });

  function handleMutationError(err: unknown): void {
    setRequestError(extractErrorMessage(err));
  }

  // ── Alert ─────────────────────────────────────────────────────────────────────
  const alert = pickAlert({
    run,
    connection,
    selectedPlanId: resolved.plan,
    validationErrors: 0,
    requestError,
    dismissedKey: dismissedAlertKey,
  });

  // ── Workspace / empty state descriptions ──────────────────────────────────────
  const workspaceName = workspace?.name ?? 'workspace';

  // Rail shows a filtered "no match" sentence or the standard empty-state copy.
  const railEmpty = filter
    ? `No plans match "${filter}".`
    : describeEmpty({
        workspace: workspaceName,
        connection,
        planCount: plans?.length ?? 0,
        plan: undefined,
      });

  // ── Action handlers ────────────────────────────────────────────────────────────

  const handleSelectPlan = useCallback(
    (id: string) => {
      select({ plan: id });
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  const handleCancelRun = useCallback(
    (planId: string) => {
      const count = rows.runningPlanIds.length;
      const label =
        count === 1
          ? `Stop the running plan "${planId}"?`
          : `Stop all ${count} running plans?`;
      if (!window.confirm(label)) return;
      cancelPlanMutation.mutate(
        { id: planId },
        { onError: (err) => handleMutationError(err) },
      );
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [rows.runningPlanIds.length, cancelPlanMutation],
  );

  const handleAlertAction = useCallback(
    (action: AlertAction) => {
      if (action.kind === 'select-task') {
        select({ plan: action.planId, task: action.taskId });
      } else if (action.kind === 'retry') {
        runPlanMutation.mutate(
          { id: action.planId, resume: true },
          { onError: (err) => handleMutationError(err) },
        );
      } else if (action.kind === 'reconnect') {
        window.location.reload();
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [runPlanMutation],
  );

  const handleRunPlans = useCallback(
    (ids: string[] | null, label: string) => {
      if (!window.confirm(`Run ${label} in dependency order?`)) return;
      const opts = ids !== null ? { plans: ids } : {};
      runPlansMutation.mutate(opts, { onError: (err) => handleMutationError(err) });
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [runPlansMutation],
  );

  // ── Keyboard handlers ─────────────────────────────────────────────────────────
  useKeyboard({
    prev: () => {
      if (rows.order.length === 0) return;
      const idx = resolved.plan !== null ? rows.order.indexOf(resolved.plan) : -1;
      const prevIdx = idx <= 0 ? rows.order.length - 1 : idx - 1;
      select({ plan: rows.order[prevIdx]! });
    },
    next: () => {
      if (rows.order.length === 0) return;
      const idx = resolved.plan !== null ? rows.order.indexOf(resolved.plan) : -1;
      const nextIdx = idx < 0 || idx >= rows.order.length - 1 ? 0 : idx + 1;
      select({ plan: rows.order[nextIdx]! });
    },
    filter: () => {
      filterInputRef.current?.focus();
    },
    escape: () => {
      if (alert !== null) {
        setDismissedAlertKey(alert.key);
      } else if (filter !== '') {
        setFilter('');
      } else if (promptOpen) {
        setPromptOpen(false);
      }
    },
    stream: () => setStreamOpen((v) => !v),
    new: () => setPromptOpen(true),
    // `r` runs / retries / re-runs the selected plan; never cancels.
    run: () => {
      const k = primaryAction.kind;
      if (k === 'run' || k === 'retry' || k === 'run-again') {
        primaryAction.perform();
      }
    },
  });

  // ── Render ────────────────────────────────────────────────────────────────────
  return (
    <div
      data-region="workspace"
      style={{
        display: 'grid',
        gridTemplateRows: 'auto auto auto 1fr',
        gridTemplateColumns: 'var(--rail-width) 1fr',
        height: '100dvh',
        overflow: 'hidden',
      }}
    >
      {/* ── Header — spans both columns ─────────────────────────────────────── */}
      <div style={{ gridColumn: '1 / -1' }}>
        <ErrorBoundary name="header">
          <Header
            runningPlanIds={rows.runningPlanIds}
            selectedPlanId={resolved.plan}
            onSelectPlan={handleSelectPlan}
            onCancelRun={handleCancelRun}
          />
        </ErrorBoundary>
      </div>

      {/* ── Alert band — spans both columns ─────────────────────────────────── */}
      <div style={{ gridColumn: '1 / -1' }}>
        <ErrorBoundary name="alert">
          <AlertBand
            alert={alert}
            onAction={handleAlertAction}
            onDismiss={setDismissedAlertKey}
          />
        </ErrorBoundary>
      </div>

      {/* ── Run band — spans both columns, row 3 (auto height; collapses when idle) */}
      <div style={{ gridColumn: '1 / -1' }}>
        <ErrorBoundary name="run-band">
          <RunBand runningPlanIds={rows.runningPlanIds} selection={resolved} />
        </ErrorBoundary>
      </div>

      {/* ── Rail — column 1, row 4 ───────────────────────────────────────────── */}
      <div
        style={{
          overflow: 'auto',
          minHeight: 0,
          borderRight: '1px solid var(--blur-border)',
        }}
      >
        <ErrorBoundary name="rail">
          <PlanRail
            result={rows}
            selectedPlanId={resolved.plan}
            filter={filter}
            onFilterChange={setFilter}
            filterInputRef={filterInputRef}
            onSelect={handleSelectPlan}
            onNewPlan={() => setPromptOpen(true)}
            onRunPlans={handleRunPlans}
            emptySentence={railEmpty}
          />
        </ErrorBoundary>
      </div>

      {/* ── Main column — column 2, row 3 ────────────────────────────────────── */}
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
          minHeight: 0,
        }}
      >
        <ErrorBoundary name="stage">
          <Stage
            plans={plans}
            loaded={!plansLoading && plans !== undefined}
            selection={resolved}
            promptOpen={promptOpen}
            workspace={workspaceName}
            onSelect={select}
            onClosePrompt={() => setPromptOpen(false)}
            onRequestError={setRequestError}
          />
        </ErrorBoundary>

        {/* Stream pane — bar always visible; body shown when open */}
        <ErrorBoundary name="stream">
          <section
            data-region="stream"
            style={{
              flexShrink: 0,
              flex: streamOpen ? '0 0 40%' : '0 0 auto',
              overflow: streamOpen ? 'auto' : 'hidden',
              minHeight: 0,
              borderTop: '1px solid var(--blur-border)',
              display: 'flex',
              flexDirection: 'column',
            }}
          >
            <StreamPane
              planId={resolved.plan}
              selectedTaskId={resolved.task}
              open={streamOpen}
              onToggle={() => setStreamOpen((v) => !v)}
            />
          </section>
        </ErrorBoundary>
      </div>

    </div>
  );
}
