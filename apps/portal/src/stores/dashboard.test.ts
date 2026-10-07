import { beforeEach, describe, expect, it } from 'vitest';
import { initialRunState } from '@/lib/runState';
import { useDashboardStore } from './dashboard';

const startedAtMs = (planId: string) => useDashboardStore.getState().run.plans[planId]?.startedAtMs;

describe('applyEvent', () => {
  beforeEach(() => useDashboardStore.setState({ run: initialRunState() }));

  it('folds an event at the time the server stamped on its frame (gap-8a1fb3)', () => {
    const { applyEvent } = useDashboardStore.getState();
    applyEvent({ type: 'plan_started', plan_id: 'p1', tasks_total: 1, ts_millis: 1_234 });
    expect(startedAtMs('p1')).toBe(1_234);
  });

  it('folds an unstamped event at the time it arrives', () => {
    const before = Date.now();
    const { applyEvent } = useDashboardStore.getState();
    applyEvent({ type: 'plan_started', plan_id: 'p2', tasks_total: 1 });
    expect(startedAtMs('p2')).toBeGreaterThanOrEqual(before);
  });
});
