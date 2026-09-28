'use client';

/**
 * RunBand.tsx — the live run band shown while plans are executing.
 *
 * Contains three cells rendered side-by-side:
 *   AGENTS  — active agent roster (buildRoster)
 *   CHECKS  — verify ladder for the focus task (checkFocusTask + buildRungs)
 *   BURN    — token totals and per-role breakdown (buildBurn)
 *
 * Returns null (zero height) when no plans are running.
 */

import React, { useEffect, useState } from 'react';
import { useQueries } from '@tanstack/react-query';
import { api } from '@/api/client';
import type { WirePlanTasks } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { useDashboardStore } from '@/stores/dashboard';
import type { Selection } from '@/lib/selection';
import { buildRoster } from '@/lib/agentRoster';
import { checkFocusTask, buildRungs } from '@/lib/rungs';
import type { Rung } from '@/lib/rungs';
import { buildBurn } from '@/lib/burn';
import { taskKey } from '@/lib/runState';
import type { CheckRun } from '@/lib/runState';
import { StatusGlyph } from '@/components/primitives/StatusGlyph';
import { compactDuration, formatTokens } from '@/lib/formatters';
import type { GlyphState } from '@/lib/glyphs';

// ── Helpers ─────────────────────────────────────────────────────────────────────

/** Map a rung state to the StatusGlyph GlyphState. */
function rungGlyphState(state: Rung['state']): GlyphState {
  switch (state) {
    case 'passed':
      return 'done';
    case 'running':
      return 'active';
    case 'failed':
      return 'failed';
    case 'pending':
      return 'pending';
  }
}

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

  // Tick once per second so elapsed times update live.
  const [nowMs, setNowMs] = useState<number>(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNowMs(Date.now()), 1_000);
    return () => clearInterval(id);
  }, []);

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
    <section data-region="run-band">
      {/* AGENTS cell */}
      <div data-cell="agents">
        {roster.rows.map((row) => (
          <div
            key={row.agentId}
            style={{
              color: `var(--role-${row.role}, var(--role-other))`,
            }}
          >
            <span>{row.role}</span>
            <span>
              {row.planId ?? ''}
              {row.taskId ? ` · ${row.taskId}` : ''}
            </span>
            <span>{row.model}</span>
            <span>
              {'► '}
              {row.elapsedMs !== null ? compactDuration(row.elapsedMs) : '—'}
            </span>
            <span>{formatTokens(row.tokens)}</span>
          </div>
        ))}
        {roster.finished > 0 && <div>+{roster.finished} finished</div>}
        {roster.idle !== null && roster.idle > 0 && (
          <div>idle ×{roster.idle}</div>
        )}
      </div>

      {/* CHECKS cell */}
      <div data-cell="checks">
        {focus !== null && (
          <>
            <div>
              {focusTaskTitle} · {focus.planId}
            </div>
            <div>
              {rungs.map((rung, i) => (
                <span key={rung.index ?? `x${i}`}>
                  <StatusGlyph state={rungGlyphState(rung.state)} />
                  {rung.label}
                </span>
              ))}
            </div>
          </>
        )}
      </div>

      {/* BURN cell */}
      <div data-cell="burn">
        <div>
          {formatTokens(burn.tokens)} tok
          {' · '}
          {burn.tokensPerMin !== null
            ? `${formatTokens(Math.round(burn.tokensPerMin))}/min`
            : '·'}
        </div>
        {burn.byRole.map((r) => (
          <div key={r.role}>
            <div style={{ width: `${(r.share * 100).toFixed(1)}%` }} />
            <span>
              {r.role} {Math.round(r.share * 100)}%
            </span>
          </div>
        ))}
      </div>
    </section>
  );
}
