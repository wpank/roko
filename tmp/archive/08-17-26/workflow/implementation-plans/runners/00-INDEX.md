# Agent Runners — Index

Each file in this folder is a **self-contained prompt** you can hand to a fresh agent (cloud agent, Cursor agent, or any code-capable LLM) to execute one or more implementation plans.

## Runners

| Runner File | Plans Covered | Dependencies |
|------------|--------------|-------------|
| [01-modelcallservice.md](01-modelcallservice.md) | Plan 01 (ModelCallService) | None — start here |
| [02-prompt-assembly.md](02-prompt-assembly.md) | Plan 02 (PromptAssembly) | 01 in progress |
| [03-feedback-service.md](03-feedback-service.md) | Plan 03 (FeedbackService) | 01, 02 in progress |
| [04-persistence.md](04-persistence.md) | Plan 04 (PersistenceService) | Independent |
| [05-07-pipeline-scheduler-driver.md](05-07-pipeline-scheduler-driver.md) | Plans 05+06+07 (Pipeline+Scheduler+Driver) | 01-04 complete |
| [08-cascade-router.md](08-cascade-router.md) | Plan 08 (CascadeRouter) | 01, 03 |
| [09-safety.md](09-safety.md) | Plan 09 (Safety) | 07 |
| [10-12-observability-convergence-retirement.md](10-12-observability-convergence-retirement.md) | Plans 10+11+12 | 01-09 complete |
| [13-16-gates-providers-cognitive-cli.md](13-16-gates-providers-cognitive-cli.md) | Plans 13+14+15+16 | 01-04 (parallelizable) |
| [17-18-demo-proofs.md](17-18-demo-proofs.md) | Plans 17+18 | All above complete |

## Execution Order

```
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│  Runner 01   │  │  Runner 02   │  │  Runner 04   │
│  ModelCall   │  │  Prompt Asm  │  │  Persistence │
│  (start now) │  │  (parallel)  │  │  (parallel)  │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                 │                 │
       └────────┬────────┘                 │
                │                          │
       ┌────────▼────────┐                 │
       │   Runner 03     │                 │
       │   Feedback      │                 │
       └────────┬────────┘                 │
                │                          │
                └─────────┬────────────────┘
                          │
               ┌──────────▼──────────┐    ┌──────────────────┐
               │  Runner 05-07       │    │  Runner 13-16    │
               │  Pipeline+Sched+Drv │    │  Gates/Prov/Cog  │
               └──────────┬──────────┘    │  (parallel)      │
                          │               └────────┬─────────┘
               ┌──────────▼──────────┐             │
               │  Runner 08 + 09     │             │
               │  Router + Safety    │             │
               │  (parallel)         │             │
               └──────────┬──────────┘             │
                          │                        │
                          └────────┬───────────────┘
                                   │
                        ┌──────────▼──────────┐
                        │  Runner 10-12       │
                        │  Observe+Converge   │
                        │  +Retire            │
                        └──────────┬──────────┘
                                   │
                        ┌──────────▼──────────┐
                        │  Runner 17-18       │
                        │  Demo + Proofs      │
                        └─────────────────────┘
```

## Parallelization

You can run up to **4 agents in parallel** during the early phases:

1. **Wave 1 (parallel):** Runners 01, 02, 04
2. **Wave 2:** Runner 03 (needs 01+02)
3. **Wave 3 (parallel):** Runners 05-07, 13-16
4. **Wave 4 (parallel):** Runners 08, 09
5. **Wave 5:** Runner 10-12
6. **Wave 6:** Runner 17-18

## How To Use

1. Pick a runner whose dependencies are satisfied
2. Open a fresh agent session (Cursor agent, background agent, etc.)
3. Paste the **entire** runner file as the first message
4. The agent reads the listed files, makes changes, runs verification commands
5. When the agent finishes, check the boxes in `ISSUE-TRACKER.md`
6. Move to the next runner

## Issue Tracker

All checkboxes live in [../ISSUE-TRACKER.md](../ISSUE-TRACKER.md). Check boxes as each runner completes its items.
