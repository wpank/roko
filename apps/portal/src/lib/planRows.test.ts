/**
 * planRows.test.ts — Tests for buildPlanRows
 *
 * Coverage (≥ 14):
 *  1. live state wins over disk
 *  2. two plans running at once
 *  3. queue positions (live, in planSet)
 *  4. no queue after the run ends
 *  5. accepted is never done
 *  6. accepted is never green (barToken)
 *  7. running plan at 90% is amber
 *  8. superseded → skipped with supersededBy
 *  9. numeric ordering within a group
 * 10. top-level plans come before grouped plans
 * 11. filter by title (case-insensitive)
 * 12. filter by id
 * 13. time: elapsed for running plan
 * 14. time: actual for finished plan
 * 15. time: estimate for not-started plan with estimated_minutes
 * 16. time: none when no timing info
 * 17. runningPlanIds: planSet order first, then others sorted
 * 18. group done count includes accepted rows
 * 19. empty filter shows all plans
 * 20. empty groups dropped after filtering
 */

import { describe, it, expect } from 'vitest';
import { buildPlanRows } from './planRows';
import { initialRunState } from './runState';
import type { WirePlanSummary } from '@/api/contracts';
import type { RunState, PlanRun } from './runState';

// ── Test helpers ───────────────────────────────────────────────────────────────

/** Minimal disk plan summary. */
function mkDisk(overrides: Partial<WirePlanSummary> = {}): WirePlanSummary {
  return {
    id: 'plan-1',
    title: 'Plan One',
    task_count: 5,
    tasks_done: 0,
    tasks_failed: 0,
    completed: false,
    status: 'pending',
    old_format: false,
    ...overrides,
  };
}

/** Minimal live PlanRun. */
function mkLive(overrides: Partial<PlanRun> = {}): PlanRun {
  return {
    planId: 'plan-1',
    title: null,
    phase: 'running',
    tasksTotal: 10,
    tasksDone: 0,
    tasksFailed: 0,
    tasksAccepted: 0,
    startedAtMs: 1000,
    finishedAtMs: null,
    etaMinutes: null,
    costUsd: 0,
    ...overrides,
  };
}

/** Return a RunState with one live plan merged in. */
function withLive(
  run: RunState,
  planId: string,
  liveOverrides: Partial<PlanRun> = {},
): RunState {
  return {
    ...run,
    plans: { ...run.plans, [planId]: mkLive({ planId, ...liveOverrides }) },
  };
}

const NOW = 10_000;

// ── Tests ──────────────────────────────────────────────────────────────────────

describe('buildPlanRows', () => {
  // 1. Live state wins over disk
  it('live state wins over disk: running plan is active', () => {
    const disk = [mkDisk({ status: 'pending', completed: false })];
    const run = withLive(initialRunState(), 'plan-1', { phase: 'running', startedAtMs: 1000 });
    const result = buildPlanRows(disk, run, { filter: '', nowMs: NOW });
    const row = result.groups[0]!.rows[0]!;
    expect(row.state).toBe('active');
    expect(row.running).toBe(true);
  });

  // 2. Two plans running at once
  it('two plans running concurrently are both active', () => {
    const disks = [
      mkDisk({ id: 'plan-1', title: 'Plan 1' }),
      mkDisk({ id: 'plan-2', title: 'Plan 2' }),
    ];
    const run: RunState = {
      ...initialRunState(),
      plans: {
        'plan-1': mkLive({ planId: 'plan-1', phase: 'running', startedAtMs: 500 }),
        'plan-2': mkLive({ planId: 'plan-2', phase: 'running', startedAtMs: 600 }),
      },
    };
    const result = buildPlanRows(disks, run, { filter: '', nowMs: NOW });
    const rows = result.groups.flatMap((g) => g.rows);
    const activeRows = rows.filter((r) => r.running);
    expect(activeRows).toHaveLength(2);
    expect(result.runningPlanIds).toHaveLength(2);
  });

  // 3. Queue positions
  it('pending plan in live planSet gets queued state with 1-based queuePosition', () => {
    const disk = [mkDisk({ id: 'plan-1' })];
    const run: RunState = {
      ...initialRunState(),
      planSet: { planIds: ['plan-1'], tasksTotal: 5, loadedAtMs: 0 },
      plans: { 'plan-1': mkLive({ planId: 'plan-1', phase: 'pending', startedAtMs: null }) },
      run: { startedAtMs: 0, durationMs: null, outcome: null },
    };
    const result = buildPlanRows(disk, run, { filter: '', nowMs: NOW });
    const row = result.groups[0]!.rows[0]!;
    expect(row.state).toBe('queued');
    expect(row.queuePosition).toBe(1);
  });

  // 4. No queue after the run ends
  it('pending plan falls back to disk state once run.durationMs is set', () => {
    const disk = [mkDisk({ id: 'plan-1', status: 'pending', completed: false })];
    const run: RunState = {
      ...initialRunState(),
      planSet: { planIds: ['plan-1'], tasksTotal: 5, loadedAtMs: 0 },
      plans: { 'plan-1': mkLive({ planId: 'plan-1', phase: 'pending', startedAtMs: null }) },
      run: { startedAtMs: 0, durationMs: 5_000, outcome: 'succeeded' },
    };
    const result = buildPlanRows(disk, run, { filter: '', nowMs: NOW });
    const row = result.groups[0]!.rows[0]!;
    expect(row.state).toBe('pending');
    expect(row.queuePosition).toBeNull();
  });

  // 5. Accepted is never done
  it('completed plan with tasksAccepted > 0 is accepted, not done', () => {
    const disk = [mkDisk({ id: 'plan-1', completed: true })];
    const run = withLive(initialRunState(), 'plan-1', {
      phase: 'completed',
      tasksAccepted: 2,
      tasksDone: 5,
    });
    const result = buildPlanRows(disk, run, { filter: '', nowMs: NOW });
    const row = result.groups[0]!.rows[0]!;
    expect(row.state).toBe('accepted');
    expect(row.state).not.toBe('done');
  });

  // 6. Accepted is never green
  it('accepted plan never gets the green barToken (--progress-high)', () => {
    const disk = [mkDisk({ id: 'plan-1', completed: true, task_count: 5, tasks_done: 5 })];
    const run = withLive(initialRunState(), 'plan-1', {
      phase: 'completed',
      tasksAccepted: 1,
      tasksDone: 5,
      tasksTotal: 5,
    });
    const result = buildPlanRows(disk, run, { filter: '', nowMs: NOW });
    const row = result.groups[0]!.rows[0]!;
    expect(row.state).toBe('accepted');
    expect(row.barToken).not.toBe('var(--progress-high)');
    expect(row.barToken).toBe('var(--progress-mid)'); // fraction 1.0 ≥ 0.3 → amber
  });

  // 7. Running plan at 90% is amber
  it('running plan at 90% fraction gets amber (--progress-mid), not green', () => {
    const disk = [mkDisk({ id: 'plan-1' })];
    const run = withLive(initialRunState(), 'plan-1', {
      phase: 'running',
      tasksDone: 9,
      tasksTotal: 10,
      startedAtMs: 1000,
    });
    const result = buildPlanRows(disk, run, { filter: '', nowMs: NOW });
    const row = result.groups[0]!.rows[0]!;
    expect(row.fraction).toBeCloseTo(0.9);
    expect(row.state).toBe('active');
    expect(row.barToken).toBe('var(--progress-mid)'); // amber, never green
  });

  // 8. Superseded
  it('superseded disk plan → state skipped with supersededBy set', () => {
    const disk = [mkDisk({ id: 'plan-1', status: 'superseded', superseded_by: 'plan-2' })];
    const result = buildPlanRows(disk, initialRunState(), { filter: '', nowMs: NOW });
    const row = result.groups[0]!.rows[0]!;
    expect(row.state).toBe('skipped');
    expect(row.supersededBy).toBe('plan-2');
  });

  // 9. Numeric ordering within a group
  it('rows within a group are sorted numerically by id', () => {
    const disks = [
      mkDisk({ id: 'plan-10', title: 'Plan 10', group: 'g' }),
      mkDisk({ id: 'plan-2', title: 'Plan 2', group: 'g' }),
      mkDisk({ id: 'plan-1', title: 'Plan 1', group: 'g' }),
    ];
    const result = buildPlanRows(disks, initialRunState(), { filter: '', nowMs: NOW });
    const group = result.groups.find((g) => g.name === 'g')!;
    expect(group.rows.map((r) => r.id)).toEqual(['plan-1', 'plan-2', 'plan-10']);
  });

  // 10. Top-level plans before grouped plans
  it('top-level (no group) plans appear before named groups', () => {
    const disks = [
      mkDisk({ id: 'b-plan', title: 'B', group: 'grp' }),
      mkDisk({ id: 'a-plan', title: 'A' }), // no group → top-level
    ];
    const result = buildPlanRows(disks, initialRunState(), { filter: '', nowMs: NOW });
    expect(result.groups[0]!.name).toBeNull();
    expect(result.groups[1]!.name).toBe('grp');
  });

  // 11. Filter by title (case-insensitive)
  it('filter by title substring is case-insensitive', () => {
    const disks = [
      mkDisk({ id: 'plan-1', title: 'My Portal App' }),
      mkDisk({ id: 'plan-2', title: 'Agent System' }),
    ];
    const result = buildPlanRows(disks, initialRunState(), { filter: 'PORTAL', nowMs: NOW });
    expect(result.count).toBe(1);
    expect(result.order).toEqual(['plan-1']);
  });

  // 12. Filter by id
  it('filter by id substring', () => {
    const disks = [
      mkDisk({ id: 'abc-plan', title: 'Some Plan' }),
      mkDisk({ id: 'xyz-plan', title: 'Other Plan' }),
    ];
    const result = buildPlanRows(disks, initialRunState(), { filter: 'abc', nowMs: NOW });
    expect(result.count).toBe(1);
    expect(result.order).toEqual(['abc-plan']);
  });

  // 13. Time: elapsed for running plan
  it('time.kind is elapsed for a running plan', () => {
    const disk = [mkDisk({ id: 'plan-1' })];
    const run = withLive(initialRunState(), 'plan-1', {
      phase: 'running',
      startedAtMs: 2_000,
    });
    const result = buildPlanRows(disk, run, { filter: '', nowMs: 7_000 });
    const row = result.groups[0]!.rows[0]!;
    expect(row.time.kind).toBe('elapsed');
    expect(row.time.ms).toBe(5_000);
  });

  // 14. Time: actual for finished plan
  it('time.kind is actual for a plan with both start and finish timestamps', () => {
    const disk = [mkDisk({ id: 'plan-1', completed: true })];
    const run = withLive(initialRunState(), 'plan-1', {
      phase: 'completed',
      startedAtMs: 1_000,
      finishedAtMs: 6_000,
    });
    const result = buildPlanRows(disk, run, { filter: '', nowMs: NOW });
    const row = result.groups[0]!.rows[0]!;
    expect(row.time.kind).toBe('actual');
    expect(row.time.ms).toBe(5_000);
  });

  // 15. Time: estimate for not-started plan with estimated_minutes
  it('time.kind is estimate for a not-started plan with estimated_minutes', () => {
    const disk = [mkDisk({ id: 'plan-1', estimated_minutes: 10 })];
    const result = buildPlanRows(disk, initialRunState(), { filter: '', nowMs: NOW });
    const row = result.groups[0]!.rows[0]!;
    expect(row.time.kind).toBe('estimate');
    expect(row.time.ms).toBe(600_000); // 10 × 60000
  });

  // 16. Time: none when no timing info
  it('time.kind is none when no timing info is available', () => {
    const disk = [mkDisk({ id: 'plan-1' })];
    const result = buildPlanRows(disk, initialRunState(), { filter: '', nowMs: NOW });
    const row = result.groups[0]!.rows[0]!;
    expect(row.time.kind).toBe('none');
    expect(row.time.ms).toBeNull();
  });

  // 17. runningPlanIds: planSet order first, others by id
  it('runningPlanIds: planSet members in planSet order, then others sorted by id', () => {
    const disks = [
      mkDisk({ id: 'extra-1' }),
      mkDisk({ id: 'set-b' }),
      mkDisk({ id: 'set-a' }),
    ];
    const run: RunState = {
      ...initialRunState(),
      planSet: { planIds: ['set-b', 'set-a'], tasksTotal: 10, loadedAtMs: 0 },
      plans: {
        'set-a': mkLive({ planId: 'set-a', phase: 'running' }),
        'set-b': mkLive({ planId: 'set-b', phase: 'running' }),
        'extra-1': mkLive({ planId: 'extra-1', phase: 'running' }),
      },
      run: { startedAtMs: 0, durationMs: null, outcome: null },
    };
    const result = buildPlanRows(disks, run, { filter: '', nowMs: NOW });
    expect(result.runningPlanIds).toEqual(['set-b', 'set-a', 'extra-1']);
  });

  // 18. Group done count includes accepted rows
  it('group.done counts both done and accepted rows', () => {
    const disks = [
      mkDisk({ id: 'plan-1', group: 'g' }),
      mkDisk({ id: 'plan-2', completed: true, group: 'g' }),
    ];
    const run: RunState = {
      ...initialRunState(),
      plans: {
        // plan-1: accepted (tasksAccepted > 0)
        'plan-1': mkLive({ planId: 'plan-1', phase: 'completed', tasksAccepted: 1, tasksDone: 3, tasksTotal: 3 }),
        // plan-2: done (tasksAccepted === 0)
        'plan-2': mkLive({ planId: 'plan-2', phase: 'completed', tasksAccepted: 0, tasksDone: 5, tasksTotal: 5 }),
      },
    };
    const result = buildPlanRows(disks, run, { filter: '', nowMs: NOW });
    const group = result.groups.find((g) => g.name === 'g')!;
    expect(group.done).toBe(2); // both accepted and done count
    expect(group.total).toBe(2);
  });

  // 19. Empty filter shows all plans
  it('empty filter shows all plans', () => {
    const disks = [
      mkDisk({ id: 'plan-1' }),
      mkDisk({ id: 'plan-2' }),
      mkDisk({ id: 'plan-3' }),
    ];
    const result = buildPlanRows(disks, initialRunState(), { filter: '', nowMs: NOW });
    expect(result.count).toBe(3);
  });

  // 20. Empty groups are dropped after filtering
  it('groups with no matching rows are dropped after filtering', () => {
    const disks = [
      mkDisk({ id: 'plan-1', title: 'Match This', group: 'grp-a' }),
      mkDisk({ id: 'plan-2', title: 'Nothing Here', group: 'grp-b' }),
    ];
    const result = buildPlanRows(disks, initialRunState(), { filter: 'match', nowMs: NOW });
    expect(result.groups).toHaveLength(1);
    expect(result.groups[0]!.name).toBe('grp-a');
    expect(result.count).toBe(1);
  });
});
