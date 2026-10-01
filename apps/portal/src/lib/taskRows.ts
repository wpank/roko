/**
 * taskRows.ts — Derives TaskRowModel[] from wire plan tasks and live RunState.
 *
 * buildTaskRows(tasks, run, planId, nowMs) → { rows, waves }
 * focusTaskId(rows, selected) → string | null
 */

import type { WirePlanTask } from '@/api/contracts';
import type { RunState, TaskStatus, CheckRun } from '@/lib/runState';
import { taskKey } from '@/lib/runState';
import type { GlyphState } from '@/lib/glyphs';
import { glyphStateForTask } from '@/lib/glyphs';
import { computeWaves } from '@/lib/waves';
import type { WaveResult } from '@/lib/waves';

// ── Types ──────────────────────────────────────────────────────────────────────

export interface TaskRowModel {
  id: string;
  title: string;
  /** Zero-based wave index from Kahn layering. */
  wave: number;
  state: GlyphState;
  status: TaskStatus | 'pending';
  role: string | null;
  model: string | null;
  time: { kind: 'estimate' | 'elapsed' | 'actual' | 'none'; ms: number | null };
  costUsd: number | null;
  attempts: number;
  checks: CheckRun[];
  dependsOn: string[];
  /**
   * IDs of dependsOn entries whose row is not passed, already_satisfied, accepted_with_failures,
   * unverified, or skipped.
   */
  waitingOn: string[];
  files: string[];
  description: string | null;
  verify: { phase: string; command: string }[];
}

// ── Constants ──────────────────────────────────────────────────────────────────

/**
 * Statuses that are "done" for waitingOn and focusTaskId purposes:
 * passed, already_satisfied, accepted_with_failures, unverified, skipped.
 *
 * Note: 'cancelled' is NOT included — a cancelled dep still blocks.
 */
const FINISHED_STATUSES: ReadonlySet<TaskStatus | 'pending'> = new Set([
  'passed',
  'already_satisfied',
  'accepted_with_failures',
  'unverified',
  'skipped',
] as const);

// ── buildTaskRows ──────────────────────────────────────────────────────────────

/**
 * Build an ordered list of TaskRowModel from wire plan tasks and live RunState.
 *
 * Row ordering: wave order, then input order within each wave. Rows never move
 * during a run; ordering is determined from the wire tasks alone via computeWaves.
 */
export function buildTaskRows(
  tasks: readonly WirePlanTask[],
  run: RunState,
  planId: string,
  nowMs: number,
): { rows: TaskRowModel[]; waves: WaveResult } {
  // ── 1. Wave ordering ───────────────────────────────────────────────────────
  const waveResult = computeWaves(tasks);

  // ── 2. Quick lookup for wire tasks ─────────────────────────────────────────
  const wireById = new Map<string, WirePlanTask>();
  for (const t of tasks) {
    wireById.set(t.id, t);
  }

  // ── 3. Plan state — needed for the "skipped after plan failure" rule ───────
  const livePlan = run.plans[planId];
  const planTerminated =
    livePlan?.phase === 'failed' || livePlan?.phase === 'cancelled';

  // ── 4. Build rows in wave order ────────────────────────────────────────────
  const rows: TaskRowModel[] = [];

  for (let wIdx = 0; wIdx < waveResult.waves.length; wIdx++) {
    const waveIds = waveResult.waves[wIdx]!;
    for (const id of waveIds) {
      const wireTask = wireById.get(id);
      if (!wireTask) continue; // should not happen; guard only

      const liveTask = run.tasks[taskKey(planId, id)];

      // ── status ──────────────────────────────────────────────────────────────
      let status: TaskStatus | 'pending';
      if (liveTask) {
        // Live run record takes precedence.
        status = liveTask.status;
      } else if (planTerminated) {
        // Plan failed/cancelled and this task never started.
        status = 'skipped';
      } else if (wireTask.completed) {
        // Wire task completed (pre-existing state, no live event yet).
        status = 'passed';
      } else {
        status = 'pending';
      }

      const state = glyphStateForTask(status);

      // ── role / model ─────────────────────────────────────────────────────
      const role = liveTask?.role ?? wireTask.role ?? null;
      const model = liveTask?.model ?? wireTask.model_hint ?? null;

      // ── costUsd ──────────────────────────────────────────────────────────
      const costUsd = liveTask != null ? liveTask.costUsd : null;

      // ── time ─────────────────────────────────────────────────────────────
      let time: TaskRowModel['time'];
      if (liveTask?.status === 'active' && liveTask.startedAtMs !== null) {
        // Currently running — show elapsed.
        time = { kind: 'elapsed', ms: nowMs - liveTask.startedAtMs };
      } else if (
        liveTask != null &&
        liveTask.startedAtMs !== null &&
        liveTask.finishedAtMs !== null
      ) {
        // Finished — show actual wall-clock duration.
        time = { kind: 'actual', ms: liveTask.finishedAtMs - liveTask.startedAtMs };
      } else if (liveTask == null && wireTask.estimated_minutes !== undefined) {
        // No live record yet but an estimate is provided.
        time = { kind: 'estimate', ms: wireTask.estimated_minutes * 60_000 };
      } else {
        time = { kind: 'none', ms: null };
      }

      // ── checks / attempts ─────────────────────────────────────────────────
      const checks = liveTask?.checks ?? [];
      const attempts = liveTask?.attempts ?? 0;

      // ── verify ────────────────────────────────────────────────────────────
      // Prefer the rich wire verify array; fall back to expanding verify_phases.
      const verify: { phase: string; command: string }[] =
        wireTask.verify != null
          ? wireTask.verify.map((v) => ({ phase: v.phase, command: v.command }))
          : wireTask.verify_phases.map((phase) => ({ phase, command: '' }));

      rows.push({
        id,
        title: wireTask.title,
        wave: wIdx,
        state,
        status,
        role,
        model,
        time,
        costUsd,
        attempts,
        checks,
        dependsOn: wireTask.depends_on,
        waitingOn: [], // filled in below after all rows are built
        files: wireTask.files,
        description: wireTask.description ?? null,
        verify,
      });
    }
  }

  // ── 5. Fill in waitingOn ───────────────────────────────────────────────────
  // Build a status map so we can look up each dependency's current status.
  const statusById = new Map<string, TaskStatus | 'pending'>();
  for (const row of rows) {
    statusById.set(row.id, row.status);
  }

  for (const row of rows) {
    row.waitingOn = row.dependsOn.filter((depId) => {
      const depStatus = statusById.get(depId);
      if (depStatus === undefined) return false; // dep not in this plan
      return !FINISHED_STATUSES.has(depStatus);
    });
  }

  return { rows, waves: waveResult };
}

// ── focusTaskId ────────────────────────────────────────────────────────────────

/**
 * Determine which task to stream / focus.
 *
 * Priority:
 *   1. `selected` when it names an existing row
 *   2. First failed row — a failure takes the stream even while other tasks
 *      run (design §11); only a selection keeps it elsewhere
 *   3. First active row
 *   4. Last finished row (passed / accepted_with_failures / skipped)
 *   5. null
 *
 * "First" and "last" use row order (wave order, then input order) — not time.
 */
export function focusTaskId(
  rows: readonly TaskRowModel[],
  selected: string | null,
): string | null {
  // 1. Explicit selection when it is a valid row ID.
  if (selected !== null && rows.some((r) => r.id === selected)) {
    return selected;
  }

  // 2. First failed row.
  const failed = rows.find((r) => r.status === 'failed');
  if (failed) return failed.id;

  // 3. First active row.
  const active = rows.find((r) => r.status === 'active');
  if (active) return active.id;

  // 4. Last finished row — iterate forward; final assignment wins.
  let lastFinished: TaskRowModel | null = null;
  for (const row of rows) {
    if (FINISHED_STATUSES.has(row.status)) {
      lastFinished = row;
    }
  }
  if (lastFinished) return lastFinished.id;

  return null;
}
