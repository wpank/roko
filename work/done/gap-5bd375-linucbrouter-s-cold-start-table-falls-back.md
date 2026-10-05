+++
id = "gap-5bd375"
kind = "gap"
title = "LinUCBRouter's cold-start table falls back to a literal, maybe-unconfigured model slug"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn/model-router"]
created = 2026-10-03
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "19f76451c"
source = "wave-5 follow-up reports 2026-10-02 (PK78 gap-2339e2)"
discovered_from = "gap-2339e2"
anchors = ["crates/roko-learn/src/model_router.rs::default_static_table", "crates/roko-learn/src/cascade_router.rs::CascadeRouter", "crates/roko-cli/src/model_selection.rs::select_candidate"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cold_start_table_refuses_an_unconfigured_slug' crates/roko-learn/ && cargo test -p roko-learn cold_start_table_refuses_an_unconfigured_slug"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T15:52:38Z"
commit = "19f76451c"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-05T09:03:11Z"
forced = false
evidence = "Gate 19 (merged 19f76451c): verify passes (cold_start_table_refuses_an_unconfigured_slug). LinUCB's cold-start table falls back to the tier's configured model, else the first configured."
+++

## Problem

`crates/roko-learn/src/model_router.rs`'s cold-start static routing table (`default_static_table`,
`pick_static_slug`, `pick_static_from_candidates`, around lines 1540-1595) falls back to a **literal, hard-coded
model slug** — `candidates[0].to_string()` in `pick_static_slug` (the candidate list itself is a hard-coded
`&["claude-haiku-4-5"]` / `&["glm-5.1", "claude-sonnet-4-6", "claude-sonnet-4-5"]` / `&["claude-opus-4-6"]` per tier)
— whenever none of the configured `model_slugs` matches one of those candidates. A deployment whose configured
models don't include any of these exact slugs gets routed to a slug that may not even be configured for that
deployment.

This path is reachable only through `LinUCBRouter`'s cold start inside `CascadeRouter::select`
(`crates/roko-learn/src/cascade_router.rs:650`). `CascadeRouter::select` is itself textually called from exactly
one non-test production site, `crates/roko-cli/src/model_selection.rs:482` inside `select_candidate` — but every
production caller of `select_candidate` (via `resolve_effective_model`/`resolve_planner_selection`, called from
`run.rs`, `commands/plan.rs`, `commands/run_cmd.rs`, `plan_generate/pipeline.rs`, `chat_inline/session.rs`,
`unified.rs`, `commands/config_cmd.rs`, `plan_authoring.rs`) passes `cascade_router: None` for the parameter that
would need to be `Some(&CascadeRouter)` to reach it (confirmed by reading `resolve_effective_model`'s and
`resolve_planner_selection`'s own call sites, and their signatures threading the parameter straight through with
no construction of a real `CascadeRouter` anywhere in that file). So the `if let Some(router) = cascade_router`
branch at `model_selection.rs:476` is live code that no production path currently exercises — team-lead's framing
("no production code calls it") holds once traced through this one extra layer.

## Why it matters

A cold-start fallback to a slug that might not be configured is a real defect if this path ever becomes reachable
(e.g. if a future caller starts passing a real `CascadeRouter` into `resolve_effective_model`), but today it is
inert: nothing a user does can reach it. Low priority, tracked so it isn't lost and so the fallback is fixed before
anything wires a real caller to it.

## Where

- `crates/roko-learn/src/model_router.rs`: `default_static_table`, `pick_static_slug`, `pick_static_from_candidates`
  (around lines 1540-1595).
- Reachability: `crates/roko-learn/src/cascade_router.rs::CascadeRouter::select` (line 650); its one production
  call site `crates/roko-cli/src/model_selection.rs::select_candidate` (line 431, the `router.select(Vec::new())`
  at line 482); every caller of `resolve_effective_model`/`resolve_planner_selection` passes `None` for
  `cascade_router`.

## Current state

Unfixed, and currently unreachable in production for the reason above.

## Plan

1. Make the literal-slug fallback fail closed instead of guessing: when none of the hard-coded candidates is in
   `model_slugs`, return an error or `None` (forcing the caller to pick explicitly) rather than
   `candidates[0].to_string()`.
2. Or, read the fallback slugs from the routing config's tier map instead of hard-coding them, consistent with how
   backlog task 9207 ("Static routing picks from the configured tier map, not hard-coded Gemini and Claude slugs,"
   part of `gap-2339e2`/PK78, implemented at `ca7db2706`) just fixed the same class of problem elsewhere in this
   same router.

## Done when

- The cold-start table either refuses to name an unconfigured model, or picks from the configured tier map.
- The `[[verify]]` command passes.

## Notes

- p3: confirmed unreachable from any production call site today (see Where); raise priority if a future change
  starts passing a real `CascadeRouter` into `resolve_effective_model`/`resolve_planner_selection`.
- Compare with 9207 (`gap-2339e2`), which fixed the literal-slug problem in the *other* static routing path in this
  same file; this fallback was apparently missed by that pass.
