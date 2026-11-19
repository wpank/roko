+++
id = "bug-1e8263"
kind = "bug"
title = "A second plan write for a plan still being written starts anyway: derive_unique_slug only checks for a plan directory on disk, which a running write hasn't made yet, so two operations wrote plans/p5-5-auval-clean and the last one's files won (phase 5.5: f43afc45 and c281646c, 11:02:57-11:03:44Z)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-serve/plans"]
created = 2026-10-28
updated = 2026-10-28
last_verified = 2026-10-28
source = "round-v7 team-lead: phase 5.5 plan write, ops f43afc45 and c281646c, roko-main-38bedecea"
discovered_from = "agent:hermes-pilot (round v7, phase 5.5), the lead's ask"
anchors = ["crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation", "crates/roko-serve/src/routes/plans/authoring.rs::derive_unique_slug"]
links = { depends_on = [], blocks = [], related = ["gap-143b5c", "bug-a0f01e", "bug-36ce72"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_second_plan_write_for_a_plan_being_written_is_refused' crates/roko-serve/src && cargo test -p roko-serve --lib a_second_plan_write_for_a_plan_being_written_is_refused"
+++

## Problem

A second plan write for a plan that is still being written starts anyway. Both operations then write the same
`plans/<slug>/`, and whichever finishes last leaves its files there.

Phase 5.5 on roko-main-38bedecea (Undertow d273d60, plan `p5-5-auval-clean`), from the serve's own records in
`~/dev/undertow/.roko`:

- `events.jsonl`: `plan_generate.started` for f43afc45 at 11:02:57.208Z, and for c281646c at 11:03:07.515Z, ten
  seconds later on the same plan id. `plan_generate.completed` for f43afc45 at 11:03:29.551Z and for c281646c at
  11:03:44.902Z.
- `state/operations.jsonl`: both `plan_generate:p5-5-auval-clean`, both `completed`. f43afc45 ran under
  `max_usd` 2.0 (roko's default; the chat named no cap) and cost $0.0374. c281646c ran under `max_usd` 0.5 and cost
  $0.0380.
- The plan's `tasks.toml` and `plan.md` are dated 11:03:44Z, so they are c281646c's. The host validated the plan
  at 11:03:29.6Z and estimated it at 11:03:31Z, on f43afc45's files, which c281646c then replaced at 11:03:44Z. It
  validated again at 11:03:45Z. Those come from `auth-audit.jsonl`: `POST /plans/p5-5-auval-clean/validate` and
  `/estimate`.
- The chat had cancelled f43afc45 before asking again. That cancel did not stop it: no plan-write operation can be
  cancelled (bug-36ce72).

## Why it matters

- **The person can see one plan and run another.** The validation and estimate the host showed described files
  that were replaced 13 seconds later.
- **Both writes are billed**, and neither knows of the other.
- **Any retry hits it.** A host that re-asks while a write is still going (a cap added, a timeout on its side) gets
  this race, not a refusal.
- Goal: golden-path. Related: gap-143b5c (how the slug is chosen), bug-a0f01e (operation records), bug-36ce72 (the
  cancel that didn't stop f43afc45).

## Where

- `crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation` derives the slug, then registers an
  `OperationHandle` in `AppState.operations` under the new operation's id, with kind `plan_generate:<slug>`. It
  never looks for a running operation of that kind. `POST /api/plans/generate` and the MCP `plan_generate` tool
  (`routes/mcp.rs::generate_plan`) both go through it.
- `crates/roko-serve/src/routes/plans/authoring.rs::derive_unique_slug` takes the next free name only by
  `plans_root.join(slug).exists()`. A write in progress has no plan directory until generation writes it at its
  end (`roko-cli`'s `plan_generate/pipeline.rs::generate_plan`), so a second request in that window gets the same
  slug.
- Nothing else claims the plan directory: it is written at the end and overwritten in place.

## Current state

- Operations are keyed by operation id only. The kind string holds the slug but is only read for display.
- Once the first write has made the directory, a later request gets `-2`. The race exists only while a write runs,
  which is 30 to 40 seconds here.

## Plan

- **Recommended: refuse, naming the running operation.** Before deriving the slug, under the operations lock, look
  for a running `plan_generate:<slug>` for the slug the request would get.
  - If there is one, answer 409 (a tool error over MCP): "plan `<slug>` is being written by operation `<id>`: follow
    it with run_status, or cancel it and ask again".
  - Register the new operation under the same lock, so a check and the registration it guards can't interleave.
- **Or queue it.** Start the second write when the first ends, and let it overwrite the plan as the person asked.
  The answer names the operation it waits for.
- Either way, a running write's slug counts as taken in `derive_unique_slug`, so another request that happens to
  derive the same slug gets `-2` rather than a race.
- Test: two `plan_generate` calls for one title while the first is running. The second is refused, naming the
  first's id; the first's files are the plan's.

## Done when

- A second plan write for a slug being written is refused (or queued) with the running operation's id, and never
  writes the plan directory while the first runs.
- The `[[verify]]` passes.

## Notes

- Evidence: `~/dev/undertow/.roko/events.jsonl`, `state/operations.jsonl` and `auth-audit.jsonl`, which records
  `POST /mcp` without the tool's name. The pilot's record of phase 5.5. Undertow is read-only for agents.
- `roko plan generate` run from a shell writes the same directory with no serve involved. It is out of scope here,
  but the same claim could cover it, with a lock file in the plan directory.
