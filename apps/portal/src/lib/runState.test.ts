import { describe, it, expect } from 'vitest';
import {
  taskKey,
  parseCheckName,
  initialRunState,
  applyEvent,
  fromSnapshot,
  MAX_TRANSCRIPT_ENTRIES,
  MAX_ERRORS,
  MAX_USAGE_SAMPLES,
  type RunState,
  type TaskRun,
  type PlanRun,
} from './runState';
import type { WireDashboardEvent, WireDashboardSnapshot } from '@/api/contracts';

// ── Helpers ───────────────────────────────────────────────────────────────────

function apply(events: WireDashboardEvent[], nowMs = 1000): RunState {
  return events.reduce(
    (s, e) => applyEvent(s, e, nowMs),
    initialRunState(),
  );
}

function startPlan(planId: string, nowMs = 1000): RunState {
  return apply([{ type: 'plan_started', plan_id: planId, tasks_total: 3 }], nowMs);
}

function startTask(planId: string, taskId: string, base?: RunState): RunState {
  const s = base ?? startPlan(planId);
  return applyEvent(s, { type: 'task_started', plan_id: planId, task_id: taskId, phase: 'impl', title: 'My task' }, 1001);
}

// ── parseCheckName ────────────────────────────────────────────────────────────

describe('parseCheckName', () => {
  it('parses "verify[3:compile]" → { index: 3, phase: "compile" }', () => {
    expect(parseCheckName('verify[3:compile]')).toEqual({ index: 3, phase: 'compile' });
  });

  it('parses "verify[2]" → { index: 2, phase: "" }', () => {
    expect(parseCheckName('verify[2]')).toEqual({ index: 2, phase: '' });
  });

  it('returns null index for plain names', () => {
    expect(parseCheckName('cargo test')).toEqual({ index: null, phase: 'cargo test' });
  });
});

// ── taskKey ───────────────────────────────────────────────────────────────────

describe('taskKey', () => {
  it('produces plan/task format', () => {
    expect(taskKey('p1', 't1')).toBe('p1/t1');
  });
});

// ── plan_set_loaded ───────────────────────────────────────────────────────────

describe('plan_set_loaded', () => {
  it('creates plans in pending phase and sets planSet', () => {
    const s = apply([{
      type: 'plan_set_loaded',
      plans: [
        { plan_id: 'p1', title: 'Plan One', tasks_total: 3 },
        { plan_id: 'p2', tasks_total: 2 },
      ],
    }]);
    expect(s.planSet).toMatchObject({ planIds: ['p1', 'p2'], tasksTotal: 5, loadedAtMs: 1000 });
    expect(s.plans['p1']!.phase).toBe('pending');
    expect(s.plans['p1']!.title).toBe('Plan One');
    expect(s.plans['p2']!.phase).toBe('pending');
    expect(s.run.startedAtMs).toBe(1000);
  });

  it('does not overwrite a running plan', () => {
    const s1 = apply([
      { type: 'plan_started', plan_id: 'p1', tasks_total: 3 },
    ]);
    const s2 = applyEvent(s1, {
      type: 'plan_set_loaded',
      plans: [{ plan_id: 'p1', tasks_total: 3 }],
    }, 2000);
    expect(s2.plans['p1']!.phase).toBe('running');
  });
});

// ── plan_started ──────────────────────────────────────────────────────────────

describe('plan_started', () => {
  it('sets phase to running and tasksTotal', () => {
    const s = startPlan('p1', 2000);
    const plan = s.plans['p1']!;
    expect(plan.phase).toBe('running');
    expect(plan.tasksTotal).toBe(3);
    expect(plan.startedAtMs).toBe(2000);
  });

  it('resets run when plan not in planSet', () => {
    const s = apply([{ type: 'plan_started', plan_id: 'p99', tasks_total: 1 }], 3000);
    expect(s.run.startedAtMs).toBe(3000);
    expect(s.run.durationMs).toBeNull();
  });

  it('does not reset run when plan is in planSet', () => {
    const s1 = apply([{
      type: 'plan_set_loaded',
      plans: [{ plan_id: 'p1', tasks_total: 2 }],
    }], 1000);
    const s2 = applyEvent(s1, { type: 'plan_started', plan_id: 'p1', tasks_total: 2 }, 2000);
    // run.startedAtMs was set by plan_set_loaded at 1000 and should NOT be overwritten
    expect(s2.run.startedAtMs).toBe(1000);
  });
});

// ── plan_completed ────────────────────────────────────────────────────────────

describe('plan_completed', () => {
  it('marks plan completed on success', () => {
    const s = applyEvent(startPlan('p1'), { type: 'plan_completed', plan_id: 'p1', success: true }, 9000);
    expect(s.plans['p1']!.phase).toBe('completed');
    expect(s.plans['p1']!.finishedAtMs).toBe(9000);
  });

  it('marks plan failed on failure', () => {
    const s = applyEvent(startPlan('p1'), { type: 'plan_completed', plan_id: 'p1', success: false }, 9000);
    expect(s.plans['p1']!.phase).toBe('failed');
  });
});

// ── run_completed ─────────────────────────────────────────────────────────────

describe('run_completed', () => {
  it('sets durationMs and outcome, marks running plans completed, interrupts active tasks', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'run_completed', outcome: 'succeeded', duration_ms: 5000 }, 9000);
    expect(s.run.durationMs).toBe(5000);
    expect(s.run.outcome).toBe('succeeded');
    expect(s.plans['p1']!.phase).toBe('completed');
    // A task still running did not finish: never passed, counted as failed,
    // as in the server's snapshot (bug-60ccba).
    expect(s.tasks[taskKey('p1', 't1')]).toMatchObject({ status: 'interrupted', finishedAtMs: 9000 });
    expect(s.plans['p1']).toMatchObject({ tasksDone: 0, tasksFailed: 1 });
    // A repeated completion counts nothing more.
    s = applyEvent(s, { type: 'run_completed', outcome: 'failed', duration_ms: 6000 }, 9100);
    expect(s.plans['p1']!.tasksFailed).toBe(1);
  });

  it('cancels active tasks when the run is cancelled, counting nothing', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'run_completed', outcome: 'cancelled', duration_ms: 100 }, 9000);
    expect(s.tasks[taskKey('p1', 't1')]!.status).toBe('cancelled');
    expect(s.plans['p1']).toMatchObject({ phase: 'cancelled', tasksDone: 0, tasksFailed: 0 });
  });

  it('takes an interrupted task back out of the failed count when it starts again', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'run_completed', outcome: 'failed', duration_ms: 100 }, 9000);
    expect(s.plans['p1']!.tasksFailed).toBe(1);
    s = applyEvent(s, { type: 'task_started', plan_id: 'p1', task_id: 't1', phase: 'impl' }, 9100);
    expect(s.tasks[taskKey('p1', 't1')]).toMatchObject({ status: 'active', attempts: 2 });
    expect(s.plans['p1']!.tasksFailed).toBe(0);
  });

  it('does not overwrite durationMs when already set', () => {
    let s = startPlan('p1');
    s = applyEvent(s, { type: 'run_completed', outcome: 'succeeded', duration_ms: 1000 }, 9000);
    s = applyEvent(s, { type: 'run_completed', outcome: 'failed', duration_ms: 2000 }, 10000);
    expect(s.run.durationMs).toBe(1000);
  });

  it('marks running plans cancelled when outcome is cancelled', () => {
    let s = startPlan('p1');
    s = applyEvent(s, { type: 'run_completed', outcome: 'cancelled', duration_ms: 100 }, 9000);
    expect(s.plans['p1']!.phase).toBe('cancelled');
  });

  it('deactivates all agents', () => {
    let s = startPlan('p1');
    s = applyEvent(s, {
      type: 'agent_spawned', agent_id: 'a1', plan_id: 'p1', task_id: 't1', role: 'impl',
    }, 1000);
    s = applyEvent(s, { type: 'run_completed', outcome: 'succeeded', duration_ms: 1 }, 9000);
    expect(s.agents['a1']!.active).toBe(false);
  });
});

// ── task_started ──────────────────────────────────────────────────────────────

describe('task_started', () => {
  it('creates a new task with status active and attempts 1', () => {
    const s = startTask('p1', 't1');
    const task = s.tasks[taskKey('p1', 't1')]!;
    expect(task.status).toBe('active');
    expect(task.attempts).toBe(1);
    expect(task.title).toBe('My task');
  });

  it('retry path: increments attempts, clears checks, appends divider', () => {
    let s = startTask('p1', 't1');
    // Complete the task
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'passed' }, 2000);
    // Add a check to verify it gets cleared
    s = applyEvent(s, { type: 'gate_rung_started', plan_id: 'p1', task_id: 't1', rung_name: 'compile' }, 2001);
    // Retry
    s = applyEvent(s, { type: 'task_started', plan_id: 'p1', task_id: 't1', phase: 'impl', title: '' }, 3000);
    const task = s.tasks[taskKey('p1', 't1')]!;
    expect(task.attempts).toBe(2);
    expect(task.checks).toHaveLength(0);
    const transcript = s.transcripts[taskKey('p1', 't1')]!;
    const last = transcript.entries[transcript.entries.length - 1]!;
    expect(last.kind).toBe('divider');
    if (last.kind === 'divider') {
      expect(last.attempt).toBe(2);
    }
  });
});

// ── task_completed ────────────────────────────────────────────────────────────

describe('task_completed', () => {
  it('classifies "passed" outcome as passed and increments tasksDone', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'passed' }, 2000);
    expect(s.tasks[taskKey('p1', 't1')]!.status).toBe('passed');
    expect(s.plans['p1']!.tasksDone).toBe(1);
  });

  it('classifies "accepted_with_failures" before fail check, increments tasksDone and tasksAccepted', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'accepted_with_failures' }, 2000);
    const task = s.tasks[taskKey('p1', 't1')]!;
    expect(task.status).toBe('accepted_with_failures');
    expect(s.plans['p1']!.tasksDone).toBe(1);
    expect(s.plans['p1']!.tasksAccepted).toBe(1);
  });

  it('classifies outcome containing "fail" as failed', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'gate_failed' }, 2000);
    expect(s.tasks[taskKey('p1', 't1')]!.status).toBe('failed');
    expect(s.plans['p1']!.tasksFailed).toBe(1);
  });

  it('classifies "skipped" and "condition-skipped" as skipped (tasksDone +1)', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'condition-skipped' }, 2000);
    expect(s.tasks[taskKey('p1', 't1')]!.status).toBe('skipped');
    expect(s.plans['p1']!.tasksDone).toBe(1);
  });

  it('classifies "already_satisfied" as its own status: done, but not passed', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'already_satisfied' }, 2000);
    expect(s.tasks[taskKey('p1', 't1')]!.status).toBe('already_satisfied');
    expect(s.plans['p1']!.tasksDone).toBe(1);
    expect(s.plans['p1']!.tasksUnverified ?? 0).toBe(0);
  });

  it('classifies "passed_with_preexisting_failures" as its own pass, counted as done', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(
      s,
      { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'passed_with_preexisting_failures' },
      2000,
    );
    expect(s.tasks[taskKey('p1', 't1')]!.status).toBe('passed_with_preexisting_failures');
    expect(s.plans['p1']).toMatchObject({ tasksDone: 1, tasksFailed: 0, tasksAccepted: 0 });
    expect(s.plans['p1']!.tasksUnverified ?? 0).toBe(0);

    // A retry takes it back out of the done count, like any pass.
    s = applyEvent(s, { type: 'task_started', plan_id: 'p1', task_id: 't1', phase: 'impl' }, 2100);
    expect(s.tasks[taskKey('p1', 't1')]).toMatchObject({ status: 'active', attempts: 2 });
    expect(s.plans['p1']!.tasksDone).toBe(0);
  });

  it('classifies "interrupted" as its own status, counted as failed', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'interrupted' }, 2000);
    expect(s.tasks[taskKey('p1', 't1')]!.status).toBe('interrupted');
    expect(s.plans['p1']).toMatchObject({ tasksDone: 0, tasksFailed: 1 });
  });

  it('classifies "blocked" as skipped, counted as neither done nor failed', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'blocked' }, 2000);
    expect(s.tasks[taskKey('p1', 't1')]!.status).toBe('skipped');
    expect(s.plans['p1']).toMatchObject({ tasksDone: 0, tasksFailed: 0 });
  });

  it('classifies "unverified" and unrecognised outcomes as unverified, never passed', () => {
    for (const outcome of ['unverified', 'completed']) {
      let s = startTask('p1', 't1');
      s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome }, 2000);
      expect(s.tasks[taskKey('p1', 't1')]!.status, outcome).toBe('unverified');
      expect(s.plans['p1']!.tasksDone, outcome).toBe(1);
      expect(s.plans['p1']!.tasksUnverified, outcome).toBe(1);
    }
  });

  it('is idempotent — second task_completed on a terminal task is ignored', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'passed' }, 2000);
    const before = JSON.stringify(s);
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'passed' }, 3000);
    expect(JSON.stringify(s)).toBe(before);
  });

  it('applies a task completion it never saw start', () => {
    // Skipped tasks never start, and a page that joins mid-run or drops an
    // event misses the start: the completion still counts, with no start time.
    let s = apply([{ type: 'plan_started', plan_id: 'p1', tasks_total: 1 }]);
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'passed' }, 2000);
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't2', outcome: 'skipped' }, 2100);
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't3', outcome: 'gate_failed' }, 2200);
    expect(s.tasks[taskKey('p1', 't1')]).toMatchObject({
      planId: 'p1',
      taskId: 't1',
      status: 'passed',
      phase: 'completed',
      attempts: 1,
      startedAtMs: null,
      finishedAtMs: 2000,
    });
    expect(s.tasks[taskKey('p1', 't2')]!.status).toBe('skipped');
    expect(s.tasks[taskKey('p1', 't3')]!.status).toBe('failed');
    expect(s.plans['p1']).toMatchObject({ tasksDone: 2, tasksFailed: 1, tasksTotal: 3 });

    // A repeat is still ignored, and a retry takes the task out of its count.
    expect(applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't3', outcome: 'passed' }, 2300)).toBe(s);
    s = applyEvent(s, { type: 'task_started', plan_id: 'p1', task_id: 't3', phase: 'impl', title: 'Retry' }, 2400);
    expect(s.tasks[taskKey('p1', 't3')]).toMatchObject({ status: 'active', attempts: 2 });
    expect(s.plans['p1']).toMatchObject({ tasksDone: 2, tasksFailed: 0 });

    // With no plan record the task is still kept; there is no count to move.
    const lone = applyEvent(initialRunState(), { type: 'task_completed', plan_id: 'p9', task_id: 't1', outcome: 'passed' }, 2000);
    expect(lone.tasks[taskKey('p9', 't1')]!.status).toBe('passed');
    expect(lone.plans['p9']).toBeUndefined();
  });
});

// ── task_blocked ──────────────────────────────────────────────────────────────

describe('task_blocked', () => {
  const blockT4: WireDashboardEvent = {
    type: 'task_blocked',
    plan_id: 'p1',
    task_id: 't4',
    title: 'Fourth',
    blocked_by: 't1',
    reason: "blocked by failed task 't1'",
  };

  it('lists a task that never started with its blocker, counted as neither done nor failed', () => {
    let s = startPlan('p1');
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'failed' }, 2000);
    s = applyEvent(s, blockT4, 2100);
    expect(s.tasks[taskKey('p1', 't4')]).toMatchObject({
      title: 'Fourth',
      status: 'skipped',
      phase: 'blocked',
      blockedBy: 't1',
      blockedReason: "blocked by failed task 't1'",
    });
    expect(s.plans['p1']).toMatchObject({ tasksDone: 0, tasksFailed: 1, tasksTotal: 3 });
  });

  it('takes back the count of a task the status poll first reported skipped', () => {
    let s = startPlan('p1');
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't4', outcome: 'skipped' }, 2000);
    expect(s.plans['p1']!.tasksDone).toBe(1);
    s = applyEvent(s, blockT4, 2100);
    expect(s.plans['p1']!.tasksDone).toBe(0);
    // A repeat counts nothing.
    s = applyEvent(s, blockT4, 2200);
    expect(s.plans['p1']!.tasksDone).toBe(0);
    expect(s.tasks[taskKey('p1', 't4')]!.title).toBe('Fourth');
  });

  it('counts the outcome of a later run that settles the blocked task', () => {
    let s = applyEvent(startPlan('p1'), blockT4, 2000);
    s = applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't4', outcome: 'passed' }, 2100);
    expect(s.tasks[taskKey('p1', 't4')]).toMatchObject({ status: 'passed', blockedBy: null });
    expect(s.plans['p1']!.tasksDone).toBe(1);

    // A blocked task that starts is on its first attempt, with nothing to take back.
    let started = applyEvent(startPlan('p1'), blockT4, 2000);
    started = applyEvent(started, { type: 'task_started', plan_id: 'p1', task_id: 't4', phase: 'impl' }, 2100);
    expect(started.tasks[taskKey('p1', 't4')]).toMatchObject({ status: 'active', attempts: 1 });
    expect(started.plans['p1']!.tasksDone).toBe(0);
  });
});

// ── agent events ──────────────────────────────────────────────────────────────

describe('agent_spawned / agent_completed / agent_heartbeat', () => {
  it('agent_spawned creates agent and links to task', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'agent_spawned', agent_id: 'a1', plan_id: 'p1', task_id: 't1', role: 'impl', model: 'claude' }, 2000);
    expect(s.agents['a1']!.active).toBe(true);
    expect(s.agents['a1']!.role).toBe('impl');
    expect(s.tasks[taskKey('p1', 't1')]!.agentId).toBe('a1');
    expect(s.tasks[taskKey('p1', 't1')]!.model).toBe('claude');
  });

  it('agent_completed deactivates the agent', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'agent_spawned', agent_id: 'a1', plan_id: 'p1', task_id: 't1', role: 'impl' }, 1000);
    s = applyEvent(s, { type: 'agent_completed', agent_id: 'a1' }, 2000);
    expect(s.agents['a1']!.active).toBe(false);
  });

  it('agent_heartbeat fills spawnedAtMs from elapsed_ms when unknown', () => {
    const s = apply([{
      type: 'agent_heartbeat', agent_id: 'a1', plan_id: 'p1', task_id: 't1', elapsed_ms: 500,
    }], 5000);
    expect(s.agents['a1']!.spawnedAtMs).toBe(4500); // 5000 - 500
  });

  it('agent_heartbeat does not overwrite known spawnedAtMs', () => {
    let s = apply([{
      type: 'agent_heartbeat', agent_id: 'a1', plan_id: 'p1', task_id: 't1', elapsed_ms: 500,
    }], 5000);
    s = applyEvent(s, { type: 'agent_heartbeat', agent_id: 'a1', plan_id: 'p1', task_id: 't1', elapsed_ms: 1000 }, 6000);
    expect(s.agents['a1']!.spawnedAtMs).toBe(4500); // unchanged
  });
});

// ── agent_output / transcript capping ────────────────────────────────────────

describe('agent_output and transcript capping', () => {
  it('appends decoded entries to transcript keyed by plan/task', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'agent_output', agent_id: 'a1', plan_id: 'p1', task_id: 't1', content: 'hello' }, 1000);
    const t = s.transcripts[taskKey('p1', 't1')]!;
    expect(t.entries).toHaveLength(1);
    expect(t.entries[0]).toEqual({ kind: 'raw', text: 'hello', malformed: false });
  });

  it('caps transcript at MAX_TRANSCRIPT_ENTRIES, incrementing dropped', () => {
    let s = startTask('p1', 't1');
    for (let i = 0; i < MAX_TRANSCRIPT_ENTRIES + 3; i++) {
      s = applyEvent(s, { type: 'agent_output', agent_id: 'a1', plan_id: 'p1', task_id: 't1', content: `line${i}` }, 1000);
    }
    const t = s.transcripts[taskKey('p1', 't1')]!;
    expect(t.entries).toHaveLength(MAX_TRANSCRIPT_ENTRIES);
    expect(t.dropped).toBe(3);
  });
});

// ── gate events ───────────────────────────────────────────────────────────────

describe('gate events', () => {
  it('gate_rung_started creates a running check with correct index/phase', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'gate_rung_started', plan_id: 'p1', task_id: 't1', rung_name: 'verify[1:compile]' }, 2000);
    const checks = s.tasks[taskKey('p1', 't1')]!.checks;
    expect(checks).toHaveLength(1);
    expect(checks[0]).toMatchObject({ name: 'verify[1:compile]', index: 1, phase: 'compile', status: 'running', output: '' });
  });

  it('gate_output_line appends to check output (creating the check if needed)', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'gate_output_line', plan_id: 'p1', task_id: 't1', gate: 'compile', line: 'ok' }, 2000);
    const checks = s.tasks[taskKey('p1', 't1')]!.checks;
    expect(checks[0]!.output).toBe('ok\n');
  });

  it('gate_result updates status and output', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'gate_rung_started', plan_id: 'p1', task_id: 't1', rung_name: 'compile' }, 2000);
    s = applyEvent(s, { type: 'gate_result', plan_id: 'p1', task_id: 't1', gate: 'compile', passed: false, output_text: 'error: type mismatch' }, 2001);
    const check = s.tasks[taskKey('p1', 't1')]!.checks.find((c) => c.name === 'compile')!;
    expect(check.status).toBe('failed');
    expect(check.output).toBe('error: type mismatch');
  });

  it('checks are sorted by index, nulls last', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'gate_rung_started', plan_id: 'p1', task_id: 't1', rung_name: 'plain' }, 2000);
    s = applyEvent(s, { type: 'gate_rung_started', plan_id: 'p1', task_id: 't1', rung_name: 'verify[2]' }, 2001);
    s = applyEvent(s, { type: 'gate_rung_started', plan_id: 'p1', task_id: 't1', rung_name: 'verify[1]' }, 2002);
    const names = s.tasks[taskKey('p1', 't1')]!.checks.map((c) => c.name);
    expect(names).toEqual(['verify[1]', 'verify[2]', 'plain']);
  });
});

// ── efficiency_event ──────────────────────────────────────────────────────────

describe('efficiency_event', () => {
  it('adds input_tokens to task, agent, totals and usage', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'agent_spawned', agent_id: 'a1', plan_id: 'p1', task_id: 't1', role: 'impl' }, 1000);
    s = applyEvent(s, { type: 'efficiency_event', plan_id: 'p1', task_id: 't1', metric: 'input_tokens', value: 42 }, 2000);
    expect(s.tasks[taskKey('p1', 't1')]!.inputTokens).toBe(42);
    expect(s.agents['a1']!.inputTokens).toBe(42);
    expect(s.totals.inputTokens).toBe(42);
    expect(s.usage).toHaveLength(1);
    expect(s.usage[0]).toEqual({ atMs: 2000, tokens: 42 });
  });

  it('adds cost_usd to task, agent, plan, totals but not usage', () => {
    let s = startTask('p1', 't1');
    s = applyEvent(s, { type: 'agent_spawned', agent_id: 'a1', plan_id: 'p1', task_id: 't1', role: 'impl' }, 1000);
    s = applyEvent(s, { type: 'efficiency_event', plan_id: 'p1', task_id: 't1', metric: 'cost_usd', value: 0.5 }, 2000);
    expect(s.tasks[taskKey('p1', 't1')]!.costUsd).toBeCloseTo(0.5);
    expect(s.plans['p1']!.costUsd).toBeCloseTo(0.5);
    expect(s.totals.costUsd).toBeCloseTo(0.5);
    expect(s.usage).toHaveLength(0); // cost does not go to usage ring
  });

  it('usage ring is capped at MAX_USAGE_SAMPLES', () => {
    let s = startTask('p1', 't1');
    for (let i = 0; i < MAX_USAGE_SAMPLES + 5; i++) {
      s = applyEvent(s, { type: 'efficiency_event', plan_id: 'p1', task_id: 't1', metric: 'input_tokens', value: 1 }, 1000 + i);
    }
    expect(s.usage).toHaveLength(MAX_USAGE_SAMPLES);
  });

  it('ignores unknown metric names', () => {
    let s = startTask('p1', 't1');
    const before = JSON.stringify(s);
    s = applyEvent(s, { type: 'efficiency_event', plan_id: 'p1', task_id: 't1', metric: 'latency_ms', value: 100 }, 2000);
    expect(JSON.stringify(s)).toBe(before);
  });
});

// ── critical_path_eta_updated ─────────────────────────────────────────────────

describe('critical_path_eta_updated', () => {
  it('sets etaMinutes on the given plan', () => {
    let s = startPlan('p1');
    s = applyEvent(s, { type: 'critical_path_eta_updated', plan_id: 'p1', eta_minutes: 12 }, 1000);
    expect(s.plans['p1']!.etaMinutes).toBe(12);
  });
});

// ── error ─────────────────────────────────────────────────────────────────────

describe('error event', () => {
  it('appends errors and caps at MAX_ERRORS', () => {
    let s = initialRunState();
    for (let i = 0; i < MAX_ERRORS + 2; i++) {
      s = applyEvent(s, { type: 'error', message: `err${i}` }, 1000 + i);
    }
    expect(s.errors).toHaveLength(MAX_ERRORS);
    // Last error should be the most recent
    expect(s.errors[s.errors.length - 1]!.message).toBe(`err${MAX_ERRORS + 1}`);
  });
});

// ── snapshot_rebased / unknown ────────────────────────────────────────────────

describe('snapshot_rebased and unknown events', () => {
  it('snapshot_rebased returns state unchanged', () => {
    const s = startPlan('p1');
    const s2 = applyEvent(s, { type: 'snapshot_rebased', revision: 42 }, 9000);
    expect(s2).toBe(s);
  });

  it('unknown event type returns state unchanged', () => {
    const s = startPlan('p1');
    // Cast to force an unknown type through
    const s2 = applyEvent(s, { type: 'snapshot_rebased', revision: 0 } as WireDashboardEvent, 9000);
    expect(s2).toBe(s);
  });
});

// ── two plans running at once ─────────────────────────────────────────────────

describe('two plans running concurrently', () => {
  it('interleaved efficiency events credit the right plan and tasks', () => {
    let s = initialRunState();
    s = applyEvent(s, { type: 'plan_started', plan_id: 'pa', tasks_total: 1 }, 1000);
    s = applyEvent(s, { type: 'plan_started', plan_id: 'pb', tasks_total: 1 }, 1000);
    s = applyEvent(s, { type: 'task_started', plan_id: 'pa', task_id: 'ta', phase: 'impl' }, 1000);
    s = applyEvent(s, { type: 'task_started', plan_id: 'pb', task_id: 'tb', phase: 'impl' }, 1000);
    s = applyEvent(s, { type: 'agent_spawned', agent_id: 'aa', plan_id: 'pa', task_id: 'ta', role: 'impl' }, 1000);
    s = applyEvent(s, { type: 'agent_spawned', agent_id: 'ab', plan_id: 'pb', task_id: 'tb', role: 'impl' }, 1000);

    s = applyEvent(s, { type: 'efficiency_event', plan_id: 'pa', task_id: 'ta', metric: 'cost_usd', value: 0.1 }, 2000);
    s = applyEvent(s, { type: 'efficiency_event', plan_id: 'pb', task_id: 'tb', metric: 'cost_usd', value: 0.2 }, 2000);

    expect(s.plans['pa']!.costUsd).toBeCloseTo(0.1);
    expect(s.plans['pb']!.costUsd).toBeCloseTo(0.2);
    expect(s.totals.costUsd).toBeCloseTo(0.3);

    // Each plan completes independently
    s = applyEvent(s, { type: 'plan_completed', plan_id: 'pa', success: true }, 9000);
    expect(s.plans['pa']!.phase).toBe('completed');
    expect(s.plans['pb']!.phase).toBe('running');
  });
});

// ── input not mutated ─────────────────────────────────────────────────────────

describe('applyEvent purity', () => {
  it('does not mutate the input state', () => {
    const s = startTask('p1', 't1');
    const before = JSON.parse(JSON.stringify(s)) as RunState;
    applyEvent(s, { type: 'task_completed', plan_id: 'p1', task_id: 't1', outcome: 'passed' }, 2000);
    expect(s).toEqual(before);
  });
});

// ── fromSnapshot ──────────────────────────────────────────────────────────────

describe('fromSnapshot', () => {
  function makeSnapshot(): WireDashboardSnapshot {
    return {
      plans: {
        p1: { plan_id: 'p1', phase: 'running', tasks_total: 2, tasks_done: 1, tasks_failed: 0, active: true },
        p2: { plan_id: 'p2', phase: 'completed', tasks_total: 1, tasks_done: 1, tasks_failed: 0, active: false },
      },
      plan_set: {
        plans: [{ plan_id: 'p1', tasks_total: 2 }, { plan_id: 'p2', tasks_total: 1 }],
        tasks_total: 3,
        loaded_at_ms: 500,
      },
      tasks: {
        'p1/t1': { task_id: 't1', plan_id: 'p1', phase: 'impl', outcome: null, title: 'Task One' },
        'p1/t2': { task_id: 't2', plan_id: 'p1', phase: 'gate', outcome: 'passed', title: '' },
      },
      agents: {
        a1: {
          agent_id: 'a1', role: 'impl', active: true, model: 'claude',
          provider: 'anthropic', input_tokens: 10, output_tokens: 5,
          cost_usd: 0.01, current_task: 't1', current_plan: 'p1',
          attempt: 1, spawned_at_ms: 1234, elapsed_ms: 0,
        },
      },
      gates: [
        { plan_id: 'p1', task_id: 't2', gate: 'compile', passed: true, ts_millis: 800 },
      ],
      task_gate_outputs: [
        { plan_id: 'p1', task_id: 't2', gate: 'compile', passed: true, lines: ['OK', 'done'] },
      ],
      task_outputs: {
        't1': ['hello'],
        // t99 has no plan → ambiguous → skipped (not in tasks at all)
      },
      errors: [{ message: 'oops', ts_millis: 300 }],
      stats: {
        cost_usd_total: 0.05,
        total_input_tokens: 100,
        total_output_tokens: 50,
      },
      run_duration_ms: 8000,
      run_outcome: 'succeeded',
      critical_path_eta_minutes: 3,
    };
  }

  it('maps plan phases correctly (active→running, completed→completed)', () => {
    const s = fromSnapshot(makeSnapshot(), 9000);
    expect(s.plans['p1']!.phase).toBe('running');
    expect(s.plans['p2']!.phase).toBe('completed');
  });

  it('sets planSet from snapshot.plan_set', () => {
    const s = fromSnapshot(makeSnapshot(), 9000);
    expect(s.planSet).toMatchObject({ planIds: ['p1', 'p2'], tasksTotal: 3, loadedAtMs: 500 });
  });

  it('maps tasks: null outcome → active, "passed" → passed', () => {
    const s = fromSnapshot(makeSnapshot(), 9000);
    expect(s.tasks[taskKey('p1', 't1')]!.status).toBe('active');
    expect(s.tasks[taskKey('p1', 't2')]!.status).toBe('passed');
  });

  it('links agent to task', () => {
    const s = fromSnapshot(makeSnapshot(), 9000);
    expect(s.tasks[taskKey('p1', 't1')]!.agentId).toBe('a1');
  });

  it('maps agent spawned_at_ms 0 to null', () => {
    const snap = makeSnapshot();
    snap.agents['a1']!.spawned_at_ms = 0;
    const s = fromSnapshot(snap, 9000);
    expect(s.agents['a1']!.spawnedAtMs).toBeNull();
  });

  it('keeps a pass over pre-existing failures apart from a clean pass', () => {
    const snap = makeSnapshot();
    snap.tasks['p1/t2']!.outcome = 'passed_with_preexisting_failures';
    const s = fromSnapshot(snap, 9000);
    expect(s.tasks[taskKey('p1', 't2')]!.status).toBe('passed_with_preexisting_failures');
  });

  it('keeps an interrupted task apart from a failed one', () => {
    const snap = makeSnapshot();
    snap.tasks['p1/t2']!.outcome = 'interrupted';
    const s = fromSnapshot(snap, 9000);
    expect(s.tasks[taskKey('p1', 't2')]!.status).toBe('interrupted');
  });

  it('keeps the blocker of a blocked task, shown as skipped', () => {
    const snap = makeSnapshot();
    snap.tasks['p1/t3'] = {
      task_id: 't3',
      plan_id: 'p1',
      phase: 'blocked',
      outcome: 'blocked',
      title: 'Third',
      blocked_by: 't1',
      blocked_reason: "blocked by failed task 't1'",
    };
    const s = fromSnapshot(snap, 9000);
    expect(s.tasks[taskKey('p1', 't3')]).toMatchObject({
      status: 'skipped',
      blockedBy: 't1',
      blockedReason: "blocked by failed task 't1'",
    });
    expect(s.tasks[taskKey('p1', 't2')]!.blockedBy).toBeUndefined();
  });

  it('populates checks from gates with output from task_gate_outputs', () => {
    const s = fromSnapshot(makeSnapshot(), 9000);
    const task = s.tasks[taskKey('p1', 't2')]!;
    expect(task.checks).toHaveLength(1);
    expect(task.checks[0]!.status).toBe('passed');
    expect(task.checks[0]!.output).toBe('OK\ndone');
  });

  it('builds transcripts from task_outputs', () => {
    const s = fromSnapshot(makeSnapshot(), 9000);
    const t = s.transcripts[taskKey('p1', 't1')];
    expect(t).toBeDefined();
    expect(t!.entries).toHaveLength(1);
  });

  it('maps errors from snapshot', () => {
    const s = fromSnapshot(makeSnapshot(), 9000);
    expect(s.errors).toEqual([{ message: 'oops', atMs: 300 }]);
  });

  it('maps totals from stats', () => {
    const s = fromSnapshot(makeSnapshot(), 9000);
    expect(s.totals.costUsd).toBe(0.05);
    expect(s.totals.inputTokens).toBe(100);
    expect(s.totals.outputTokens).toBe(50);
  });

  it('sets run.startedAtMs from plan_set.loaded_at_ms and durationMs from run_duration_ms', () => {
    const s = fromSnapshot(makeSnapshot(), 9000);
    expect(s.run.startedAtMs).toBe(500);
    expect(s.run.durationMs).toBe(8000);
    expect(s.run.outcome).toBe('succeeded');
  });

  it('sets etaMinutes on the single running plan', () => {
    const s = fromSnapshot(makeSnapshot(), 9000);
    expect(s.plans['p1']!.etaMinutes).toBe(3);
    expect(s.plans['p2']!.etaMinutes).toBeNull();
  });

  it('usage starts empty', () => {
    const s = fromSnapshot(makeSnapshot(), 9000);
    expect(s.usage).toHaveLength(0);
  });

  it('does not assign eta when multiple plans are running', () => {
    const snap = makeSnapshot();
    // Make p2 also active/running
    snap.plans['p2']!.active = true;
    const s = fromSnapshot(snap, 9000);
    expect(s.plans['p1']!.etaMinutes).toBeNull();
    expect(s.plans['p2']!.etaMinutes).toBeNull();
  });
});
