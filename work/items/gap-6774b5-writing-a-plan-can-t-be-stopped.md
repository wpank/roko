+++
id = "gap-6774b5"
kind = "gap"
title = "Writing a plan can't be stopped and shows no spend: roko serve has no route that cancels a write and reports its cost only with its result, and the plan list has no row for a plan being written"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "M"
subsystem = ["roko-serve/plans"]
created = 2026-11-12
updated = 2026-11-12
source = "round v8 portal-v8, split from gap-6c4d24 (2026-11-12)"
discovered_from = "agent:portal-v8 (round v8, 33c33e738)"
anchors = ["crates/roko-serve/src/routes/plans/authoring.rs", "apps/portal/src/components/stage/PromptPanel.tsx::PromptPanel"]
links = { depends_on = [], blocks = [], related = ["gap-6c4d24"], supersedes = [], duplicate_of = "" }
+++

## Problem

While roko writes a plan, the portal can show the time and the request (gap-6c4d24), but nothing else the design
asks for:

- **No stop.** roko serve has no route that cancels a write: `/api/operations/{id}` is read-only
  (`crates/roko-serve/src/routes/status/mod.rs`), so a person who asked for the wrong thing waits for it to finish.
- **No spend.** A write's cost arrives only in its result (`cost_usd`, `routes/plans/authoring.rs`), so the glass
  can't show SPENT while roko writes.
- **No row.** The plan doesn't exist until it is written, so the plan list can't show "writing the plan" for it;
  the write lives only in the box that asked for it, and closing the box loses sight of it.

## Why it matters

Goal visibility: asking for a plan is the first thing a new user does, and a write takes up to a minute of paid
model calls they can't see or stop.

## Where

- `crates/roko-serve/src/routes/plans/authoring.rs` (the generate and revise operations),
  `crates/roko-serve/src/routes/status/mod.rs` (`/api/operations/{id}`).
- `apps/portal/src/components/stage/PromptPanel.tsx` (the writing view), `apps/portal/src/components/plans/PlanList.tsx`.

## Plan

1. roko serve: `POST /api/operations/{id}/cancel` for a write in progress, and the write's spend so far on
   `GET /api/operations/{id}` (or as events).
2. Portal: a stop key and SPENT on the writing view; a "writing the plan" row in the plan list while a write runs.

## Done when

A write can be stopped from the portal, and its spend shows while it runs (tests in roko-serve's authoring routes
and the portal's authoring acceptance).

## Notes

- Split from gap-6c4d24 (round v8 portal-v8), whose portal half shipped without these server parts.
