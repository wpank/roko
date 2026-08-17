# Wave E: Demo Redesign (5 Scenarios)

## Root Cause

The demo app has 14 scenarios with a generic sidebar (label + description, no live data).
SCENARIO-REDESIGN.md specified 5 scenarios with custom React panels consuming SSE events.
That document was never implemented.

Additionally, the terminal session layer has fundamental bugs:
- `resolveRoko()` reads its own echo (always resolves to bare `roko` which doesn't exist)
- Every command gets redundant `--repo` (300+ chars, typed character-by-character)
- Module-level singletons persist across scenario resets
- Exit codes are misdetected from clobbered output buffers

---

## Key Discovery: SSE Infrastructure Already Exists

**Audit finding**: The demo app already has SSE primitives:
- `EventStreamContext` in roko-serve (manages SSE connections)
- `SseAdapter` converts RuntimeEvent → SSE format
- `useBenchSSE` React hook in demo-app (consumes SSE for benchmark scenarios)

**Strategy**: Build `useOperationEvents(opId, types[])` on top of existing infrastructure.
Don't replace it. Extend it.

---

## Task E1: Fix Terminal Session Fundamentals

**Root cause**: From TERMINAL-SESSION-REDESIGN.md — `resolveRoko()` in
`terminal-session.ts` runs a detection command whose output markers appear in the
command ECHO, so the check always matches the echo rather than the output.

**Fixes** (all in `demo/demo-app/src/lib/terminal-session.ts`):

1. **resolveRoko**: Use unique random markers that can't appear in the echo:
   ```typescript
   const marker = `__ROKO_${crypto.randomUUID().slice(0, 8)}__`;
   const cmd = `command -v roko && echo "${marker}PATH" || echo "${marker}NONE"`;
   // Parse output looking for marker prefix, ignoring echo
   ```

2. **Remove --repo injection**: Commands already `cd` into the workspace.

3. **Remove --model injection from commands that don't use it**: `prd idea`,
   `prd draft promote`, `plan validate`, `status` don't accept `--model`.

4. **Reset module-level state on scenario reset**: `provider-race.ts` has `const state`,
   `gate-retry.ts` has `let runOutcome`, `knowledge-transfer.ts` has `let betaWorkspaceDir`.
   All must reset to initial values on scenario reset.

5. **Snapshot output before exit-check**: Take `commandOutput` snapshot before running
   the exit code detection command.

**Verification**: Click any scenario, all commands execute without "command not found".
Reset and re-run — no stale state.

---

## Task E2: Archive Old Scenarios

**Action**: Move all 14 old scenario runners to `src/lib/scenario-runners/archive/`.
Keep importable for reference but remove from active scenario list.

**Verification**: Demo sidebar shows only 5 new scenarios.

---

## Task E3: SSE Client Infrastructure (useOperationEvents hook)

**Root cause**: The demo app's existing `useBenchSSE` hook is specialized for bench scenarios.
Custom panels need a generic hook filtered by operation ID and event types.

**Design**: Build on existing `EventStreamContext`:

```typescript
// demo/demo-app/src/hooks/useOperationEvents.ts
import { useContext, useState, useEffect } from 'react';
import { EventStreamContext } from '../context/EventStreamContext';

export function useOperationEvents(
  operationId: string | null,
  eventTypes?: string[]
) {
  const { events } = useContext(EventStreamContext);
  const [filtered, setFiltered] = useState<ServerEvent[]>([]);

  useEffect(() => {
    if (!operationId) return;
    const relevant = events.filter(e =>
      e.operation_id === operationId &&
      (!eventTypes || eventTypes.includes(e.type))
    );
    setFiltered(relevant);
  }, [events, operationId, eventTypes]);

  return filtered;
}

// Convenience hooks for specific data
export function useInferenceCosts(operationId: string | null) {
  const events = useOperationEvents(operationId, ['inference_completed']);
  return events.reduce((acc, e) => ({
    totalCost: acc.totalCost + e.data.cost_usd,
    totalTokens: acc.totalTokens + e.data.input_tokens + e.data.output_tokens,
    calls: acc.calls + 1,
  }), { totalCost: 0, totalTokens: 0, calls: 0 });
}

export function usePipelineProgress(operationId: string | null) {
  const events = useOperationEvents(operationId, [
    'plan_started', 'task_started', 'task_completed', 'gate_result', 'plan_completed'
  ]);
  // ... derive pipeline state from events
}
```

**Key**: This doesn't replace `EventStreamContext` — it builds on it. The existing
EventSource connection management stays the same.

**Verification**: DevTools Network tab shows active EventSource. Hook returns filtered
events when operations execute.

---

## Task E4: Scenario 1 — Cost (The System Is the Variable)

**Proof point**: Same task, same model class — cascade routing cuts cost 3-5x.

**Layout**: 2 panes (naive vs cascade) + comparison sidebar

**Commands**:
```
Pane 0: roko do "build a function that checks if a number is prime" --no-cascade
Pane 1: roko do "build a function that checks if a number is prime"
```

**Custom sidebar panel**: `CostComparisonPanel.tsx`
- 3-column layout: Naive | Cascade | Delta
- Rows: tokens in/out, cost ($), time (s), model used, gates passed
- Live-updating: each cell updates as `inference_completed` events arrive
- On completion: delta column highlights percentage savings (green/red)
- Uses `useInferenceCosts(op0Id)` and `useInferenceCosts(op1Id)`

**SSE events consumed**: `inference_completed`, `plan_completed`

**Verification**: Click "Run", both panes execute, sidebar shows diverging costs,
final delta shows 50-80% savings.

---

## Task E5: Scenario 2 — Pipeline (Idea → Code in One Shot)

**Proof point**: One command takes an idea to working, validated code.

**Layout**: 1 terminal + pipeline sidebar

**Command**:
```
roko do "Build a Rust CLI that converts temperatures between Celsius and Fahrenheit"
```

**Custom sidebar panel**: `PipelineStagesPanel.tsx`
- Stages: Classify → Plan → Execute → Gate → Done
- Each stage lights up as reached (derived from event sequence)
- Tasks list with per-task status (pending/running/done/failed)
- Gate results inline (compile ✔, clippy ✔, test ✔)
- Metrics: cost, tokens, time, model
- Uses `usePipelineProgress(opId)`

**SSE events consumed**: `run_started`, `task_started`, `task_completed`,
`gate_result`, `run_completed`, `inference_completed`

**Verification**: Click "Run", terminal shows streaming output, sidebar pipeline
fills stage-by-stage, gates show pass/fail.

---

## Task E6: Scenario 3 — Memory (Compounding Intelligence)

**Proof point**: Second agent inherits first agent's knowledge and is measurably faster.

**Layout**: 2 panes (cold run THEN warm run — sequential, not simultaneous) + comparison sidebar

**Commands** (strictly sequential — Pane 1 starts AFTER Pane 0 completes):
```
Pane 0: roko do "Build a Rust CLI that parses CSV and outputs JSON"
[wait for completion — knowledge automatically written to neuro store]
Pane 1: roko do "Build a Rust CLI that parses TOML and outputs JSON"
[this run consumes knowledge from first run]
```

**IMPORTANT**: This is NOT a side-by-side race. Pane 1 MUST start after Pane 0 finishes
because the point is that Pane 1 benefits from Pane 0's knowledge.

**Custom sidebar panel**: `KnowledgeDeltaPanel.tsx`
- Cold run metrics (time, cost, tokens) — locked after completion
- Knowledge items generated (from `knowledge_ingested` events)
- Warm run metrics (live-updating)
- Delta: % faster, % cheaper, tokens saved
- Knowledge items CONSUMED by warm run (from `knowledge_consumed` events)

**SSE events consumed**: `inference_completed`, `knowledge_ingested`, `knowledge_consumed`,
`run_completed`

**Risk note** (from audit): Knowledge retrieval depends on neuro store's keyword matching.
If the prompts are too different, warm run may not retrieve cold run's knowledge.
Mitigation: Use similar prompts (CSV→JSON, TOML→JSON) that share structural patterns.

**Verification**: Cold run generates knowledge, warm run consumes it, delta visible.

---

## Task E7: Scenario 4 — ISFR (Agent Swarm)

**Proof point**: 4 specialized agents compute DeFi's risk-free rate from live data.

**Layout**: 4 panes (Lending Scout, Staking Scout, Aggregator, Validator) + rate display

**Pre-condition**: mirage-rs running with Ethereum mainnet fork

**Commands**:
```
Pane 0: Health check (cast bn)
Panes 0-3: Launch all 4 agents simultaneously
```

**Custom sidebar panel**: `SwarmPanel.tsx`
- 4 agent cards with role, status, model
- Directional arrows showing knowledge flow (scouts → aggregator → validator)
- Live rates from each scout
- Final ISFR rate when validator signs off
- Block number and freshness indicator

**SSE events consumed**: `agent_spawned`, `agent_output`, `agent_trace`

**Verification**: All 4 agents start, scouts produce rates, aggregator combines,
validator approves, final ISFR displayed.

---

## Task E8: Scenario 5 — Oracle (DeFi Data + Agent Intelligence)

**Proof point**: Agent reads on-chain data, reasons about it, writes to knowledge store.
Second agent reads knowledge, produces actionable recommendation.

**Layout**: 1 terminal (sequential commands) + Oracle sidebar

**Pre-condition**: mirage-rs running with Ethereum mainnet fork (localhost:8545)

**Commands** (sequential):
```
Step 1: curl -s localhost:8545 -X POST -d '{"jsonrpc":"2.0","method":"eth_blockNumber","id":1}' | jq .result
Step 2: roko run "Query Aave V3 and Compound lending rates on the local Anvil fork. Write structured analysis to knowledge store."
Step 3: roko run "Read DeFi rate analysis from knowledge store. Recommend optimal USDC allocation across protocols."
```

**Custom sidebar panel**: `OraclePanel.tsx`
```
┌─────────────────────────┐
│ ORACLE                  │
│                         │
│ ● DATA AGENT            │
│   scanning protocols... │
│                         │
│ ○ STRATEGY AGENT        │
│   waiting for data...   │
├─────────────────────────┤
│ RATES FOUND             │
│ Aave V3 USDC:  3.21%   │
│ Compound V3:   2.84%   │
│ Aave V3 DAI:   3.05%   │
├─────────────────────────┤
│ RECOMMENDATION          │
│ ━━━━━━━━━━━━━ pending   │
├─────────────────────────┤
│ CHAIN                   │
│ Block: 21,234,567       │
│ Chain: Anvil (31337)    │
│ Status: ● connected     │
├─────────────────────────┤
│ KNOWLEDGE FLOW          │
│ Data Agent → neuro/     │
│ neuro/ → Strategy Agent │
│ Entries: 0              │
└─────────────────────────┘
```

**SSE events consumed**: `agent_output`, `inference_completed`, `knowledge_ingested`,
`knowledge_consumed`

**Verification**: Chain connected, data agent produces rate analysis, knowledge store
shows entries, strategy agent reads and recommends.

---

## Dependency Graph

```
E1 (terminal fixes) ─── prerequisite for all scenarios
E2 (archive old)   ─── can be done immediately
E3 (SSE hook)      ─── prerequisite for E4-E8

E4 (Cost)     ─── independent
E5 (Pipeline) ─── independent (BEST FIRST SCENARIO — simplest)
E6 (Memory)   ─── independent (risky — test knowledge retrieval early)
E7 (ISFR)     ─── independent (needs mirage-rs)
E8 (Oracle)   ─── independent (needs mirage-rs, exercises everything)
```

E1, E2, E3 are prerequisites. E4-E8 are fully parallel.

**Recommended order**: E5 (Pipeline) first — simplest, 1 pane, proves the full stack.
Then E4 (Cost), E6 (Memory), E7 (ISFR), E8 (Oracle).

---

## Design Principles (from SCENARIO-REDESIGN.md)

1. **The sidebar IS the demo.** Terminal is supporting evidence. Sidebar tells the story.
2. **One click, one result.** First command produces the punchline.
3. **Under 2 minutes per scenario.** Small prompts, fast models, streaming.
4. **No hidden fragility.** No slugs, no hardcoded paths, no singletons, no shared workspaces.
5. **Show the delta, not the feature.** Not "roko has routing" — show "$0.14 → $0.03."
