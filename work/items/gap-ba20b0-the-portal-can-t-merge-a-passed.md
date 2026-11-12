+++
id = "gap-ba20b0"
kind = "gap"
title = "The portal can't merge a passed run's work: roko serve has no merge route (the Hermes plugin merges in its own Python), so the portal's run card shows the merge command with copy and review as its orange key where the plugin's is merge"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "M"
hold = "The lead's call (2026-11-12): changing the person's checkout from a web page is a separate, security-sensitive item; not round v8"
subsystem = ["roko-serve/runs"]
created = 2026-11-12
updated = 2026-11-12
source = "round v8 portal-v8 audit (tmp/hermes-roko-v8/design/portal/before, main bd93521ee), the lead's approved proposal (tmp/hermes-roko-v8/design/portal/README.md, 2026-11-12)"
discovered_from = "agent:portal-v8 (round v8, bd93521ee)"
anchors = ["crates/roko-serve/src/routes/runs.rs", "crates/roko-serve/src/routes/run_result.rs::run_result"]
links = { depends_on = [], blocks = [], related = ["gap-6d2dd2"], supersedes = [], duplicate_of = "" }
+++

## Problem

The portal can't take a passed run's work into the person's branch. roko serve has no merge route; the Hermes plugin
merges in its own Python (`hermes-plugin-roko`, `review.py::merge_run`: fast-forward or `--no-ff`, a confirm for
unverified work, a check that the branch did not move since the review, an undo command). So the portal's run card
shows the merge command with `copy`, and its orange key after a pass is `review`, where the plugin's is `merge`.

## Why it matters

Goal visibility: the design system's run card has `merge` as the passed state's orange key (§4.8).

## Where

- `crates/roko-serve/src/routes/runs.rs` (a new `POST /api/runs/{id}/merge` beside `summary` and `diff`),
  `crates/roko-serve/src/routes/run_result.rs::run_result` (`merge_command`, `merged`).

## Plan

Not this round. If it is taken up: a route that merges only the reviewed commit (the client sends the commit it
showed), fast-forward when it can, refuses a dirty checkout or a moved branch, asks for confirmation on unverified
work, returns the undo command, and needs the write scope; mirror the plugin's rules.

## Done when

A held design item; it gets a verify when it is taken up.

## Notes

- The lead's call (2026-11-12): changing the person's checkout from a web page is a separate, security-sensitive
  item; the portal shows the merge command with `copy` and `review` as the orange key this round.
