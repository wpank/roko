# Portal Redesign — Phase 2: Plan Workflow

Wire the complete flow: prompt → execute → watch → done.

## Dependency DAG

```
P2-T01 (prompt submission)
  ↓
P2-T02 (execution + live updates)
  ↓                               P2-T03 (output panel wiring)
P2-T04 (done state + costs)
```

## Context

Spec: `tmp/portal-audit/SPEC.md`
API: `tmp/portal-audit/API.md`

## Acceptance

- Type prompt → agent executes → files created
- Execute button → tasks update live → progress bar fills
- Agent output streams in bottom panel
- Done state shows costs
