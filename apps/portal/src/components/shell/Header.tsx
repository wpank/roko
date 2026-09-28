'use client';

/**
 * Header — sparse single-row shell header.
 *
 * Displays:
 * - Workspace name (and branch when reported) from useWorkspace(); sets
 *   document.title to "roko · <name>" via an effect.
 * - Live run progress when plans are running: plan-set summary or single-plan
 *   progress, elapsed + ETA (always together), and accumulated cost.  The
 *   entire section is a button that cycles selectedPlanId through runningPlanIds.
 *   A ■ cancel button stops the whole run.
 * - Connection dot with data-connection attribute and descriptive title.
 */

import { useEffect, useState } from 'react';
import { useWorkspace } from '@/api/queries';
import { useDashboardStore } from '@/stores/dashboard';
import { formatCost, compactDuration } from '@/lib/formatters';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface HeaderProps {
  runningPlanIds: string[];
  selectedPlanId: string | null;
  onSelectPlan(id: string): void;
  onCancelRun(planId: string): void;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function Header({
  runningPlanIds,
  selectedPlanId,
  onSelectPlan,
  onCancelRun,
}: HeaderProps) {
  const { data: workspace } = useWorkspace();
  const run = useDashboardStore((s) => s.run);
  const connection = useDashboardStore((s) => s.connection);

  // Tick once a second so elapsed display stays current.
  const [nowMs, setNowMs] = useState<number>(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNowMs(Date.now()), 1_000);
    return () => clearInterval(id);
  }, []);

  // document.title — "roko · <workspace name>"
  const workspaceName = workspace?.name;
  useEffect(() => {
    if (workspaceName) {
      document.title = `roko · ${workspaceName}`;
    }
  }, [workspaceName]);

  // Workspace label: "name" or "name · branch"
  const workspaceLabel = workspace
    ? workspace.branch
      ? `${workspace.name} · ${workspace.branch}`
      : workspace.name
    : null;

  // ── Running-state display ─────────────────────────────────────────────────

  const isRunning = runningPlanIds.length > 0;

  // Plans currently in the 'running' phase (used for ETA).
  const runningPlans = Object.values(run.plans).filter((p) => p.phase === 'running');

  // Build the main run label (plan-set or single plan).
  let runLabel = '';
  if (isRunning) {
    if (run.planSet !== null) {
      // Plan-set run: multiple plans executing concurrently.
      const setPlans = run.planSet.planIds.flatMap((id) => {
        // Record<string, PlanRun> — key may be absent at runtime
        const p = run.plans[id as keyof typeof run.plans];
        return p !== undefined ? [p] : [];
      });
      const donePlans = setPlans.filter(
        (p) => p.phase === 'completed' || p.phase === 'failed',
      ).length;
      const totalPlans = run.planSet.planIds.length;
      const doneTasks = setPlans.reduce((s, p) => s + p.tasksDone + p.tasksFailed, 0);
      const totalTasks = run.planSet.tasksTotal;
      runLabel = `${runningPlans.length} running · ${donePlans}/${totalPlans} plans · ${doneTasks}/${totalTasks} tasks`;
    } else {
      // Single plan outside a set.
      const plan = runningPlans[0];
      if (plan) {
        const done = plan.tasksDone + plan.tasksFailed;
        runLabel = `${plan.planId} ${done}/${plan.tasksTotal}`;
      }
    }
  }

  // Elapsed since run started; freezes when the run ends (durationMs is set).
  const elapsedMs =
    run.run.startedAtMs !== null && run.run.durationMs === null
      ? nowMs - run.run.startedAtMs
      : run.run.durationMs ?? null;

  // ETA: largest etaMinutes across all currently running plans.
  const maxEtaMinutes = runningPlans.reduce<number | null>((max, p) => {
    if (p.etaMinutes === null) return max;
    return max === null ? p.etaMinutes : Math.max(max, p.etaMinutes);
  }, null);

  // Elapsed and ETA are always shown together or not at all.
  // "The operator's question is 'am I behind' — elapsed alone doesn't answer it."
  const timingLabel =
    elapsedMs !== null && maxEtaMinutes !== null
      ? `${compactDuration(elapsedMs)} ~${maxEtaMinutes}m`
      : null;

  const costLabel = formatCost(run.totals.costUsd);

  // Cycle selectedPlanId through runningPlanIds on each click.
  function handleSelectNext() {
    if (runningPlanIds.length === 0) return;
    const currentIdx =
      selectedPlanId !== null ? runningPlanIds.indexOf(selectedPlanId) : -1;
    const nextIdx = (currentIdx + 1) % runningPlanIds.length;
    onSelectPlan(runningPlanIds[nextIdx]!);
  }

  // ── Connection dot ────────────────────────────────────────────────────────

  const connectionTitles: Record<string, string> = {
    connected: 'Connected',
    connecting: 'Connecting…',
    disconnected: 'Disconnected',
    error: 'Connection error',
  };
  const connectionTitle = connectionTitles[connection] ?? String(connection);

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <header data-region="header">
      {workspaceLabel !== null && <span>{workspaceLabel}</span>}

      {isRunning && (
        <>
          <button type="button" onClick={handleSelectNext}>
            {runLabel}
            {timingLabel !== null && <> · {timingLabel}</>}
            {' '}
            {costLabel}
          </button>
          <button
            type="button"
            data-action="cancel-run"
            onClick={() => onCancelRun(runningPlanIds[0]!)}
          >
            ■
          </button>
        </>
      )}

      <span data-connection={connection} title={connectionTitle}>
        ●
      </span>
    </header>
  );
}
