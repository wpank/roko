+++
id = "bug-9434c4"
kind = "bug"
title = "roko config set rejects learning.t0_reflexes and every learning.dreams key"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-gtd-split's report on bug-94151f)"
anchors = ["crates/roko-cli/src/config.rs::parse_value_for_key", "crates/roko-core/src/config/learning.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = ["bug-94151f", "bug-b16d55"], blocks = [], related = ["gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn config_set_accepts_learning_opt_in_keys' crates/roko-cli/src/ && cargo test -p roko-cli --lib config_set_accepts_learning_opt_in_keys"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:58Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:13:54Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Already fixed at BASE ebdc0f5d5; the item's 2026-10-01 note gives the evidence."
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
- 2026-10-01 (wk-cfg): implemented on work/bug-ccfa0d; cargo verification deferred to the batch check.
  The behaviour was already fixed at BASE `ebdc0f5d5`: `learning.t0_reflexes` and the four `learning.dreams` fields
  are non-`Option` (`crates/roko-core/src/config/learning.rs:26-50`, `:151`), so they are in the schema tree, and
  `parse_value_for_key` falls back to `parse_value_from_schema` (`crates/roko-cli/src/config.rs:1752`, added by
  3bf3af074, merged in b351d2be5). Plan step 2 is that fallback. Only the regression test was missing: added
  `config_set_accepts_learning_opt_in_keys` in `crates/roko-cli/src/config.rs`, which sets all five keys through
  `set_toml_dotted_key`, checks validation accepts them, and loads them into `RokoConfig`.
