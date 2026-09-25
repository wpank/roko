# Portal Redesign — Phase 1: New Shell

Delete 72 old page files. Create the 3-panel workspace layout.

## Dependency DAG

```
P1-T01 (delete old files)
  ↓
P1-T02 (WorkspaceShell) ← P1-T03 (PlanSidebar) ← P1-T05 (OutputPanel)
  ↓
P1-T04 (MainArea, depends on T02 + T03)
  ↓
P1-T06 (wire page.tsx, depends on T02-T05)
```

## Context

Spec: `tmp/portal-audit/SPEC.md`
API: `tmp/portal-audit/API.md`
Plan: `tmp/portal-audit/PLAN.md`

## Acceptance

- Portal renders at localhost:3000 with sidebar + main area
- Plan list shows plans from API
- Clicking a plan shows its tasks
- TypeScript compiles clean
- Production build succeeds
