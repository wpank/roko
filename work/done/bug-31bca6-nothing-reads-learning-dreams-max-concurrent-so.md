+++
id = "bug-31bca6"
kind = "bug"
title = "Nothing reads learning.dreams.max_concurrent, so the ACP trigger starts another dream on every turn while one runs"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-acp", "roko-dreams"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-acp-dream's report on bug-b16d55)"
anchors = ["crates/roko-acp/src/bridge_events/cost.rs::maybe_spawn_dream_consolidation", "crates/roko-acp/src/bridge_events/cost.rs::acp_dream_due", "crates/roko-core/src/config/learning.rs::DreamsConfig"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-b16d55", "gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn acp_starts_no_dream_while_one_is_running' crates/roko-acp/src/ && cargo test -p roko-acp --lib acp_starts_no_dream_while_one_is_running"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:12Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:13:54Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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

## Notes

- 2026-10-01 (wk-cfg): implemented on work/bug-ccfa0d; cargo verification deferred to the batch check.
  Premise checked at BASE `ebdc0f5d5`: the ACP trigger is opt-in there (bug-b16d55 merged), and nothing read
  `max_concurrent`. `crates/roko-acp/src/bridge_events/cost.rs` now counts running ACP dreams in a process-wide
  `DreamSlots` (an atomic counter, not a semaphore, so a config reload that changes the limit applies at once).
  `claim_acp_dream` takes a slot under `DreamsConfig::effective_max_concurrent()` (new; 0 counts as 1) or skips
  with a debug log, and the spawned dream drops its `DreamSlot` when it ends. Test
  `acp_starts_no_dream_while_one_is_running`. The `max_concurrent` doc in `learning.rs` and
  `docs/v3/depth/21-config/01-schema-sections.md` now say what the code does.
- Plan step 3: the plan-completion trigger (`DreamConsolidationSink` in
  `crates/roko-cli/src/runtime_feedback/plan_completion.rs`) already guards with its own `running` flag, so it runs
  one dream at a time and ignores `max_concurrent`; the doc says so. Not changed here (outside this packet's files).
- Left out: a cross-process guard. Two `roko acp` processes on one workspace can each run `max_concurrent` dreams.
  A lock file under `.roko/dreams/` needs `fs2` in roko-acp or roko-dreams, a new dependency that changes
  `Cargo.lock`, which a static-only round can't regenerate.
