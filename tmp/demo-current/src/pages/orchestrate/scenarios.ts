import type { Scenario, TaskState, PrdArtifact, PlanArtifact, Metrics } from '../../lib/types';

export const scenarios: Scenario[] = [
  {
    id: 'simple',
    label: 'Simple',
    complexity: 'simple',
    prompt: 'Build a status CLI that prints system uptime',
    description: 'Build a CLI that checks and prints the current system uptime, load average, and memory usage in a clean, colored table format.',
    detail: '3 tasks, 1 model, ~5 seconds \u2014 see the basic pipeline',
    taskCount: 3,
    tierCount: 1,
    estimatedCost: '$0.01',
    estimatedTime: '5s',
    slug: 'status-cli',
  },
  {
    id: 'medium',
    label: 'Medium',
    complexity: 'medium',
    prompt: 'Build a GitHub release watcher with email alerts',
    description: 'Build a CLI that watches GitHub repositories for new releases and sends email notifications when a new version is published.',
    detail: '5 tasks, 2 models, API integration \u2014 see multi-tier routing',
    taskCount: 5,
    tierCount: 2,
    estimatedCost: '$0.03',
    estimatedTime: '12s',
    slug: 'github-release-watcher',
  },
  {
    id: 'complex',
    label: 'Complex',
    complexity: 'complex',
    prompt: 'Build a BTC funding alert CLI from Hyperliquid',
    description: 'Build a CLI that fetches BTC funding rates from Hyperliquid and emails an alert when funding flips negative.',
    detail: '6 tasks, 3 models, DeFi + email \u2014 see the full system',
    taskCount: 6,
    tierCount: 3,
    estimatedCost: '$0.05',
    estimatedTime: '18s',
    slug: 'btc-funding-alert-cli',
  },
];

interface ScenarioData {
  prd: PrdArtifact;
  plan: PlanArtifact;
  tasks: TaskState[];
  finalMetrics: Metrics;
}

export const scenarioData: Record<string, ScenarioData> = {
  simple: {
    prd: {
      title: 'Status CLI',
      slug: 'status-cli',
      requirementsCount: 3,
      acceptancesCount: 2,
      summary: 'A command-line tool that prints system uptime, load average, and memory usage in a formatted table.',
    },
    plan: {
      title: 'STATUS CLI',
      slug: 'status-cli',
      taskCount: 3,
      tierBreakdown: [{ tier: 'T1', model: 'haiku', count: 3 }],
      estimatedCost: '$0.01',
      summary: 'Define CLI structure, implement data collection, add output formatting.',
    },
    tasks: [
      { id: 't1', name: 'Define CLI contract and arg parser', status: 'pending', tier: 'T1', role: 'implementer', model: 'claude-haiku-4.5', modelReason: 'Low complexity, simple scaffolding', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }] },
      { id: 't2', name: 'Implement system metrics collection', status: 'pending', tier: 'T1', role: 'implementer', model: 'claude-haiku-4.5', modelReason: 'Standard library usage', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }] },
      { id: 't3', name: 'Add formatted table output', status: 'pending', tier: 'T1', role: 'implementer', model: 'claude-haiku-4.5', modelReason: 'Simple formatting task', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }, { name: 'clippy', status: 'pending' }] },
    ],
    finalMetrics: { totalCost: 0.009, totalTokens: 4200, elapsed: 5.1, passRate: 1.0, tasksComplete: 3, tasksTotal: 3 },
  },
  medium: {
    prd: {
      title: 'GitHub Release Watcher',
      slug: 'github-release-watcher',
      requirementsCount: 4,
      acceptancesCount: 3,
      summary: 'A CLI that polls GitHub API for release events and dispatches email notifications via SMTP.',
    },
    plan: {
      title: 'GITHUB RELEASE WATCHER',
      slug: 'github-release-watcher',
      taskCount: 5,
      tierBreakdown: [
        { tier: 'T1', model: 'haiku', count: 2 },
        { tier: 'T2', model: 'sonnet', count: 3 },
      ],
      estimatedCost: '$0.03',
      summary: 'CLI scaffold, GitHub API client, release diff engine, email dispatcher, integration test.',
    },
    tasks: [
      { id: 't1', name: 'Define CLI contract and config schema', status: 'pending', tier: 'T1', role: 'implementer', model: 'claude-haiku-4.5', modelReason: 'Simple scaffolding', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }] },
      { id: 't2', name: 'Implement GitHub API release fetcher', status: 'pending', tier: 'T2', role: 'implementer', model: 'claude-sonnet-4', modelReason: 'API integration requires reasoning', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }] },
      { id: 't3', name: 'Build release diff and change detector', status: 'pending', tier: 'T2', role: 'implementer', model: 'claude-sonnet-4', modelReason: 'Complex comparison logic', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }] },
      { id: 't4', name: 'Add email notification dispatcher', status: 'pending', tier: 'T2', role: 'implementer', model: 'claude-sonnet-4', modelReason: 'SMTP integration', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }] },
      { id: 't5', name: 'Integration test with dry-run mode', status: 'pending', tier: 'T1', role: 'implementer', model: 'claude-haiku-4.5', modelReason: 'Test scaffolding', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }, { name: 'clippy', status: 'pending' }] },
    ],
    finalMetrics: { totalCost: 0.031, totalTokens: 11800, elapsed: 12.4, passRate: 1.0, tasksComplete: 5, tasksTotal: 5 },
  },
  complex: {
    prd: {
      title: 'BTC Funding Alert CLI',
      slug: 'btc-funding-alert-cli',
      requirementsCount: 5,
      acceptancesCount: 4,
      summary: 'A CLI that fetches BTC perpetual funding rates from Hyperliquid DEX and emails alerts when funding turns negative.',
    },
    plan: {
      title: 'BTC FUNDING ALERT CLI',
      slug: 'btc-funding-alert-cli',
      taskCount: 6,
      tierBreakdown: [
        { tier: 'T1', model: 'haiku', count: 2 },
        { tier: 'T2', model: 'sonnet', count: 3 },
        { tier: 'T3', model: 'opus', count: 1 },
      ],
      estimatedCost: '$0.05',
      summary: 'CLI scaffold, DeFi data ingestion, flip detection, email integration, orchestration, verification.',
    },
    tasks: [
      { id: 't1', name: 'Define CLI contract and dry-run config', status: 'pending', tier: 'T1', role: 'implementer', model: 'claude-haiku-4.5', modelReason: 'Low complexity, simple scaffolding', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }] },
      { id: 't2', name: 'Implement DeFi data fetcher', status: 'pending', tier: 'T2', role: 'implementer', model: 'claude-sonnet-4', modelReason: 'Complex implementation, requires reasoning', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }] },
      { id: 't3', name: 'Add email notification module', status: 'pending', tier: 'T2', role: 'implementer', model: 'claude-sonnet-4', modelReason: 'SMTP integration complexity', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }] },
      { id: 't4', name: 'Wire configuration and CLI args', status: 'pending', tier: 'T1', role: 'implementer', model: 'claude-haiku-4.5', modelReason: 'Simple config wiring', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }] },
      { id: 't5', name: 'Integration tests with dry-run mode', status: 'pending', tier: 'T3', role: 'implementer', model: 'claude-opus-4', modelReason: 'Complex test design, needs strong reasoning', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }, { name: 'clippy', status: 'pending' }] },
      { id: 't6', name: 'Final verification and smoke test', status: 'pending', tier: 'T1', role: 'implementer', model: 'claude-haiku-4.5', modelReason: 'Simple verification', gates: [{ name: 'compile', status: 'pending' }, { name: 'test', status: 'pending' }, { name: 'clippy', status: 'pending' }, { name: 'diff', status: 'pending' }] },
    ],
    finalMetrics: { totalCost: 0.042, totalTokens: 18200, elapsed: 18.3, passRate: 1.0, tasksComplete: 6, tasksTotal: 6 },
  },
};
