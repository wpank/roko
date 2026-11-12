+++
id = "gap-60f6b6"
kind = "gap"
title = "The portal's plan view is a dense table under a run band and a stream pane that repeat it, and it hides tries (a Graph run's second try arrives as agent_spawned attempt 2, which the portal never counts): make the plan page the run card at full width, with the glass readings, the sequencer, one sentence and one orange key per state, details folded, and a task's checks, tries and output on click"
status = "open"
triage = "unverified"
severity = "p1"
goal = "visibility"
size = "L"
subsystem = ["apps/portal"]
created = 2026-11-12
updated = 2026-11-12
source = "round v8 portal-v8 audit (tmp/hermes-roko-v8/design/portal/before, main bd93521ee), the lead's approved proposal (tmp/hermes-roko-v8/design/portal/README.md, 2026-11-12)"
discovered_from = "agent:portal-v8 (round v8, bd93521ee)"
anchors = ["apps/portal/src/components/stage/PlanView.tsx::PlanView", "apps/portal/src/components/stage/TaskList.tsx::TaskList", "apps/portal/src/components/run/RunBand.tsx::RunBand", "apps/portal/src/components/stream/StreamPane.tsx::StreamPane", "apps/portal/src/lib/runState.ts", "plans/portal-programme/09-acceptance/browser-flow.cjs"]
links = { depends_on = ["gap-195374"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF 'counts a second try from agent_spawned' apps/portal/src && (cd apps/portal && npx vitest run src/lib/runState.tries.test.ts src/components/plan/planPage.accept.test.tsx)"
+++

## Problem

The plan view is a dense table under two other regions that repeat it, and it hides tries.

- **Three regions say the same thing:** the run band (`BURN 0 tok`, `AGENTS idle ×2`, `CHECKS · T1 … rung[compile]`),
  the plan view (a progress bar, a status line, a sentence, `W1/W2` wave chips, an 8-column task table) and the
  bottom stream pane (`transcript │ checks`). Progress shows in four places at once.
- **Engine words:** `W1`, `parallel 2`, `implementer·glm-4.7` in pink on every row, `rung[compile]`, `agents 0/2`,
  task ids as the main label; `BURN` stays at `0 tok` for a whole run; `~30m` left on a 20 s run.
- **Tries are invisible.** Task 3 of the greeting plan passed on its second try; no row shows a try count and the
  transcript runs both tries together. `runState.ts` counts a retry only when `task_started` follows a terminal
  record, but a Graph run's second try arrives as `agent_spawned` with `attempt: 2` (checked in the run's events).
- **At 1024px wide every task title disappears:** the row's grid gives the title column 0px.

Screens: `tmp/hermes-roko-v8/design/portal/before/02-plan.png`, `05`-`08`, `12-laptop-small.png`.

## Why it matters

Goal visibility. This is the page a person watches while roko works, and the one round v8's roko guide captures.
The approved design (`tmp/hermes-roko-v8/design/portal/README.md`) makes it the design system's run card at full
width, word for word the plugin's run page.

## Where

- `apps/portal/src/components/stage/` (`PlanView.tsx`, `TaskList.tsx`, `WaveStrip.tsx`, `RunChoice.tsx`,
  `ReviewPane.tsx`), `apps/portal/src/components/run/RunBand.tsx`, `apps/portal/src/components/stream/`.
- `apps/portal/src/lib/runState.ts` (`task_started`, `agent_spawned`: the try count),
  `taskRows.ts`, `waves.ts`, `checks.ts`, `statusLine.ts`.
- The CI browser check that drives these regions: `plans/portal-programme/09-acceptance/browser-flow.cjs`.

## Plan

1. **The head:** the state's pill (ready, running, passed, failed, interrupted, needs you, stopped) and the title.
2. **The glass:** `TASKS 1/3` with pips, `TIME 0:09` with an estimate only once it is credible, `SPENT $0.21` with a
   meter and "of $3.00" (the cap from `GET /api/plans/{id}/costs`), `CHECKS 2/7` with pips, in dot-matrix whose
   tone is the state. Before a run it sits unlit.
3. **The sequencer:** steps (today's waves) as columns, each task a pad (number, title, tries as pips, time), the
   dependency lines between them; the line into a running pad flows. A click opens the task.
4. **One sentence and at most one orange key, by state:** ready ("Each task has a check your project runs…",
   `edit` · **`run plan`**), running (one line per running task: "Task 2 · writing cli.py · glm-4.7", `stop`),
   interrupted (`start over` · **`resume`**, keeping gap-0eb763's question and wording), a task held for review
   (`decline` · **`approve`**). Passed and failed come in gap-6d2dd2.
5. **`▸ details`:** each task's tries with the model, the plan check, tokens.
6. **A task, opened:** what it does, its checks with their commands, each try with its model, spend and the reason it
   failed, and what the agent did (the transcript, with a divider per try), folded.
7. **Tries:** count a try from `agent_spawned.attempt` (and the transcript's per-try divider with it).
8. Remove the run band, the bottom stream pane, the wave strip and the task table, and move the browser check's
   selectors to the new page.

## Done when

- CI's portal job passes (typecheck, tests, orphans, export), and the browser check passes on the round serve.
- `src/lib/runState.tries.test.ts` ("counts a second try from agent_spawned") and
  `src/components/plan/planPage.accept.test.tsx` pass.
- Screenshots, day and night, desktop and phone, match the mock (`design/portal/after/ready-*`, `running-*`, `task-*`).

## Notes

- Depends on gap-195374 (the look and the kit).
- The words follow the design system's §6: "Task 3", not `T3`; "try 2", "tried again".
