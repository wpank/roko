export interface Scenario {
  id: string;
  label: string;
  complexity: 'simple' | 'medium' | 'complex';
  prompt: string;
  description: string;
  detail: string;
  taskCount: number;
  tierCount: number;
  estimatedCost: string;
  estimatedTime: string;
  slug: string;
}

export interface PrdArtifact {
  title: string;
  slug: string;
  requirementsCount: number;
  acceptancesCount: number;
  summary: string;
}

export interface PlanArtifact {
  title: string;
  slug: string;
  taskCount: number;
  tierBreakdown: { tier: string; model: string; count: number }[];
  estimatedCost: string;
  summary: string;
}

export interface TaskState {
  id: string;
  name: string;
  status: 'pending' | 'running' | 'done' | 'failed';
  tier: string;
  role: string;
  model: string;
  modelReason?: string;
  cost?: number;
  duration?: number;
  tokens?: number;
  gates: GateState[];
}

export interface GateState {
  name: string;
  status: 'pass' | 'fail' | 'running' | 'pending';
}

export interface Metrics {
  totalCost: number;
  totalTokens: number;
  elapsed: number;
  passRate: number;
  tasksComplete: number;
  tasksTotal: number;
}

export type Phase = 'idle' | 'idea' | 'prd' | 'plan' | 'tasks' | 'running' | 'complete';

export type DataMode = 'live' | 'seed' | 'reconnecting' | 'offline';

export interface HealthData {
  status: string;
  uptime: string;
  version: string;
}

export interface CFactorData {
  composite: number;
  gatePassRate: number;
  costEfficiency: number;
  speed: number;
  reuseRate: number;
  learningRate: number;
}

export interface Episode {
  id: string;
  agent: string;
  task: string;
  result: 'pass' | 'fail';
  cost: number;
  duration: number;
}

export interface Agent {
  name: string;
  role: string;
  tier: string;
  taskCount: number;
  totalCost: number;
  lastActive: string;
  status: 'active' | 'idle' | 'offline';
}

export interface KnowledgeEntry {
  id: string;
  content: string;
  citations: number;
  tier: number;
  tags: string[];
}

export interface RoutingDecision {
  model: string;
  tier: string;
  passRate: number;
  avgCost: number;
  bestFor: string;
  count: number;
}

export interface DreamPhase {
  name: string;
  description: string;
  status: 'complete' | 'active' | 'pending';
  entries: number;
}

export interface ProviderHealth {
  name: string;
  status: 'healthy' | 'degraded' | 'down';
}

export interface BenchRun {
  id: string;
  date: string;
  suite: string;
  model: string;
  passRate: number;
  totalCost: number;
  taskCount: number;
  duration: number;
}
