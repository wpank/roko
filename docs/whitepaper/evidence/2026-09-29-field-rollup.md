# Field evidence rollup

> Generated 2026-09-29T14:37:51 by `tools/field_rollup.py` from 42 snapshot(s) and 124 field note(s). Observational data from real runs, not a controlled experiment; see `README.md` for definitions and caveats. Do not edit by hand.

> Backfilled runs (captured after the fact) keep only each plan's latest run: failed attempts from an earlier run of the same plan, before a resume or re-run, are not included, so first-try rates for backfilled runs are upper bounds. Runs captured live by the watcher do not have this gap.

> Each field note counts once: in the run its `run` field names, else in the latest run of its plan that started before the note, else on the note's own day. A retried pass is an auto recovery only when no operator or user intervention note on that task joined the same run.

## Totals

| Runs | Tasks | Verified passes | Unverified | First-try passes | Attempts | Auto recoveries | Interventions | Autonomy index | Escapes (linked to a captured run + other) | Escape rate (linked) | Cost | Cost per verified |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 42 | 158 | 158 | 0 | 155 (98%) | 253 | 2 | 39 | 2/41 (5%) | 11 + 22 | 7% | $192.92 | $1.22 |

## By day

| Day | Runs | Tasks | Verified | First-try | Retries | Auto recoveries | Interventions | Autonomy | Escapes (linked + other) | Cost |
|---|---|---|---|---|---|---|---|---|---|---|
| 2026-08-22 | 1 | 0 | 0 | 0 | 3 | 0 | 0 | 0/0 | 0 + 0 | $2.87 |
| 2026-08-31 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 + 1 | $0.00 |
| 2026-09-14 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 + 0 | $3.96 |
| 2026-09-15 | 5 | 0 | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 + 0 | $1.03 |
| 2026-09-18 | 5 | 0 | 0 | 0 | 0 | 0 | 2 | 0/2 (0%) | 0 + 1 | $1.60 |
| 2026-09-19 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 + 1 | $0.00 |
| 2026-09-21 | 9 | 0 | 0 | 0 | 0 | 0 | 0 | 0/0 | 0 + 1 | $4.79 |
| 2026-09-22 | 2 | 0 | 0 | 0 | 6 | 0 | 0 | 0/0 | 0 + 0 | $0.09 |
| 2026-09-23 | 1 | 0 | 0 | 0 | 1 | 0 | 0 | 0/0 | 0 + 0 | $5.78 |
| 2026-09-25 | 0 | 0 | 0 | 0 | 0 | 0 | 4 | 0/4 (0%) | 0 + 12 | $0.00 |
| 2026-09-26 | 2 | 0 | 0 | 0 | 9 | 0 | 8 | 0/8 (0%) | 4 + 0 | $13.44 |
| 2026-09-28 | 9 | 89 | 89 | 88 | 1 | 1 | 15 | 1/16 (6%) | 4 + 5 | $60.83 |
| 2026-09-29 | 7 | 69 | 69 | 67 | 5 | 1 | 10 | 1/11 (9%) | 3 + 1 | $98.53 |

## By run

| Date | Workspace | Plan | Status | Tasks | Passed | Unver. | Attempts | First-try | Auto rec. | Interv. | Escapes | Models | Cost | $/verified | Busy min | Mean/peak conc. (cap) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-09-14 | roko | async-runtime-antipatterns | succeeded | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | – | $3.96 | – | 0.0 | None/0 (1) |
| 2026-09-15 | roko | speed-benchmark | succeeded | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | – | $0.75 | – | 0.0 | None/0 (3) |
| 2026-08-22 | roko | cli-ux-consistency | succeeded | 0 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | claude-sonnet-4-6 ×6 | $2.87 | – | 10.0 | 1.0/1 (2) |
| 2026-09-15 | roko | speed-openai | succeeded | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | gpt-4o-mini ×1 | $0.00 | – | 0.0 | None/0 (1) |
| 2026-09-15 | roko | speed-moonshot | succeeded | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | kimi-k2.6 ×1 | $0.01 | – | 0.0 | None/0 (1) |
| 2026-09-15 | roko | speed-zai | succeeded | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | glm-5v-turbo ×1 | $0.02 | – | 0.0 | None/0 (1) |
| 2026-09-15 | roko | speed-minimal | succeeded | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | claude-sonnet-4-6 ×1 | $0.25 | – | 0.0 | None/0 (1) |
| 2026-09-18 | roko | disabled-providers-list | succeeded | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | gpt-oss-120b ×1 | $0.43 | – | 0.0 | None/0 (1) |
| 2026-09-18 | roko | gate-failures-jsonl | succeeded | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | gpt-oss-120b ×1 | $0.07 | – | 0.0 | None/0 (1) |
| 2026-09-18 | roko | hardcoded-backend-strings | succeeded | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | gpt-oss-120b ×1 | $0.67 | – | 0.0 | None/0 (1) |
| 2026-09-18 | roko | legacy-page-removal | succeeded | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | gpt-oss-120b ×1 | $0.03 | – | 0.0 | None/0 (1) |
| 2026-09-18 | roko | plugin-load-verification | succeeded | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | gpt-oss-120b ×2 | $0.40 | – | 0.0 | None/0 (1) |
| 2026-09-21 | roko | demo-incident-tabletop | succeeded | 0 | 0 | 0 | 4 | 0 | 0 | 0 | 0 | gpt-oss-120b ×4 | $2.03 | – | 0.0 | None/0 (1) |
| 2026-09-21 | roko | demo-multistage | succeeded | 0 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | gpt-oss-120b ×5 | $0.50 | – | 0.0 | None/0 (1) |
| 2026-09-21 | roko | demo-parallel-integration | succeeded | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | gpt-oss-120b ×3 | $0.43 | – | 0.0 | None/0 (2) |
| 2026-09-21 | roko | validate-foundation | succeeded | 0 | 0 | 0 | 4 | 0 | 0 | 0 | 0 | gpt-oss-120b ×4 | $0.52 | – | 0.0 | None/0 (2) |
| 2026-09-21 | roko | validate-roles | succeeded | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | gpt-oss-120b ×2 | $0.67 | – | 0.0 | None/0 (2) |
| 2026-09-21 | roko | validate-dispatch | succeeded | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | gpt-oss-120b ×3 | $0.28 | – | 0.0 | None/0 (2) |
| 2026-09-21 | roko | validate-quality | succeeded | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | gpt-oss-120b ×3 | $0.20 | – | 0.0 | None/0 (3) |
| 2026-09-21 | roko | validate-resilience | succeeded | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | gpt-oss-120b ×3 | $0.09 | – | 0.0 | None/0 (3) |
| 2026-09-21 | roko | validate-safety | succeeded | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | gpt-oss-120b ×3 | $0.07 | – | 0.0 | None/0 (3) |
| 2026-09-22 | roko | speed-cerebras | succeeded | 0 | 0 | 0 | 7 | 0 | 0 | 0 | 0 | gpt-oss-120b ×7 | $0.02 | – | 0.0 | None/0 (1) |
| 2026-09-22 | roko | validate-xl | succeeded | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | gpt-oss-120b ×3 | $0.07 | – | 0.0 | None/0 (3) |
| 2026-09-23 | roko | example-hello-world | succeeded | 0 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | gpt-oss-120b ×1, claude-sonnet-4-6 ×5 | $5.78 | – | 0.0 | None/0 (None) |
| 2026-09-26 | roko | 01-backend-plan-service | succeeded | 0 | 0 | 0 | 19 | 0 | 0 | 2 | 3 | claude-sonnet-4-6 ×19 | $3.85 | – | 0.0 | None/0 (1) |
| 2026-09-26 | roko | 02-backend-plan-execution | failed | 0 | 0 | 0 | 7 | 0 | 0 | 2 | 1 | claude-sonnet-4-6 ×7 | $9.59 | – | 0.0 | None/0 (1) |
| 2026-09-28 | roko-portal-wt | 05-portal-foundation | succeeded | 14 | 14 | 0 | 14 | 14 | 0 | 2 | 0 | claude-sonnet-4-6 ×14 | $9.15 | $0.65 | 0.0 | None/0 (1) |
| 2026-09-28 | roko-portal-wt | 06-portal-shell | succeeded | 9 | 9 | 0 | 9 | 9 | 0 | 0 | 0 | claude-sonnet-4-6 ×9 | $4.97 | $0.55 | 0.0 | None/0 (3) |
| 2026-09-28 | roko-portal-wt | 07-portal-compose | succeeded | 10 | 10 | 0 | 10 | 10 | 0 | 0 | 0 | claude-sonnet-4-6 ×10 | $7.44 | $0.74 | 0.0 | None/0 (3) |
| 2026-09-28 | roko-portal-wt | 08-portal-run | succeeded | 10 | 10 | 0 | 10 | 10 | 0 | 0 | 1 | claude-sonnet-4-6 ×10 | $6.32 | $0.63 | 0.0 | None/0 (3) |
| 2026-09-28 | roko-backend-wt | 03-backend-live-events | succeeded | 7 | 7 | 0 | 7 | 7 | 0 | 0 | 0 | claude-sonnet-4-6 ×7 | $6.40 | $0.91 | 0.0 | None/0 (1) |
| 2026-09-28 | roko-portal2-wt | 08b-portal-polish | succeeded | 16 | 16 | 0 | 16 | 16 | 0 | 1 | 1 | claude-sonnet-4-6 ×16 | $11.34 | $0.71 | 0.0 | None/0 (4) |
| 2026-09-28 | roko-portal2-wt | 08c-portal-live-steps | succeeded | 5 | 5 | 0 | 5 | 5 | 0 | 0 | 0 | claude-sonnet-4-6 ×5 | $3.23 | $0.65 | 0.0 | None/0 (2) |
| 2026-09-28 | roko-portal2-wt | 08d-portal-legibility | succeeded | 11 | 11 | 0 | 11 | 11 | 0 | 0 | 1 | claude-sonnet-4-6 ×11 | $8.25 | $0.75 | 18.1 | 1.9/4 (4) |
| 2026-09-28 | roko-portal2-wt | 08e-portal-refine | succeeded | 7 | 7 | 0 | 8 | 6 | 1 | 0 | 1 | claude-sonnet-4-6 ×8 | $3.73 | $0.53 | 18.6 | 1.4/3 (3) |
| 2026-09-29 | roko-backend-wt | 03b-backend-workspace-server | succeeded | 18 | 18 | 0 | 18 | 18 | 0 | 1 | 0 | claude-sonnet-4-6 ×18 | $29.31 | $1.63 | 0.0 | None/0 (1) |
| 2026-09-29 | roko-backend2-wt | 04-backend-plan-authoring | succeeded | 15 | 15 | 0 | 15 | 15 | 0 | 0 | 0 | claude-sonnet-4-6 ×15 | $20.20 | $1.35 | 185.8 | 1.14/2 (2) |
| 2026-09-29 | roko-backend2-wt | 04b-backend-plan-revision | succeeded | 6 | 6 | 0 | 6 | 6 | 0 | 0 | 0 | claude-sonnet-4-6 ×6 | $4.70 | $0.78 | 76.4 | 1.0/1 (1) |
| 2026-09-29 | roko-backend-wt | 03c-backend-local-access | succeeded | 20 | 20 | 0 | 21 | 19 | 1 | 0 | 0 | claude-sonnet-4-6 ×21 | $37.82 | $1.89 | 313.2 | 1.39/2 (2) |
| 2026-09-29 | roko | 08f-final-polish | succeeded | 6 | 6 | 0 | 9 | 5 | 0 | 3 | 1 | claude-sonnet-4-6 ×9 | $3.73 | $0.62 | 39.3 | 1.19/3 (4) |
| 2026-09-29 | roko | 08g-first-run | succeeded | 3 | 3 | 0 | 3 | 3 | 0 | 0 | 0 | claude-sonnet-4-6 ×3 | $1.00 | $0.33 | 3.7 | 1.0/1 (1) |
| 2026-09-29 | roko | 09-acceptance | failed | 1 | 1 | 0 | 3 | 1 | 0 | 2 | 2 | claude-sonnet-4-6 ×3 | $1.77 | $1.77 | 21.9 | 1.0/1 (1) |

## Field notes by kind and category

- Kinds: intervention 39, escape 33, measurement 19, loop-event 15, anecdote 9, decision 9
- Categories: other 17, product-defect 13, weak-gate 11, learning-loop 10, isolation 9, plan-authoring 8, false-green 6, telemetry 5, scheduler 4, tooling-bug 4, safety 4, timeout 3, cost-accounting 3, disk 3, role-policy 3, turn-cap 3, resume 3, provider-failover 3, merge-break 3, ux 2, infra 2, session-limit 2, sibling-contamination 2, spec-defect 1
- Detected by → fixed by: operator → operator (40), ? → operator (5), browser-pass → roko (4), audit → operator (4), roko → operator (3), audit → roko (3), operator → none (2), audit → - (2), operator → - (2), review → none (2), browser-pass → operator (1), operator → roko (1), smoke-test → operator (1), audit → none (1), browser-pass → - (1)
- Joined to a captured run: 43 of 124
- Retried passes not counted as auto recoveries (an intervention note names the task): 08f-final-polish T05
