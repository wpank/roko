'use client';

import React, { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import { ErrorBoundary } from '@/components/ErrorBoundary';
import { Key } from '@/components/kit/Key';
import { Led } from '@/components/kit/Led';
import { PlanList } from '@/components/plans/PlanList';
import { AlertBand } from '@/components/shell/AlertBand';
import { SignIn } from '@/components/shell/SignIn';
import { TopBar } from '@/components/shell/TopBar';
import { Stage } from '@/components/stage/Stage';
import type { PromptSeed } from '@/components/stage/Stage';
import { usePrimaryAction } from '@/components/plan/usePrimaryAction';
import {
  usePlans,
  usePlanTasks,
  useRunPlan,
  useRunPlans,
  useRunStarts,
  useWorkspace,
} from '@/api/queries';
import { confirmDiscard, useDashboardStore } from '@/stores/dashboard';
import { useSelection } from '@/lib/useSelection';
import { useKeyboard } from '@/lib/useKeyboard';
import { buildPlanRows } from '@/lib/planRows';
import { resolveSelection } from '@/lib/selection';
import { pickAlert } from '@/lib/alerts';
import { describeEmpty } from '@/lib/emptyState';
import { useNow } from '@/lib/useNow';
import { planSetActive, runFactsStamp } from '@/lib/planSet';
import { planWords } from '@/lib/planWords';
import { runSpentUsd } from '@/lib/runReadings';
import { splitWrites } from '@/lib/writing';
import type { SeenFailures } from '@/lib/statusChip';
import { describeRequestError, isMissingRoute } from '@/lib/apiErrors';
import type { AlertAction } from '@/lib/alerts';

// ── Workspace ──────────────────────────────────────────────────────────────────

/**
 * Workspace — the screen owner.
 *
 * The top bar spans the page. Under it, the plan list and the main column; signed out, the sign-in page instead.
 * The main column holds the alert (one sentence, when there is one) and the stage, whose plan page shows what runs
 * and how each task did. Below 820px wide the plan list becomes a picker that opens it over the page.
 *
 * Every scrollable region has `overflow: auto; min-height: 0`. The page itself never scrolls.
 */
export function Workspace() {
  // ── Local state ──────────────────────────────────────────────────────────────
  const [filter, setFilter] = useState('');
  const [promptOpen, setPromptOpen] = useState(false);
  // What the request box opens with, when a write stopped from its own view gave its request back (gap-f8c57a).
  const [promptSeed, setPromptSeed] = useState<PromptSeed | null>(null);
  const [dismissedAlertKey, setDismissedAlertKey] = useState<string | null>(null);
  const [requestError, setRequestError] = useState<string | null>(null);
  const [requestNotice, setRequestNotice] = useState<string | null>(null);
  // Phones: the plan list opens over the page.
  const [plansOpen, setPlansOpen] = useState(false);
  // Failed runs the person has opened since they failed: the status chip stops pointing at them.
  const [seenFailures, setSeenFailures] = useState<SeenFailures>({});

  // Routes the error to the right slot: missing-route → notice (info); anything
  // else → error.  Both slots are consumed by pickAlert below.
  const reportRequestError = useCallback(
    (err: unknown, action: string) => {
      const msg = describeRequestError(err, action);
      if (isMissingRoute(err)) {
        setRequestNotice(msg);
      } else {
        setRequestError(msg);
      }
    },
    [],
  );

  // Filter input ref — keyboard '/' handler focuses it.
  const filterInputRef = useRef<HTMLInputElement | null>(null);

  // ── Global data ──────────────────────────────────────────────────────────────
  const { data: listed, isLoading: plansLoading, error: plansError } = usePlans();
  // The plans on disk, and the plans being written, which have their own rows (gap-f8c57a).
  const { plans, writes } = splitWrites(listed);
  const { data: workspace } = useWorkspace();
  const run = useDashboardStore((s) => s.run);
  const connection = useDashboardStore((s) => s.connection);
  const unsavedPlan = useDashboardStore((s) => s.unsavedPlan);
  const { plan: selectedPlanId, task: selectedTaskId, select } = useSelection();

  // ── Build rows ───────────────────────────────────────────────────────────────
  // anyRunning drives useNow so the rail clock keeps ticking between events.
  const anyRunning = Object.values(run.plans).some((p) => p.phase === 'running');
  const nowMs = useNow(anyRunning);
  // When roko accepted each run the page knows, which the list's and the top bar's clocks count from (bug-b6d5cb).
  const runStarts = useRunStarts(
    Object.values(run.plans)
      .filter((p) => p.startedAtMs !== null)
      .map((p) => ({ id: p.planId, live: p.phase === 'running', stamp: runFactsStamp(run, p.planId) })),
  );
  const rows = buildPlanRows(plans ?? [], run, { filter, nowMs, runStarts });

  // ── Resolve selection ────────────────────────────────────────────────────────
  // An unknown plan or task clears silently, whatever the filter shows. With no
  // plan on load (plan list and run snapshot both in), the first running plan
  // is selected; the selection never jumps on its own afterwards.
  const loaded = !plansLoading && plans !== undefined;
  // A plan being written can be selected too: its row opens its writing view. It has no tasks, detail or
  // validation to read until it is written.
  const diskIds = (plans ?? []).map((p) => p.id);
  const planIds = [...diskIds, ...writes.map((w) => w.id)];
  const { data: selectedTasks } = usePlanTasks(
    selectedPlanId !== null && diskIds.includes(selectedPlanId) ? selectedPlanId : undefined,
  );
  const ready = loaded && connection === 'connected';
  const [settled, setSettled] = useState(false);
  const resolved = resolveSelection(
    { plan: selectedPlanId, task: selectedTaskId },
    {
      loaded,
      planIds,
      runningPlanIds: rows.runningPlanIds,
      taskIds: selectedTasks?.tasks.map((t) => t.id),
      firstLoad: ready && !settled,
    },
  );
  // Load is over once the URL holds the resolved plan (Next applies
  // replaceState in a transition, so this can be a render later).
  useEffect(() => {
    if (ready && resolved.plan === selectedPlanId) setSettled(true);
  }, [ready, resolved.plan, selectedPlanId]);

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

  // ── Plan list error → requestError ──────────────────────────────────────────
  // GET /api/plans returns an error when two plan folders share one id
  // (discovery refuses to guess which one to use).
  useEffect(() => {
    if (plansError) {
      reportRequestError(plansError, 'listing plans');
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [plansError]);

  // ── Mutations ─────────────────────────────────────────────────────────────────
  const runPlanMutation = useRunPlan();
  const runPlansMutation = useRunPlans();

  // ── Primary action for the selected plan (used by `r` key binding) ───────────
  const primaryAction = usePrimaryAction(resolved.plan !== null && diskIds.includes(resolved.plan) ? resolved.plan : null, {
    onError: setRequestError,
  });

  // ── planTitles — id → title map for human-readable alert text ──────────────
  const planTitles = useMemo<Readonly<Record<string, string>>>(
    () => Object.fromEntries((plans ?? []).map((p) => [p.id, p.title])),
    [plans],
  );

  // ── Alert ─────────────────────────────────────────────────────────────────────
  const alert = pickAlert({
    run,
    connection,
    selectedPlanId: resolved.plan,
    requestError,
    requestNotice,
    planTitles,
    dismissedKey: dismissedAlertKey,
  });

  // ── Workspace / empty state descriptions ──────────────────────────────────────
  const workspaceName = workspace?.name ?? 'workspace';

  // Rail shows a filtered "no match" sentence or the standard empty-state copy.
  // With no plans the stage says what roko is and asks for a request, so the list only says it is empty.
  const railEmpty = filter
    ? `No plans match "${filter}".`
    : connection === 'connected' && (plans?.length ?? 0) === 0
      ? 'No plans yet.'
      : describeEmpty({
          workspace: workspaceName,
          connection,
          planCount: plans?.length ?? 0,
        });

  // ── Action handlers ────────────────────────────────────────────────────────────

  // The generate field replaces the plan view, and with it any unsaved editor text.
  const openPrompt = () => {
    if (confirmDiscard(unsavedPlan !== null)) setPromptOpen(true);
  };

  const handleSelectPlan = useCallback(
    (id: string) => {
      select({ plan: id });
      setPlansOpen(false);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  const handleAlertAction = useCallback(
    (action: AlertAction) => {
      if (action.kind === 'select-task') {
        select({ plan: action.planId, task: action.taskId });
      } else if (action.kind === 'retry') {
        runPlanMutation.mutate(
          { id: action.planId, resume: true },
          { onError: (err) => reportRequestError(err, 'running plans') },
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
      const action = ids === null ? 'running all plans' : 'running plan groups';
      runPlansMutation.mutate(opts, {
        onError: (err) => reportRequestError(err, action),
      });
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
    // The generate and revise prompts and the editor close on Esc themselves.
    escape: () => {
      if (alert !== null) {
        setDismissedAlertKey(alert.key);
      } else if (filter !== '') {
        setFilter('');
      }
    },
    new: openPrompt,
    // `r` runs / retries / re-runs the selected plan; never cancels.
    run: () => {
      const k = primaryAction.kind;
      if (k === 'run' || k === 'retry' || k === 'run-again') {
        primaryAction.perform();
      }
    },
  });

  // ── A failed run counts as seen once its plan is open ────────────────────────
  const openPlan = resolved.plan !== null ? run.plans[resolved.plan] : undefined;
  const openFailedAt = openPlan?.phase === 'failed' ? openPlan.finishedAtMs : undefined;
  useEffect(() => {
    if (resolved.plan === null || openFailedAt === undefined) return;
    const planId = resolved.plan;
    setSeenFailures((seen) => (seen[planId] === openFailedAt ? seen : { ...seen, [planId]: openFailedAt }));
  }, [resolved.plan, openFailedAt]);

  // What each plan's latest run spent, as its SPENT reads it, for the plan list's status lines (bug-b6d5cb).
  const costs = useMemo<Readonly<Record<string, number>>>(
    () => Object.fromEntries(Object.values(run.plans).map((p) => [p.planId, runSpentUsd(run, p.planId)])),
    [run],
  );

  // The phone picker names the open plan, with its light.
  const openRow = rows.groups.flatMap((g) => g.rows).find((r) => r.id === resolved.plan) ?? null;

  // ── Render ────────────────────────────────────────────────────────────────────
  return (
    <div data-region="workspace" className="pt">
      <ErrorBoundary name="header">
        <TopBar seenFailures={seenFailures} runStarts={runStarts} onOpenPlan={handleSelectPlan} />
      </ErrorBoundary>

      {connection === 'unauthorized' ? (
        <SignIn />
      ) : (
        <div className="pt-body" data-plans-open={plansOpen ? '' : undefined}>
          <ErrorBoundary name="rail">
            <PlanList
              result={rows}
              writes={writes}
              costs={costs}
              selectedPlanId={resolved.plan}
              filter={filter}
              onFilterChange={setFilter}
              filterInputRef={filterInputRef}
              onSelect={handleSelectPlan}
              onNewPlan={() => {
                setPlansOpen(false);
                openPrompt();
              }}
              onRunPlans={handleRunPlans}
              emptySentence={railEmpty}
              runDisabledReason={
                planSetActive(run) ? 'A run is already in progress' :
                unsavedPlan !== null ? 'Save or discard your edits first' :
                null
              }
            />
          </ErrorBoundary>

          <div className="pt-main">
            <div className="pt-picker">
              <Led signal={openRow ? planWords(openRow).signal : 'waiting'} />
              <span>{openRow ? openRow.title || openRow.id : 'Plans'}</span>
              <Key size="s" aria-expanded={plansOpen} onClick={() => setPlansOpen((v) => !v)}>
                plans
              </Key>
            </div>

            <ErrorBoundary name="alert">
              <AlertBand alert={alert} onAction={handleAlertAction} onDismiss={setDismissedAlertKey} />
            </ErrorBoundary>

            <ErrorBoundary name="stage">
              <Stage
                plans={plans}
                writes={writes}
                loaded={loaded}
                selection={resolved}
                promptOpen={promptOpen}
                promptSeed={promptSeed}
                workspace={workspaceName}
                onSelect={select}
                onClosePrompt={() => {
                  setPromptOpen(false);
                  setPromptSeed(null);
                }}
                onReopenPrompt={(seed) => {
                  setPromptSeed(seed);
                  setPromptOpen(true);
                }}
                onRequestError={setRequestError}
              />
            </ErrorBoundary>

          </div>
        </div>
      )}
    </div>
  );
}
