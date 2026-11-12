import { describe, it, expect } from 'vitest';
import { describeEmpty } from './emptyState';
import type { EmptyStateInput } from './emptyState';
import { SIGN_IN_HINT } from './bootstrap';

// ── Helpers ────────────────────────────────────────────────────────────────────

function input(overrides: Partial<EmptyStateInput> = {}): EmptyStateInput {
  return {
    workspace: 'my-project',
    connection: 'connected',
    planCount: 1,
    ...overrides,
  };
}

// ── Case 1: disconnected ───────────────────────────────────────────────────────

describe('case: disconnected', () => {
  it('returns reconnecting message when disconnected', () => {
    const result = describeEmpty(input({ connection: 'disconnected' }));
    expect(result).toBe('Lost the server; reconnecting.');
  });
});

// ── Case 2: error ──────────────────────────────────────────────────────────────

describe('case: error', () => {
  it('returns reconnecting message when connection errored', () => {
    const result = describeEmpty(input({ connection: 'error' }));
    expect(result).toBe('Lost the server; reconnecting.');
  });
});

// ── Case 3: signed out ────────────────────────────────────────────────────────

describe('case: unauthorized', () => {
  it('says how to sign in', () => {
    expect(describeEmpty(input({ connection: 'unauthorized' }))).toBe(SIGN_IN_HINT);
  });
});

// ── Case 4: no plans ──────────────────────────────────────────────────────────

describe('case: no plans', () => {
  it('prompts to describe work when there are no plans', () => {
    const result = describeEmpty(input({ planCount: 0, workspace: 'roko-core' }));
    expect(result).toBe('No plans in roko-core yet. Describe what you want to build.');
  });
});

// ── Case 5: plans but none selected ───────────────────────────────────────────

describe('case: plans but none selected', () => {
  it('prompts to select a plan', () => {
    const result = describeEmpty(input({ planCount: 3 }));
    expect(result).toBe('Select a plan, or press n to describe a new one.');
  });
});

// ── Invariant ─────────────────────────────────────────────────────────────────

describe('invariant: no empty string or "no data"', () => {
  const allInputs: EmptyStateInput[] = [
    input({ connection: 'disconnected' }),
    input({ connection: 'error' }),
    input({ connection: 'unauthorized' }),
    input({ connection: 'connecting' }),
    input({ connection: 'connected' }),
    input({ planCount: 0 }),
    input({ planCount: 2 }),
  ];

  it('no input produces an empty string', () => {
    for (const i of allInputs) {
      const result = describeEmpty(i);
      expect(result, `Input: ${JSON.stringify(i)}`).not.toBe('');
      expect(result.length, `Input: ${JSON.stringify(i)}`).toBeGreaterThan(0);
    }
  });

  it('no input produces the phrase "no data"', () => {
    for (const i of allInputs) {
      const result = describeEmpty(i);
      expect(result.toLowerCase(), `Input: ${JSON.stringify(i)}`).not.toContain('no data');
    }
  });
});
