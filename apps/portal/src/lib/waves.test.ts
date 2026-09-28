/**
 * waves.test.ts — computeWaves coverage (≥ 9 tests)
 *
 *  1. empty input
 *  2. single task
 *  3. linear chain
 *  4. diamond dependency
 *  5. parallel roots preserve input order
 *  6. dangling dependency
 *  7. two-node cycle
 *  8. self-cycle
 *  9. cycle beside independent tasks
 */

import { describe, it, expect } from 'vitest';
import { computeWaves } from './waves';

describe('computeWaves', () => {
  // 1. Empty input → no waves, no diagnostics
  it('empty input yields empty waves and no diagnostics', () => {
    const result = computeWaves([]);
    expect(result.waves).toEqual([]);
    expect(result.diagnostics).toEqual([]);
  });

  // 2. Single task with no deps → one wave containing that task
  it('single task with no deps produces one wave', () => {
    const result = computeWaves([{ id: 'T01', depends_on: [] }]);
    expect(result.waves).toEqual([['T01']]);
    expect(result.diagnostics).toEqual([]);
  });

  // 3. Linear chain T01 ← T02 ← T03 → three sequential waves
  it('linear chain yields one task per wave in dependency order', () => {
    const tasks = [
      { id: 'T01', depends_on: [] },
      { id: 'T02', depends_on: ['T01'] },
      { id: 'T03', depends_on: ['T02'] },
    ];
    const result = computeWaves(tasks);
    expect(result.waves).toEqual([['T01'], ['T02'], ['T03']]);
    expect(result.diagnostics).toEqual([]);
  });

  // 4. Diamond: T01 → [T02, T03] → T04
  it('diamond dependency collapses correctly into three waves', () => {
    const tasks = [
      { id: 'T01', depends_on: [] },
      { id: 'T02', depends_on: ['T01'] },
      { id: 'T03', depends_on: ['T01'] },
      { id: 'T04', depends_on: ['T02', 'T03'] },
    ];
    const result = computeWaves(tasks);
    expect(result.waves).toEqual([['T01'], ['T02', 'T03'], ['T04']]);
    expect(result.diagnostics).toEqual([]);
  });

  // 5. Parallel roots preserve input order within the first wave
  it('parallel roots appear in the first wave in input order', () => {
    // Input order: T02, T01 — wave 0 must respect that
    const tasks = [
      { id: 'T02', depends_on: [] },
      { id: 'T01', depends_on: [] },
    ];
    const result = computeWaves(tasks);
    expect(result.waves).toEqual([['T02', 'T01']]);
    expect(result.diagnostics).toEqual([]);
  });

  // 6. Dangling dependency → task placed as if dep absent, diagnostic emitted
  it('dangling dependency is reported and the task is still placed', () => {
    const tasks = [
      { id: 'T01', depends_on: [] },
      { id: 'T03', depends_on: ['T99'] }, // T99 does not exist
    ];
    const result = computeWaves(tasks);
    // T03 has no known deps after stripping T99, so it lands in wave 0
    expect(result.waves).toEqual([['T01', 'T03']]);
    expect(result.diagnostics).toHaveLength(1);
    const diag = result.diagnostics[0]!;
    expect(diag.kind).toBe('dangling');
    expect(diag.taskIds).toContain('T03');
    expect(diag.message).toMatch(/T03.*T99.*does not exist/);
  });

  // 7. Two-node cycle → cycle diagnostic, both tasks in a final wave
  it('two-node cycle emits a cycle diagnostic and a final wave', () => {
    const tasks = [
      { id: 'T04', depends_on: ['T05'] },
      { id: 'T05', depends_on: ['T04'] },
    ];
    const result = computeWaves(tasks);
    // No clean waves (both are stuck); one cycle wave at the end
    expect(result.waves).toHaveLength(1);
    expect(result.waves[0]).toEqual(['T04', 'T05']);
    const cycleDiags = result.diagnostics.filter((d) => d.kind === 'cycle');
    expect(cycleDiags).toHaveLength(1);
    expect(cycleDiags[0]!.message).toMatch(/T04.*T05.*T04.*form a cycle/);
  });

  // 8. Self-cycle (task depends on itself)
  it('self-cycle emits a cycle diagnostic', () => {
    const tasks = [{ id: 'T06', depends_on: ['T06'] }];
    const result = computeWaves(tasks);
    expect(result.waves).toEqual([['T06']]);
    const cycleDiags = result.diagnostics.filter((d) => d.kind === 'cycle');
    expect(cycleDiags).toHaveLength(1);
    expect(cycleDiags[0]!.message).toMatch(/T06.*T06.*form a cycle/);
    expect(cycleDiags[0]!.taskIds).toContain('T06');
  });

  // 9. Cycle beside independent tasks — independent tasks land in wave 0,
  //    cycle tasks land in a final wave
  it('cycle beside independent tasks: independent tasks placed first', () => {
    const tasks = [
      { id: 'T01', depends_on: [] },
      { id: 'T04', depends_on: ['T05'] },
      { id: 'T05', depends_on: ['T04'] },
    ];
    const result = computeWaves(tasks);
    // wave 0: T01 (no deps)
    // final wave: T04, T05 (cycle)
    expect(result.waves).toHaveLength(2);
    expect(result.waves[0]).toEqual(['T01']);
    expect(result.waves[1]).toEqual(['T04', 'T05']);
    const cycleDiags = result.diagnostics.filter((d) => d.kind === 'cycle');
    expect(cycleDiags).toHaveLength(1);
    expect(cycleDiags[0]!.kind).toBe('cycle');
  });
});
