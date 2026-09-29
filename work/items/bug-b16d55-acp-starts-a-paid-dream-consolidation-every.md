+++
id = "bug-b16d55"
kind = "bug"
title = "ACP starts a paid dream consolidation every 10 episodes, and no config flag turns it off"
status = "open"
triage = "unverified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-acp/bridge_events", "roko-dreams"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (14:32, wk-dream-default's report on bug-470de8)"
anchors = ["crates/roko-acp/src/bridge_events/cost.rs::maybe_spawn_dream_consolidation", "crates/roko-acp/src/bridge_events/cost.rs:241", "crates/roko-core/src/config/learning.rs::DreamsConfig"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-470de8", "dec-e70592"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn acp_episodes_start_no_dream_when_dreams_are_off' crates/roko-acp/src/ && cargo test -p roko-acp --lib acp_episodes_start_no_dream_when_dreams_are_off"
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
