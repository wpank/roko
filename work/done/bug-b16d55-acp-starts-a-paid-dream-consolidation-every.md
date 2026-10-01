+++
id = "bug-b16d55"
kind = "bug"
title = "ACP starts a paid dream consolidation every 10 episodes, and no config flag turns it off"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-acp/bridge_events", "roko-dreams"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "584abd414"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (14:32, wk-dream-default's report on bug-470de8)"
anchors = ["crates/roko-acp/src/bridge_events/cost.rs::maybe_spawn_dream_consolidation", "crates/roko-acp/src/bridge_events/cost.rs:241", "crates/roko-core/src/config/learning.rs::DreamsConfig"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-470de8", "dec-e70592"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn acp_episodes_start_no_dream_when_dreams_are_off' crates/roko-acp/src/ && cargo test -p roko-acp --lib acp_episodes_start_no_dream_when_dreams_are_off"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "ACP no longer starts paid dreams by default: learning.dreams.trigger_on_acp_episodes (default false) and acp_episode_threshold gate maybe_spawn_dream_consolidation via acp_dream_due; tests acp_episodes_start_no_dream_when_dreams_are_off and acp_dream_trigger_defaults_to_off; docs [learning.dreams] table (c482329be, merged 584abd414). Batch 2 gate (work/rust-batch-2; crates tree identical to MAIN after the merges): cargo check --workspace --tests clean; nightly rustfmt clean; clippy -p roko-cli -p roko-graph -p roko-execution -p roko-core -p roko-acp -p roko-learn --no-deps -D warnings clean; lib tests roko-cli 3060 passed (8 threads; two timing tests flaked only under full parallel load and pass alone), roko-graph 460, roko-execution 252, roko-core 1913, roko-acp 196, roko-learn 1166."
+++

## Problem

After every ACP turn, the episode sink appends the episode to `.roko/episodes.jsonl` and then calls
`maybe_spawn_dream_consolidation(workdir, roko_config)` (`bridge_events/cost.rs:241`). That function:

1. counts the episodes recorded since the last dream report, or every line of the log when there is no report yet;
2. at `DREAM_EPISODE_THRESHOLD = 10` (`cost.rs:246`), spawns `DreamRunner::consolidate_async` with a hard-coded
   `DreamLoopConfig { auto_dream: true, idle_threshold_mins: 0, min_episodes_for_dream: 1, .. }` and a `claude` CLI
   agent on `agent.default_model` (`cost.rs:292-322`).

It reads no config flag at all: not `learning.dream_on_completion`, not `learning.dreams.trigger_on_plan_complete`,
and no ACP-specific key. The count uses the shared episode log, so plan-run episodes count too: in a workspace with
10 or more recorded episodes and no dream report, the first ACP turn starts a dream.

## Why it matters

Each dream is a model run that nobody reads. Research note B6 found the immune boundary denied all 56 LLM
distillations of the last dream run. tldr/05 P0 #6 and its decision 12 park dreams, and bug-470de8 turns
`dream_on_completion` off by default for plan runs. That fix does not reach ACP, so after it lands every ACP session
(the editor and Hermes surface) still pays for a dream every 10 episodes, and an operator has no key to stop it. Part of
epic spec-9a3131.

## Where

- `crates/roko-acp/src/bridge_events/cost.rs::maybe_spawn_dream_consolidation` (`:251`) and its call at `:241`, at the
  end of the ACP episode sink.
- `crates/roko-core/src/config/learning.rs::DreamsConfig` (`:17-44`) and `LearningConfig::dream_on_completion`: the
  existing dream switches, which only the plan-completion sink reads (`runtime_feedback/plan_completion.rs:85`).

## Current state

Checked at `4c0326dfc`: as described. bug-470de8's branch (`work/bug-470de8` at `698111a61`, waiting for the batch
cargo check) changes `learning.rs`, `presets.rs`, `plan_completion.rs` and `roko.toml`, but not `roko-acp`.

## Plan

1. Put the ACP trigger behind config, off by default. Two options:
   - **a new key**, for example `learning.dreams.trigger_on_acp_episodes = false` with a threshold field (default 10),
     next to `trigger_on_plan_complete`; recommended, because the plan-completion switches mean something else;
   - or reuse `learning.dream_on_completion && learning.dreams.trigger_on_plan_complete`.
2. Split the decision into a pure helper (for example `should_spawn_acp_dream(&RokoConfig, episodes_since) -> bool`)
   and return before reading the episode log when dreams are off.
3. Add the test `acp_episodes_start_no_dream_when_dreams_are_off`: 12 episodes, no dream report and the default
   config start no dream; the opt-in key with threshold 10 does.

## Done when

- [ ] With the default config, an ACP session never starts a dream consolidation, however many episodes it records.
- [ ] An explicit opt-in key restores today's behaviour, with its threshold read from config.
- [ ] Test `acp_episodes_start_no_dream_when_dreams_are_off` exists and passes, and the `[[verify]]` command passes.

## Notes

- `learning.rs` is also edited by bug-470de8. If the new key goes in `DreamsConfig`, merge after bug-470de8.
- The same sink also starts an LLM distillation of every ACP episode (`roko_neuro::spawn_episode_distillation` with a
  model caller, `cost.rs:228-237`). Whether that should sit behind a flag too is a separate question and is not
  in this item.
- `roko-acp` is not a hot file (lane `rust-cold`).
- Premise check at `594e58f93`: as described. bug-470de8 (merged at `d5b17759b`) changed no `roko-acp` code, so
  the ACP trigger still read no config.
- Chose the recommended new keys: `learning.dreams.trigger_on_acp_episodes` (serde default `false`) and
  `learning.dreams.acp_episode_threshold` (default 10, zero treated as one). `roko.toml` is left unchanged on
  purpose: `DreamsConfig` denies unknown fields, so a binary built before this change would reject a `roko.toml`
  that names the new keys, and the default is already off.
- Implemented on `work/bug-b16d55` at `c482329be`; cargo verification deferred to the batch check.
