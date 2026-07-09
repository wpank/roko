import type { BenchRun } from '../lib/types';

export const seedBenchRuns: BenchRun[] = [
  { id: 'br-003', date: '2026-04-28T14:23:00Z', suite: 'Smoke', model: 'claude-sonnet-4', passRate: 1.0, totalCost: 0.042, taskCount: 5, duration: 12.4 },
  { id: 'br-002', date: '2026-04-27T10:15:00Z', suite: 'Learnable Rust', model: 'claude-sonnet-4', passRate: 0.83, totalCost: 0.087, taskCount: 6, duration: 24.1 },
  { id: 'br-001', date: '2026-04-26T16:42:00Z', suite: 'Smoke', model: 'claude-haiku-4.5', passRate: 0.80, totalCost: 0.018, taskCount: 5, duration: 8.2 },
];

export const benchSuites = [
  { id: 'smoke', name: 'Smoke', taskCount: 5 },
  { id: 'learnable-rust', name: 'Learnable Rust', taskCount: 6 },
  { id: 'roko-bench', name: 'Roko Bench', taskCount: 8 },
  { id: 'codegen', name: 'Codegen', taskCount: 10 },
];

export const benchStrategies = [
  { id: 'minimal', name: 'Minimal' },
  { id: 'context-enriched', name: 'Context-Enriched' },
  { id: 'neuro-augmented', name: 'Neuro-Augmented' },
  { id: 'full', name: 'Full' },
];
