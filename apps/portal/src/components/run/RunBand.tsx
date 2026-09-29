'use client';

/**
 * RunBand.tsx — the live run band shown while plans are executing.
 *
 * Contains three cells rendered side-by-side:
 *   BURN    — token totals and per-role breakdown (buildBurn)
 *   AGENTS  — active agent roster (buildRoster)
 *   CHECKS  — verify ladder for the focus task (checkFocusTask + buildRungs)
 *
 * Each cell opens with a compact `.rd-band__head` line: its title and a summary
 * note beside it.  Only BAND_ROWS content lines appear under the head; extras
 * are counted in the note as "+N more".
 * Returns null (zero height) when no plans are running.
 */

import React from 'react';
import { useQueries } from '@tanstack/react-query';
import { api } from '@/api/client';
import type { WirePlanTasks } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { useDashboardStore } from '@/stores/dashboard';
import type { Selection } from '@/lib/selection';
import { buildRoster } from '@/lib/agentRoster';
import { checkFocusTask, buildRungs } from '@/lib/rungs';
import { buildBurn } from '@/lib/burn';
import { taskKey } from '@/lib/runState';
import type { CheckRun } from '@/lib/runState';
import { StatusGlyph } from '@/components/primitives/StatusGlyph';
import { compactDuration, formatTokens, shortModel } from '@/lib/formatters';
import { glyphStateForCheck } from '@/lib/glyphs';
import { useNow } from '@/lib/useNow';

// ── Constants ────────────────────────────────────────────────────────────────────

/** Number of content rows rendered per cell; the rest are counted in the head note. */
const BAND_ROWS = 2;

// ── RunBand ─────────────────────────────────────────────────────────────────────

/**
 * Run band — shown while at least one plan is executing.
 *
 * Returns null when `runningPlanIds` is empty so the band costs zero height.
 */
export function RunBand({
  runningPlanIds,
  selection,
}: {
  runningPlanIds: string[];
  selection: Selection;
}) {
  if (runningPlanIds.length === 0) return null;
  return <RunBandInner runningPlanIds={runningPlanIds} selection={selection} />;
}

// ── RunBandInner ─────────────────────────────────────────────────────────────────

/**
 * Inner component — only rendered when there is at least one running plan.
 * All hooks live here so they are not called conditionally in the outer shell.
 */
function RunBandInner({
  runningPlanIds,
  selection,
}: {
  runningPlanIds: string[];
  selection: Selection;
}) {
  const run = useDashboardStore((s) => s.run);

  // Wall clock — ticks every second while running. Drives elapsed time display.
  const nowMs = useNow(true);

  // Fetch tasks for every running plan (max_parallel + verify steps).
  // One query per plan, cached under queryKeys.planTasks(id).
  const taskQueries = useQueries({
    queries: runningPlanIds.map((id) => ({
      queryKey: queryKeys.planTasks(id),
      queryFn: () =>
        api.get<WirePlanTasks>(`/api/plans/${encodeURIComponent(id)}/tasks`),
    })),
  });

  // Build maxParallelByPlan from whatever has already loaded.
  const maxParallelByPlan: Record<string, number | undefined> = {};
  for (let i = 0; i < runningPlanIds.length; i++) {
    const planId = runningPlanIds[i]!;
    maxParallelByPlan[planId] = taskQueries[i]?.data?.max_parallel;
  }

  // ── AGENTS ──────────────────────────────────────────────────────────────────
  const roster = buildRoster(run, { nowMs, runningPlanIds, maxParallelByPlan });

  // Head note: "+N more" for overflow active rows, "+N finished", "idle ×N".
  const agentsNoteParts: string[] = [];
  if (roster.rows.length > BAND_ROWS) agentsNoteParts.push(`+${roster.rows.length - BAND_ROWS} more`);
  if (roster.finished > 0) agentsNoteParts.push(`+${roster.finished} finished`);
  if (roster.idle !== null && roster.idle > 0) agentsNoteParts.push(`idle ×${roster.idle}`);

  // ── CHECKS ──────────────────────────────────────────────────────────────────
  // Adapt Selection (plan/task) → the shape checkFocusTask expects.
  const selected = { planId: selection.plan, taskId: selection.task };
  const focus = checkFocusTask(run, selected, runningPlanIds);

  // Resolve the declared verify ladder and live checks for the focus task.
  let declaredVerify: readonly { phase: string }[] | null = null;
  let focusChecks: readonly CheckRun[] = [];
  let focusTaskTitle = '';

  if (focus !== null) {
    const focusPlanIdx = runningPlanIds.indexOf(focus.planId);
    const planData =
      focusPlanIdx >= 0 ? taskQueries[focusPlanIdx]?.data : undefined;
    if (planData) {
      const wirePlanTask = planData.tasks.find((t) => t.id === focus.taskId);
      if (wirePlanTask?.verify && wirePlanTask.verify.length > 0) {
        declaredVerify = wirePlanTask.verify;
      }
    }
    const taskRun = run.tasks[taskKey(focus.planId, focus.taskId)];
    focusChecks = taskRun?.checks ?? [];
    focusTaskTitle = taskRun?.title || focus.taskId;
  }

  const rungs = focus !== null ? buildRungs(focusChecks, declaredVerify) : [];

  // ── BURN ─────────────────────────────────────────────────────────────────────
  const burn = buildBurn(run, { nowMs });

  // ── Render ───────────────────────────────────────────────────────────────────
  return (
    <section data-region="run-band" className="rd-band">
      {/* BURN cell — compact, sits above the rail */}
      <div data-cell="burn" className="rd-band__cell">
        <div className="rd-band__head">
          <span className="rd-band__title">BURN</span>
          <span className="rd-band__note rd-band__total">
            {formatTokens(burn.tokens)} tok
            {burn.tokensPerMin !== null &&
              ` · ${formatTokens(Math.round(burn.tokensPerMin))}/min`}
            {burn.byRole.length > BAND_ROWS &&
              ` · +${burn.byRole.length - BAND_ROWS} more`}
          </span>
        </div>
        {burn.byRole.slice(0, BAND_ROWS).map((r) => (
          <div key={r.role} data-role-share={r.role} className="rd-band__share">
            <span className="rd-band__bar">
              <span
                style={{
                  width: `${(r.share * 100).toFixed(1)}%`,
                  background: `var(--role-${r.role}, var(--role-other))`,
                }}
              />
            </span>
            <span className="rd-band__num">
              {r.role} {Math.round(r.share * 100)}%
            </span>
          </div>
        ))}
      </div>

      {/* AGENTS cell */}
      <div data-cell="agents" className="rd-band__cell">
        <div className="rd-band__head">
          <span className="rd-band__title">AGENTS</span>
          {agentsNoteParts.length > 0 && (
            <span className="rd-band__note">{agentsNoteParts.join(' · ')}</span>
          )}
        </div>
        {roster.rows.slice(0, BAND_ROWS).map((row) => (
          <div key={row.agentId} data-agent={row.agentId} className="rd-band__agent">
            <span
              data-role={row.role}
              className="rd-band__role"
              style={{ color: `var(--role-${row.role}, var(--role-other))` }}
            >
              {row.role}
            </span>
            <span>
              {row.planId ?? ''}
              {row.taskId ? ` · ${row.taskId}` : ''}
            </span>
            <span className="rd-band__muted" title={row.model}>{shortModel(row.model)}</span>
            {row.elapsedMs !== null && (
              <span className="rd-band__num">► {compactDuration(row.elapsedMs)}</span>
            )}
            <span className="rd-band__num">{formatTokens(row.tokens)} tok</span>
          </div>
        ))}
        {roster.rows.length === 0 && roster.finished === 0 && (
          <div className="rd-band__empty">no agent working yet</div>
        )}
      </div>

      {/* CHECKS cell */}
      <div data-cell="checks" className="rd-band__cell">
        <div className="rd-band__head">
          <span className="rd-band__title">
            {focus !== null ? `CHECKS · ${focus.taskId}` : 'CHECKS'}
          </span>
          {focus !== null && (
            <span className="rd-band__note rd-band__focus">
              {focusTaskTitle} · {focus.planId}
            </span>
          )}
        </div>
        {focus !== null ? (
          <div className="rd-band__rungs">
            {rungs.map((rung, i) => (
              <span
                key={rung.index ?? `x${i}`}
                data-rung={rung.state}
                className="rd-band__rung"
              ><StatusGlyph state={glyphStateForCheck(rung.state)} />{' '}{rung.label}</span>
            ))}
          </div>
        ) : (
          <div className="rd-band__empty">no task is being checked</div>
        )}
      </div>
    </section>
  );
}
