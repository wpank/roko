// @vitest-environment jsdom
/**
 * Acceptance: a task links the agent that was announced for it even when the
 * announcement came first. The dispatcher publishes agent_spawned just before
 * task_started, so the fold found no task to link, and the pane never said
 * "agent working" nor marked a live step as live (found by 09's browser smoke
 * on the merged tree).
 * Copied verbatim from plans/portal-programme/08g-first-run/accept/.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup } from '@testing-library/react';
import type { WireDashboardEvent, WirePlanTasks } from '@/api/contracts';
import { queryKeys } from '@/api/queries';
import { StreamPane } from '@/components/stream/StreamPane';
import { initialRunState, taskKey } from '@/lib/runState';
import { STREAM_RECORD_PREFIX } from '@/lib/streamRecord';
import { foldEvents, renderWithClient, setStore, textOf } from '@/test/dom';

beforeEach(() => setStore(initialRunState()));
afterEach(() => cleanup());

const spawned = (agent: string, task: string): WireDashboardEvent => ({
  type: 'agent_spawned', agent_id: agent, plan_id: 'hello', task_id: task, role: 'implementer', model: 'claude-sonnet-4-6',
});
const started = (task: string): WireDashboardEvent => ({ type: 'task_started', plan_id: 'hello', task_id: task, phase: 'graph-executing' });
const step: WireDashboardEvent = {
  type: 'agent_output',
  agent_id: 'hello/T01',
  plan_id: 'hello',
  task_id: 'T01',
  content: STREAM_RECORD_PREFIX + JSON.stringify({ kind: 'tool_start', live: true, agent_id: 'hello/T01', plan_id: 'hello', task_id: 'T01', attempt: 0, payload: { tool_id: 'toolu_1', tool: 'Write', target: 'hello/main.rs' } }),
};
const PLAN_STARTED: WireDashboardEvent = { type: 'plan_started', plan_id: 'hello', tasks_total: 2 };

describe('the fold', () => {
  it('builds the same task record whichever of agent_spawned and task_started comes first', () => {
    const pick = (events: WireDashboardEvent[]) => {
      const t = foldEvents(events).tasks[taskKey('hello', 'T01')]!;
      return { agentId: t.agentId, role: t.role, model: t.model, status: t.status };
    };
    expect(pick([PLAN_STARTED, spawned('hello/T01', 'T01'), started('T01')])).toEqual(
      pick([PLAN_STARTED, started('T01'), spawned('hello/T01', 'T01')]),
    );
  });

  it('links a task to the agent announced before it started', () => {
    const task = foldEvents([PLAN_STARTED, spawned('hello/T01', 'T01'), started('T01')]).tasks[taskKey('hello', 'T01')]!;
    expect(task.agentId).toBe('hello/T01');
    expect(task.role).toBe('implementer');
    expect(task.model).toBe('claude-sonnet-4-6');
  });

  it('links a retry to its new agent, not the finished one', () => {
    const run = foldEvents([
      PLAN_STARTED,
      spawned('a1', 'T01'),
      started('T01'),
      { type: 'agent_completed', agent_id: 'a1', plan_id: 'hello', task_id: 'T01' },
      { type: 'task_completed', plan_id: 'hello', task_id: 'T01', outcome: 'failed' },
      spawned('a2', 'T01'),
      started('T01'),
    ]);
    expect(run.tasks[taskKey('hello', 'T01')]!.agentId).toBe('a2');
  });

  it('never links another task’s agent', () => {
    const run = foldEvents([PLAN_STARTED, spawned('hello/T02', 'T02'), started('T01')]);
    expect(run.tasks[taskKey('hello', 'T01')]!.agentId).toBeNull();
  });
});

describe('the stream', () => {
  it('says agent working and marks the live step while that agent runs', () => {
    const tasks: WirePlanTasks = {
      plan_id: 'hello',
      task_count: 1,
      max_parallel: 1,
      tasks: [{ id: 'T01', title: 'Write the hello world program', tier: 'mechanical', status: 'pending', depends_on: [], files: ['hello/main.rs'], completed: false, verify_phases: ['build'] }],
    };
    setStore(foldEvents([PLAN_STARTED, spawned('hello/T01', 'T01'), started('T01'), step]));
    renderWithClient(<StreamPane planId="hello" selectedTaskId="T01" open onToggle={() => {}} />, {
      seed: [
        [queryKeys.planTasks('hello'), tasks],
        [queryKeys.workspace, { name: 'hello', path: '/tmp/hello', branch: null }],
      ],
    });
    const transcript = document.querySelector('[data-region="transcript"]')!;
    expect(textOf(transcript)).toContain('agent working');
    expect(transcript.querySelector('[data-step] [data-live]')).not.toBeNull();
  });
});
