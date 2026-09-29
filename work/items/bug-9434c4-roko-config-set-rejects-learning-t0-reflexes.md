+++
id = "bug-9434c4"
kind = "bug"
title = "roko config set rejects learning.t0_reflexes and every learning.dreams key"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-gtd-split's report on bug-94151f)"
anchors = ["crates/roko-cli/src/config.rs::parse_value_for_key", "crates/roko-core/src/config/learning.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = ["bug-94151f", "bug-b16d55"], blocks = [], related = ["gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn config_set_accepts_learning_opt_in_keys' crates/roko-cli/src/ && cargo test -p roko-cli --lib config_set_accepts_learning_opt_in_keys"
+++

## Problem

Two branches added opt-in learning flags:

- bug-94151f (`9a4d8db5c` on `work/bug-94151f`) added `[learning] t0_reflexes` (bool, default false), which turns on the T0 reflex path;
- bug-b16d55 added `learning.dreams.trigger_on_acp_episodes` and `learning.dreams.acp_episode_threshold`.

`roko config set learning.t0_reflexes true` fails with `unknown key`. `config set` goes through `set_toml_dotted_key` to `parse_value_for_key` in `crates/roko-cli/src/config.rs`, which is a hand-written list of the settable keys and their types, and none of these keys is on it. No `learning.dreams.*` key is on it at all: the `["dreams", …]` entries are for the top-level `[dreams]` table.

## Why it matters

These flags are how you turn an opt-in learning feature on for an experiment, and `config set` is the documented way to change config. Epic spec-9a3131.

## Where

- `crates/roko-cli/src/config.rs::parse_value_for_key`: a match over the key's segments, whose catch-all arm returns `unknown key: {key}`.
- `crates/roko-core/src/config/learning.rs`: `LearningConfig::t0_reflexes` and `DreamsConfig`.

## Current state

Each new config field needs its own arm in this list, and new fields keep missing it.

## Plan

1. Add `learning.t0_reflexes` and the `learning.dreams` keys, each with its type: `trigger_on_plan_complete`, `trigger_on_acp_episodes`, `acp_episode_threshold` and `max_concurrent`, whichever exist when this lands.
2. Better, if it is small: derive the settable keys and their types from the serialized default `RokoConfig`, as the test helper `known_config_keys` already does for top-level keys. New fields would then need no list edit. If you don't do this here, file it as its own item.
3. Add `config_set_accepts_learning_opt_in_keys`: `parse_value_for_key("learning.t0_reflexes", "true")` and `parse_value_for_key("learning.dreams.trigger_on_acp_episodes", "true")` both return `true`.

## Done when

- [ ] `roko config set learning.t0_reflexes true` writes the key.
- [ ] The `[[verify]]` command passes.

## Notes

- Depends on bug-94151f and bug-b16d55, which add the keys. They are not on BASE yet.
- **2026-09-29 (wk-filer4):** fixed in substance by 3bf3af074: `learning.t0_reflexes` and the four `learning.dreams` fields are non-Option, so they serialize into the schema tree, and `config set` accepts every schema key since bug-1c93b4 (b351d2be5). The verify names `config_set_accepts_learning_opt_in_keys`, which doesn't exist, so closing needs that test or a re-pointed verify.
