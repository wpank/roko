'use client';

/**
 * usePrimaryAction — the one action a plan's page offers next, from its live phase and its validation: run plan,
 * stop, resume or run again. The plan page's key and the `r` key both use it.
 */

import { useCallback, useMemo, useRef } from 'react';
import { fetchPlanDetail, useCancelPlan, usePlanDetail, useValidation } from '@/api/queries';
import { useStartRun } from '@/components/stage/RunChoice';
import { describeRequestError } from '@/lib/apiErrors';
import { detailStamp, planSetActive, queuePosition, runFinishing, waitReason } from '@/lib/planSet';
import { useDashboardStore } from '@/stores/dashboard';

// ── usePrimaryAction ───────────────────────────────────────────────────────────

export interface PrimaryAction {
  kind: 'run' | 'cancel' | 'retry' | 'run-again' | 'none';
  label: string;
  disabled: boolean;
  reason: string | null;
  perform(): void;
}

/**
 * Derive the primary CTA for a plan from its live execution phase and
 * the current validation state.
 *
 * - running          → cancel, the `stop` key (window.confirm first; stops the whole run)
 * - failed/cancelled → retry, the `resume` key (resume: true: only what did not pass runs again); once the plan's
 *                      detail says no resume would continue its last run, run-again, the `run again` key
 * - completed        → run-again (a fresh run)
 * - otherwise        → run, the `run plan` key
 *
 * A fresh run sets aside the plan's last run (contract P-7), so run and
 * run-again read the plan's `resumable_run` first. When a resume would
 * continue that run, they ask Resume or Start over (RunChoice) instead of
 * running (gap-0eb763).
 *
 * Disabled while:
 *   • the plan's editor holds unsaved text ("Save or discard your edits first")
 *   • validation reports errors ("Fix N validation errors first")
 *   • a plan-set run is active and this plan is not running in it
 *   • the plan has ended but the run that holds it is still finishing ("roko is finishing the run")
 *
 * The header button and the `r` key each call this hook; both read the unsaved
 * text from the store, so neither runs a plan other than the one on screen.
 */
export function usePrimaryAction(
  planId: string | null,
  opts: { onError?: (msg: string) => void } = {},
): PrimaryAction {
  const { onError } = opts;

  // Live run state
  const run = useDashboardStore((s) => s.run);
  const unsaved = useDashboardStore((s) => planId !== null && s.unsavedPlan === planId);

  // Validation (disabled when planId is null)
  const { data: validation } = useValidation(planId ?? undefined);

  // Mutations
  const startRun = useStartRun(onError);
  const cancelPlan = useCancelPlan();
  const setRunChoice = useDashboardStore((s) => s.setRunChoice);
  // A Run whose plan read is still out: a second press starts nothing.
  const reading = useRef(false);

  // ── Phase ─────────────────────────────────────────────────────────────────

  const livePlan = planId ? run.plans[planId] : undefined;
  const phase = livePlan?.phase ?? null;
  // A failed or stopped plan resumes only when its last run can be continued; its detail says.
  const stopped = phase === 'failed' || phase === 'cancelled';
  const { data: detail } = usePlanDetail(
    planId ?? '',
    planId ? detailStamp(run, planId) : '',
    planId !== null && stopped,
  );

  // ── Kind ──────────────────────────────────────────────────────────────────

  let kind: PrimaryAction['kind'];
  if (!planId) {
    kind = 'none';
  } else if (phase === 'running') {
    kind = 'cancel';
  } else if (stopped) {
    kind = detail?.resumable_run === null ? 'run-again' : 'retry';
  } else if (phase === 'completed') {
    kind = 'run-again';
  } else {
    // pending or no live record (never run)
    kind = 'run';
  }

  // ── Disabled / reason ─────────────────────────────────────────────────────

  let disabled = !planId;
  let reason: string | null = null;

  if (planId) {
    if (unsaved) {
      disabled = true;
      reason = 'Save or discard your edits first';
    } else {
      const errorCount = validation
        ? validation.diagnostics.filter((d) => d.severity === 'error').length
        : 0;

      if (errorCount > 0) {
        disabled = true;
        reason = `Fix ${errorCount} validation error${errorCount === 1 ? '' : 's'} first`;
      } else if (planSetActive(run) && phase !== 'running') {
        // A plan-set run is active but this plan is not running in it.
        disabled = true;
        const pos = queuePosition(run, planId);
        const wait = waitReason(run, planId);
        if (pos !== null && wait !== null) {
          reason = `Queued #${pos} — ${wait}`;
        } else {
          reason = 'Another run is already in progress';
        }
      } else if (runFinishing(run, planId, Date.now())) {
        // The server refuses to run the plan again until the run that holds it ends.
        disabled = true;
        reason = 'roko is finishing the run';
      }
    }
  }

  // ── Running plan count for cancel confirmation ────────────────────────────

  const runningPlanCount = useMemo(
    () =>
      Object.values(run.plans).filter((p) => p.phase === 'running').length,
    [run.plans],
  );

  // ── perform ───────────────────────────────────────────────────────────────

  const perform = useCallback(() => {
    if (!planId || disabled) return;

    /** Surface mutation errors to the caller. */
    const handleError = (err: unknown): void => {
      if (!onError) return;
      const action = kind === 'cancel' ? 'cancelling runs' : 'running plans';
      onError(describeRequestError(err, action));
    };

    switch (kind) {
      case 'cancel': {
        const n = runningPlanCount;
        const planWord = n === 1 ? 'the running plan' : `${n} running plans`;
        if (
          !window.confirm(
            `Cancel this run? This will stop ${planWord}. The entire run will be cancelled.`,
          )
        )
          return;
        cancelPlan.mutate({ id: planId }, { onError: handleError });
        break;
      }
      case 'retry': {
        // resume: true — completed tasks are skipped; plan re-runs fresh if edited
        startRun(planId, true);
        break;
      }
      case 'run':
      case 'run-again': {
        // Ask before a fresh run sets aside a run a resume would continue.
        if (reading.current) return;
        reading.current = true;
        fetchPlanDetail(planId)
          .then((fresh) => {
            const last = fresh.resumable_run;
            if (last?.resumable) setRunChoice({ planId, run: last });
            else startRun(planId, false);
          }, handleError)
          .finally(() => {
            reading.current = false;
          });
        break;
      }
      default:
        break;
    }
  }, [planId, disabled, kind, runningPlanCount, cancelPlan, startRun, setRunChoice, onError]);

  // ── Label ─────────────────────────────────────────────────────────────────

  const label =
    kind === 'cancel' ? 'stop' :
    kind === 'retry' ? 'resume' :
    kind === 'run-again' ? 'run again' :
    kind === 'run' ? 'run plan' :
    '';

  return { kind, label, disabled, reason, perform };
}
