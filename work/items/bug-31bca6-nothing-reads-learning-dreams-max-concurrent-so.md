+++
id = "bug-31bca6"
kind = "bug"
title = "Nothing reads learning.dreams.max_concurrent, so the ACP trigger starts another dream on every turn while one runs"
status = "open"
triage = "unverified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-acp", "roko-dreams"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-acp-dream's report on bug-b16d55)"
anchors = ["crates/roko-acp/src/bridge_events/cost.rs::maybe_spawn_dream_consolidation", "crates/roko-acp/src/bridge_events/cost.rs::acp_dream_due", "crates/roko-core/src/config/learning.rs::DreamsConfig"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-b16d55", "gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn acp_starts_no_dream_while_one_is_running' crates/roko-acp/src/ && cargo test -p roko-acp --lib acp_starts_no_dream_while_one_is_running"
+++

## Problem

`learning.dreams.max_concurrent` (`DreamsConfig` in `crates/roko-core/src/config/learning.rs`) is documented as "Maximum number of concurrent dream consolidation runs. Additional triggers are silently dropped while a run is in progress." No code reads it.

The ACP trigger, `maybe_spawn_dream_consolidation` in `crates/roko-acp/src/bridge_events/cost.rs`, asks `acp_dream_due` whether a dream is due. `acp_dream_due` counts the episodes since the latest dream report, and a report is written only when a dream finishes. So once the threshold is met, every ACP turn spawns another background dream until the first one finishes. `DreamRunner` doesn't guard against overlapping runs either.

## Why it matters

Each dream runs an agent, so overlapping dreams multiply spend and race on `.roko/dreams/`. bug-b16d55 (`c482329be` on `work/bug-b16d55`) made the ACP trigger opt-in (`learning.dreams.trigger_on_acp_episodes`, default false), so today this happens only when someone turns the trigger on. At BASE the trigger is always on. The config doc also promises a guard that doesn't exist; gap-7a3527 tracks other dead keys. Epic spec-9a3131.

## Where

- `crates/roko-acp/src/bridge_events/cost.rs`: `acp_dream_due`, and `maybe_spawn_dream_consolidation`, which does a `tokio::spawn` of `DreamRunner::consolidate_async`.
- `crates/roko-core/src/config/learning.rs`: `DreamsConfig::max_concurrent`.
- `crates/roko-dreams/src/runner.rs::load_latest_dream_report`.

## Current state

On `work/bug-b16d55`, `git grep max_concurrent` finds the field, its default function and its default value, and no reader.

## Plan

1. Guard dream runs. Keep a process-wide semaphore sized by `max_concurrent`:
   - `maybe_spawn_dream_consolidation` takes a permit with `try_acquire`, and skips with a debug log when none is free;
   - the spawned task releases the permit when it ends.

   The CLI and ACP can run dreams in separate processes, so consider a lock file under `.roko/dreams/` as well.
2. Alternatively, delete the key and its doc if one guard is all that's wanted. Either way, the doc must match the code.
3. Check whether the plan-completion trigger has the same gap.
4. Add `acp_starts_no_dream_while_one_is_running`. With the trigger on and the threshold met, a second trigger while the first run holds the guard must spawn nothing.

## Done when

- [ ] While dreams run, further triggers start no more than `max_concurrent` of them.
- [ ] The `[[verify]]` command passes.
