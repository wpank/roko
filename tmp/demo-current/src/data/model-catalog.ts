export interface ModelDef {
  id: string;
  name: string;
  inputCostPer1k: number;
  outputCostPer1k: number;
  tier: string;
  description: string;
}

export const models: ModelDef[] = [
  {
    id: 'claude-haiku-4.5',
    name: 'Claude Haiku 4.5',
    inputCostPer1k: 0.001,
    outputCostPer1k: 0.005,
    tier: 'T1',
    description: 'Fast, cheap ($0.001/task)',
  },
  {
    id: 'claude-sonnet-4',
    name: 'Claude Sonnet 4',
    inputCostPer1k: 0.003,
    outputCostPer1k: 0.015,
    tier: 'T2',
    description: 'Balanced ($0.008/task)',
  },
  {
    id: 'claude-opus-4',
    name: 'Claude Opus 4',
    inputCostPer1k: 0.015,
    outputCostPer1k: 0.075,
    tier: 'T3',
    description: 'Powerful ($0.03/task)',
  },
];
