# Implementation Order

This file is the human-readable execution order for `plans/`. `plans/INDEX.md`
is generated and shows status/counts; this file explains what to run next.

Current corpus: 30 executable plans with 144 tasks, plus two superseded plans
with 66 excluded tasks. The separately recovered 24-task
`architecture-core-queue` is included in those current generated totals. The
120-task sealed P08-P34/side-queue boundary and its canonical ownership mapping
are recorded in [`EXECUTION-OWNERSHIP.md`](EXECUTION-OWNERSHIP.md).

## Source Of Truth

- Treat each `tasks.toml` `[meta].status` as authoritative.
- Do not execute plans marked `superseded` or `archived`.
- Within a plan, follow each task's `depends_on` graph.
- Across plans, follow the dependency order in the canonical
  [`MASTER-EXECUTION-CHECKLIST.md`](../../tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md)
  and the ownership mappings in `EXECUTION-OWNERSHIP.md`. The numeric sequence
  below is a queue identity/order, not permission to bypass those dependencies.

## Already Complete

These are complete and should not be rerun as normal backlog work:

1. `W01-wire-system-prompts`
2. `P06-process-management`
3. `P07-autofix-retry`

These are historical plan names, not current runnable roots: none has a tracked
`plans/<name>/tasks.toml` in the current tree.

## Primary Queue

Run these in numeric order. `P34-verification-sweep` is the final verification
pass for this queue.

1. `P08-search-command-fix`
2. `P09-tool-alias-fix`
3. `P10-slash-command-flags`
4. `P11-runner-v2-default`
5. `P12-runner-parallelism`
6. `P13-rate-limit-retry`
7. `P14-gate-rung-fix`
8. `P15-error-recovery-wiring`
9. `P16-safety-contracts`
10. `P17-cli-output-format`
11. `P18-tui-agent-data`
12. `P19-cascade-router-acp`
13. `P20-zero-config`
14. `P21-acp-streaming`
15. `P22-acp-tool-permission`
16. `P23-prd-pipeline-fix`
17. `P24-workspace-paths`
18. `P25-mcp-acp-passthrough`
19. `P26-hdc-similarity-lookup`
20. `P27-provider-error-ux`
21. `P28-image-support`
22. `P29-develop-command-wire`
23. `P30-onboarding-doctor`
24. `P31-note-and-context`
25. `P32-cli-polish`
26. `P33-model-ux`
27. `P34-verification-sweep`

## Separate Queues

- [`architecture-core-queue`](../architecture-core-queue/tasks.toml) is a
  tracked, non-empty, `ready` 24-task architecture queue. It was recovered from
  the byte-identical historical source and is separate from the sealed 120-task
  P08-P34/side population.
- [`architecture-defi-critical-path`](../architecture-defi-critical-path/tasks.toml)
  is a tracked, non-empty, `ready` three-task queue. Its three parity records
  reference `architecture-core-queue#Q14-chain-registries-defi-foundation`; do
  not run it before that prerequisite has reached its required accepted state.
- [`e2e-smoke`](../e2e-smoke/tasks.toml) is the only retained standalone
  side/demo queue. It is tracked, non-empty, `ready`, and contains two tasks.

## Removed Historical Roots

These names appeared in the pre-removal plan corpus but are not current plan
roots. Commit `7899494d336d83a7bf3dc95b6592f1b90de02c8f` deleted all three manifests.
They are absent from the generated index and must not be passed to `roko plan
run` or recreated as empty directories.

| Historical root | Last tracked contents | Current disposition and mapping |
|---|---|---|
| `dry-run-flag` | 10 ready tasks for a proposed workflow-engine preview flag | Removed in `7899494d`; no current manifest or task-for-task supersession exists. P11/E01/self-heal own related execution-honesty outcomes, but they are not an equivalent dry-run feature replacement. Any revival requires a newly reviewed canonical plan against current engines. |
| `live-demo-phase1` | 2 ready synthetic `roko-std` greeting tasks | Removed in `7899494d`; no current replacement. Not mapped to `e2e-smoke`, whose share-token acceptance is different. |
| `live-demo-phase2` | 2 ready synthetic `roko-std` farewell tasks, historically ordered after phase 1 | Removed in `7899494d`; no current replacement. Its former dependency is historical and non-runnable. |

## Superseded

Do not run these directly. Their still-ready tasks were consolidated into the
P08-P34 queue.

- `self-dev-ux`
- `self-dev-extras`
