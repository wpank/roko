# Implementation Order

This file is the human-readable execution order for `plans/`. `plans/INDEX.md`
is generated and shows status/counts; this file explains what to run next.

## Source Of Truth

- Treat each `tasks.toml` `[meta].status` as authoritative.
- Do not execute plans marked `superseded` or `archived`.
- Within a plan, follow each task's `depends_on` graph.
- Across the main self-development queue, run one plan directory at a time in
  the sequence below unless you intentionally split work.

## Already Complete

These are complete and should not be rerun as normal backlog work:

1. `W01-wire-system-prompts`
2. `P06-process-management`
3. `P07-autofix-retry`

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

- `architecture-core-queue` is the larger architecture queue. It is separate
  from the P08-P34 self-development repair queue.
- `architecture-defi-critical-path` depends on the architecture queue's Q14
  chain foundation work. Do not run it before the relevant architecture-core
  prerequisite is implemented.
- `dry-run-flag`, `e2e-smoke`, `live-demo-phase1`, and `live-demo-phase2` are
  standalone side/demo queues. Run `live-demo-phase1` before
  `live-demo-phase2`.

## Superseded

Do not run these directly. Their still-ready tasks were consolidated into the
P08-P34 queue.

- `self-dev-ux`
- `self-dev-extras`
