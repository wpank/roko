/**
 * The status chip says the one most urgent thing, first match wins: off, waiting for you, failed, running, idle.
 */
import { describe, expect, it } from 'vitest';
import type { WireDashboardEvent } from '@/api/contracts';
import { statusChip } from '@/lib/statusChip';
import { initialRunState } from '@/lib/runState';
import { foldEvents } from '@/test/dom';

const T0 = 1_750_000_000_000;
const NONE = {};

const running = (id: string): WireDashboardEvent[] => [
  { type: 'plan_started', plan_id: id, tasks_total: 2 },
  { type: 'task_started', plan_id: id, task_id: 'T1', phase: 'implement' },
];
const failed = (id: string): WireDashboardEvent[] => [
  ...running(id),
  { type: 'task_completed', plan_id: id, task_id: 'T1', outcome: 'failed' },
  { type: 'plan_completed', plan_id: id, success: false },
];

describe('the status chip', () => {
  it('is idle when nothing is going on', () => {
    expect(statusChip(initialRunState(), 'connected', { nowMs: T0, seenFailures: NONE })).toMatchObject({
      signal: 'off',
      word: 'idle',
      planId: null,
    });
  });

  it('is off when roko serve is not answering', () => {
    const run = foldEvents(running('a'), T0);
    expect(statusChip(run, 'disconnected', { nowMs: T0, seenFailures: NONE })).toMatchObject({ signal: 'off', word: 'off' });
  });

  it('shows the newest run’s time while plans run', () => {
    const run = foldEvents([...running('a'), ...running('b')], T0, 1_000);
    const chip = statusChip(run, 'connected', { nowMs: T0 + 9_000, seenFailures: NONE });
    expect(chip).toMatchObject({ signal: 'live', word: '2 running', planId: 'b', running: true });
    expect(chip.reading).toEqual({ text: '0:07', label: '7 seconds' });
  });

  it('points at a failed run the person has not opened, before a running one', () => {
    const run = foldEvents([...failed('a'), ...running('b')], T0);
    expect(statusChip(run, 'connected', { nowMs: T0, seenFailures: NONE })).toMatchObject({
      signal: 'fail',
      word: '1 failed',
      planId: 'a',
    });
  });

  it('lets a failure go once its plan was opened', () => {
    const run = foldEvents(failed('a'), T0);
    const seen = { a: run.plans.a!.finishedAtMs };
    expect(statusChip(run, 'connected', { nowMs: T0, seenFailures: seen })).toMatchObject({ word: 'idle' });
  });

  it('asks for the person first when a task waits for review', () => {
    const run = foldEvents(
      [
        ...failed('a'),
        ...running('b'),
        { type: 'task_phase_changed', plan_id: 'b', task_id: 'T1', old_phase: 'verify', new_phase: 'awaiting_approval' },
      ],
      T0,
    );
    expect(statusChip(run, 'connected', { nowMs: T0, seenFailures: NONE })).toMatchObject({
      signal: 'you',
      word: '1 waiting',
      planId: 'b',
    });
  });
});
