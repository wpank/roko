# Learning Summary — Batch 05

Concise runtime picture for agents entering `05` without prior context.

## What Is Already Real

- `LearningRuntime::record_completed_run(...)` is a genuine runtime fan-out hub.
- playbooks, playbook rules, skills, routing logs, router updates, metrics, cost tables, provider health, latency, C-Factor, and local reward all already exist with real persistence surfaces.
- the learning stack is far more runtime-real than earlier parity batches.

## What Is Misleading Today

- the main learned-context production path underuses the richer rule and skill selectors it already has,
- regression detection is headline-rich in docs but still overall-only in code,
- predictive calibration has real consumers but not the full primary pipeline the docs imply,
- some large learning modules still have no production caller,
- later framework docs sometimes read more “shipped” than the code supports.

## What Batch 05 Should Usually Do

1. make current learned-context selection richer,
2. make regression and calibration outputs more actionable,
3. resolve ambiguous dead scaffolding,
4. tighten partial routing/experiment loops,
5. explicitly defer research-heavy or governance-heavy work.
