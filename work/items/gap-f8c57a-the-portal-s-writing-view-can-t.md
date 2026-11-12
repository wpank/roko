+++
id = "gap-f8c57a"
kind = "gap"
title = "The portal's writing view can't stop a write or show what it has spent, and the plan list loses a write in progress: no stop key, TIME is the only reading on its glass, and a plan being written has no row, so closing the box loses sight of it"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "M"
subsystem = ["apps/portal"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round v8 portal-v8, the lead's ask (2026-11-12)"
discovered_from = "agent:portal-v8 (round v8)"
anchors = ["apps/portal/src/components/stage/PromptPanel.tsx::PromptPanel", "apps/portal/src/lib/operation.ts::waitForOperation", "apps/portal/src/components/plans/PlanList.tsx"]
links = { depends_on = ["bug-36ce72", "gap-6774b5"], blocks = [], related = ["gap-6c4d24"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cd apps/portal && npx vitest run src/components/stage/writing.accept.test.tsx"
+++

## Problem

While roko writes a plan, the portal's writing view (gap-6c4d24) shows the time and the request, and nothing else
the design asks for:

- **No stop.** A person who asked for the wrong thing waits for the write to finish, and pays for it.
- **No spend.** The glass has TIME only. What the write has spent shows nowhere until it ends, and then only in
  the operation's result.
- **No row.** The plan doesn't exist until it is written, so the plan list has no row for it. Close the box, or
  open the portal in another tab, and the write is out of sight until a plan appears.

roko serve gains the parts this needs: a write is cancellable through `POST /api/plans/{id}/cancel` (bug-36ce72);
its calls are priced on the event stream as they settle, and `GET /api/plans` lists a row for a plan being written
(gap-6774b5).

## Why it matters

Goal visibility: asking for a plan is the first thing a new user does. A write takes up to a minute of paid model
calls, which they can neither see nor stop.

## Where

- `apps/portal/src/components/stage/PromptPanel.tsx`: the writing view.
- `apps/portal/src/lib/operation.ts`: `waitForOperation`, which follows the write.
- `apps/portal/src/components/plans/PlanList.tsx`, `apps/portal/src/lib/planRows.ts`, `apps/portal/src/lib/planWords.ts`:
  the plan list.
- `apps/portal/src/api/contracts.ts`: the wire shapes.

## Current state

- `waitForOperation` knows `running`, `completed` and `failed`. A `cancelled` write would poll until its timeout.
- The writing view's glass has one cell, TIME.
- The plan list reads `GET /api/plans` and has no state for a plan being written.

## Plan

1. **Stop:** a `stop` key on the writing view. It posts `POST /api/plans/{operation id}/cancel`, and the write ends
   `stopped`. The box comes back with the request in it, to edit, and says what the write spent: "Stopped. Writing
   the plan spent $0.04." A write that ended first shows its plan, as it would have.
2. **SPENT on glass:** a second cell beside TIME, with its cap: the write's `cost_usd` from its progress, which moves
   as each planner call settles, and "of $2.00" from the cap that the generate answer gave.
3. **The row:** a plan being written gets a row in the plan list. It has a live light, the request as its title,
   and "writing the plan · $0.04". Selecting it opens the writing view on that write, with its stop key and readings.
   The row goes when the write ends: the plan's own row replaces it, or nothing does when the write failed or
   stopped.
4. Older servers: no `writing` rows, and a cancel that answers 404 or 405 hides the stop key with a sentence.
5. Tests: a new `writing.accept.test.tsx`; the browser check's write step presses stop once.

## Done when

- A write can be stopped from the portal, the glass shows SPENT with its cap while roko writes, and a write in
  progress has a row in the plan list that opens it.
- The `[[verify]]` passes.

## Notes

- Round v8 portal-v8, the lead's ask (2026-11-12): "the portal's writing view gets a `stop` key and SPENT on glass".
