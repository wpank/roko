import type {
  HealthData, CFactorData, Episode, Agent, KnowledgeEntry,
  RoutingDecision, DreamPhase, ProviderHealth,
} from '../lib/types';

export const seedHealth: HealthData = {
  status: 'online',
  uptime: '8h 16m',
  version: '0.1.0',
};

export const seedCFactor: CFactorData = {
  composite: 0.847,
  gatePassRate: 0.93,
  costEfficiency: 0.78,
  speed: 0.91,
  reuseRate: 0.64,
  learningRate: 0.82,
};

export const seedEpisodes: Episode[] = [
  { id: 'ep-020', agent: 'rustsmith', task: 'wire-chain', result: 'pass', cost: 0.024, duration: 2.1 },
  { id: 'ep-019', agent: 'fullstack', task: 'add-health', result: 'pass', cost: 0.017, duration: 3.2 },
  { id: 'ep-018', agent: 'auditor', task: 'review-prd', result: 'pass', cost: 0.031, duration: 4.1 },
  { id: 'ep-017', agent: 'rustsmith', task: 'fix-gate-order', result: 'pass', cost: 0.012, duration: 1.8 },
  { id: 'ep-016', agent: 'ethdev', task: 'deploy-proxy', result: 'fail', cost: 0.043, duration: 6.3 },
  { id: 'ep-015', agent: 'rustsmith', task: 'add-retry-logic', result: 'pass', cost: 0.019, duration: 2.4 },
  { id: 'ep-014', agent: 'fullstack', task: 'auth-middleware', result: 'pass', cost: 0.028, duration: 3.7 },
  { id: 'ep-013', agent: 'auditor', task: 'review-security', result: 'pass', cost: 0.035, duration: 5.2 },
];

export const seedAgents: Agent[] = [
  { name: 'rustsmith', role: 'implementer', tier: 'T1', taskCount: 247, totalCost: 0.42, lastActive: '2m ago', status: 'active' },
  { name: 'ethdev', role: 'implementer', tier: 'T2', taskCount: 312, totalCost: 1.18, lastActive: '5m ago', status: 'active' },
  { name: 'auditor', role: 'reviewer', tier: 'T3', taskCount: 268, totalCost: 0.89, lastActive: '8m ago', status: 'active' },
];

export const seedKnowledge: KnowledgeEntry[] = [
  { id: 'k-001', content: 'Cargo workspace builds require all members to share compatible dependency versions', citations: 7, tier: 3, tags: ['rust', 'build'] },
  { id: 'k-002', content: 'Gate pipelines should run compile before test to fail fast on syntax errors', citations: 12, tier: 3, tags: ['gates', 'optimization'] },
  { id: 'k-003', content: 'Claude Haiku handles simple scaffolding tasks with 94% pass rate at T1 tier', citations: 5, tier: 2, tags: ['routing', 'haiku'] },
  { id: 'k-004', content: 'Agent dispatch benefits from pre-warming context with project-specific knowledge', citations: 3, tier: 1, tags: ['agents', 'context'] },
  { id: 'k-005', content: 'Clippy warnings about unused imports are the most common gate failure on first pass', citations: 9, tier: 2, tags: ['gates', 'clippy'] },
  { id: 'k-006', content: 'PRD-generated plans with 6+ tasks benefit from parallel execution in dependency groups', citations: 4, tier: 2, tags: ['planning', 'execution'] },
];

export const seedRouting: RoutingDecision[] = [
  { model: 'claude-haiku-4.5', tier: 'T1', passRate: 0.94, avgCost: 0.001, bestFor: 'Scaffolding, config, simple transforms', count: 142 },
  { model: 'claude-sonnet-4', tier: 'T2', passRate: 0.97, avgCost: 0.008, bestFor: 'Implementation, API integration', count: 89 },
  { model: 'claude-opus-4', tier: 'T3', passRate: 0.99, avgCost: 0.032, bestFor: 'Complex reasoning, architecture', count: 34 },
];

export const seedDreams: DreamPhase[] = [
  { name: 'Hypnagogia', description: 'Reviewing recent episodes for patterns', status: 'complete', entries: 47 },
  { name: 'Imagination', description: 'Generating new strategies from patterns', status: 'complete', entries: 12 },
  { name: 'Consolidation', description: 'Compressing insights into durable knowledge', status: 'active', entries: 6 },
];

export const seedProviders: ProviderHealth[] = [
  { name: 'Anthropic', status: 'healthy' },
  { name: 'OpenAI', status: 'healthy' },
  { name: 'Google', status: 'degraded' },
  { name: 'Ollama', status: 'healthy' },
  { name: 'Perplexity', status: 'down' },
];
