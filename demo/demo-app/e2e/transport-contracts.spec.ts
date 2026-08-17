import { expect, test } from '@playwright/test';
import { useDataHub } from '../src/app/DataHub';
import { parseBenchSSEEvent } from '../src/lib/bench-types';
import { SseAdapter } from '../src/transport/sse';
import {
  parseDashboardEvent,
  parseDashboardGap,
  type DashboardSnapshot,
} from '../src/transport/types';

function emptySnapshot(): DashboardSnapshot {
  return {
    plans: {},
    tasks: {},
    agents: {},
    gates: [],
    diagnoses: [],
    experiment_winners: [],
    agent_topology: { nodes: [], edges: [], timestamp: 0 },
    efficiency_trend: [],
    cfactor_trend: [],
    gate_trends: {},
    gate_recent_failures: [],
    episodes: [],
    errors: [],
    event_log: [],
    task_outputs: {},
    cascade_router_json: '',
    gate_thresholds_json: '',
    marketplace_jobs: [],
    atelier_prds: [],
    atelier_tasks: {},
    knowledge_entries: [],
    payment_count: 0,
    total_payment_korai: 0,
    payments_by_protocol: {},
    settlement_count: 0,
    inbox_items: {},
    inbox_resolved_ids: [],
    inbox_pending_count: 0,
    stats: {
      plans_active: 0,
      plans_completed: 0,
      plans_failed: 0,
      tasks_active: 0,
      tasks_completed: 0,
      tasks_failed: 0,
      agents_active: 0,
      gates_passed: 0,
      gates_failed: 0,
      errors_total: 0,
      episodes_total: 0,
      cost_usd_total: 0,
    },
  };
}

test.describe.configure({ mode: 'serial' });

test('recognized dashboard and bench frames require their complete wire shape', () => {
  expect(parseDashboardEvent({ type: 'plan_completed', plan_id: 'p', success: true }))
    .toEqual({ type: 'plan_completed', plan_id: 'p', success: true });
  expect(parseDashboardEvent({ type: 'plan_completed', plan_id: 'p', success: 'true' }))
    .toBeNull();

  expect(parseBenchSSEEvent({
    type: 'MatrixRunCompleted',
    matrix_id: 'matrix-1',
    summary: [{ lane_id: 'lane-1', pass_rate: 1, cost_usd: 0.25 }],
  })).not.toBeNull();
  expect(parseBenchSSEEvent({
    type: 'MatrixRunCompleted',
    matrix_id: 'matrix-1',
    summary: [{ lane_id: 'lane-1', pass_rate: 1 }],
  })).toBeNull();
  expect(parseBenchSSEEvent({
    type: 'BenchProgress',
    bench_id: 'bench-1',
    completed: '1',
    total: 2,
    cost_so_far: 0,
  })).toBeNull();
});

test('validated gap snapshot hydrates DataHub in one state transition', () => {
  const snapshot = emptySnapshot();
  snapshot.plans['plan-1'] = {
    plan_id: 'plan-1',
    phase: 'execute',
    tasks_total: 2,
    tasks_done: 1,
    tasks_failed: 0,
    active: true,
  };
  snapshot.agents['agent-1'] = {
    agent_id: 'agent-1',
    role: 'implementer',
    active: true,
    output_bytes: 10,
    model: 'test-model',
    input_tokens: 5,
    output_tokens: 7,
    cache_read_tokens: 0,
    cache_write_tokens: 0,
    cost_usd: 0.1,
    current_task: 'task-1',
    current_plan: 'plan-1',
    attempt: 1,
    spawned_at_ms: 1,
    last_event_at_ms: 2,
  };
  snapshot.stats.cost_usd_total = 0.1;

  const gap = parseDashboardGap({
    type: 'gap',
    missed_events: 4,
    last_materialized_seq: 42,
    snapshot,
  });
  expect(gap).not.toBeNull();

  let transitions = 0;
  const unsubscribe = useDataHub.subscribe(() => { transitions += 1; });
  useDataHub.getState().hydrateDashboardSnapshot(gap!.snapshot, gap!);
  unsubscribe();

  expect(transitions).toBe(1);
  expect(useDataHub.getState()).toMatchObject({
    activePlanId: 'plan-1',
    activePhase: 'execute',
    totalCost: 0.1,
    totalTokens: 12,
    dashboardMissedEvents: 4,
    dashboardLastMaterializedSeq: 42,
  });

  expect(parseDashboardGap({
    type: 'gap',
    missed_events: 4,
    last_materialized_seq: 42,
    snapshot: { ...snapshot, stats: { ...snapshot.stats, plans_active: '1' } },
  })).toBeNull();
});

test('named gap lastEventId is retained and sent on reconnect', async () => {
  class FakeEventSource {
    static instances: FakeEventSource[] = [];

    readonly url: string;
    onopen: (() => void) | null = null;
    onmessage: ((event: MessageEvent) => void) | null = null;
    onerror: (() => void) | null = null;
    private listeners = new Map<string, Array<(event: MessageEvent) => void>>();

    constructor(url: string | URL) {
      this.url = String(url);
      FakeEventSource.instances.push(this);
    }

    addEventListener(type: string, listener: (event: MessageEvent) => void) {
      this.listeners.set(type, [...(this.listeners.get(type) ?? []), listener]);
    }

    emit(type: string, data: unknown, lastEventId: string) {
      const event = { data: JSON.stringify(data), lastEventId } as MessageEvent;
      for (const listener of this.listeners.get(type) ?? []) listener(event);
    }

    close() {}
  }

  const original = globalThis.EventSource;
  globalThis.EventSource = FakeEventSource as unknown as typeof EventSource;
  const received: Record<string, unknown>[] = [];
  const adapter = new SseAdapter({
    url: 'http://localhost/api/events',
    onEvent: (event) => received.push(event),
    onStatusChange: () => {},
    baseBackoffMs: 1,
    maxBackoffMs: 1,
  });

  try {
    adapter.connect();
    FakeEventSource.instances[0].emit('gap', {
      missed_events: 3,
      last_materialized_seq: 42,
      snapshot: emptySnapshot(),
    }, '42');

    expect(adapter.lastEventId).toBe('42');
    expect(received[0]?.type).toBe('gap');

    FakeEventSource.instances[0].onerror?.();
    await new Promise((resolve) => setTimeout(resolve, 10));
    expect(FakeEventSource.instances).toHaveLength(2);
    expect(FakeEventSource.instances[1].url).toContain('lastEventId=42');
  } finally {
    adapter.destroy();
    globalThis.EventSource = original;
  }
});
