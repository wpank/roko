'use client';

/**
 * Header — sparse single-row shell header.
 *
 * Displays:
 * - Workspace name (and branch when reported) from useWorkspace(); sets
 *   document.title to "roko · <name>" via an effect.
 * - Live run progress when plans are running: plan-set summary or single-plan
 *   progress, elapsed time, ETA when known, and the run's cost when > 0.
 *   The entire section is a button that cycles selectedPlanId through
 *   runningPlanIds. A ■ cancel button stops the whole run.
 * - Connection dot with data-connection attribute and descriptive title.
 */

import { useEffect } from 'react';
import { useWorkspace } from '@/api/queries';
import { useDashboardStore } from '@/stores/dashboard';
import { compactDuration, formatCost, middleEllipsis } from '@/lib/formatters';
import { runCostUsd } from '@/lib/runState';
import { useNow } from '@/lib/useNow';
import { StatusGlyph } from '@/components/primitives/StatusGlyph';

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

  const isRunning = runningPlanIds.length > 0;

  // Clock that only ticks while something is running.
  const nowMs = useNow(isRunning);

  // document.title — "roko · <workspace name>"
  const workspaceName = workspace?.name;
  useEffect(() => {
    if (workspaceName) {
      document.title = `roko · ${workspaceName}`;
    }
  }, [workspaceName]);

  // ── Running-state display ─────────────────────────────────────────────────

  // Plans currently in the 'running' phase.
  const runningPlans = Object.values(run.plans).filter((p) => p.phase === 'running');

  // Is this a multi-plan set (more than one plan in the set)?
  const isMultiPlanSet = run.planSet !== null && run.planSet.planIds.length > 1;

  // Build the main run label (plan-set or single plan).
  let mainLabel = '';
  if (isRunning) {
    if (isMultiPlanSet && run.planSet !== null) {
      // Plan-set run: multiple plans executing concurrently.
      const setPlans = run.planSet.planIds.flatMap((id) => {
        const p = run.plans[id as keyof typeof run.plans];
        return p !== undefined ? [p] : [];
      });
      const donePlans = setPlans.filter(
        (p) => p.phase === 'completed' || p.phase === 'failed' || p.phase === 'cancelled',
      ).length;
      const totalPlans = run.planSet.planIds.length;
      const doneTasks = setPlans.reduce((s, p) => s + p.tasksDone + p.tasksFailed, 0);
      const totalTasks = run.planSet.tasksTotal;
      mainLabel = `${runningPlans.length} running · ${donePlans}/${totalPlans} plans · ${doneTasks}/${totalTasks} tasks`;
    } else {
      // Single plan (or single-plan set): use title, middle-ellipsised to 32.
      const plan = runningPlans[0];
      if (plan) {
        const done = plan.tasksDone + plan.tasksFailed;
        const label = plan.title ? middleEllipsis(plan.title, 32) : plan.planId;
        mainLabel = `${label} ${done}/${plan.tasksTotal}`;
      }
    }
  }

  // Elapsed time: use run.run.startedAtMs, else earliest running plan's startedAtMs.
  const startMs =
    run.run.startedAtMs ??
    runningPlans.reduce<number | null>((earliest, p) => {
      if (p.startedAtMs === null) return earliest;
      return earliest === null ? p.startedAtMs : Math.min(earliest, p.startedAtMs);
    }, null);
  const elapsedMs = startMs !== null && isRunning ? nowMs - startMs : null;

  // ETA: largest etaMinutes across all currently running plans.
  const maxEtaMinutes = runningPlans.reduce<number | null>((max, p) => {
    if (p.etaMinutes === null) return max;
    return max === null ? p.etaMinutes : Math.max(max, p.etaMinutes);
  }, null);

  // Build run summary parts joined with ' · '.
  const runParts: string[] = [];
  if (mainLabel) runParts.push(mainLabel);
  if (elapsedMs !== null) runParts.push(compactDuration(elapsedMs));
  if (maxEtaMinutes !== null) runParts.push(`~${maxEtaMinutes}m`);
  // The run's own cost; `run.totals` is everything since the server started.
  const costUsd = runCostUsd(run);
  if (costUsd > 0) runParts.push(formatCost(costUsd));
  const runSummaryText = runParts.join(' · ');

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
    <header data-region="header" className="rd-header">
      <span data-slot="workspace" className="rd-header__workspace">
        {workspace?.name}
        {workspace?.branch && (
          <span className="rd-header__branch"> · {workspace.branch}</span>
        )}
      </span>

      {isRunning && (
        <span data-slot="run" className="rd-header__run">
          <button
            type="button"
            data-action="next-running"
            className="rd-header__summary"
            onClick={handleSelectNext}
          >
            <StatusGlyph state="active" />
            <span>{runSummaryText}</span>
          </button>
          <button
            type="button"
            data-action="cancel-run"
            className="rd-header__cancel"
            aria-label="Cancel the run"
            onClick={() => onCancelRun(runningPlanIds[0]!)}
          >
            ■
          </button>
        </span>
      )}

      <span
        data-slot="connection"
        data-connection={connection}
        className="rd-header__conn"
        title={connectionTitle}
      >
        ●
      </span>
    </header>
  );
}
