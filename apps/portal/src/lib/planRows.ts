/**
 * planRows.ts — derive PlanRowModel / PlanGroupModel / PlanRowsResult from
 * the disk plan list and live RunState.
 *
 * Live state (`run.plans[id]`) always wins over the disk summary.
 * See task specification for the complete rule set.
 *
 * Rules summary:
 * - Live phase: running → active; pending (live run) → queued; completed → done/accepted;
 *   failed → failed; cancelled → skipped.  Pending after the run ends → disk fallback.
 * - Disk only: superseded → skipped; completed → done; tasks_failed > 0 → failed; else pending.
 * - barToken: takes its plan's state colour (GLYPHS[state].token) — not a fraction band.
 *   A running bar is the running colour (--state-active) at any fraction.
 * - Groups: top-level (no group) first; then groups sorted numerically; rows sorted numerically.
 */

import type { WirePlanSummary } from '@/api/contracts';
import type { RunState, PlanRun } from '@/lib/runState';
import type { GlyphState } from '@/lib/glyphs';
import { GLYPHS } from '@/lib/glyphs';
import { queuePosition as getPlanQueuePosition, waitReason as getPlanWaitReason } from '@/lib/planSet';

// ── Public interfaces ──────────────────────────────────────────────────────────

export interface PlanRowModel {
  id: string;
  title: string;
  group: string | null;
  state: GlyphState;
  done: number;
  total: number;
  fraction: number;
  barToken: string;
  time: { kind: 'estimate' | 'elapsed' | 'actual' | 'none'; ms: number | null };
  queuePosition: number | null;
  waitReason: string | null;
  supersededBy: string | null;
  running: boolean;
}

export interface PlanGroupModel {
  name: string | null;
  rows: PlanRowModel[];
  done: number;
  total: number;
}

export interface PlanRowsResult {
  groups: PlanGroupModel[];
  order: string[];
  runningPlanIds: string[];
  count: number;
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/**
 * Return the progress-bar CSS token for a plan's state.
 * The bar always takes its state's colour (GLYPHS[state].token) —
 * not a fraction band. A running bar is the running colour at any fraction.
 */
function planBarToken(state: GlyphState): string {
  return GLYPHS[state].token;
}

/** Locale-aware numeric compare for plan-id sorting. */
function numericCompare(a: string, b: string): number {
  return a.localeCompare(b, undefined, { numeric: true });
}

/**
 * Derive GlyphState from the disk summary alone (no live state available).
 */
function diskGlyphState(disk: WirePlanSummary): GlyphState {
  if (disk.status === 'superseded') return 'skipped';
  if (disk.completed) return 'done';
  if (disk.tasks_failed > 0) return 'failed';
  return 'pending';
}

// ── buildPlanRows ──────────────────────────────────────────────────────────────

/**
 * Derive the full plan-rail model from the disk plan list and live RunState.
 *
 * @param plans  - Plans loaded from disk (GET /api/plans).
 * @param run    - Live RunState assembled from SSE events.
 * @param opts   - `filter`: case-insensitive substring filter; `nowMs`: wall clock.
 */
export function buildPlanRows(
  plans: readonly WirePlanSummary[],
  run: RunState,
  opts: { filter: string; nowMs: number },
): PlanRowsResult {
  const { filter, nowMs } = opts;
  const filterLo = filter.toLowerCase();

  // ── Build one PlanRowModel per disk plan ──────────────────────────────────
  const allRows: PlanRowModel[] = plans.map((disk): PlanRowModel => {
    const live: PlanRun | undefined = run.plans[disk.id];
    const group: string | null = disk.group ?? null;

    // ── State resolution ───────────────────────────────────────────────────
    let state: GlyphState;
    let supersededBy: string | null = null;
    let queuePosition: number | null = null;
    let waitReason: string | null = null;
    let running = false;

    if (live) {
      switch (live.phase) {
        case 'running':
          state = 'active';
          running = true;
          break;

        case 'pending': {
          // Queued exactly when the plan-set is active and the plan is a pending member.
          const pos = getPlanQueuePosition(run, disk.id);
          if (pos !== null) {
            state = 'queued';
            queuePosition = pos;
            waitReason = getPlanWaitReason(run, disk.id);
          } else {
            // Set inactive (all members finished) or plan not in set → disk fallback.
            state = diskGlyphState(disk);
            supersededBy = disk.superseded_by ?? null;
          }
          break;
        }

        case 'completed':
          // accepted_with_failures is amber and NEVER green.
          state = live.tasksAccepted > 0 ? 'accepted' : 'done';
          break;

        case 'failed':
          state = 'failed';
          break;

        case 'cancelled':
          state = 'skipped';
          break;

        default:
          // Unknown live phase — fall back to disk.
          state = diskGlyphState(disk);
          supersededBy = disk.superseded_by ?? null;
      }
    } else {
      // No live state — use disk summary.
      state = diskGlyphState(disk);
      supersededBy = disk.superseded_by ?? null;
    }

    // ── done / total / fraction ────────────────────────────────────────────
    let done: number;
    let total: number;

    if (live && live.tasksTotal > 0) {
      done = live.tasksDone;
      total = live.tasksTotal;
    } else {
      done = disk.tasks_done ?? (disk.completed ? disk.task_count : 0);
      total = disk.task_count;
    }

    const fraction = total > 0 ? done / total : 0;

    // ── barToken ───────────────────────────────────────────────────────────
    const barToken = planBarToken(state);

    // ── time ───────────────────────────────────────────────────────────────
    let time: PlanRowModel['time'];

    if (running && live && live.startedAtMs !== null) {
      // Elapsed: plan is currently running.
      time = { kind: 'elapsed', ms: nowMs - live.startedAtMs };
    } else if (live && live.startedAtMs !== null && live.finishedAtMs !== null) {
      // Actual: plan has finished and we have both timestamps.
      time = { kind: 'actual', ms: live.finishedAtMs - live.startedAtMs };
    } else if (!running && disk.estimated_minutes != null) {
      // Estimate: plan has not started but has a predicted duration.
      time = { kind: 'estimate', ms: disk.estimated_minutes * 60_000 };
    } else {
      time = { kind: 'none', ms: null };
    }

    return {
      id: disk.id,
      title: disk.title,
      group,
      state,
      done,
      total,
      fraction,
      barToken,
      time,
      queuePosition,
      waitReason,
      supersededBy,
      running,
    };
  });

  // ── Filter ────────────────────────────────────────────────────────────────
  const filtered: PlanRowModel[] = filterLo
    ? allRows.filter(
        (r) =>
          r.id.toLowerCase().includes(filterLo) ||
          r.title.toLowerCase().includes(filterLo),
      )
    : allRows;

  // ── Group and sort ────────────────────────────────────────────────────────
  const topLevelRows: PlanRowModel[] = [];
  const byGroup = new Map<string, PlanRowModel[]>();

  for (const row of filtered) {
    if (row.group === null) {
      topLevelRows.push(row);
    } else {
      let bucket = byGroup.get(row.group);
      if (!bucket) {
        bucket = [];
        byGroup.set(row.group, bucket);
      }
      bucket.push(row);
    }
  }

  const sortRows = (rs: PlanRowModel[]): PlanRowModel[] =>
    [...rs].sort((a, b) => numericCompare(a.id, b.id));

  const countDone = (rs: PlanRowModel[]): number =>
    rs.filter((r) => r.state === 'done' || r.state === 'accepted').length;

  // Top-level group (name = null) comes first; omit when empty.
  const sortedTop = sortRows(topLevelRows);
  const groups: PlanGroupModel[] = [];

  if (sortedTop.length > 0) {
    groups.push({
      name: null,
      rows: sortedTop,
      done: countDone(sortedTop),
      total: sortedTop.length,
    });
  }

  // Named groups, sorted numerically by name.
  const sortedGroupNames = [...byGroup.keys()].sort((a, b) => numericCompare(a, b));
  for (const name of sortedGroupNames) {
    const rows = sortRows(byGroup.get(name)!);
    groups.push({ name, rows, done: countDone(rows), total: rows.length });
  }

  // ── order ─────────────────────────────────────────────────────────────────
  const order: string[] = groups.flatMap((g) => g.rows.map((r) => r.id));

  // ── runningPlanIds ─────────────────────────────────────────────────────────
  // planSet members first (in planSet order), then any remaining running plans
  // sorted by id.
  const runningIds = new Set(filtered.filter((r) => r.running).map((r) => r.id));
  const runningInSetOrder: string[] = [];

  if (run.planSet) {
    for (const id of run.planSet.planIds) {
      if (runningIds.has(id)) runningInSetOrder.push(id);
    }
  }

  const runningOthers: string[] = [...runningIds]
    .filter((id) => !runningInSetOrder.includes(id))
    .sort((a, b) => numericCompare(a, b));

  const runningPlanIds = [...runningInSetOrder, ...runningOthers];

  return {
    groups,
    order,
    runningPlanIds,
    count: filtered.length,
  };
}
