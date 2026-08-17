import { useState, useCallback } from 'react';

// ── Types ────────────────────────────────────────────────────

export interface InferenceCall {
  model: string;
  tier: 'T0' | 'T1' | 'T2';
  cost: number;
  inputTokens: number;
  outputTokens: number;
  latencyMs: number;
  timestamp: number;
}

export interface InferenceTraceTotals {
  cost: number;
  tokens: number;
  calls: number;
  avgLatencyMs: number;
}

export interface InferenceTraceState {
  calls: InferenceCall[];
  totals: InferenceTraceTotals;
  costSeries: number[];
  reset: () => void;
}

// ── Hook ─────────────────────────────────────────────────────

export function useInferenceTrace(): InferenceTraceState {
  const [calls, setCalls] = useState<InferenceCall[]>([]);
  const [costSeries, setCostSeries] = useState<number[]>([]);

  const reset = useCallback(() => {
    setCalls([]);
    setCostSeries([]);
  }, []);

  // `/api/events` intentionally exposes DashboardEvent, which has no
  // per-inference payload. Populate this trace only when a real inference
  // telemetry endpoint is added; do not fabricate it from unrelated metrics.

  // Derive totals from the calls buffer (not a separate accumulator)
  const totals: InferenceTraceTotals = {
    cost: calls.reduce((s, c) => s + c.cost, 0),
    tokens: calls.reduce((s, c) => s + c.inputTokens + c.outputTokens, 0),
    calls: calls.length,
    avgLatencyMs: calls.length > 0
      ? calls.reduce((s, c) => s + c.latencyMs, 0) / calls.length
      : 0,
  };

  return { calls, totals, costSeries, reset };
}
