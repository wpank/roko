import { describe, it, expect } from 'vitest';
import { parseSelection, selectionSearch, resolveSelection } from './selection';
import type { Selection } from './selection';

// ── parseSelection ─────────────────────────────────────────────────────────────

describe('parseSelection', () => {
  it('returns nulls for an empty string', () => {
    expect(parseSelection('')).toEqual({ plan: null, task: null });
  });

  it('parses plan and task from a query string with leading ?', () => {
    expect(parseSelection('?plan=foo&task=bar')).toEqual({ plan: 'foo', task: 'bar' });
  });

  it('parses plan and task from a query string without leading ?', () => {
    expect(parseSelection('plan=foo&task=bar')).toEqual({ plan: 'foo', task: 'bar' });
  });

  it('normalises empty string values to null', () => {
    expect(parseSelection('?plan=&task=')).toEqual({ plan: null, task: null });
  });

  it('returns null for task when only plan is present', () => {
    expect(parseSelection('?plan=abc')).toEqual({ plan: 'abc', task: null });
  });
});

// ── selectionSearch ────────────────────────────────────────────────────────────

describe('selectionSearch', () => {
  it('returns "" when the result would be empty', () => {
    expect(selectionSearch('', { plan: null, task: null })).toBe('');
  });

  it('adds plan and task params', () => {
    expect(selectionSearch('', { plan: 'p1', task: 't1' })).toBe('?plan=p1&task=t1');
  });

  it('changing plan clears task when patch does not include task', () => {
    const result = selectionSearch('?plan=a&task=b', { plan: 'c' });
    // plan is updated, task is removed
    expect(result).toBe('?plan=c');
  });

  it('changing plan does NOT clear task when patch also sets task', () => {
    const result = selectionSearch('?plan=a&task=b', { plan: 'c', task: 'd' });
    expect(result).toBe('?plan=c&task=d');
  });

  it('drops keys with null value', () => {
    const result = selectionSearch('?plan=a&task=b', { task: null });
    expect(result).toBe('?plan=a');
  });

  it('keeps unrelated params untouched', () => {
    const result = selectionSearch('?plan=a&other=x', { task: 't1' });
    expect(result).toBe('?plan=a&other=x&task=t1');
  });

  it('setting plan to null clears both plan and task (plan in patch, task not)', () => {
    const result = selectionSearch('?plan=a&task=b', { plan: null });
    // plan in patch → task cleared; plan: null → plan deleted → empty
    expect(result).toBe('');
  });

  it('produces "" when all params resolve to null/empty', () => {
    expect(selectionSearch('?plan=a', { plan: null })).toBe('');
  });

  it('only updates task when plan is not in the patch', () => {
    const result = selectionSearch('?plan=a&task=b', { task: 'c' });
    // plan not in patch → task is NOT cleared first; task is then set to 'c'
    expect(result).toBe('?plan=a&task=c');
  });
});

// ── resolveSelection ───────────────────────────────────────────────────────────

describe('resolveSelection', () => {
  const known = (opts: {
    loaded?: boolean;
    planIds?: string[];
    runningPlanIds?: string[];
  }) => ({
    loaded: opts.loaded ?? true,
    planIds: opts.planIds ?? [],
    runningPlanIds: opts.runningPlanIds ?? [],
  });

  it('does not clear anything before the list loads', () => {
    const sel: Selection = { plan: 'unknown-plan', task: 't1' };
    expect(
      resolveSelection(sel, known({ loaded: false, planIds: ['p1'] })),
    ).toEqual(sel);
  });

  it('clears an unknown plan once loaded', () => {
    expect(
      resolveSelection(
        { plan: 'ghost', task: 't1' },
        known({ loaded: true, planIds: ['p1', 'p2'] }),
      ),
    ).toEqual({ plan: null, task: null });
  });

  it('keeps a known plan unchanged', () => {
    const sel: Selection = { plan: 'p1', task: 't1' };
    expect(
      resolveSelection(sel, known({ loaded: true, planIds: ['p1'] })),
    ).toEqual(sel);
  });

  it('selects the first running plan when no plan is selected', () => {
    expect(
      resolveSelection(
        { plan: null, task: null },
        known({ loaded: true, planIds: ['p1', 'p2'], runningPlanIds: ['p2', 'p1'] }),
      ),
    ).toEqual({ plan: 'p2', task: null });
  });

  it('does not auto-select a running plan when a plan is already selected', () => {
    expect(
      resolveSelection(
        { plan: 'p1', task: null },
        known({ loaded: true, planIds: ['p1'], runningPlanIds: ['p1'] }),
      ),
    ).toEqual({ plan: 'p1', task: null });
  });

  it('does nothing when loaded=true, no plan selected, and no running plans', () => {
    expect(
      resolveSelection(
        { plan: null, task: null },
        known({ loaded: true, planIds: ['p1'], runningPlanIds: [] }),
      ),
    ).toEqual({ plan: null, task: null });
  });

  it('clears to first running plan when unknown plan is replaced', () => {
    // unknown plan → clear → then auto-select first running
    expect(
      resolveSelection(
        { plan: 'ghost', task: 't1' },
        known({ loaded: true, planIds: ['p1'], runningPlanIds: ['p1'] }),
      ),
    ).toEqual({ plan: 'p1', task: null });
  });
});
